//! Execution trace construction and the two Winterfell prover configurations.

use merkle_core::{rescue, Hash, Witness};
use winterfell::{
    crypto::{DefaultRandomCoin, Digest, ElementHasher, Hasher, MerkleTree},
    math::{fields::f128::BaseElement, FieldElement},
    matrix::ColMatrix,
    AuxRandElements, CompositionPoly, CompositionPolyTrace, ConstraintCompositionCoefficients,
    DefaultConstraintCommitment, DefaultConstraintEvaluator, DefaultTraceLde, PartitionOptions,
    Proof, ProofOptions, Prover, ProverError, StarkDomain, Trace, TraceInfo, TracePolyTable,
    TraceTable,
};

use crate::{
    air::{
        trace_length, MerkleAir, PublicInputs, SessionId, HASH_CYCLE_LEN, HASH_STATE_WIDTH,
        INDEX_BIT_COLUMN, TRACE_WIDTH,
    },
    compact::{CompactConstraintCommitment, CompactTraceLde},
    config::{CommitmentHasher, ProverMode},
    heap,
};

/// Domain separator for the column-blinding values derived from the witness.
const BLINDING_DOMAIN: u64 = u64::from_le_bytes(*b"blind\0\0\0");

/// Builds the execution trace for a Merkle path.
///
/// Column 6 is blinded. Only one row per cycle is constrained, to the index bit; all others get
/// pseudorandom field elements. If those rows held a predictable value, the column would depend on
/// nothing but the leaf index, and a verifier could recover the index by computing the column's
/// low-degree extension for each of the `2^depth` candidates and comparing it to the opened query
/// rows. Blinding is derived from the witness, as in RFC 6979, so the prover stays deterministic
/// and needs no randomness source. This hides column 6 only; see `docs/security-and-privacy.md`.
pub fn build_trace(witness: &Witness, session: SessionId) -> TraceTable<BaseElement> {
    let seed = blinding_seed(witness, session);
    let blind = |row: usize| {
        let digest = CommitmentHasher::merge_with_int(seed, row as u64).as_bytes();
        let mut limb = [0u8; 16];
        limb.copy_from_slice(&digest[..16]);
        BaseElement::new(u128::from_le_bytes(limb))
    };

    let mut trace = TraceTable::new(TRACE_WIDTH, trace_length(witness.depth()));
    trace.fill(
        |state| {
            state[0] = witness.value[0];
            state[1] = witness.value[1];
            state[2..HASH_STATE_WIDTH].fill(BaseElement::ZERO);
            state[INDEX_BIT_COLUMN] = blind(0);
        },
        |step, state| {
            let cycle = step / HASH_CYCLE_LEN;
            let cycle_position = step % HASH_CYCLE_LEN;

            if cycle_position < rescue::NUM_ROUNDS {
                rescue::apply_round(&mut state[..HASH_STATE_WIDTH], step);
                state[INDEX_BIT_COLUMN] = blind(step + 1);
            } else {
                // Load the next path node: it goes to registers [2, 3] when the index bit is 0, and
                // to registers [0, 1] (moving the accumulated hash to [2, 3]) when the bit is 1.
                let [node0, node1] = witness.path[cycle].to_elements();
                let bit = (witness.index >> cycle) & 1;
                if bit == 0 {
                    state[2] = node0;
                    state[3] = node1;
                } else {
                    state[2] = state[0];
                    state[3] = state[1];
                    state[0] = node0;
                    state[1] = node1;
                }
                state[4] = BaseElement::ZERO;
                state[5] = BaseElement::ZERO;
                state[INDEX_BIT_COLUMN] = BaseElement::new(bit as u128);
            }
        },
    );
    trace
}

fn blinding_seed(witness: &Witness, session: SessionId) -> <CommitmentHasher as Hasher>::Digest {
    let mut elements = Vec::with_capacity(5 + 2 * witness.depth());
    elements.extend_from_slice(&session.0);
    elements.extend_from_slice(&witness.value);
    elements.push(BaseElement::new(witness.index as u128));
    elements.extend_from_slice(Hash::hashes_as_elements(&witness.path));
    CommitmentHasher::merge_with_int(CommitmentHasher::hash_elements(&elements), BLINDING_DOMAIN)
}

fn public_inputs(trace: &TraceTable<BaseElement>, session: SessionId) -> PublicInputs {
    let last_step = trace.length() - 1;
    PublicInputs {
        tree_root: [trace.get(0, last_step), trace.get(1, last_step)],
        session,
    }
}

/// Proves an already built trace with explicit protocol parameters and storage.
pub fn prove_trace_with(
    trace: TraceTable<BaseElement>,
    options: ProofOptions,
    compact_storage: bool,
    session: SessionId,
) -> Result<Proof, ProverError> {
    if compact_storage {
        CompactProver { options, session }.prove(trace)
    } else {
        DefaultProver { options, session }.prove(trace)
    }
}

/// Proves an already built trace, using this mode's parameters and storage.
pub fn prove_trace(
    trace: TraceTable<BaseElement>,
    mode: ProverMode,
    depth: usize,
    session: SessionId,
) -> Result<Proof, ProverError> {
    prove_trace_with(
        trace,
        mode.proof_options(depth),
        mode.uses_compact_storage(),
        session,
    )
}

/// Builds the trace for a witness and proves it, bound to `session`.
pub fn prove(
    witness: &Witness,
    mode: ProverMode,
    session: SessionId,
) -> Result<Proof, ProverError> {
    prove_trace(
        build_trace(witness, session),
        mode,
        witness.depth(),
        session,
    )
}

/// Stores the trace LDE, composition LDE, and Merkle trees the way Winterfell does by default.
pub struct DefaultProver {
    pub options: ProofOptions,
    pub session: SessionId,
}

/// Same protocol and identical proof bytes as [`DefaultProver`], using less heap to get there.
pub struct CompactProver {
    pub options: ProofOptions,
    pub session: SessionId,
}

impl Prover for DefaultProver {
    type BaseField = BaseElement;
    type Air = MerkleAir;
    type Trace = TraceTable<BaseElement>;
    type HashFn = CommitmentHasher;
    type VC = MerkleTree<CommitmentHasher>;
    type RandomCoin = DefaultRandomCoin<CommitmentHasher>;
    type TraceLde<E: FieldElement<BaseField = BaseElement>> =
        DefaultTraceLde<E, CommitmentHasher, Self::VC>;
    type ConstraintCommitment<E: FieldElement<BaseField = BaseElement>> =
        DefaultConstraintCommitment<E, CommitmentHasher, Self::VC>;
    type ConstraintEvaluator<'a, E: FieldElement<BaseField = BaseElement>> =
        DefaultConstraintEvaluator<'a, MerkleAir, E>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        public_inputs(trace, self.session)
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }

    fn new_trace_lde<E: FieldElement<BaseField = BaseElement>>(
        &self,
        trace_info: &TraceInfo,
        main_trace: &ColMatrix<BaseElement>,
        domain: &StarkDomain<BaseElement>,
        partition_option: PartitionOptions,
    ) -> (Self::TraceLde<E>, TracePolyTable<E>) {
        heap::checkpoint("prover_setup");
        let result = DefaultTraceLde::new(trace_info, main_trace, domain, partition_option);
        heap::checkpoint("trace_commitment");
        result
    }

    fn new_evaluator<'a, E: FieldElement<BaseField = BaseElement>>(
        &self,
        air: &'a MerkleAir,
        aux_rand_elements: Option<AuxRandElements<E>>,
        composition_coefficients: ConstraintCompositionCoefficients<E>,
    ) -> Self::ConstraintEvaluator<'a, E> {
        DefaultConstraintEvaluator::new(air, aux_rand_elements, composition_coefficients)
    }

    fn build_constraint_commitment<E: FieldElement<BaseField = BaseElement>>(
        &self,
        composition_poly_trace: CompositionPolyTrace<E>,
        num_constraint_composition_columns: usize,
        domain: &StarkDomain<BaseElement>,
        partition_options: PartitionOptions,
    ) -> (Self::ConstraintCommitment<E>, CompositionPoly<E>) {
        heap::checkpoint("constraint_evaluation");
        let result = DefaultConstraintCommitment::new(
            composition_poly_trace,
            num_constraint_composition_columns,
            domain,
            partition_options,
        );
        heap::checkpoint("composition_commitment");
        result
    }
}

impl Prover for CompactProver {
    type BaseField = BaseElement;
    type Air = MerkleAir;
    type Trace = TraceTable<BaseElement>;
    type HashFn = CommitmentHasher;
    type VC = MerkleTree<CommitmentHasher>;
    type RandomCoin = DefaultRandomCoin<CommitmentHasher>;
    type TraceLde<E: FieldElement<BaseField = BaseElement>> = CompactTraceLde<E, CommitmentHasher>;
    type ConstraintCommitment<E: FieldElement<BaseField = BaseElement>> =
        CompactConstraintCommitment<E, CommitmentHasher>;
    type ConstraintEvaluator<'a, E: FieldElement<BaseField = BaseElement>> =
        DefaultConstraintEvaluator<'a, MerkleAir, E>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> PublicInputs {
        public_inputs(trace, self.session)
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }

    fn new_trace_lde<E: FieldElement<BaseField = BaseElement>>(
        &self,
        trace_info: &TraceInfo,
        main_trace: &ColMatrix<BaseElement>,
        domain: &StarkDomain<BaseElement>,
        _partition_option: PartitionOptions,
    ) -> (Self::TraceLde<E>, TracePolyTable<E>) {
        heap::checkpoint("prover_setup");
        let result = CompactTraceLde::new(trace_info, main_trace, domain);
        heap::checkpoint("trace_commitment");
        result
    }

    fn new_evaluator<'a, E: FieldElement<BaseField = BaseElement>>(
        &self,
        air: &'a MerkleAir,
        aux_rand_elements: Option<AuxRandElements<E>>,
        composition_coefficients: ConstraintCompositionCoefficients<E>,
    ) -> Self::ConstraintEvaluator<'a, E> {
        DefaultConstraintEvaluator::new(air, aux_rand_elements, composition_coefficients)
    }

    fn build_constraint_commitment<E: FieldElement<BaseField = BaseElement>>(
        &self,
        composition_poly_trace: CompositionPolyTrace<E>,
        num_constraint_composition_columns: usize,
        domain: &StarkDomain<BaseElement>,
        _partition_options: PartitionOptions,
    ) -> (Self::ConstraintCommitment<E>, CompositionPoly<E>) {
        heap::checkpoint("constraint_evaluation");
        let result = CompactConstraintCommitment::new(
            composition_poly_trace,
            num_constraint_composition_columns,
            domain,
        );
        heap::checkpoint("composition_commitment");
        result
    }
}
