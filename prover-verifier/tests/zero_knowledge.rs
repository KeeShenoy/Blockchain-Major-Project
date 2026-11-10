//! Correctness and privacy checks for the zero-knowledge mode.
//!
//! The formal argument is in docs/zero-knowledge.md. These tests support it; they do not replace it.

use std::collections::HashSet;

use merkle_core::{Hash, Rescue128, Witness};
use prover_verifier::{
    air::{trace_length, TRACE_WIDTH},
    config::BLOWUP_FACTOR,
    zk::{
        check_openings, prove_zk, replay_transcript, simulator::simulate_openings, verify_zk,
        verify_zk_bytes, ZkError, ZkParameters, ZkProof,
    },
    SessionId,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use winterfell::{
    crypto::MerkleTree,
    math::{fft, fields::f128::BaseElement, FieldElement, StarkField},
};

const DEPTH: usize = 7;

fn rng() -> StdRng {
    StdRng::seed_from_u64(20261109)
}

fn parameters() -> ZkParameters {
    ZkParameters::for_depth(DEPTH).unwrap()
}

fn witness() -> Witness {
    Witness::synthetic(DEPTH, 1234).unwrap()
}

fn prove(witness: &Witness, session: SessionId, rng: &mut StdRng) -> ZkProof {
    prove_zk(witness, session, rng).unwrap()
}

/// A batch of real credentials under one root, with every holder's witness.
fn batch() -> ([BaseElement; 2], Vec<Witness>) {
    let mut rng = rng();
    let values: Vec<[BaseElement; 2]> = (0..1usize << DEPTH)
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
    let witnesses = values
        .iter()
        .enumerate()
        .map(|(index, &value)| Witness::new(value, index, tree.prove(index).unwrap().1).unwrap())
        .collect();
    (tree.root().to_elements(), witnesses)
}

// CORRECTNESS
// ================================================================================================

#[test]
fn valid_proofs_verify_at_every_supported_depth() {
    let mut rng = rng();
    for depth in [7, 15, 31, 63] {
        let witness = Witness::synthetic(depth, 7).unwrap();
        let parameters = ZkParameters::for_depth(depth).unwrap();
        let proof = prove(&witness, SessionId::NONE, &mut rng);
        assert_eq!(
            verify_zk(&proof, witness.root(), &parameters, SessionId::NONE),
            Ok(()),
            "depth {depth}"
        );
    }
}

#[test]
fn serialized_proof_round_trips_and_verifies() {
    let witness = witness();
    let proof = prove(&witness, SessionId::NONE, &mut rng());
    let bytes = proof.to_bytes();
    assert_eq!(
        verify_zk_bytes(&bytes, witness.root(), DEPTH, SessionId::NONE),
        Ok(())
    );
    assert_eq!(
        ZkProof::from_bytes(&bytes, &parameters())
            .unwrap()
            .to_bytes(),
        bytes
    );
}

#[test]
fn unsupported_depth_is_refused() {
    let shallow = Witness::synthetic(3, 1).unwrap();
    assert!(prove_zk(&shallow, SessionId::NONE, &mut rng()).is_none());
    assert_eq!(
        verify_zk_bytes(&[0; 8], shallow.root(), 3, SessionId::NONE),
        Err(ZkError::UnsupportedDepth(3))
    );
}

#[test]
fn security_meets_the_target_after_masking() {
    for depth in [7, 15, 31, 63] {
        let security = ZkParameters::for_depth(depth).unwrap().security();
        assert!(security.conjectured_bits >= 83, "{security:?}");
        assert!(security.proven_ldr_bits >= 41, "{security:?}");
    }
}

#[test]
fn wrong_roots_are_rejected() {
    let witness = witness();
    let root = witness.root();
    let proof = prove(&witness, SessionId::NONE, &mut rng());
    for wrong in [
        [root[1], root[0]],
        [root[0] + BaseElement::ONE, root[1]],
        [BaseElement::ZERO, BaseElement::ZERO],
    ] {
        assert!(verify_zk(&proof, wrong, &parameters(), SessionId::NONE).is_err());
    }
}

#[test]
fn tampered_witness_does_not_verify_against_the_issued_root() {
    let issued = witness();
    let mut path = issued.path.clone();
    path[2] = Hash::new(BaseElement::new(5), BaseElement::new(6));
    let tampered = Witness::new(issued.value, issued.index, path).unwrap();
    let proof = prove(&tampered, SessionId::NONE, &mut rng());
    assert!(verify_zk(&proof, issued.root(), &parameters(), SessionId::NONE).is_err());
}

#[test]
fn wrong_credential_does_not_verify() {
    let issued = witness();
    let forged = Witness::new(
        [issued.value[0] + BaseElement::ONE, issued.value[1]],
        issued.index,
        issued.path.clone(),
    )
    .unwrap();
    let proof = prove(&forged, SessionId::NONE, &mut rng());
    assert!(verify_zk(&proof, issued.root(), &parameters(), SessionId::NONE).is_err());
}

#[test]
fn proof_for_another_depth_is_rejected() {
    let shallow = Witness::synthetic(15, 3).unwrap();
    let bytes = prove(&shallow, SessionId::NONE, &mut rng()).to_bytes();
    assert!(matches!(
        verify_zk_bytes(&bytes, shallow.root(), DEPTH, SessionId::NONE),
        Err(ZkError::Malformed(_))
    ));
}

#[test]
fn proof_replayed_in_another_session_is_rejected() {
    let witness = witness();
    let session = SessionId([BaseElement::new(21), BaseElement::new(22)]);
    let other = SessionId([BaseElement::new(21), BaseElement::new(23)]);
    let proof = prove(&witness, session, &mut rng());
    assert_eq!(
        verify_zk(&proof, witness.root(), &parameters(), session),
        Ok(())
    );
    assert!(verify_zk(&proof, witness.root(), &parameters(), other).is_err());
    assert!(verify_zk(&proof, witness.root(), &parameters(), SessionId::NONE).is_err());
}

#[test]
fn every_corrupted_byte_is_rejected_without_a_crash() {
    let witness = witness();
    let root = witness.root();
    let bytes = prove(&witness, SessionId::NONE, &mut rng()).to_bytes();
    for position in 0..bytes.len() {
        for mask in [0x01, 0x80] {
            let mut corrupted = bytes.clone();
            corrupted[position] ^= mask;
            assert!(
                verify_zk_bytes(&corrupted, root, DEPTH, SessionId::NONE).is_err(),
                "flip at byte {position} of {} accepted",
                bytes.len()
            );
        }
    }
    assert!(verify_zk_bytes(&bytes[..bytes.len() / 2], root, DEPTH, SessionId::NONE).is_err());
    let mut extended = bytes.clone();
    extended.push(0);
    assert!(verify_zk_bytes(&extended, root, DEPTH, SessionId::NONE).is_err());
    assert!(verify_zk_bytes(&[], root, DEPTH, SessionId::NONE).is_err());
}

/// A batched polynomial altered at any coefficient must fail, because no folding layer exists to
/// catch it: this comparison is the entire low-degree check.
#[test]
fn altered_batched_polynomial_is_rejected() {
    let witness = witness();
    let parameters = parameters();
    let mut proof = prove(&witness, SessionId::NONE, &mut rng());
    proof.openings.batched[parameters.batched_length / 2] += BaseElement::ONE;
    assert!(verify_zk(&proof, witness.root(), &parameters, SessionId::NONE).is_err());
}

// PRIVACY
// ================================================================================================

#[test]
fn repeated_proofs_use_fresh_masks_everywhere() {
    let witness = witness();
    let mut rng = StdRng::from_entropy();
    let first = prove(&witness, SessionId::NONE, &mut rng);
    let second = prove(&witness, SessionId::NONE, &mut rng);
    assert_ne!(first.trace_root, second.trace_root);
    assert_ne!(first.composition_root, second.composition_root);

    let shared = |a: &[BaseElement], b: &[BaseElement]| {
        let set: HashSet<u128> = a.iter().map(|value| value.as_int()).collect();
        b.iter()
            .filter(|value| set.contains(&value.as_int()))
            .count()
    };
    assert_eq!(
        shared(&first.openings.ood_trace, &second.openings.ood_trace),
        0
    );
    assert_eq!(
        shared(&first.openings.ood_chunks, &second.openings.ood_chunks),
        0
    );
    assert_eq!(shared(&first.openings.batched, &second.openings.batched), 0);
    assert_eq!(
        shared(&first.openings.trace_rows, &second.openings.trace_rows),
        0
    );
    assert_eq!(
        shared(
            &first.openings.composition_rows,
            &second.openings.composition_rows
        ),
        0
    );
}

#[test]
fn transcript_values_do_not_contain_the_witness() {
    let witness = witness();
    let bytes = prove(&witness, SessionId::NONE, &mut rng()).to_bytes();
    let mut secrets: Vec<[u8; 16]> = witness
        .value
        .iter()
        .map(|element| element.as_int().to_le_bytes())
        .collect();
    for node in &witness.path {
        for element in node.to_elements() {
            secrets.push(element.as_int().to_le_bytes());
        }
    }
    for secret in &secrets {
        assert!(!bytes.windows(16).any(|window| window == secret));
    }
}

/// The attack that identified holders in the non-zero-knowledge modes: recompute a candidate's
/// trace column and test whether the opened values lie on its extension.
#[test]
fn verifier_who_knows_the_batch_cannot_match_openings_to_a_holder() {
    let (root, witnesses) = batch();
    let parameters = parameters();
    let holder = &witnesses[42];
    let proof = prove(holder, SessionId::NONE, &mut rng());
    assert_eq!(
        verify_zk(&proof, root, &parameters, SessionId::NONE),
        Ok(())
    );

    let length = trace_length(DEPTH);
    let opened: Vec<u128> = proof
        .openings
        .trace_rows
        .chunks_exact(TRACE_WIDTH + 1)
        .map(|row| row[0].as_int())
        .collect();
    let matches = |candidate: &Witness| {
        let trace = prover_verifier::build_trace(candidate, SessionId::NONE);
        let mut column: Vec<BaseElement> = (0..length).map(|row| trace.get(0, row)).collect();
        fft::interpolate_poly(&mut column, &fft::get_inv_twiddles::<BaseElement>(length));
        let extension: HashSet<u128> = fft::evaluate_poly_with_offset(
            &column,
            &fft::get_twiddles::<BaseElement>(length),
            BaseElement::GENERATOR,
            BLOWUP_FACTOR,
        )
        .iter()
        .map(|value| value.as_int())
        .collect();
        opened.iter().any(|value| extension.contains(value))
    };
    assert_eq!(
        witnesses
            .iter()
            .filter(|candidate| matches(candidate))
            .count(),
        0
    );
}

/// The simulator knows no witness, yet its view passes every algebraic check of the verifier.
#[test]
fn simulated_views_pass_the_verifier_without_a_witness() {
    let witness = witness();
    let parameters = parameters();
    let mut rng = rng();
    let root = witness.root();
    let proof = prove(&witness, SessionId::NONE, &mut rng);
    let (air, challenges) = replay_transcript(&proof, root, &parameters, SessionId::NONE).unwrap();
    assert_eq!(
        check_openings(&air, &parameters, &challenges, &proof.openings),
        Ok(())
    );
    for _ in 0..50 {
        let view = simulate_openings(&air, &parameters, &challenges, &mut rng);
        assert_eq!(
            check_openings(&air, &parameters, &challenges, &view),
            Ok(())
        );
    }
}

/// Supporting statistics, not evidence of zero knowledge on their own: opened values for two
/// different holders should be indistinguishable from uniform at the bit level.
#[test]
fn opened_values_have_balanced_bits_across_holders() {
    let (_, witnesses) = batch();
    let mut rng = rng();
    for holder in [&witnesses[3], &witnesses[100]] {
        let (mut ones, mut total) = (0u64, 0u64);
        for _ in 0..40 {
            let proof = prove(holder, SessionId::NONE, &mut rng);
            for value in proof
                .openings
                .trace_rows
                .iter()
                .chain(&proof.openings.ood_chunks)
            {
                ones += u64::from(value.as_int().count_ones());
                total += 128;
            }
        }
        let share = ones as f64 / total as f64;
        assert!((share - 0.5).abs() < 0.005, "bit balance {share}");
    }
}
