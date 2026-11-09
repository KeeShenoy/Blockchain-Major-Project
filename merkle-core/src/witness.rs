//! JSON witness and root encodings, plus validation of untrusted witness input.

use std::fmt;

use serde::{Deserialize, Serialize};
use winterfell::math::{fields::f128::BaseElement, StarkField};

use crate::{root_from_path, Hash};

/// Largest supported tree depth. The leaf index is a `usize`, so its bits must cover the path, and
/// the trace length `8 * (depth + 1)` must be a power of two.
pub const MAX_TREE_DEPTH: usize = 63;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessError {
    /// Depth is zero, too large, or does not give a power-of-two trace length.
    InvalidDepth(usize),
    /// A value is not a decimal integer below the field modulus. Larger values must not be reduced
    /// silently: two different encodings would then denote the same root or credential.
    NonCanonicalElement(&'static str),
    IndexOutOfRange {
        index: usize,
        depth: usize,
    },
}

impl fmt::Display for WitnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDepth(depth) => write!(
                f,
                "tree depth {depth} is unsupported: depth + 1 must be a power of two \
                 (1, 3, 7, 15, 31 or 63)"
            ),
            Self::NonCanonicalElement(field) => {
                write!(f, "{field} is not a canonical field element")
            }
            Self::IndexOutOfRange { index, depth } => {
                write!(f, "leaf index {index} does not fit a tree of depth {depth}")
            }
        }
    }
}

impl std::error::Error for WitnessError {}

/// Checks that a tree of this depth has a valid execution trace length.
pub fn validate_depth(depth: usize) -> Result<(), WitnessError> {
    if depth == 0 || depth > MAX_TREE_DEPTH || !(depth + 1).is_power_of_two() {
        return Err(WitnessError::InvalidDepth(depth));
    }
    Ok(())
}

/// Parses a decimal string into a field element, rejecting values at or above the modulus.
pub fn parse_element(value: &str, field: &'static str) -> Result<BaseElement, WitnessError> {
    match value.parse::<u128>() {
        Ok(parsed) if parsed < BaseElement::MODULUS => Ok(BaseElement::new(parsed)),
        _ => Err(WitnessError::NonCanonicalElement(field)),
    }
}

/// Public root file written by the issuer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootFile {
    pub root_limb0: String,
    pub root_limb1: String,
}

impl RootFile {
    pub fn from_elements(root: [BaseElement; 2]) -> Self {
        Self {
            root_limb0: root[0].as_int().to_string(),
            root_limb1: root[1].as_int().to_string(),
        }
    }

    pub fn to_elements(&self) -> Result<[BaseElement; 2], WitnessError> {
        Ok([
            parse_element(&self.root_limb0, "root_limb0")?,
            parse_element(&self.root_limb1, "root_limb1")?,
        ])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathNode {
    pub limb0: String,
    pub limb1: String,
}

/// Private witness file written by the issuer for one holder.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessFile {
    pub value_limb0: String,
    pub value_limb1: String,
    pub index: usize,
    pub path: Vec<PathNode>,
}

/// A validated private witness. It deliberately has no `Debug` implementation, so it cannot end up
/// in logs or error messages.
#[derive(Clone)]
pub struct Witness {
    pub value: [BaseElement; 2],
    pub index: usize,
    pub path: Vec<Hash>,
}

impl Witness {
    pub fn new(
        value: [BaseElement; 2],
        index: usize,
        path: Vec<Hash>,
    ) -> Result<Self, WitnessError> {
        let depth = path.len();
        validate_depth(depth)?;
        // depth <= 63 here, so the shift cannot overflow a 64-bit usize.
        if index >= 1usize << depth {
            return Err(WitnessError::IndexOutOfRange { index, depth });
        }
        Ok(Self { value, index, path })
    }

    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// The root this witness resolves to. Matching it against the trusted root is the verifier's
    /// job: the prover's own view of the root carries no authority.
    pub fn root(&self) -> [BaseElement; 2] {
        root_from_path(self.value, &self.path, self.index).to_elements()
    }

    /// Builds a deterministic witness from a seed by choosing the path directly.
    ///
    /// Proving cost depends only on the path, so this gives valid workloads for depths whose full
    /// tree could not be materialized (for example 2^63 leaves). It is a benchmark and test
    /// fixture, not an issued credential.
    pub fn synthetic(depth: usize, seed: u64) -> Result<Self, WitnessError> {
        validate_depth(depth)?;
        let mut state = seed;
        let mut next = || {
            let high = u128::from(splitmix64(&mut state));
            let low = u128::from(splitmix64(&mut state));
            BaseElement::new(high << 64 | low)
        };
        let value = [next(), next()];
        let path = (0..depth).map(|_| Hash::new(next(), next())).collect();
        let index = (splitmix64(&mut state) as usize) & ((1usize << depth) - 1);
        Self::new(value, index, path)
    }
}

impl TryFrom<&WitnessFile> for Witness {
    type Error = WitnessError;

    fn try_from(file: &WitnessFile) -> Result<Self, WitnessError> {
        let value = [
            parse_element(&file.value_limb0, "value_limb0")?,
            parse_element(&file.value_limb1, "value_limb1")?,
        ];
        let path = file
            .path
            .iter()
            .map(|node| {
                Ok(Hash::new(
                    parse_element(&node.limb0, "path limb0")?,
                    parse_element(&node.limb1, "path limb1")?,
                ))
            })
            .collect::<Result<Vec<_>, WitnessError>>()?;
        Self::new(value, file.index, path)
    }
}

impl From<&Witness> for WitnessFile {
    fn from(witness: &Witness) -> Self {
        Self {
            value_limb0: witness.value[0].as_int().to_string(),
            value_limb1: witness.value[1].as_int().to_string(),
            index: witness.index,
            path: witness
                .path
                .iter()
                .map(|node| {
                    let [limb0, limb1] = node.to_elements();
                    PathNode {
                        limb0: limb0.as_int().to_string(),
                        limb1: limb1.as_int().to_string(),
                    }
                })
                .collect(),
        }
    }
}

// Deterministic fixture generator; never used for real credentials.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_canonical_elements_are_rejected() {
        let modulus = BaseElement::MODULUS;
        assert!(parse_element(&(modulus - 1).to_string(), "x").is_ok());
        assert_eq!(
            parse_element(&modulus.to_string(), "x"),
            Err(WitnessError::NonCanonicalElement("x"))
        );
        assert!(parse_element("-1", "x").is_err());
        assert!(parse_element("0x10", "x").is_err());
    }

    #[test]
    fn depth_must_give_a_power_of_two_trace() {
        for depth in [1, 3, 7, 15, 31, 63] {
            assert!(validate_depth(depth).is_ok());
        }
        for depth in [0, 2, 4, 5, 8, 127] {
            assert_eq!(
                validate_depth(depth),
                Err(WitnessError::InvalidDepth(depth))
            );
        }
    }

    #[test]
    fn out_of_range_index_is_rejected() {
        let witness = Witness::synthetic(3, 1).unwrap();
        assert!(matches!(
            Witness::new(witness.value, 8, witness.path),
            Err(WitnessError::IndexOutOfRange { index: 8, depth: 3 })
        ));
    }

    #[test]
    fn witness_file_round_trip_preserves_root() {
        let witness = Witness::synthetic(7, 9).unwrap();
        let file = WitnessFile::from(&witness);
        assert_eq!(Witness::try_from(&file).unwrap().root(), witness.root());
    }

    #[test]
    fn synthetic_witness_is_deterministic() {
        let a = Witness::synthetic(7, 5).unwrap();
        let b = Witness::synthetic(7, 5).unwrap();
        assert_eq!(a.root(), b.root());
        assert_eq!(a.index, b.index);
        assert_ne!(a.root(), Witness::synthetic(7, 6).unwrap().root());
    }
}
