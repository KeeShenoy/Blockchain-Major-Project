//! Experimental configurations and the protocol parameters they share.

#[cfg(not(feature = "blake3-192"))]
use winterfell::crypto::hashers::Blake3_256;
use winterfell::{math::fields::f128::BaseElement, BatchingMethod, FieldExtension, ProofOptions};

use crate::air::trace_length;

/// Hash for STARK commitments and Fiat-Shamir. The credential tree separately uses Rescue.
#[cfg(not(feature = "blake3-192"))]
pub type CommitmentHasher = Blake3_256<BaseElement>;

/// Measurement-only alternative: shorter digests, but 96-bit collision resistance instead of 128.
#[cfg(feature = "blake3-192")]
pub type CommitmentHasher = winterfell::crypto::hashers::Blake3_192<BaseElement>;

pub const NUM_QUERIES: usize = 28;
pub const BLOWUP_FACTOR: usize = 8;
pub const GRINDING_FACTOR: u32 = 0;
pub const FRI_REMAINDER_MAX_DEGREE: usize = 31;
pub const BASELINE_FRI_FOLDING_FACTOR: usize = 8;
pub const FOLDED_FRI_FOLDING_FACTOR: usize = 16;
pub const OPTIMIZED_FRI_FOLDING_FACTOR: usize = 4;
/// Winterfell's upper limit on the FRI remainder degree.
pub const MAX_FRI_REMAINDER_DEGREE: usize = 255;

/// A prover configuration. Modes differ in FRI folding factor and in how the prover stores its
/// commitments, never in query count, blowup, grinding, or field.
///
/// | mode             | FRI folding | prover storage |
/// |------------------|-------------|----------------|
/// | `baseline`       | 8           | Winterfell     |
/// | `split`          | 8           | compact        |
/// | `folded-default` | 16          | Winterfell     |
/// | `folded`         | 16          | compact        |
/// | `optimized`      | 4, or none  | compact        |
///
/// `split` and `folded-default` exist so that the effect of each change can be measured alone.
/// `optimized` is the configuration the project recommends; see [`ProverMode::proof_options`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProverMode {
    Baseline,
    SplitOptimized,
    FoldedDefault,
    Folded,
    Optimized,
}

impl ProverMode {
    pub const ALL: [Self; 5] = [
        Self::Baseline,
        Self::SplitOptimized,
        Self::FoldedDefault,
        Self::Folded,
        Self::Optimized,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.label() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::SplitOptimized => "split",
            Self::FoldedDefault => "folded-default",
            Self::Folded => "folded",
            Self::Optimized => "optimized",
        }
    }

    pub fn uses_compact_storage(self) -> bool {
        matches!(self, Self::SplitOptimized | Self::Folded | Self::Optimized)
    }

    pub fn fri_folding_factor(self) -> usize {
        match self {
            Self::Baseline | Self::SplitOptimized => BASELINE_FRI_FOLDING_FACTOR,
            Self::FoldedDefault | Self::Folded => FOLDED_FRI_FOLDING_FACTOR,
            Self::Optimized => OPTIMIZED_FRI_FOLDING_FACTOR,
        }
    }

    /// The exact parameters a proof in this mode must use. The verifier enforces them too.
    ///
    /// `optimized` chooses its FRI shape by measurement (see `docs/results.md`). The remainder may
    /// be as large as the DEEP polynomial itself, up to Winterfell's limit of degree 255. For trees
    /// up to depth 31 that means no FRI folding layer at all: the prover sends the polynomial and
    /// the verifier checks it directly at the query positions, which is FRI's own final step.
    /// Deeper trees fold by 4. Query count, blowup, grinding, and field are unchanged.
    pub fn proof_options(self, tree_depth: usize) -> ProofOptions {
        let remainder = match self {
            Self::Optimized => (trace_length(tree_depth) - 1).min(MAX_FRI_REMAINDER_DEGREE),
            _ => FRI_REMAINDER_MAX_DEGREE,
        };
        options_with_fri(self.fri_folding_factor(), remainder)
    }
}

/// Protocol parameters with the given FRI shape and the project's fixed query count, blowup,
/// grinding, and field.
pub fn options_with_fri(folding_factor: usize, remainder_max_degree: usize) -> ProofOptions {
    ProofOptions::new(
        NUM_QUERIES,
        BLOWUP_FACTOR,
        GRINDING_FACTOR,
        FieldExtension::None,
        folding_factor,
        remainder_max_degree,
        BatchingMethod::Linear,
        BatchingMethod::Linear,
    )
}
