//! The zero-knowledge verifier.

use std::{fmt, panic};

use winterfell::{
    crypto::{Hasher, MerkleTree},
    math::{batch_inversion, fields::f128::BaseElement, FieldElement, StarkField},
    Air, ConstraintCompositionCoefficients, EvaluationFrame, TraceInfo,
};

use super::{
    constraints::quotient_at, evaluate, proof::ZkOpenings, public_inputs, Transcript, ZkParameters,
    ZkProof, COMMITTED_TRACE_WIDTH, MASK_COLUMN,
};
use crate::{
    air::{MerkleAir, SessionId, TRACE_WIDTH},
    config::CommitmentHasher,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZkError {
    UnsupportedDepth(usize),
    Malformed(String),
    /// The quotient chunks do not recompose to the constraint evaluation at `z`.
    InconsistentQuotient,
    /// The opened rows do not match their Merkle commitment.
    BadOpening,
    /// The batched polynomial disagrees with the openings at a query position.
    BadLowDegreeCheck,
}

impl fmt::Display for ZkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDepth(depth) => {
                write!(
                    f,
                    "zero-knowledge proofs are not available at depth {depth}"
                )
            }
            Self::Malformed(reason) => write!(f, "malformed proof: {reason}"),
            Self::InconsistentQuotient => {
                write!(
                    f,
                    "proof rejected: constraints do not hold at the challenge point"
                )
            }
            Self::BadOpening => write!(f, "proof rejected: invalid Merkle opening"),
            Self::BadLowDegreeCheck => write!(f, "proof rejected: failed the low-degree check"),
        }
    }
}

impl std::error::Error for ZkError {}

/// Decodes and verifies proof bytes received from an untrusted party.
pub fn verify_zk_bytes(
    bytes: &[u8],
    root: [BaseElement; 2],
    tree_depth: usize,
    session: SessionId,
) -> Result<(), ZkError> {
    let parameters =
        ZkParameters::for_depth(tree_depth).ok_or(ZkError::UnsupportedDepth(tree_depth))?;
    panic::catch_unwind(|| {
        let proof = ZkProof::from_bytes(bytes, &parameters)
            .map_err(|error| ZkError::Malformed(error.to_string()))?;
        if proof.to_bytes() != bytes {
            return Err(ZkError::Malformed("non-canonical encoding".into()));
        }
        verify_zk(&proof, root, &parameters, session)
    })
    .unwrap_or_else(|_| Err(ZkError::Malformed("verification panicked".into())))
}

/// The challenges the verifier derives from a proof's transcript.
pub struct Challenges {
    pub coefficients: ConstraintCompositionCoefficients<BaseElement>,
    pub z: BaseElement,
    pub trace_weights: Vec<BaseElement>,
    pub chunk_weights: Vec<BaseElement>,
    pub positions: Vec<usize>,
}

/// Replays the Fiat-Shamir transcript of a proof.
pub fn replay_transcript(
    proof: &ZkProof,
    root: [BaseElement; 2],
    parameters: &ZkParameters,
    session: SessionId,
) -> Result<(MerkleAir, Challenges), ZkError> {
    let air = air_for(root, parameters, session);
    let mut transcript = Transcript::new(&public_inputs(root, session), parameters);
    let trace_root = read_digest(&proof.trace_root)?;
    let composition_root = read_digest(&proof.composition_root)?;

    transcript.absorb_digest(trace_root);
    let coefficients = transcript.constraint_coefficients(&air);
    transcript.absorb_digest(composition_root);
    let z = transcript.element();
    let openings = &proof.openings;
    let mut ood = openings.ood_trace.clone();
    ood.extend_from_slice(&openings.ood_chunks);
    transcript.absorb_elements(&ood);
    let trace_weights = transcript.elements(COMMITTED_TRACE_WIDTH);
    let chunk_weights = transcript.elements(parameters.num_chunks);
    transcript.absorb_elements(&openings.batched);
    let positions = transcript.positions(parameters);
    Ok((
        air,
        Challenges {
            coefficients,
            z,
            trace_weights,
            chunk_weights,
            positions,
        },
    ))
}

/// Verifies a decoded proof against a trusted root, for a registry of the depth in `parameters`.
pub fn verify_zk(
    proof: &ZkProof,
    root: [BaseElement; 2],
    parameters: &ZkParameters,
    session: SessionId,
) -> Result<(), ZkError> {
    if usize::from(proof.tree_depth) != parameters.tree_depth {
        return Err(ZkError::Malformed(
            "proof is for a different tree depth".into(),
        ));
    }
    let (air, challenges) = replay_transcript(proof, root, parameters, session)?;
    if challenges.positions.len() != usize::from(proof.num_unique_queries) {
        return Err(ZkError::Malformed(
            "query count disagrees with the transcript".into(),
        ));
    }

    check_merkle(
        &proof.trace_root,
        &proof.trace_opening,
        &proof.openings.trace_rows,
        COMMITTED_TRACE_WIDTH,
        &challenges.positions,
    )?;
    check_merkle(
        &proof.composition_root,
        &proof.composition_opening,
        &proof.openings.composition_rows,
        parameters.num_chunks + 1,
        &challenges.positions,
    )?;
    check_openings(&air, parameters, &challenges, &proof.openings)
}

/// The algebraic checks of the protocol. The simulator's output passes exactly these.
pub fn check_openings(
    air: &MerkleAir,
    parameters: &ZkParameters,
    challenges: &Challenges,
    openings: &ZkOpenings,
) -> Result<(), ZkError> {
    let n = parameters.trace_length;
    let generator = BaseElement::get_root_of_unity(n.ilog2());
    let z = challenges.z;
    let shifted_z = z * generator;

    // Out-of-domain check: the chunks must recompose to the constraint quotient at z, plus m(gz).
    let (ood_current, ood_next) = openings.ood_trace.split_at(COMMITTED_TRACE_WIDTH);
    let frame = EvaluationFrame::from_rows(
        ood_current[..TRACE_WIDTH].to_vec(),
        ood_next[..TRACE_WIDTH].to_vec(),
    );
    let z_to_n = z.exp_vartime((n as u64).into());
    if z_to_n == BaseElement::ONE {
        return Err(ZkError::InconsistentQuotient); // z lies in the trace domain
    }
    let expected = quotient_at(air, &challenges.coefficients, &frame, z) + ood_next[MASK_COLUMN];
    let recomposed = openings
        .ood_chunks
        .iter()
        .rev()
        .fold(BaseElement::ZERO, |sum, &chunk| sum * z_to_n + chunk);
    if expected != recomposed {
        return Err(ZkError::InconsistentQuotient);
    }

    // Low-degree check: f must agree with the batched openings at every query position.
    let domain_generator = BaseElement::get_root_of_unity(parameters.domain_size.ilog2());
    let offset = air.domain_offset();
    let points: Vec<BaseElement> = challenges
        .positions
        .iter()
        .map(|&position| offset * domain_generator.exp_vartime((position as u64).into()))
        .collect();
    let denominators: Vec<BaseElement> = points
        .iter()
        .flat_map(|&x| [x - z, x - shifted_z])
        .collect();
    let inverses = batch_inversion(&denominators);

    let chunk_columns = parameters.num_chunks;
    for (index, &x) in points.iter().enumerate() {
        let trace_row =
            &openings.trace_rows[index * COMMITTED_TRACE_WIDTH..][..COMMITTED_TRACE_WIDTH];
        let composition_row =
            &openings.composition_rows[index * (chunk_columns + 1)..][..chunk_columns + 1];
        let (over_z, over_shifted) = (inverses[2 * index], inverses[2 * index + 1]);

        let mut batched = composition_row[chunk_columns];
        for (i, &value) in trace_row.iter().enumerate() {
            let weight = challenges.trace_weights[i];
            batched += (value - ood_current[i]) * weight * over_z
                + (value - ood_next[i]) * weight * over_shifted;
        }
        for (j, &value) in composition_row[..chunk_columns].iter().enumerate() {
            batched += (value - openings.ood_chunks[j]) * challenges.chunk_weights[j] * over_z;
        }
        if batched != evaluate(&openings.batched, x) {
            return Err(ZkError::BadLowDegreeCheck);
        }
    }
    Ok(())
}

pub(crate) fn air_for(
    root: [BaseElement; 2],
    parameters: &ZkParameters,
    session: SessionId,
) -> MerkleAir {
    MerkleAir::new(
        TraceInfo::new(TRACE_WIDTH, parameters.trace_length),
        public_inputs(root, session),
        parameters.proof_options(),
    )
}

fn read_digest(bytes: &[u8; 32]) -> Result<<CommitmentHasher as Hasher>::Digest, ZkError> {
    use winter_prover::{Deserializable, SliceReader};
    let mut reader = SliceReader::new(bytes);
    <CommitmentHasher as Hasher>::Digest::read_from(&mut reader)
        .map_err(|_| ZkError::Malformed("bad digest".into()))
}

fn check_merkle(
    root: &[u8; 32],
    opening: &winterfell::crypto::BatchMerkleProof<CommitmentHasher>,
    rows: &[BaseElement],
    width: usize,
    positions: &[usize],
) -> Result<(), ZkError> {
    use winterfell::crypto::ElementHasher;
    let leaves: Vec<_> = rows
        .chunks_exact(width)
        .map(CommitmentHasher::hash_elements)
        .collect();
    let root = read_digest(root)?;
    MerkleTree::<CommitmentHasher>::verify_batch(&root, positions, &leaves, opening)
        .map_err(|_| ZkError::BadOpening)
}
