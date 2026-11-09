//! Proof verification against a trusted root.
//!
//! Everything the proof claims about itself is untrusted. The verifier fixes the parameters from
//! its own configuration before running the STARK checks.

use std::{fmt, panic};

use winter_prover::{
    proof::Context, ByteReader, Deserializable, DeserializationError, SliceReader,
};
use winterfell::{
    crypto::{DefaultRandomCoin, MerkleTree},
    math::fields::f128::BaseElement,
    AcceptableOptions, Proof, ProofOptions, VerifierError,
};

use crate::{
    air::{trace_length, MerkleAir, PublicInputs, SessionId, TRACE_WIDTH},
    config::{CommitmentHasher, ProverMode},
};

const DIGEST_BYTES: usize = if cfg!(feature = "blake3-192") { 24 } else { 32 };

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// The bytes do not decode to a well-formed proof.
    Malformed(String),
    /// The proof was generated with parameters other than this mode's.
    UnexpectedParameters,
    /// The proof's trace shape does not match the tree depth of the registry.
    UnexpectedTraceShape {
        expected_length: usize,
        length: usize,
        width: usize,
    },
    /// The STARK checks failed, for example because the root does not match.
    Rejected(VerifierError),
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(reason) => write!(f, "malformed proof: {reason}"),
            Self::UnexpectedParameters => {
                write!(f, "proof parameters do not match the selected mode")
            }
            Self::UnexpectedTraceShape {
                expected_length,
                length,
                width,
            } => write!(
                f,
                "proof has a trace of {length} rows and {width} columns; this tree depth requires \
                 {expected_length} rows and {TRACE_WIDTH} columns"
            ),
            Self::Rejected(error) => write!(f, "proof rejected: {error}"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Verifies a decoded proof against a trusted root, for a registry of the given tree depth.
///
/// Both parameter checks are necessary for soundness:
/// - Taking the acceptable options from the proof itself would let a forger select one query and
///   next to no security, so they come from `mode`.
/// - Depth is not part of the AIR. A shorter trace would prove a different statement about the
///   root than membership of a leaf at the real depth.
pub fn verify(
    proof: Proof,
    root: [BaseElement; 2],
    mode: ProverMode,
    tree_depth: usize,
    session: SessionId,
) -> Result<(), VerifyError> {
    verify_with_options(
        proof,
        root,
        mode.proof_options(tree_depth),
        tree_depth,
        session,
    )
}

/// [`verify`] with explicit protocol parameters, for configuration experiments.
pub fn verify_with_options(
    proof: Proof,
    root: [BaseElement; 2],
    expected_options: ProofOptions,
    tree_depth: usize,
    session: SessionId,
) -> Result<(), VerifyError> {
    if *proof.options() != expected_options {
        return Err(VerifyError::UnexpectedParameters);
    }

    // Without FRI folding layers Winterfell never reads this field, so check it here: otherwise
    // one proof would have many valid encodings. This prover always uses a single partition.
    if proof.fri_proof.num_partitions() != 1 {
        return Err(VerifyError::UnexpectedParameters);
    }

    let info = proof.trace_info();
    let expected_length = trace_length(tree_depth);
    if info.length() != expected_length || info.width() != TRACE_WIDTH || info.is_multi_segment() {
        return Err(VerifyError::UnexpectedTraceShape {
            expected_length,
            length: info.length(),
            width: info.width(),
        });
    }

    winterfell::verify::<
        MerkleAir,
        CommitmentHasher,
        DefaultRandomCoin<CommitmentHasher>,
        MerkleTree<CommitmentHasher>,
    >(
        proof,
        PublicInputs {
            tree_root: root,
            session,
        },
        &AcceptableOptions::OptionSet(vec![expected_options]),
    )
    .map_err(VerifyError::Rejected)
}

/// Decodes and verifies proof bytes received from an untrusted party.
///
/// Only the canonical encoding is accepted, because the decoder tolerates trailing bytes, and a
/// proof should have one byte representation. A panic inside the proof library on hostile input is
/// reported as a failed verification.
pub fn verify_bytes(
    bytes: &[u8],
    root: [BaseElement; 2],
    mode: ProverMode,
    tree_depth: usize,
    session: SessionId,
) -> Result<(), VerifyError> {
    panic::catch_unwind(|| {
        check_declared_lengths(bytes).map_err(|error| VerifyError::Malformed(error.to_string()))?;
        let proof =
            Proof::from_bytes(bytes).map_err(|error| VerifyError::Malformed(error.to_string()))?;
        if proof.to_bytes() != bytes {
            return Err(VerifyError::Malformed("non-canonical encoding".into()));
        }
        verify(proof, root, mode, tree_depth, session)
    })
    .unwrap_or_else(|_| Err(VerifyError::Malformed("proof library panicked".into())))
}

/// Walks the serialized proof, and checks every length prefix against the bytes actually present.
///
/// Winterfell's decoder calls `Vec::with_capacity(declared_length)` before any bounds check for
/// query values, opening paths, and the Merkle nodes inside them. One flipped bit there requests
/// gigabytes, and a failed allocation aborts the process, which `catch_unwind` cannot stop.
fn check_declared_lengths(bytes: &[u8]) -> Result<(), DeserializationError> {
    let mut reader = SliceReader::new(bytes);
    let context = Context::read_from(&mut reader)?;
    if context.trace_info().num_segments() != 1 {
        return Err(DeserializationError::InvalidValue(
            "the Merkle AIR has one trace segment".into(),
        ));
    }
    reader.read_u8()?; // number of unique queries
    skip_short_field(&mut reader)?; // commitments

    for _ in 0..2 {
        // Trace queries, then constraint queries.
        let values = reader.read_usize()?;
        reader.read_slice(values)?;
        let paths = reader.read_usize()?;
        check_opening_proof(reader.read_slice(paths)?)?;
    }

    skip_short_field(&mut reader)?; // out-of-domain trace states
    skip_short_field(&mut reader)?; // out-of-domain constraint evaluations

    for _ in 0..reader.read_u8()? {
        // FRI layers.
        let values = reader.read_u32()? as usize;
        reader.read_slice(values)?;
        let paths = reader.read_u32()? as usize;
        check_opening_proof(reader.read_slice(paths)?)?;
    }
    Ok(())
}

fn skip_short_field(reader: &mut SliceReader<'_>) -> Result<(), DeserializationError> {
    let length = reader.read_u16()? as usize;
    reader.read_slice(length)?;
    Ok(())
}

/// Checks a serialized `BatchMerkleProof`: a depth byte, then vectors of digests.
fn check_opening_proof(bytes: &[u8]) -> Result<(), DeserializationError> {
    let too_long = || DeserializationError::InvalidValue("declared length exceeds proof".into());
    let mut reader = SliceReader::new(bytes);
    reader.read_u8()?;
    let vectors = reader.read_usize()?;
    if vectors > bytes.len() {
        return Err(too_long());
    }
    for _ in 0..vectors {
        let digests = reader.read_usize()?;
        let digest_bytes = digests.checked_mul(DIGEST_BYTES).ok_or_else(too_long)?;
        reader.read_slice(digest_bytes)?;
    }
    Ok(())
}
