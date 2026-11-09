//! Canonical Rescue128 Merkle primitives and witness format shared by the issuer and
//! prover/verifier, so both sides hash and encode credentials identically.

pub mod rescue;
mod utils;
pub mod witness;

use winterfell::math::fields::f128::BaseElement;

pub use rescue::{Hash, Rescue128};
pub use utils::{are_equal, is_binary, is_zero, not, EvaluationResult};
pub use witness::{parse_element, validate_depth, RootFile, Witness, WitnessError, WitnessFile};

/// Reconstructs a Rescue Merkle root from a leaf value, authentication path, and index.
pub fn root_from_path(value: [BaseElement; 2], path: &[Hash], index: usize) -> Hash {
    let mut current = Rescue128::digest(&value);
    for (level, sibling) in path.iter().enumerate() {
        let [a0, a1] = current.to_elements();
        let [b0, b1] = sibling.to_elements();
        let node = if ((index >> level) & 1) == 0 {
            [a0, a1, b0, b1]
        } else {
            [b0, b1, a0, a1]
        };
        current = Rescue128::digest(&node);
    }
    current
}
