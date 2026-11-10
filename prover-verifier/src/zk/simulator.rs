//! The zero-knowledge simulator: produces an accepting view without a witness.
//!
//! This is the constructive half of the security argument in `docs/zero-knowledge.md`, and is used
//! only by tests. Given the public statement and the verifier's challenges, it samples exactly the
//! distribution the proof claims the real prover induces. If the two distributions are equal, the
//! real transcript carries no information beyond what this function can compute from public data.

use rand::RngCore;
use winterfell::{
    math::{batch_inversion, fields::f128::BaseElement, FieldElement, StarkField},
    Air, EvaluationFrame,
};

use super::{
    constraints::quotient_at, evaluate, proof::ZkOpenings, random_element, random_polynomial,
    verifier::Challenges, ZkParameters, COMMITTED_TRACE_WIDTH, MASK_COLUMN,
};
use crate::air::{MerkleAir, TRACE_WIDTH};

/// Samples the view of a verifier with these challenges. No witness is involved.
pub fn simulate_openings<R: RngCore>(
    air: &MerkleAir,
    parameters: &ZkParameters,
    challenges: &Challenges,
    rng: &mut R,
) -> ZkOpenings {
    let n = parameters.trace_length;
    let size = parameters.domain_size;
    let chunks = parameters.num_chunks;
    let row_step = size / n;
    let z = challenges.z;
    let z_to_n = z.exp_vartime((n as u64).into());
    let generator = BaseElement::get_root_of_unity(n.ilog2());
    let positions = &challenges.positions;
    let domain_generator = BaseElement::get_root_of_unity(size.ilog2());
    let points: Vec<BaseElement> = positions
        .iter()
        .map(|&p| air.domain_offset() * domain_generator.exp_vartime((p as u64).into()))
        .collect();

    // Trace and mask evaluations: uniform at every opened point (Lemma 1).
    let ood_trace = random_polynomial(rng, 2 * COMMITTED_TRACE_WIDTH);
    let trace_rows = random_polynomial(rng, COMMITTED_TRACE_WIDTH * positions.len());
    let row = |index: usize| &trace_rows[index * COMMITTED_TRACE_WIDTH..][..COMMITTED_TRACE_WIDTH];

    // Chunks at z: uniform, subject to recomposing to Q'(z) (Lemma 3).
    let (ood_current, ood_next) = ood_trace.split_at(COMMITTED_TRACE_WIDTH);
    let frame = EvaluationFrame::from_rows(
        ood_current[..TRACE_WIDTH].to_vec(),
        ood_next[..TRACE_WIDTH].to_vec(),
    );
    let quotient_z = quotient_at(air, &challenges.coefficients, &frame, z) + ood_next[MASK_COLUMN];
    let ood_chunks = chunks_summing_to(quotient_z, z_to_n, chunks, rng);

    // Chunks at the query positions. Q'(x) is uniform unless g x is also queried, in which case it
    // is a function of opened rows (Lemma 2).
    let mut composition_rows = Vec::with_capacity((chunks + 1) * positions.len());
    let mut chunk_values = Vec::with_capacity(chunks * positions.len());
    for (index, &position) in positions.iter().enumerate() {
        let x = points[index];
        let next_position = (position + row_step) % size;
        let quotient = match positions.iter().position(|&p| p == next_position) {
            Some(next) => {
                let frame = EvaluationFrame::from_rows(
                    row(index)[..TRACE_WIDTH].to_vec(),
                    row(next)[..TRACE_WIDTH].to_vec(),
                );
                quotient_at(air, &challenges.coefficients, &frame, x) + row(next)[MASK_COLUMN]
            }
            None => random_element(rng),
        };
        chunk_values.extend(chunks_summing_to(
            quotient,
            x.exp_vartime((n as u64).into()),
            chunks,
            rng,
        ));
    }

    // f is uniform; R at the query positions is then forced by f and the openings (Lemma 4).
    let batched = random_polynomial(rng, parameters.batched_length);
    let shifted_z = z * generator;
    let denominators: Vec<BaseElement> = points
        .iter()
        .flat_map(|&x| [x - z, x - shifted_z])
        .collect();
    let inverses = batch_inversion(&denominators);
    for (index, &x) in points.iter().enumerate() {
        let (over_z, over_shifted) = (inverses[2 * index], inverses[2 * index + 1]);
        let row_chunks = &chunk_values[index * chunks..][..chunks];
        let mut known = BaseElement::ZERO;
        for (i, &value) in row(index).iter().enumerate() {
            let weight = challenges.trace_weights[i];
            known += (value - ood_current[i]) * weight * over_z
                + (value - ood_next[i]) * weight * over_shifted;
        }
        for (j, &value) in row_chunks.iter().enumerate() {
            known += (value - ood_chunks[j]) * challenges.chunk_weights[j] * over_z;
        }
        composition_rows.extend_from_slice(row_chunks);
        composition_rows.push(evaluate(&batched, x) - known);
    }

    ZkOpenings {
        ood_trace,
        ood_chunks,
        batched,
        trace_rows,
        composition_rows,
    }
}

/// `count` uniform values whose recomposition `sum_j v_j y^j`, with `y = x^n`, equals `target`.
fn chunks_summing_to<R: RngCore>(
    target: BaseElement,
    y: BaseElement,
    count: usize,
    rng: &mut R,
) -> Vec<BaseElement> {
    let mut values = random_polynomial(rng, count - 1);
    let sum = values
        .iter()
        .rev()
        .fold(BaseElement::ZERO, |sum, &value| sum * y + value);
    let top_weight = y.exp_vartime(((count - 1) as u64).into());
    values.push((target - sum) / top_weight);
    values
}
