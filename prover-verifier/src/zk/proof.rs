//! The zero-knowledge proof object and its canonical encoding.
//!
//! Every field has a length fixed by the tree depth, except for the two Merkle opening proofs,
//! whose node counts depend on the query positions. Decoding checks each length against what is
//! present, so a corrupted proof cannot request a large allocation.

use std::fmt;

use winter_prover::{ByteReader, Deserializable, Serializable, SliceReader};
use winterfell::{
    crypto::BatchMerkleProof,
    math::{fields::f128::BaseElement, StarkField},
};

use super::{ZkParameters, COMMITTED_TRACE_WIDTH};
use crate::config::CommitmentHasher;

const ELEMENT_BYTES: usize = 16;
const DIGEST_BYTES: usize = 32;
const FORMAT_VERSION: u8 = 1;

/// Everything in a proof apart from commitments and Merkle openings. The simulator produces this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZkOpenings {
    /// Committed trace columns at `z`, then at `g z`.
    pub ood_trace: Vec<BaseElement>,
    /// Quotient chunks at `z`.
    pub ood_chunks: Vec<BaseElement>,
    /// The batched polynomial, in full: coefficients from low to high degree.
    pub batched: Vec<BaseElement>,
    /// Committed trace rows at the query positions.
    pub trace_rows: Vec<BaseElement>,
    /// Chunk rows at the query positions, each followed by the batching mask `R`.
    pub composition_rows: Vec<BaseElement>,
}

pub struct ZkProof {
    pub tree_depth: u8,
    pub num_unique_queries: u8,
    pub trace_root: [u8; DIGEST_BYTES],
    pub composition_root: [u8; DIGEST_BYTES],
    pub openings: ZkOpenings,
    pub trace_opening: BatchMerkleProof<CommitmentHasher>,
    pub composition_opening: BatchMerkleProof<CommitmentHasher>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub &'static str);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl ZkProof {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(FORMAT_VERSION);
        out.push(self.tree_depth);
        out.push(self.num_unique_queries);
        out.extend_from_slice(&self.trace_root);
        out.extend_from_slice(&self.composition_root);
        for section in [
            &self.openings.ood_trace,
            &self.openings.ood_chunks,
            &self.openings.batched,
            &self.openings.trace_rows,
            &self.openings.composition_rows,
        ] {
            for element in section {
                out.extend_from_slice(&element.as_int().to_le_bytes());
            }
        }
        for opening in [&self.trace_opening, &self.composition_opening] {
            let bytes = opening.to_bytes();
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        out
    }

    /// Decodes a proof for the given parameters, accepting only the canonical encoding.
    pub fn from_bytes(bytes: &[u8], parameters: &ZkParameters) -> Result<Self, DecodeError> {
        let mut reader = SliceReader::new(bytes);
        let truncated = DecodeError("proof is truncated");
        if reader.read_u8().map_err(|_| truncated.clone())? != FORMAT_VERSION {
            return Err(DecodeError("unknown proof format"));
        }
        let tree_depth = reader.read_u8().map_err(|_| truncated.clone())?;
        if usize::from(tree_depth) != parameters.tree_depth {
            return Err(DecodeError("proof is for a different tree depth"));
        }
        let num_unique_queries = reader.read_u8().map_err(|_| truncated.clone())?;
        let unique = usize::from(num_unique_queries);
        if unique == 0 || unique > parameters.num_queries {
            return Err(DecodeError("invalid query count"));
        }
        let mut digest = || -> Result<[u8; DIGEST_BYTES], DecodeError> {
            let slice = reader
                .read_slice(DIGEST_BYTES)
                .map_err(|_| DecodeError("proof is truncated"))?;
            Ok(slice.try_into().expect("slice has digest length"))
        };
        let trace_root = digest()?;
        let composition_root = digest()?;

        let mut elements = |count: usize| -> Result<Vec<BaseElement>, DecodeError> {
            let slice = reader
                .read_slice(count * ELEMENT_BYTES)
                .map_err(|_| DecodeError("proof is truncated"))?;
            slice
                .chunks_exact(ELEMENT_BYTES)
                .map(|chunk| {
                    let value = u128::from_le_bytes(chunk.try_into().expect("16-byte chunk"));
                    if value < BaseElement::MODULUS {
                        Ok(BaseElement::new(value))
                    } else {
                        Err(DecodeError("non-canonical field element"))
                    }
                })
                .collect()
        };
        let ood_trace = elements(COMMITTED_TRACE_WIDTH * 2)?;
        let ood_chunks = elements(parameters.num_chunks)?;
        let batched = elements(parameters.batched_length)?;
        let trace_rows = elements(COMMITTED_TRACE_WIDTH * unique)?;
        let composition_rows = elements((parameters.num_chunks + 1) * unique)?;

        let mut opening = || -> Result<BatchMerkleProof<CommitmentHasher>, DecodeError> {
            let length = reader
                .read_u32()
                .map_err(|_| DecodeError("proof is truncated"))? as usize;
            let slice = reader
                .read_slice(length)
                .map_err(|_| DecodeError("proof is truncated"))?;
            check_opening_lengths(slice)?;
            let mut inner = SliceReader::new(slice);
            let parsed = BatchMerkleProof::read_from(&mut inner)
                .map_err(|_| DecodeError("malformed Merkle opening"))?;
            if inner.has_more_bytes() {
                return Err(DecodeError("trailing bytes in Merkle opening"));
            }
            Ok(parsed)
        };
        let trace_opening = opening()?;
        let composition_opening = opening()?;
        if reader.has_more_bytes() {
            return Err(DecodeError("trailing bytes after proof"));
        }

        let expected_depth = parameters.domain_size.ilog2() as u8;
        if trace_opening.depth != expected_depth || composition_opening.depth != expected_depth {
            return Err(DecodeError("Merkle opening has the wrong depth"));
        }
        Ok(Self {
            tree_depth,
            num_unique_queries,
            trace_root,
            composition_root,
            openings: ZkOpenings {
                ood_trace,
                ood_chunks,
                batched,
                trace_rows,
                composition_rows,
            },
            trace_opening,
            composition_opening,
        })
    }
}

/// Winterfell's `BatchMerkleProof` decoder allocates declared lengths before checking them.
fn check_opening_lengths(bytes: &[u8]) -> Result<(), DecodeError> {
    let error = DecodeError("malformed Merkle opening");
    let mut reader = SliceReader::new(bytes);
    reader.read_u8().map_err(|_| error.clone())?;
    let vectors = reader.read_usize().map_err(|_| error.clone())?;
    if vectors > bytes.len() {
        return Err(error);
    }
    for _ in 0..vectors {
        let digests = reader.read_usize().map_err(|_| error.clone())?;
        let total = digests.checked_mul(DIGEST_BYTES).ok_or(error.clone())?;
        reader.read_slice(total).map_err(|_| error.clone())?;
    }
    Ok(())
}
