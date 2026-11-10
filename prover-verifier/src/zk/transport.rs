//! Executable form of the zero-knowledge argument, for tests.
//!
//! Take a real proof made with one witness. For any *other* witness with the same root, this
//! constructs the masking polynomials that would have produced exactly the same opened values
//! under the same challenges. Because masks are uniform and the map from masks to openings is
//! affine, equally many mask choices lead to that view for each witness, which is what witness
//! independence means. Each construction below is one lemma of `docs/zero-knowledge.md`.

use merkle_core::Witness;
use winterfell::{
    math::{fft, fields::f128::BaseElement, polynom, FieldElement, StarkField},
    Air, EvaluationFrame, StarkDomain, Trace,
};

use super::{
    constraints::{quotient_at, quotient_on_domain},
    evaluate,
    proof::ZkOpenings,
    verifier::Challenges,
    ZkParameters, COMMITTED_TRACE_WIDTH, MASK_COLUMN,
};
use crate::{
    air::{MerkleAir, SessionId, TRACE_WIDTH},
    compact::evaluate_long_columns,
    prover::build_trace,
};

/// The polynomial of degree below `points.len()` through the given points.
fn interpolate(points: &[BaseElement], values: &[BaseElement]) -> Vec<BaseElement> {
    polynom::interpolate(points, values, false)
}

/// Pads to `length` coefficients. Fails if a mask had to cover more points than it has coefficients,
/// which would mean the masking is too small for the openings.
fn pad(mut polynomial: Vec<BaseElement>, length: usize) -> Vec<BaseElement> {
    assert!(
        polynomial.len() <= length,
        "a mask was asked to cover too many points"
    );
    polynomial.resize(length, BaseElement::ZERO);
    polynomial
}

/// Reconstructs masks for `witness` that reproduce the view, and returns the opened values they
/// imply. The caller compares them with the original view.
pub(crate) fn transport_view(
    air: &MerkleAir,
    parameters: &ZkParameters,
    challenges: &Challenges,
    view: &ZkOpenings,
    witness: &Witness,
    session: SessionId,
) -> ZkOpenings {
    let n = parameters.trace_length;
    let domain = StarkDomain::new(air);
    let generator = BaseElement::get_root_of_unity(n.ilog2());
    let domain_generator = BaseElement::get_root_of_unity(parameters.domain_size.ilog2());
    let z = challenges.z;
    let shifted_z = z * generator;
    let positions = &challenges.positions;
    let width = COMMITTED_TRACE_WIDTH;
    let chunk_count = parameters.num_chunks;
    let point_at = |position: usize| {
        air.domain_offset() * domain_generator.exp_vartime((position as u64).into())
    };
    let points: Vec<BaseElement> = positions.iter().map(|&p| point_at(p)).collect();
    let vanishing = |x: BaseElement| x.exp_vartime((n as u64).into()) - BaseElement::ONE;
    let trace_row = |index: usize| &view.trace_rows[index * width..][..width];

    // Lemma 1. Each column's mask is chosen to hit the viewed value at every opened point.
    let trace = build_trace(witness, session);
    let polys = trace.main_segment().interpolate_columns();
    let mut opened_points = points.clone();
    opened_points.push(z);
    opened_points.push(shifted_z);
    let mut columns: Vec<Vec<BaseElement>> = Vec::with_capacity(width);
    for i in 0..TRACE_WIDTH {
        let base = polys.get_column(i);
        let mut residuals: Vec<BaseElement> = (0..points.len())
            .map(|q| (trace_row(q)[i] - evaluate(base, points[q])) / vanishing(points[q]))
            .collect();
        residuals.push((view.ood_trace[i] - evaluate(base, z)) / vanishing(z));
        residuals
            .push((view.ood_trace[width + i] - evaluate(base, shifted_z)) / vanishing(shifted_z));
        let mask = pad(
            interpolate(&opened_points, &residuals),
            parameters.trace_mask,
        );

        let mut masked = vec![BaseElement::ZERO; n + parameters.trace_mask];
        masked[..n].copy_from_slice(base);
        for (j, &coefficient) in mask.iter().enumerate() {
            masked[j] -= coefficient;
            masked[n + j] += coefficient;
        }
        columns.push(masked);
    }

    // Lemma 2. The mask column additionally pads the quotient at each g x that is not opened.
    let opened_quotient = |q: usize| -> BaseElement {
        let row = &view.composition_rows[q * (chunk_count + 1)..][..chunk_count];
        let y = points[q].exp_vartime((n as u64).into());
        row.iter()
            .rev()
            .fold(BaseElement::ZERO, |sum, &value| sum * y + value)
    };
    let mut mask_points = opened_points.clone();
    let mut mask_values: Vec<BaseElement> = (0..points.len())
        .map(|q| trace_row(q)[MASK_COLUMN])
        .collect();
    mask_values.push(view.ood_trace[MASK_COLUMN]);
    mask_values.push(view.ood_trace[width + MASK_COLUMN]);
    for (q, &position) in positions.iter().enumerate() {
        let next = (position + domain.trace_to_lde_blowup()) % parameters.domain_size;
        if positions.contains(&next) {
            continue; // m(g x) is opened already, and the quotient is a function of opened rows.
        }
        let x = points[q];
        let shifted_x = x * generator;
        let current = columns[..TRACE_WIDTH]
            .iter()
            .map(|c| evaluate(c, x))
            .collect();
        let following = columns[..TRACE_WIDTH]
            .iter()
            .map(|c| evaluate(c, shifted_x))
            .collect();
        let frame = EvaluationFrame::from_rows(current, following);
        mask_points.push(shifted_x);
        mask_values
            .push(opened_quotient(q) - quotient_at(air, &challenges.coefficients, &frame, x));
    }
    columns.push(pad(
        interpolate(&mask_points, &mask_values),
        parameters.shift_mask,
    ));

    // The new witness's masked quotient, as coefficients.
    let rows = evaluate_long_columns(&columns, &domain);
    let mut quotient = quotient_on_domain(air, &challenges.coefficients, &rows, width, &domain);
    let inverse = fft::get_inv_twiddles::<BaseElement>(quotient.len());
    fft::interpolate_poly_with_offset(&mut quotient, &inverse, domain.offset());
    let mut power = BaseElement::ONE;
    for (i, &coefficient) in columns[MASK_COLUMN].iter().enumerate() {
        quotient[i] += coefficient * power;
        power *= generator;
    }
    let base_chunks: Vec<&[BaseElement]> = quotient[..chunk_count * n].chunks(n).collect();

    // Lemma 3. Chunk masks are solved in order at each opened point; the last chunk then has no
    // freedom left, and matching it is the actual check.
    let mut chunk_points = points.clone();
    chunk_points.push(z);
    let view_chunks = |p: usize, j: usize| {
        if p < points.len() {
            view.composition_rows[p * (chunk_count + 1) + j]
        } else {
            view.ood_chunks[j]
        }
    };
    let mut recovered = vec![Vec::with_capacity(chunk_points.len()); chunk_count - 1];
    let mut last_chunk_values = Vec::with_capacity(chunk_points.len());
    for (p, &point) in chunk_points.iter().enumerate() {
        let y = point.exp_vartime((n as u64).into());
        let mut previous = BaseElement::ZERO;
        for j in 0..chunk_count - 1 {
            let mask = (view_chunks(p, j) - evaluate(base_chunks[j], point) + previous) / y;
            recovered[j].push(mask);
            previous = mask;
        }
        last_chunk_values.push(evaluate(base_chunks[chunk_count - 1], point) - previous);
    }
    let _masks: Vec<Vec<BaseElement>> = recovered
        .iter()
        .map(|values| pad(interpolate(&chunk_points, values), parameters.chunk_mask))
        .collect();

    // Rebuild what these masks imply. The last chunk is computed, not copied from the view.
    let mut composition_rows = view.composition_rows.clone();
    for q in 0..points.len() {
        composition_rows[q * (chunk_count + 1) + chunk_count - 1] = last_chunk_values[q];
    }
    let mut ood_chunks = view.ood_chunks.clone();
    ood_chunks[chunk_count - 1] = last_chunk_values[points.len()];

    // Lemma 4. The batching mask is f minus the new batched polynomial, so f is untouched and only
    // its evaluations at the query positions are visible. Those follow from the openings above.
    ZkOpenings {
        ood_trace: view.ood_trace.clone(),
        ood_chunks,
        batched: view.batched.clone(),
        trace_rows: view.trace_rows.clone(),
        composition_rows,
    }
}
