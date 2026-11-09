use winterfell::math::FieldElement;

/// Returns zero only when a == b.
pub fn are_equal<E: FieldElement>(a: E, b: E) -> E {
    a - b
}

/// Returns zero only when a == zero.
pub fn is_zero<E: FieldElement>(a: E) -> E {
    a
}

/// Returns zero only when a is zero or one.
pub fn is_binary<E: FieldElement>(a: E) -> E {
    a * a - a
}

/// Returns one when a is zero and zero when a is one.
pub fn not<E: FieldElement>(a: E) -> E {
    E::ONE - a
}

pub trait EvaluationResult<E> {
    fn agg_constraint(&mut self, index: usize, flag: E, value: E);
}

impl<E: FieldElement> EvaluationResult<E> for [E] {
    fn agg_constraint(&mut self, index: usize, flag: E, value: E) {
        self[index] += flag * value;
    }
}

impl<E: FieldElement> EvaluationResult<E> for Vec<E> {
    fn agg_constraint(&mut self, index: usize, flag: E, value: E) {
        self[index] += flag * value;
    }
}
