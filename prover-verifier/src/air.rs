// Adapted from the Winterfell Merkle example.
// Copyright (c) Facebook, Inc. and its affiliates. Licensed under the MIT license.

//! AIR for a Rescue Merkle path.
//!
//! The trace has 7 columns and `8 * (depth + 1)` rows, in 8-row cycles. Columns 0-5 hold the
//! Rescue state: 7 rows of rounds, then one row that loads the next path node. Column 6 carries the
//! index bit only in the row each cycle loads a node, and is otherwise unconstrained.

use merkle_core::{are_equal, is_binary, is_zero, not, rescue, EvaluationResult};
use winterfell::{
    math::{fields::f128::BaseElement, FieldElement, ToElements},
    Air, AirContext, Assertion, EvaluationFrame, ProofOptions, TraceInfo,
    TransitionConstraintDegree,
};

pub const HASH_CYCLE_LEN: usize = rescue::CYCLE_LENGTH;
pub const HASH_STATE_WIDTH: usize = rescue::STATE_WIDTH;
pub const INDEX_BIT_COLUMN: usize = 6;
pub const TRACE_WIDTH: usize = 7;

/// Execution trace length for a tree of this depth: one leaf hash plus `depth` node hashes.
pub fn trace_length(depth: usize) -> usize {
    (depth + 1) * HASH_CYCLE_LEN
}

/// A verifier-chosen challenge that a proof is bound to. Public inputs seed Fiat-Shamir, so a proof
/// made for one session fails for every other. [`SessionId::NONE`] means no binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionId(pub [BaseElement; 2]);

impl SessionId {
    pub const NONE: Self = Self([BaseElement::ZERO; 2]);
}

pub struct PublicInputs {
    pub tree_root: [BaseElement; 2],
    pub session: SessionId,
}

impl ToElements<BaseElement> for PublicInputs {
    fn to_elements(&self) -> Vec<BaseElement> {
        let [root0, root1] = self.tree_root;
        let [session0, session1] = self.session.0;
        vec![root0, root1, session0, session1]
    }
}

pub struct MerkleAir {
    context: AirContext<BaseElement>,
    tree_root: [BaseElement; 2],
}

impl Air for MerkleAir {
    type BaseField = BaseElement;
    type PublicInputs = PublicInputs;

    fn new(trace_info: TraceInfo, pub_inputs: PublicInputs, options: ProofOptions) -> Self {
        let degrees = vec![
            TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN]),
            TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN]),
            TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN]),
            TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN]),
            TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN]),
            TransitionConstraintDegree::with_cycles(5, vec![HASH_CYCLE_LEN]),
            TransitionConstraintDegree::with_cycles(2, vec![HASH_CYCLE_LEN]),
        ];
        assert_eq!(TRACE_WIDTH, trace_info.width());
        MerkleAir {
            context: AirContext::new(trace_info, degrees, 6, options),
            tree_root: pub_inputs.tree_root,
        }
    }

    fn context(&self) -> &AirContext<Self::BaseField> {
        &self.context
    }

    fn evaluate_transition<E: FieldElement + From<Self::BaseField>>(
        &self,
        frame: &EvaluationFrame<E>,
        periodic_values: &[E],
        result: &mut [E],
    ) {
        let current = frame.current();
        let next = frame.next();
        debug_assert_eq!(TRACE_WIDTH, current.len());
        debug_assert_eq!(TRACE_WIDTH, next.len());

        let hash_flag = periodic_values[0];
        let ark = &periodic_values[1..];

        // In round rows, the next state must be one Rescue round of the current state.
        rescue::enforce_round(
            result,
            &current[..HASH_STATE_WIDTH],
            &next[..HASH_STATE_WIDTH],
            ark,
            hash_flag,
        );

        // In the node-loading row, the accumulated hash must stay in registers [0, 1] when the
        // index bit is 0, and move to registers [2, 3] when it is 1.
        let hash_init_flag = not(hash_flag);
        let bit = next[INDEX_BIT_COLUMN];
        let not_bit = not(bit);
        result.agg_constraint(0, hash_init_flag, not_bit * are_equal(current[0], next[0]));
        result.agg_constraint(1, hash_init_flag, not_bit * are_equal(current[1], next[1]));
        result.agg_constraint(2, hash_init_flag, bit * are_equal(current[0], next[2]));
        result.agg_constraint(3, hash_init_flag, bit * are_equal(current[1], next[3]));

        // The capacity registers restart at zero for every hash.
        result.agg_constraint(4, hash_init_flag, is_zero(next[4]));
        result.agg_constraint(5, hash_init_flag, is_zero(next[5]));

        // Only the index bit read by the swap above must be binary. Leaving the other rows free
        // lets the prover fill them with random values, which hides the column; see
        // `prover::build_trace`.
        result[6] = hash_init_flag * is_binary(bit);
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        let last_step = self.trace_length() - 1;
        vec![
            // The path must resolve to the public root.
            Assertion::single(0, last_step, self.tree_root[0]),
            Assertion::single(1, last_step, self.tree_root[1]),
            // The first hash must be a leaf: a two-element credential, zero-extended. Without this,
            // the two children of an internal node would also pass as a "credential".
            Assertion::single(2, 0, BaseElement::ZERO),
            Assertion::single(3, 0, BaseElement::ZERO),
            // Every hash starts with zeroed capacity registers.
            Assertion::periodic(4, 0, HASH_CYCLE_LEN, BaseElement::ZERO),
            Assertion::periodic(5, 0, HASH_CYCLE_LEN, BaseElement::ZERO),
        ]
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Self::BaseField>> {
        let mut result = vec![HASH_CYCLE_MASK.to_vec()];
        result.append(&mut rescue::get_round_constants());
        result
    }
}

/// 1 in the seven Rescue round rows of a cycle, 0 in the node-loading row.
const HASH_CYCLE_MASK: [BaseElement; HASH_CYCLE_LEN] = [
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ONE,
    BaseElement::ZERO,
];
