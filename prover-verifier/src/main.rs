//! Phase 2 baseline: an unoptimized Winterfell STARK proving Merkle-path membership.
//! Ethereum and the issuer remain deliberately out of scope until later phases.

mod air;
mod prover;
mod utils;

use std::{marker::PhantomData, time::Instant};

use core_utils::Serializable;
use rand_utils::{rand_value, rand_vector};
use winterfell::{
    crypto::{hashers::Blake3_256, DefaultRandomCoin, ElementHasher, MerkleTree},
    math::{fields::f128::BaseElement, FieldElement, StarkField},
    BatchingMethod, FieldExtension, Proof, ProofOptions, Prover, VerifierError,
};

use air::{MerkleAir, PublicInputs};
use prover::MerkleProver;
use utils::rescue::{
    self, Hash, Rescue128, CYCLE_LENGTH as HASH_CYCLE_LEN, NUM_ROUNDS as NUM_HASH_ROUNDS,
    STATE_WIDTH as HASH_STATE_WIDTH,
};

const DEFAULT_TREE_DEPTH: usize = 7;
const TRACE_WIDTH: usize = 7;

struct BaselineRun {
    tree_depth: usize,
    tree_build_ms: u128,
    trace_build_ms: u128,
    proving_ms: u128,
    proof_size_bytes: usize,
    verification_ms: u128,
    root: [BaseElement; 2],
    proof: Proof,
}

fn proof_options() -> ProofOptions {
    ProofOptions::new(
        28,
        8,
        0,
        FieldExtension::None,
        8,
        31,
        BatchingMethod::Linear,
        BatchingMethod::Linear,
    )
}

fn build_merkle_tree(depth: usize, value: [BaseElement; 2], index: usize) -> MerkleTree<Rescue128> {
    let leaf_elements: Vec<BaseElement> = rand_vector((1usize << depth) * 2);
    let mut leaves: Vec<Hash> = leaf_elements
        .chunks_exact(2)
        .map(|elements| Hash::new(elements[0], elements[1]))
        .collect();
    leaves[index] = Rescue128::digest(&value);
    MerkleTree::new(leaves).expect("generated leaves form a valid Merkle tree")
}

fn create_baseline_run(tree_depth: usize) -> BaselineRun {
    assert!(tree_depth > 0 && tree_depth < usize::BITS as usize);

    let value = [BaseElement::new(42), BaseElement::new(43)];
    let index = (rand_value::<BaseElement>().as_int() % (1u128 << tree_depth)) as usize;

    let tree_started = Instant::now();
    let tree = build_merkle_tree(tree_depth, value, index);
    let tree_build_ms = tree_started.elapsed().as_millis();

    let (leaf, authentication_path) = tree.prove(index).expect("index belongs to the tree");
    let mut branch = vec![leaf];
    branch.extend_from_slice(&authentication_path);

    let prover = MerkleProver::<Blake3_256<BaseElement>>::new(proof_options());
    let trace_started = Instant::now();
    let trace = prover.build_trace(value, &branch, index);
    let trace_build_ms = trace_started.elapsed().as_millis();

    let proving_started = Instant::now();
    let proof = prover
        .prove(trace)
        .expect("baseline proof generation must succeed");
    let proving_ms = proving_started.elapsed().as_millis();
    let root = tree.root().to_elements();

    let verification_started = Instant::now();
    verify_proof(proof.clone(), root).expect("generated proof must verify");
    let verification_ms = verification_started.elapsed().as_millis();

    BaselineRun {
        tree_depth,
        tree_build_ms,
        trace_build_ms,
        proving_ms,
        proof_size_bytes: proof.to_bytes().len(),
        verification_ms,
        root,
        proof,
    }
}

fn verify_proof(proof: Proof, root: [BaseElement; 2]) -> Result<(), VerifierError> {
    let acceptable_options =
        winterfell::AcceptableOptions::OptionSet(vec![proof.options().clone()]);
    winterfell::verify::<
        MerkleAir,
        Blake3_256<BaseElement>,
        DefaultRandomCoin<Blake3_256<BaseElement>>,
        MerkleTree<Blake3_256<BaseElement>>,
    >(proof, PublicInputs { tree_root: root }, &acceptable_options)
}

fn main() {
    let tree_depth = std::env::args()
        .nth(1)
        .map(|value| {
            value
                .parse()
                .expect("tree depth must be a positive integer")
        })
        .unwrap_or(DEFAULT_TREE_DEPTH);
    let run = create_baseline_run(tree_depth);

    println!(
        concat!(
            "{{\"tree_depth\":{},\"num_leaves\":{},\"tree_build_ms\":{},",
            "\"trace_build_ms\":{},\"proving_ms\":{},\"proof_size_bytes\":{},",
            "\"verification_ms\":{},\"verified\":true}}"
        ),
        run.tree_depth,
        1usize << run.tree_depth,
        run.tree_build_ms,
        run.trace_build_ms,
        run.proving_ms,
        run.proof_size_bytes,
        run.verification_ms,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_roundtrip_verifies() {
        let run = create_baseline_run(3);
        assert!(verify_proof(run.proof, run.root).is_ok());
    }

    #[test]
    fn tampered_root_fails_verification() {
        let run = create_baseline_run(3);
        let wrong_root = [run.root[1], run.root[0]];
        assert!(verify_proof(run.proof, wrong_root).is_err());
    }
}
