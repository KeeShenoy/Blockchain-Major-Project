//! A zero-knowledge variant of the membership proof.
//!
//! The AIR, the Rescue arithmetic, and the compact Merkle storage are shared with the other modes.
//! The difference is that every polynomial the verifier sees evaluations of is randomized, so the
//! whole transcript can be simulated without the witness. The protocol, the simulator, and the
//! security argument are written out in `docs/zero-knowledge.md`; the short version:
//!
//! - Each trace column becomes `T_i + Z_H * r_i` with `deg r_i < h`. It agrees with the trace on the
//!   trace domain `H`, so the AIR is unchanged, and any `h` evaluations outside `H` are uniform.
//! - One extra committed column `m`, which is not part of the AIR, is added to the quotient as
//!   `Q'(X) = Q(X) + m(gX)`. An opened quotient value at `x` depends on the trace at `g x`, which is
//!   not otherwise opened, and `m(g x)` is a one-time pad over it. Soundness is unaffected, because
//!   `Z * m(gX)` is a multiple of the divisor.
//! - The quotient is cut into chunks and masked so that they telescope back to `Q'`. An opening of
//!   the chunks then reveals only `Q'` at that point.
//! - The batched DEEP polynomial gets a uniform polynomial `R` of the same degree bound added to
//!   it, committed before the batching coefficients are drawn. `f = R + B` is uniform and is sent
//!   in full, so no folding layer exists to leak anything.
//! - Every masking polynomial keeps at least one degree of freedom beyond what is opened, so that
//!   rows that are only hashed have conditional entropy and the Merkle commitments hide them.

mod constraints;
mod proof;
mod prover;
pub mod simulator;
#[cfg(test)]
mod transport;
mod verifier;

use rand::RngCore;
use winter_prover::proof::Context;
use winterfell::{
    crypto::{Digest, ElementHasher, RandomCoin},
    math::{fields::f128::BaseElement, FieldElement, StarkField, ToElements},
    BatchingMethod, ConstraintCompositionCoefficients, FieldExtension, ProofOptions, TraceInfo,
};

use crate::{
    air::{MerkleAir, PublicInputs, SessionId, HASH_CYCLE_LEN, TRACE_WIDTH},
    config::{CommitmentHasher, BLOWUP_FACTOR},
};

pub use proof::{ZkOpenings, ZkProof};
pub use prover::{prove_trace_zk, prove_zk};
pub use verifier::{
    check_openings, replay_transcript, verify_zk, verify_zk_bytes, Challenges, ZkError,
};

pub(crate) type Coin = winterfell::crypto::DefaultRandomCoin<CommitmentHasher>;

/// Conjectured security the parameters must reach, matching the non-zero-knowledge modes.
const TARGET_SECURITY_BITS: u32 = 83;
const FIELD_BITS: u32 = 128;
/// Collision resistance of Blake3-256.
const HASH_SECURITY_BITS: u32 = 128;
/// Trace columns as committed: the AIR's columns plus the shifted masking column `m`.
pub const COMMITTED_TRACE_WIDTH: usize = TRACE_WIDTH + 1;
/// Index of `m` within a committed trace row.
pub const MASK_COLUMN: usize = TRACE_WIDTH;
/// Domain separator for the transcript, so that a zero-knowledge proof cannot be replayed as
/// anything else.
const TRANSCRIPT_TAG: u64 = u64::from_le_bytes(*b"zkmember");

/// Masking and domain sizes for a tree depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZkParameters {
    pub tree_depth: usize,
    /// Rows in the execution trace, `n`.
    pub trace_length: usize,
    /// Size of the evaluation domain `D`, a coset disjoint from the trace domain.
    pub domain_size: usize,
    /// Queries, `s`. Chosen so that the masked code rate still reaches the target security.
    pub num_queries: usize,
    /// Coefficients in each `r_i`: `s + 3`, for `s` queries, `z`, `g z`, and one spare.
    pub trace_mask: usize,
    /// Coefficients in `m`: `2s + 3`, for the `s` opened points, the `s` shifted pads, `z`, `g z`,
    /// and one spare.
    pub shift_mask: usize,
    /// Coefficients in each chunk mask: `s + 2`, for `s` queries, `z`, and one spare.
    pub chunk_mask: usize,
    /// Coefficients of the batched polynomial `f`: `n + trace_mask - 1`. Determines the code rate.
    pub batched_length: usize,
    /// Number of quotient chunks.
    pub num_chunks: usize,
}

impl ZkParameters {
    /// Returns the parameters for a tree of this depth, or `None` if the trace is too short for the
    /// masking to fit: depth 3, for example, has fewer rows than the masks need.
    pub fn for_depth(tree_depth: usize) -> Option<Self> {
        merkle_core::validate_depth(tree_depth).ok()?;
        let n = crate::air::trace_length(tree_depth);
        let domain_size = n * BLOWUP_FACTOR;
        let num_queries = (1..=255).find(|&s| {
            let batched = n + s + 2;
            conjectured_security(s, domain_size, batched) >= TARGET_SECURITY_BITS
        })?;
        let trace_mask = num_queries + 3;
        let shift_mask = 2 * num_queries + 3;
        let chunk_mask = num_queries + 2;
        let batched_length = n + trace_mask - 1;
        if trace_mask >= n || shift_mask > batched_length || chunk_mask > trace_mask {
            return None;
        }

        // Degree of the transition quotient over the randomized columns: constraint degree 5 in
        // the trace, the periodic columns, and the divisor (of degree n - 1).
        let quotient_degree =
            5 * (n + trace_mask - 1) + (HASH_CYCLE_LEN - 1) * n / HASH_CYCLE_LEN - (n - 1);
        if quotient_degree >= domain_size {
            return None;
        }
        Some(Self {
            tree_depth,
            trace_length: n,
            domain_size,
            num_queries,
            trace_mask,
            shift_mask,
            chunk_mask,
            batched_length,
            num_chunks: (quotient_degree + 1).div_ceil(n),
        })
    }

    /// Winterfell parameters for this configuration. FRI is not run here: `f` is sent in full. The
    /// FRI fields are required by the type and only feed the transcript's context.
    pub fn proof_options(&self) -> ProofOptions {
        ProofOptions::new(
            self.num_queries,
            BLOWUP_FACTOR,
            0,
            FieldExtension::None,
            4,
            31,
            BatchingMethod::Linear,
            BatchingMethod::Linear,
        )
    }

    pub fn security(&self) -> ZkSecurity {
        let (ldr, udr) = proven_security(self);
        ZkSecurity {
            conjectured_bits: conjectured_security(
                self.num_queries,
                self.domain_size,
                self.batched_length,
            ),
            proven_ldr_bits: ldr,
            proven_udr_bits: udr,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZkSecurity {
    pub conjectured_bits: u32,
    pub proven_ldr_bits: u32,
    pub proven_udr_bits: u32,
}

/// Conjectured soundness of `s` queries against the code of dimension `dimension` on the domain.
/// This is Winterfell's conjectured formula with its integer blowup replaced by the true rate.
fn conjectured_security(num_queries: usize, domain_size: usize, dimension: usize) -> u32 {
    let per_query = (domain_size as f64 / dimension as f64).log2();
    let query_security = (num_queries as f64 * per_query).floor() as u32;
    let field_security = FIELD_BITS - domain_size.ilog2();
    query_security
        .min(field_security)
        .saturating_sub(1)
        .min(HASH_SECURITY_BITS)
}

/// Winterfell's proven-security estimate (Theorems 2 and 3 of ePrint 2024/1553), with the code rate
/// replaced by the rate of the masked code and no FRI folding rounds.
fn proven_security(parameters: &ZkParameters) -> (u32, u32) {
    let field = FIELD_BITS as f64;
    let n = parameters.trace_length as f64;
    let domain = parameters.domain_size as f64;
    let dimension = parameters.batched_length as f64;
    let queries = parameters.num_queries as f64;
    let rho = dimension / domain;
    let openings = 2.0;
    let max_degree = BLOWUP_FACTOR as f64 + 1.0;

    // List-decoding regime: search the proximity parameter m, as Winterfell does.
    let m_max = (1.0 / (2.0 * (((n + 2.0) / n).sqrt() - 1.0))).ceil() as u32;
    let list_bits = |m: f64| -> f64 {
        let alpha = (1.0 + 0.5 / m) * rho.sqrt();
        let list = m / (rho - 2.0 * m / domain);
        let ali = field - list.log2();
        let deep = field
            - (list * list * (max_degree * (dimension + openings - 1.0) + dimension - 1.0)).log2();
        let commit = field - ((m + 0.5).powf(7.0) / (3.0 * rho.powf(1.5)) * domain * domain).log2();
        let query = -queries * alpha.log2();
        ali.min(deep).min(commit).min(query)
    };
    let ldr = (3..m_max)
        .map(|m| list_bits(m as f64))
        .fold(0.0_f64, f64::max);

    // Unique-decoding regime.
    let rho_plus = (dimension + openings) / domain;
    let udr_query = -queries * ((1.0 + rho_plus) * 0.5).log2();
    let udr_deep = field - (max_degree * (dimension + openings - 1.0) + dimension - 1.0).log2();
    let udr_commit = field - domain.log2();
    let udr = udr_query.min(udr_deep).min(udr_commit).min(field);

    (
        (ldr as u32).min(HASH_SECURITY_BITS),
        (udr as u32).min(HASH_SECURITY_BITS),
    )
}

/// The transcript shared by the prover, the verifier, and the simulator.
pub(crate) struct Transcript {
    coin: Coin,
}

impl Transcript {
    pub(crate) fn new(air_inputs: &PublicInputs, parameters: &ZkParameters) -> Self {
        let trace_info = TraceInfo::new(TRACE_WIDTH, parameters.trace_length);
        let context =
            Context::new::<BaseElement>(trace_info, parameters.proof_options(), NUM_CONSTRAINTS);
        let mut seed = context.to_elements();
        seed.push(BaseElement::new(TRANSCRIPT_TAG as u128));
        seed.push(BaseElement::new(parameters.num_chunks as u128));
        seed.push(BaseElement::new(parameters.trace_mask as u128));
        seed.extend(air_inputs.to_elements());
        Self {
            coin: Coin::new(&seed),
        }
    }

    pub(crate) fn absorb_digest(
        &mut self,
        digest: <CommitmentHasher as winterfell::crypto::Hasher>::Digest,
    ) {
        self.coin.reseed(digest);
    }

    pub(crate) fn absorb_elements(&mut self, elements: &[BaseElement]) {
        self.coin.reseed(CommitmentHasher::hash_elements(elements));
    }

    pub(crate) fn constraint_coefficients(
        &mut self,
        air: &MerkleAir,
    ) -> ConstraintCompositionCoefficients<BaseElement> {
        use winterfell::Air;
        air.get_constraint_composition_coefficients(&mut self.coin)
            .expect("the coin never runs out of elements")
    }

    pub(crate) fn element(&mut self) -> BaseElement {
        self.coin
            .draw()
            .expect("the coin never runs out of elements")
    }

    pub(crate) fn elements(&mut self, count: usize) -> Vec<BaseElement> {
        (0..count).map(|_| self.element()).collect()
    }

    pub(crate) fn positions(&mut self, parameters: &ZkParameters) -> Vec<usize> {
        let mut positions = self
            .coin
            .draw_integers(parameters.num_queries, parameters.domain_size, 0)
            .expect("the coin never runs out of elements");
        positions.sort_unstable();
        positions.dedup();
        positions
    }
}

/// Transition constraints plus assertions in the Merkle AIR; recorded in the transcript's context.
const NUM_CONSTRAINTS: usize = 13;

pub(crate) fn public_inputs(root: [BaseElement; 2], session: SessionId) -> PublicInputs {
    PublicInputs {
        tree_root: root,
        session,
    }
}

/// A CSPRNG with a fresh operating-system seed. Each proof should use its own: the OS is asked once
/// per proof, not once per mask.
pub fn fresh_rng() -> rand_chacha::ChaCha20Rng {
    use rand::SeedableRng;
    rand_chacha::ChaCha20Rng::from_rng(rand::rngs::OsRng)
        .expect("the OS random source is available")
}

/// A uniform field element, by rejection sampling. Reducing a random `u128` would bias the result.
pub(crate) fn random_element<R: RngCore>(rng: &mut R) -> BaseElement {
    loop {
        let value = (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64());
        if value < BaseElement::MODULUS {
            return BaseElement::new(value);
        }
    }
}

pub(crate) fn random_polynomial<R: RngCore>(rng: &mut R, length: usize) -> Vec<BaseElement> {
    (0..length).map(|_| random_element(rng)).collect()
}

/// Horner evaluation of a polynomial given by coefficients from low to high degree.
pub(crate) fn evaluate(coefficients: &[BaseElement], x: BaseElement) -> BaseElement {
    coefficients
        .iter()
        .rev()
        .fold(BaseElement::ZERO, |sum, &c| sum * x + c)
}

/// Digest bytes as stored in a proof.
pub(crate) fn digest_bytes(
    digest: <CommitmentHasher as winterfell::crypto::Hasher>::Digest,
) -> [u8; 32] {
    digest.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_reach_the_target_at_supported_depths() {
        for depth in [7, 15, 31, 63] {
            let parameters = ZkParameters::for_depth(depth).unwrap();
            assert!(
                parameters.security().conjectured_bits >= 83,
                "{parameters:?}"
            );
            // The evaluation domain is not enlarged.
            assert_eq!(parameters.domain_size, 8 * parameters.trace_length);
        }
    }

    /// Executable witness independence: any other holder under the same root can reach the same
    /// opened values with suitable masks. See `transport.rs`.
    #[test]
    fn another_holders_masks_reproduce_the_same_view() {
        use merkle_core::{Hash, Rescue128, Witness};
        use rand::{rngs::StdRng, Rng, SeedableRng};
        use winterfell::crypto::MerkleTree;

        use super::verifier::replay_transcript;

        let mut rng = StdRng::seed_from_u64(77);
        let depth = 7;
        let values: Vec<[BaseElement; 2]> = (0..1usize << depth)
            .map(|_| {
                [
                    BaseElement::new(rng.gen::<u64>() as u128),
                    BaseElement::new(rng.gen::<u64>() as u128),
                ]
            })
            .collect();
        let leaves: Vec<Hash> = values
            .iter()
            .map(|value| Rescue128::digest(value))
            .collect();
        let tree = MerkleTree::<Rescue128>::new(leaves).unwrap();
        let witness_of = |index: usize| {
            Witness::new(values[index], index, tree.prove(index).unwrap().1).unwrap()
        };
        let parameters = ZkParameters::for_depth(depth).unwrap();
        let session = SessionId::NONE;
        let holder = witness_of(11);
        let root = holder.root();
        for trial in 0..4 {
            let proof = prove_zk(&holder, session, &mut rng).unwrap();
            let (air, challenges) = replay_transcript(&proof, root, &parameters, session).unwrap();
            let other = witness_of(trial * 20 + 40);
            let transported = super::transport::transport_view(
                &air,
                &parameters,
                &challenges,
                &proof.openings,
                &other,
                session,
            );
            assert_eq!(transported, proof.openings, "trial {trial}");
        }
    }

    #[test]
    fn shallow_trees_are_refused() {
        assert_eq!(ZkParameters::for_depth(3), None);
        assert_eq!(ZkParameters::for_depth(1), None);
    }

    #[test]
    fn masks_cover_every_opening_with_one_spare() {
        let p = ZkParameters::for_depth(7).unwrap();
        assert_eq!(p.trace_mask, p.num_queries + 2 + 1);
        assert_eq!(p.chunk_mask, p.num_queries + 1 + 1);
        assert_eq!(p.shift_mask, 2 * p.num_queries + 2 + 1);
    }
}
