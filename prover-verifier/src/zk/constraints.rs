//! Constraint evaluation over randomized trace columns.
//!
//! Winterfell's `DefaultConstraintEvaluator` is not used here. In debug builds it checks that
//! evaluations have the degree the AIR declares for an unmasked trace, which a randomized trace
//! deliberately does not. The combination formula is the same one Winterfell's verifier applies.

use winterfell::{
    math::{batch_inversion, fields::f128::BaseElement, polynom, FieldElement, StarkField},
    Air, ConstraintCompositionCoefficients, ConstraintDivisor, EvaluationFrame, StarkDomain,
};

use crate::air::{MerkleAir, HASH_CYCLE_LEN, TRACE_WIDTH};

/// Evaluates the quotient at one point: the verifier's check at `z`, and the simulator's values.
pub(crate) fn quotient_at(
    air: &MerkleAir,
    coefficients: &ConstraintCompositionCoefficients<BaseElement>,
    frame: &EvaluationFrame<BaseElement>,
    x: BaseElement,
) -> BaseElement {
    let transition = air.get_transition_constraints(&coefficients.transition);
    let periodic: Vec<BaseElement> = air
        .get_periodic_column_polys()
        .iter()
        .map(|poly| {
            let cycles = (air.trace_length() / poly.len()) as u32;
            polynom::eval(poly, x.exp_vartime(cycles.into()))
        })
        .collect();

    let mut evaluations = [BaseElement::ZERO; TRACE_WIDTH];
    air.evaluate_transition(frame, &periodic, &mut evaluations);
    let mut result = transition.combine_evaluations::<BaseElement>(&evaluations, &[], x);

    let boundary = air.get_boundary_constraints(None, &coefficients.boundary);
    for group in boundary.main_constraints() {
        result += group.evaluate_at(frame.current(), x);
    }
    result
}

/// Evaluates the quotient on every point of the evaluation domain, given `rows` in row-major order
/// with `row_width` columns (of which the first [`TRACE_WIDTH`] belong to the AIR).
pub(crate) fn quotient_on_domain(
    air: &MerkleAir,
    coefficients: &ConstraintCompositionCoefficients<BaseElement>,
    rows: &[BaseElement],
    row_width: usize,
    domain: &StarkDomain<BaseElement>,
) -> Vec<BaseElement> {
    let size = domain.lde_domain_size();
    let next_row = domain.trace_to_lde_blowup();
    let generator = BaseElement::get_root_of_unity(size.ilog2());
    let points: Vec<BaseElement> = {
        let mut x = domain.offset();
        (0..size)
            .map(|_| {
                let point = x;
                x *= generator;
                point
            })
            .collect()
    };

    let transition = air.get_transition_constraints(&coefficients.transition);
    let transition_weights = transition.main_constraint_coef();
    let transition_divisor = transition.divisor();
    let transition_inverse = divisor_inverses(transition_divisor, &points);

    let boundary = air.get_boundary_constraints(None, &coefficients.boundary);
    let groups: Vec<_> = boundary
        .main_constraints()
        .iter()
        .map(|group| (group, divisor_inverses(group.divisor(), &points)))
        .collect();

    // Periodic columns repeat over the evaluation domain: a column of cycle length c is a
    // polynomial in x^(n / c), which takes only size * c / n distinct values.
    let polys = air.get_periodic_column_polys();
    let period = size * HASH_CYCLE_LEN / air.trace_length();
    let periodic_table: Vec<Vec<BaseElement>> = points[..period]
        .iter()
        .map(|&x| {
            polys
                .iter()
                .map(|poly| {
                    let cycles = (air.trace_length() / poly.len()) as u32;
                    polynom::eval(poly, x.exp_vartime(cycles.into()))
                })
                .collect()
        })
        .collect();

    let mut frame = EvaluationFrame::new(TRACE_WIDTH);
    let mut evaluations = [BaseElement::ZERO; TRACE_WIDTH];
    let mut result = Vec::with_capacity(size);
    let row = |index: usize| &rows[index * row_width..index * row_width + TRACE_WIDTH];
    for (step, &x) in points.iter().enumerate() {
        frame.current_mut().copy_from_slice(row(step));
        frame
            .next_mut()
            .copy_from_slice(row((step + next_row) % size));
        // The AIR accumulates into this buffer rather than overwriting it.
        evaluations.fill(BaseElement::ZERO);
        air.evaluate_transition(&frame, &periodic_table[step % period], &mut evaluations);

        let combined = evaluations
            .iter()
            .zip(&transition_weights)
            .fold(BaseElement::ZERO, |sum, (&value, &weight)| {
                sum + value * weight
            });
        let exemptions = transition_divisor.evaluate_exemptions_at(x);
        let mut value = combined * transition_inverse.at(step) * exemptions;

        for (group, inverse) in &groups {
            let numerator =
                group
                    .constraints()
                    .iter()
                    .fold(BaseElement::ZERO, |sum, constraint| {
                        let trace_value = frame.current()[constraint.column()];
                        sum + constraint.evaluate_at(x, trace_value) * *constraint.cc()
                    });
            value += numerator * inverse.at(step) * group.divisor().evaluate_exemptions_at(x);
        }
        result.push(value);
    }
    result
}

/// Inverses of a divisor's numerator over the domain. A numerator such as `X^n - 1` repeats with a
/// short period, so only one period is computed and inverted.
struct PeriodicInverses {
    values: Vec<BaseElement>,
}

impl PeriodicInverses {
    fn at(&self, step: usize) -> BaseElement {
        self.values[step % self.values.len()]
    }
}

fn divisor_inverses(
    divisor: &ConstraintDivisor<BaseElement>,
    points: &[BaseElement],
) -> PeriodicInverses {
    let size = points.len();
    // The numerator is a product of terms X^d - c. X^d repeats every size / gcd(size, d) steps.
    let period = divisor
        .numerator()
        .iter()
        .map(|&(degree, _)| size >> (size.trailing_zeros().min(degree.trailing_zeros())))
        .max()
        .unwrap_or(1);
    let numerators: Vec<BaseElement> = points[..period]
        .iter()
        .map(|&x| {
            divisor
                .numerator()
                .iter()
                .fold(BaseElement::ONE, |product, &(degree, constant)| {
                    product * (x.exp_vartime((degree as u64).into()) - constant)
                })
        })
        .collect();
    PeriodicInverses {
        values: batch_inversion(&numerators),
    }
}
