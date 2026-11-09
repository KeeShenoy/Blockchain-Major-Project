//! End-to-end checks of what the proof system must accept and reject.

use std::collections::HashSet;

use merkle_core::Witness;
use prover_verifier::{
    air::{trace_length, HASH_CYCLE_LEN, INDEX_BIT_COLUMN, TRACE_WIDTH},
    build_trace,
    config::{CommitmentHasher, BLOWUP_FACTOR, FRI_REMAINDER_MAX_DEGREE, GRINDING_FACTOR},
    prove, prove_trace,
    prover::{CompactProver, DefaultProver},
    verify, verify_bytes, ProverMode, SessionId, VerifyError,
};
use winterfell::{
    crypto::MerkleTree,
    math::{fft, fields::f128::BaseElement, FieldElement, StarkField},
    BatchingMethod, FieldExtension, Proof, ProofOptions, Prover, Trace, TraceTable,
};

const DEPTH: usize = 7;

fn witness() -> Witness {
    Witness::synthetic(DEPTH, 1234).unwrap()
}

#[test]
fn valid_proofs_verify_in_every_mode() {
    let witness = witness();
    for mode in ProverMode::ALL {
        let proof = prove(&witness, mode, SessionId::NONE).unwrap();
        assert_eq!(
            verify(proof, witness.root(), mode, DEPTH, SessionId::NONE),
            Ok(()),
            "{}",
            mode.label()
        );
    }
}

#[test]
fn every_supported_depth_proves_and_verifies() {
    for depth in [1, 3, 7, 15, 31] {
        let witness = Witness::synthetic(depth, 99).unwrap();
        for mode in [
            ProverMode::Baseline,
            ProverMode::Folded,
            ProverMode::Optimized,
        ] {
            let proof = prove(&witness, mode, SessionId::NONE).unwrap();
            assert_eq!(
                verify(proof, witness.root(), mode, depth, SessionId::NONE),
                Ok(())
            );
        }
    }
}

#[test]
fn modes_keep_their_folding_factor_and_share_security_parameters() {
    assert_eq!(
        ProverMode::Baseline
            .proof_options(DEPTH)
            .to_fri_options()
            .folding_factor(),
        8
    );
    assert_eq!(
        ProverMode::SplitOptimized
            .proof_options(DEPTH)
            .to_fri_options()
            .folding_factor(),
        8
    );
    assert_eq!(
        ProverMode::Folded
            .proof_options(DEPTH)
            .to_fri_options()
            .folding_factor(),
        16
    );
    assert_eq!(
        ProverMode::FoldedDefault
            .proof_options(DEPTH)
            .to_fri_options()
            .folding_factor(),
        16
    );

    for mode in ProverMode::ALL {
        let options = mode.proof_options(DEPTH);
        assert_eq!(options.num_queries(), 28);
        assert_eq!(options.blowup_factor(), 8);
        assert_eq!(options.grinding_factor(), 0);
        assert_eq!(options.field_extension(), FieldExtension::None);
        if mode != ProverMode::Optimized {
            assert_eq!(options.to_fri_options().remainder_max_degree(), 31);
        }
    }

    let witness = witness();
    let security = |mode| {
        let proof = prove(&witness, mode, SessionId::NONE).unwrap();
        let proven = proof.proven_security::<CommitmentHasher>();
        (
            proof.conjectured_security::<CommitmentHasher>().bits(),
            proven.ldr_bits(),
            proven.udr_bits(),
        )
    };
    assert_eq!(security(ProverMode::Baseline), (83, 41, 23));
    assert_eq!(security(ProverMode::Folded), (83, 41, 23));
}

/// The memory work must change nothing about the proof: same transcript, same bytes.
#[test]
fn compact_prover_output_is_byte_identical() {
    for depth in [3, 7, 15, 31] {
        for seed in 0..3 {
            let witness = Witness::synthetic(depth, seed).unwrap();
            for mode in [ProverMode::Baseline, ProverMode::FoldedDefault] {
                let options = mode.proof_options(DEPTH);
                let default = DefaultProver {
                    options: options.clone(),
                    session: SessionId::NONE,
                }
                .prove(build_trace(&witness, SessionId::NONE))
                .unwrap();
                let compact = CompactProver {
                    options,
                    session: SessionId::NONE,
                }
                .prove(build_trace(&witness, SessionId::NONE))
                .unwrap();
                assert_eq!(
                    default.to_bytes(),
                    compact.to_bytes(),
                    "depth {depth} seed {seed} {}",
                    mode.label()
                );
            }
        }
    }
}

/// Every configuration must report the same security, so a smaller or faster proof is never bought
/// with weaker parameters.
#[test]
fn optimized_mode_reports_unchanged_security_at_every_depth() {
    for depth in [3, 7, 15, 31, 63] {
        let witness = Witness::synthetic(depth, 77).unwrap();
        let levels = |mode| {
            let proof = prove(&witness, mode, SessionId::NONE).unwrap();
            let proven = proof.proven_security::<CommitmentHasher>();
            (
                proof.conjectured_security::<CommitmentHasher>().bits(),
                proven.ldr_bits(),
                proven.udr_bits(),
            )
        };
        assert_eq!(levels(ProverMode::Optimized), levels(ProverMode::Baseline));
        let options = ProverMode::Optimized.proof_options(depth);
        assert_eq!(options.num_queries(), 28);
        assert_eq!(options.blowup_factor(), 8);
        assert_eq!(options.grinding_factor(), 0);
    }
}

/// Without a folding layer, the check that remains is the comparison against the explicit
/// polynomial, so a proof whose DEEP evaluations do not match must fail.
#[test]
fn optimized_proof_with_altered_remainder_is_rejected() {
    let witness = witness();
    let mode = ProverMode::Optimized;
    let bytes = prove(&witness, mode, SessionId::NONE).unwrap().to_bytes();
    // The remainder is the last large field of the proof; flip bits throughout its tail.
    let tail_start = bytes.len() - 600;
    for position in (tail_start..bytes.len() - 16).step_by(17) {
        let mut corrupted = bytes.clone();
        corrupted[position] ^= 0x10;
        assert!(verify_bytes(&corrupted, witness.root(), mode, DEPTH, SessionId::NONE).is_err());
    }
}

#[test]
fn proving_is_deterministic() {
    let witness = witness();
    for mode in ProverMode::ALL {
        assert_eq!(
            prove(&witness, mode, SessionId::NONE).unwrap().to_bytes(),
            prove(&witness, mode, SessionId::NONE).unwrap().to_bytes()
        );
    }
}

#[test]
fn wrong_roots_are_rejected_without_panicking() {
    let witness = witness();
    let root = witness.root();
    let wrong_roots = [
        [root[1], root[0]],
        [root[0] + BaseElement::ONE, root[1]],
        [root[0], root[1] + BaseElement::ONE],
        [BaseElement::ZERO, BaseElement::ZERO],
    ];
    for mode in [
        ProverMode::Baseline,
        ProverMode::Folded,
        ProverMode::Optimized,
    ] {
        for wrong_root in wrong_roots {
            let proof = prove(&witness, mode, SessionId::NONE).unwrap();
            assert!(matches!(
                verify(proof, wrong_root, mode, DEPTH, SessionId::NONE),
                Err(VerifyError::Rejected(_))
            ));
        }
    }
}

#[test]
fn tampered_witness_does_not_verify_against_the_issued_root() {
    let issued = witness();
    let mut path = issued.path.clone();
    path[3] = merkle_core::Hash::new(BaseElement::new(1), BaseElement::new(2));
    let tampered = Witness::new(issued.value, issued.index, path).unwrap();
    for mode in [
        ProverMode::Baseline,
        ProverMode::Folded,
        ProverMode::Optimized,
    ] {
        let proof = prove(&tampered, mode, SessionId::NONE).unwrap();
        assert!(verify(proof, issued.root(), mode, DEPTH, SessionId::NONE).is_err());
    }
}

#[test]
fn wrong_leaf_index_does_not_verify_against_the_issued_root() {
    let issued = witness();
    let other_index = issued.index ^ 1;
    let tampered = Witness::new(issued.value, other_index, issued.path.clone()).unwrap();
    let proof = prove(&tampered, ProverMode::Folded, SessionId::NONE).unwrap();
    assert!(verify(
        proof,
        issued.root(),
        ProverMode::Folded,
        DEPTH,
        SessionId::NONE
    )
    .is_err());
}

#[test]
fn proof_from_another_mode_is_rejected() {
    let witness = witness();
    let baseline = prove(&witness, ProverMode::Baseline, SessionId::NONE).unwrap();
    assert_eq!(
        verify(
            baseline,
            witness.root(),
            ProverMode::Folded,
            DEPTH,
            SessionId::NONE
        ),
        Err(VerifyError::UnexpectedParameters)
    );
    let folded = prove(&witness, ProverMode::Folded, SessionId::NONE).unwrap();
    assert_eq!(
        verify(
            folded,
            witness.root(),
            ProverMode::Baseline,
            DEPTH,
            SessionId::NONE
        ),
        Err(VerifyError::UnexpectedParameters)
    );
}

/// A forger picks the proof's parameters, so the verifier must not take them from the proof. The
/// original verifier passed `proof.options()` as the acceptable set, and accepted this proof.
#[test]
fn proof_with_weakened_parameters_is_rejected() {
    let witness = witness();
    let weak = ProofOptions::new(
        2,
        BLOWUP_FACTOR,
        GRINDING_FACTOR,
        FieldExtension::None,
        8,
        FRI_REMAINDER_MAX_DEGREE,
        BatchingMethod::Linear,
        BatchingMethod::Linear,
    );
    let proof = DefaultProver {
        options: weak,
        session: SessionId::NONE,
    }
    .prove(build_trace(&witness, SessionId::NONE))
    .unwrap();
    assert!(proof.conjectured_security::<CommitmentHasher>().bits() < 10);

    // The proof is internally consistent, and only the verifier's own parameters catch it.
    let accepts_own_options = winterfell::verify::<
        prover_verifier::air::MerkleAir,
        CommitmentHasher,
        winterfell::crypto::DefaultRandomCoin<CommitmentHasher>,
        MerkleTree<CommitmentHasher>,
    >(
        proof.clone(),
        prover_verifier::air::PublicInputs {
            tree_root: witness.root(),
            session: SessionId::NONE,
        },
        &winterfell::AcceptableOptions::OptionSet(vec![proof.options().clone()]),
    );
    assert!(accepts_own_options.is_ok());

    assert_eq!(
        verify(
            proof,
            witness.root(),
            ProverMode::Baseline,
            DEPTH,
            SessionId::NONE
        ),
        Err(VerifyError::UnexpectedParameters)
    );
}

/// Depth is not part of the AIR, so it is pinned by the verifier.
#[test]
fn proof_for_a_different_tree_depth_is_rejected() {
    let shallow = Witness::synthetic(3, 5).unwrap();
    let proof = prove(&shallow, ProverMode::Folded, SessionId::NONE).unwrap();
    assert!(matches!(
        verify(
            proof,
            shallow.root(),
            ProverMode::Folded,
            DEPTH,
            SessionId::NONE
        ),
        Err(VerifyError::UnexpectedTraceShape { .. })
    ));
}

#[test]
fn corrupted_proof_bytes_are_never_accepted() {
    let witness = witness();
    let root = witness.root();
    for mode in [
        ProverMode::Baseline,
        ProverMode::Folded,
        ProverMode::Optimized,
    ] {
        let bytes = prove(&witness, mode, SessionId::NONE).unwrap().to_bytes();
        assert_eq!(
            verify_bytes(&bytes, root, mode, DEPTH, SessionId::NONE),
            Ok(())
        );

        for position in (0..bytes.len()).step_by(61) {
            let mut corrupted = bytes.clone();
            corrupted[position] ^= 0x41;
            assert!(
                verify_bytes(&corrupted, root, mode, DEPTH, SessionId::NONE).is_err(),
                "flip at byte {position} of {} accepted",
                bytes.len()
            );
        }

        assert!(verify_bytes(
            &bytes[..bytes.len() / 2],
            root,
            mode,
            DEPTH,
            SessionId::NONE
        )
        .is_err());
        assert!(verify_bytes(&[], root, mode, DEPTH, SessionId::NONE).is_err());
        let mut extended = bytes.clone();
        extended.push(0);
        assert!(verify_bytes(&extended, root, mode, DEPTH, SessionId::NONE).is_err());
    }
}

/// Regression: a corrupted length prefix made the proof library allocate gigabytes, and the failed
/// allocation aborted the whole process, as a test abort here would show. Every byte is covered,
/// so the test also covers the Merkle node counts deep inside the proof.
#[test]
fn corrupted_length_prefixes_do_not_abort_the_verifier() {
    let witness = witness();
    for mode in [ProverMode::Baseline, ProverMode::Optimized] {
        let bytes = prove(&witness, mode, SessionId::NONE).unwrap().to_bytes();
        for position in 0..bytes.len() {
            let mut corrupted = bytes.clone();
            corrupted[position] ^= 1;
            assert!(
                verify_bytes(&corrupted, witness.root(), mode, DEPTH, SessionId::NONE).is_err(),
                "{} accepted a flip at byte {position}",
                mode.label()
            );
        }
    }
}

#[test]
fn serialized_proof_round_trips() {
    let witness = witness();
    let bytes = prove(&witness, ProverMode::Folded, SessionId::NONE)
        .unwrap()
        .to_bytes();
    let decoded = Proof::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.to_bytes(), bytes);
}

/// The witness must not be readable from the proof in the clear.
#[test]
fn proof_bytes_do_not_contain_witness_values() {
    let witness = witness();
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
    for mode in [
        ProverMode::Baseline,
        ProverMode::Folded,
        ProverMode::Optimized,
    ] {
        let bytes = prove(&witness, mode, SessionId::NONE).unwrap().to_bytes();
        for secret in &secrets {
            assert!(!bytes.windows(16).any(|window| window == secret));
        }
    }
}

// INDEX-RECOVERY ATTACK
// ================================================================================================

/// Runs the attack a verifier could mount with nothing but the proof: for each candidate leaf
/// index, compute what column 6 would be if it held only index bits, extend it over the LDE domain,
/// and test whether every opened value lies in that extension. Returns the matching candidates.
fn recover_index_candidates(proof: &Proof, depth: usize) -> Vec<usize> {
    let length = trace_length(depth);
    let lde_size = length * BLOWUP_FACTOR;
    let (_, opened) = proof.trace_queries[0]
        .clone()
        .parse::<BaseElement, CommitmentHasher, MerkleTree<CommitmentHasher>>(
            lde_size,
            proof.num_unique_queries as usize,
            TRACE_WIDTH,
        )
        .unwrap();
    let opened_column: Vec<BaseElement> = opened.rows().map(|row| row[INDEX_BIT_COLUMN]).collect();

    let inverse_twiddles = fft::get_inv_twiddles::<BaseElement>(length);
    let twiddles = fft::get_twiddles::<BaseElement>(length);

    (0..1usize << depth)
        .filter(|&candidate| {
            let mut column = unblinded_index_column(candidate, depth);
            fft::interpolate_poly(&mut column, &inverse_twiddles);
            let extension: HashSet<u128> = fft::evaluate_poly_with_offset(
                &column,
                &twiddles,
                BaseElement::GENERATOR,
                BLOWUP_FACTOR,
            )
            .iter()
            .map(|value| value.as_int())
            .collect();
            opened_column
                .iter()
                .all(|value| extension.contains(&value.as_int()))
        })
        .collect()
}

/// Column 6 as the original prover built it: each bit repeats until the next one is loaded, with
/// the fixed `1` the original wrote into row 1.
fn unblinded_index_column(index: usize, depth: usize) -> Vec<BaseElement> {
    let mut column = vec![BaseElement::ZERO; trace_length(depth)];
    let mut bit = BaseElement::ZERO;
    for (row, cell) in column.iter_mut().enumerate() {
        if row > 0 && row % HASH_CYCLE_LEN == 0 {
            bit = BaseElement::new(((index >> (row / HASH_CYCLE_LEN - 1)) & 1) as u128);
        }
        *cell = bit;
    }
    column[1] = BaseElement::ONE;
    column
}

fn with_unblinded_index_column(witness: &Witness) -> TraceTable<BaseElement> {
    let blinded = build_trace(witness, SessionId::NONE);
    let column = unblinded_index_column(witness.index, witness.depth());
    let mut trace = TraceTable::new(TRACE_WIDTH, blinded.length());
    for (row, &index_bit) in column.iter().enumerate() {
        for col in 0..TRACE_WIDTH {
            let value = if col == INDEX_BIT_COLUMN {
                index_bit
            } else {
                blinded.get(col, row)
            };
            trace.set(col, row, value);
        }
    }
    trace
}

#[test]
fn leaf_index_is_not_recoverable_from_the_index_column() {
    let witness = witness();

    for mode in [ProverMode::Folded, ProverMode::Optimized] {
        // The attack works on the original construction: it pins down the holder's exact position.
        let leaking = prove_trace(
            with_unblinded_index_column(&witness),
            mode,
            DEPTH,
            SessionId::NONE,
        )
        .unwrap();
        assert_eq!(
            verify(
                leaking.clone(),
                witness.root(),
                mode,
                DEPTH,
                SessionId::NONE
            ),
            Ok(())
        );
        assert_eq!(
            recover_index_candidates(&leaking, DEPTH),
            vec![witness.index]
        );

        // With the blinded column, no candidate matches.
        let blinded = prove(&witness, mode, SessionId::NONE).unwrap();
        assert!(recover_index_candidates(&blinded, DEPTH).is_empty());
    }
}

// SESSION BINDING
// ================================================================================================

#[test]
fn proof_replayed_in_another_session_is_rejected() {
    let witness = witness();
    let session = SessionId([BaseElement::new(11), BaseElement::new(12)]);
    let other = SessionId([BaseElement::new(11), BaseElement::new(13)]);
    for mode in [
        ProverMode::Baseline,
        ProverMode::Folded,
        ProverMode::Optimized,
    ] {
        let proof = prove(&witness, mode, session).unwrap();
        assert_eq!(
            verify(proof.clone(), witness.root(), mode, DEPTH, session),
            Ok(())
        );
        assert!(matches!(
            verify(proof.clone(), witness.root(), mode, DEPTH, other),
            Err(VerifyError::Rejected(_))
        ));
        assert!(verify(proof, witness.root(), mode, DEPTH, SessionId::NONE).is_err());
    }
}

// KNOWN LIMITATION
// ================================================================================================

/// Documents the absence of zero knowledge: whoever knows the batch can recognise the holder.
/// For a candidate credential and path, every trace value is determined, so its extension either
/// contains the opened Rescue-state values or it does not. If this test ever fails, the privacy
/// section of docs/security-and-privacy.md is out of date in a good way.
#[test]
fn verifier_who_knows_the_batch_can_identify_the_holder() {
    let holder = witness();
    let other = Witness::synthetic(DEPTH, 4321).unwrap();
    let proof = prove(&holder, ProverMode::Folded, SessionId::NONE).unwrap();

    let length = trace_length(DEPTH);
    let lde_size = length * BLOWUP_FACTOR;
    let (_, opened) = proof.trace_queries[0]
        .clone()
        .parse::<BaseElement, CommitmentHasher, MerkleTree<CommitmentHasher>>(
            lde_size,
            proof.num_unique_queries as usize,
            TRACE_WIDTH,
        )
        .unwrap();
    let opened_state: Vec<u128> = opened.rows().map(|row| row[0].as_int()).collect();

    let matches = |candidate: &Witness| {
        let trace = build_trace(candidate, SessionId::NONE);
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
        opened_state.iter().all(|value| extension.contains(value))
    };
    assert!(matches(&holder));
    assert!(!matches(&other));
}
