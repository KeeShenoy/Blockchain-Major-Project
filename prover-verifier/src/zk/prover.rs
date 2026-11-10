//! The zero-knowledge prover. See `docs/zero-knowledge.md` for the construction.

use merkle_core::Witness;
use rand::RngCore;
use winterfell::{
    math::{
        add_in_place, fft, fields::f128::BaseElement, mul_acc, polynom, FieldElement, StarkField,
    },
    Air, StarkDomain, Trace, TraceInfo, TraceTable,
};

use crate::stage;

use super::{
    constraints::quotient_on_domain, digest_bytes, evaluate, proof::ZkOpenings, public_inputs,
    random_polynomial, Transcript, ZkParameters, ZkProof, COMMITTED_TRACE_WIDTH, MASK_COLUMN,
};
use crate::{
    air::{MerkleAir, SessionId, TRACE_WIDTH},
    compact::{evaluate_long_columns, RowOracle},
    config::CommitmentHasher,
    prover::build_trace,
};

type Oracle = RowOracle<BaseElement, CommitmentHasher>;

/// Proves membership in zero knowledge. `rng` must be a fresh, cryptographic randomness source
/// (`rand::rngs::OsRng`): reusing masks across proofs breaks zero knowledge.
pub fn prove_zk<R: RngCore>(witness: &Witness, session: SessionId, rng: &mut R) -> Option<ZkProof> {
    let parameters = ZkParameters::for_depth(witness.depth())?;
    Some(prove_trace_zk(
        build_trace(witness, session),
        &parameters,
        session,
        rng,
    ))
}

/// Proves an already built trace in zero knowledge.
pub fn prove_trace_zk<R: RngCore>(
    trace: TraceTable<BaseElement>,
    parameters: &ZkParameters,
    session: SessionId,
    rng: &mut R,
) -> ZkProof {
    let n = parameters.trace_length;
    assert_eq!(trace.length(), n, "trace does not match the parameters");
    let last = n - 1;
    let root = [trace.get(0, last), trace.get(1, last)];
    let inputs = public_inputs(root, session);
    let mut transcript = Transcript::new(&inputs, parameters);

    let air = MerkleAir::new(
        TraceInfo::new(TRACE_WIDTH, n),
        public_inputs(root, session),
        parameters.proof_options(),
    );
    let domain = StarkDomain::new(&air);
    let generator = BaseElement::get_root_of_unity(n.ilog2());

    // Randomize each column outside the trace domain: T + Z_H * r, with Z_H = X^n - 1.
    let mut columns: Vec<Vec<BaseElement>> = trace
        .main_segment()
        .interpolate_columns()
        .columns()
        .map(|column| {
            let mut masked = vec![BaseElement::ZERO; n + parameters.trace_mask];
            masked[..n].copy_from_slice(column);
            let mask = random_polynomial(rng, parameters.trace_mask);
            for (i, &coefficient) in mask.iter().enumerate() {
                masked[i] -= coefficient;
                masked[n + i] += coefficient;
            }
            masked
        })
        .collect();
    drop(trace);
    // The column that pads the quotient's dependence on the next row.
    columns.push(random_polynomial(rng, parameters.shift_mask));

    let trace_rows = stage!("zk_trace_lde", evaluate_long_columns(&columns, &domain));
    let trace_oracle = stage!(
        "zk_trace_commit",
        Oracle::commit(trace_rows, COMMITTED_TRACE_WIDTH)
    );
    transcript.absorb_digest(trace_oracle.commitment());

    // The quotient, as coefficients, then Q' = Q + m(gX).
    let coefficients = transcript.constraint_coefficients(&air);
    let mut quotient = stage!(
        "zk_constraints",
        quotient_on_domain(
            &air,
            &coefficients,
            trace_oracle.rows(),
            COMMITTED_TRACE_WIDTH,
            &domain,
        )
    );
    let inverse_twiddles = fft::get_inv_twiddles::<BaseElement>(quotient.len());
    fft::interpolate_poly_with_offset(&mut quotient, &inverse_twiddles, domain.offset());
    let mut power = BaseElement::ONE;
    for (i, &coefficient) in columns[MASK_COLUMN].iter().enumerate() {
        quotient[i] += coefficient * power;
        power *= generator;
    }

    let mut chunks = split_quotient(&quotient, parameters, rng);
    drop(quotient);
    let batching_mask = random_polynomial(rng, parameters.batched_length);

    chunks.push(batching_mask);
    let composition_rows = stage!(
        "zk_composition_lde",
        evaluate_long_columns(&chunks, &domain)
    );
    let composition_oracle = stage!(
        "zk_composition_commit",
        Oracle::commit(composition_rows, parameters.num_chunks + 1)
    );
    let batching_mask = chunks.pop().expect("the mask was just pushed");
    transcript.absorb_digest(composition_oracle.commitment());

    // Out-of-domain openings.
    let z = transcript.element();
    let shifted_z = z * generator;
    let mut ood_trace = Vec::with_capacity(2 * COMMITTED_TRACE_WIDTH);
    ood_trace.extend(columns.iter().map(|column| evaluate(column, z)));
    ood_trace.extend(columns.iter().map(|column| evaluate(column, shifted_z)));
    let ood_chunks: Vec<BaseElement> = chunks.iter().map(|chunk| evaluate(chunk, z)).collect();
    let mut ood = ood_trace.clone();
    ood.extend_from_slice(&ood_chunks);
    transcript.absorb_elements(&ood);

    // The batched polynomial f = R + B, sent in full.
    let trace_weights = transcript.elements(COMMITTED_TRACE_WIDTH);
    let chunk_weights = transcript.elements(parameters.num_chunks);
    let length = n + parameters.trace_mask;
    let mut over_z = vec![BaseElement::ZERO; length];
    let mut over_shifted_z = vec![BaseElement::ZERO; length];
    for (i, column) in columns.iter().enumerate() {
        accumulate(&mut over_z, column, ood_trace[i], trace_weights[i]);
        accumulate(
            &mut over_shifted_z,
            column,
            ood_trace[COMMITTED_TRACE_WIDTH + i],
            trace_weights[i],
        );
    }
    for (j, chunk) in chunks.iter().enumerate() {
        accumulate(&mut over_z, chunk, ood_chunks[j], chunk_weights[j]);
    }
    polynom::syn_div_in_place(&mut over_z, 1, z);
    polynom::syn_div_in_place(&mut over_shifted_z, 1, shifted_z);
    add_in_place(&mut over_z, &over_shifted_z);
    drop(over_shifted_z);
    drop(columns);
    drop(chunks);
    add_in_place(&mut over_z[..parameters.batched_length], &batching_mask);
    debug_assert_eq!(over_z[parameters.batched_length], BaseElement::ZERO);
    over_z.truncate(parameters.batched_length);
    let batched = over_z;
    transcript.absorb_elements(&batched);

    // Query phase.
    let positions = transcript.positions(parameters);
    let trace_rows = stage!("zk_open", oracle_rows(&trace_oracle, &positions));
    let composition_rows = oracle_rows(&composition_oracle, &positions);
    ZkProof {
        tree_depth: parameters.tree_depth as u8,
        num_unique_queries: positions.len() as u8,
        trace_root: digest_bytes(trace_oracle.commitment()),
        composition_root: digest_bytes(composition_oracle.commitment()),
        openings: ZkOpenings {
            ood_trace,
            ood_chunks,
            batched,
            trace_rows,
            composition_rows,
        },
        trace_opening: trace_oracle.open(&positions),
        composition_opening: composition_oracle.open(&positions),
    }
}

/// Cuts `Q'` into chunks of `n` coefficients and masks them: chunk `j` gets `X^n rho_j - rho_{j-1}`.
/// The masks telescope away in `sum_j X^(j n) chunk_j`, so the chunks still recompose to `Q'`.
fn split_quotient<R: RngCore>(
    quotient: &[BaseElement],
    parameters: &ZkParameters,
    rng: &mut R,
) -> Vec<Vec<BaseElement>> {
    let n = parameters.trace_length;
    debug_assert!(
        quotient[parameters.num_chunks * n..]
            .iter()
            .all(|&c| c == BaseElement::ZERO),
        "the quotient has higher degree than its chunks can hold"
    );
    let mut chunks: Vec<Vec<BaseElement>> = quotient[..parameters.num_chunks * n]
        .chunks(n)
        .map(|chunk| {
            let mut padded = vec![BaseElement::ZERO; n + parameters.chunk_mask];
            padded[..n].copy_from_slice(chunk);
            padded
        })
        .collect();
    for j in 0..parameters.num_chunks - 1 {
        let mask = random_polynomial(rng, parameters.chunk_mask);
        for (i, &coefficient) in mask.iter().enumerate() {
            chunks[j][n + i] += coefficient;
            chunks[j + 1][i] -= coefficient;
        }
    }
    chunks
}

/// Adds `weight * (polynomial - value)` to the accumulator.
fn accumulate(
    accumulator: &mut [BaseElement],
    polynomial: &[BaseElement],
    value: BaseElement,
    weight: BaseElement,
) {
    mul_acc(&mut accumulator[..polynomial.len()], polynomial, weight);
    accumulator[0] -= value * weight;
}

fn oracle_rows(oracle: &Oracle, positions: &[usize]) -> Vec<BaseElement> {
    let mut rows = Vec::new();
    for &position in positions {
        rows.extend_from_slice(oracle.row(position));
    }
    rows
}
