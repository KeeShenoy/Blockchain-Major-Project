//! STARK membership proofs for a Rescue Merkle tree, built on Winterfell.
//!
//! The statement proved is: "I know a two-element credential and an authentication path that hash
//! to this public root." The verifier takes the root from a trusted source (the Ethereum registry).
//! Winterfell does not provide formal zero knowledge; `docs/security-and-privacy.md` states what is
//! and is not hidden.

pub mod air;
pub mod compact;
pub mod config;
pub mod heap;
pub mod memory;
pub mod prover;
pub mod stages;
pub mod verifier;

pub use air::SessionId;
pub use config::{CommitmentHasher, ProverMode};
pub use prover::{build_trace, prove, prove_trace, prove_trace_with};
pub use verifier::{verify, verify_bytes, verify_with_options, VerifyError};
