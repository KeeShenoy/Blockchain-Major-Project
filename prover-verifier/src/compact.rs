//! Memory-efficient storage for the prover's committed matrices.
//!
//! These replace `DefaultTraceLde` and `DefaultConstraintCommitment` through Winterfell's extension
//! traits. They compute the same evaluations and the same Merkle commitments, so proofs are
//! byte-identical (enforced by the `compact_prover_output_is_byte_identical` test) and soundness
//! does not depend on this module: the verifier cannot see the difference.
//!
//! Two things reduce heap use compared with the defaults:
//!
//! 1. **Partial Merkle trees.** The lowest [`UNSTORED_LEVELS`] tree levels (the row hashes) are
//!    computed but not kept. Query openings rehash only the rows they touch.
//! 2. **Rows of exact width.** Winterfell evaluates columns in groups of eight and keeps the padding,
//!    so 5 composition columns occupy the space of 8, and 7 trace columns the space of 8. The
//!    composition columns are evaluated one column and one coset at a time, which also skips the
//!    padding FFTs. The trace columns keep the grouped FFT, measured faster for seven columns, and
//!    are compacted right after evaluation.

use std::collections::BTreeMap;

// Winterfell's facade crate does not re-export these two extension-point types.
use winter_prover::{proof::Queries, ConstraintCommitment};
use winterfell::{
    crypto::{BatchMerkleProof, ElementHasher, MerkleTree},
    math::{fft, FieldElement, StarkField},
    matrix::{ColMatrix, RowMatrix},
    CompositionPoly, CompositionPolyTrace, EvaluationFrame, StarkDomain, TraceInfo, TraceLde,
    TracePolyTable,
};

/// Columns per evaluation group in Winterfell's default trace LDE.
const DEFAULT_SEGMENT_WIDTH: usize = 8;

/// Merkle tree levels, counted from the row hashes up, that are recomputed when opening instead of
/// stored. One level halves the stored tree; each further level halves what is left, at the price
/// of doubling the rows rehashed per opened query.
const UNSTORED_LEVELS: u32 = if cfg!(feature = "previous-compact-storage") {
    1
} else {
    2
};

/// Rows of a matrix over the LDE domain, with the partial Merkle tree committing to them.
pub(crate) struct RowOracle<F: FieldElement, H: ElementHasher<BaseField = F::BaseField>> {
    /// Row-major values, exactly `width` per row.
    rows: Vec<F>,
    width: usize,
    /// Stored tree levels in heap order: root at 1, children of `i` at `2i` and `2i + 1`.
    upper_nodes: Vec<H::Digest>,
}

impl<F: FieldElement, H: ElementHasher<BaseField = F::BaseField>> RowOracle<F, H> {
    pub(crate) fn commit(rows: Vec<F>, width: usize) -> Self {
        let mut oracle = Self {
            rows,
            width,
            upper_nodes: Vec::new(),
        };
        oracle.build_upper_tree();
        oracle
    }

    fn num_rows(&self) -> usize {
        self.rows.len() / self.width
    }

    pub(crate) fn rows(&self) -> &[F] {
        &self.rows
    }

    pub(crate) fn row(&self, index: usize) -> &[F] {
        &self.rows[index * self.width..(index + 1) * self.width]
    }

    pub(crate) fn commitment(&self) -> H::Digest {
        self.upper_nodes[1]
    }

    fn num_subtrees(&self) -> usize {
        self.num_rows() >> UNSTORED_LEVELS
    }

    /// Fills `nodes` with one unstored subtree in heap order: its root at 1, row hashes at the end.
    fn hash_subtree(&self, subtree: usize, nodes: &mut [H::Digest]) {
        let leaves = 1usize << UNSTORED_LEVELS;
        for offset in 0..leaves {
            nodes[leaves + offset] =
                H::hash_elements(self.row((subtree << UNSTORED_LEVELS) + offset));
        }
        for node in (1..leaves).rev() {
            nodes[node] = H::merge(&[nodes[2 * node], nodes[2 * node + 1]]);
        }
    }

    fn build_upper_tree(&mut self) {
        let subtrees = self.num_subtrees();
        assert!(subtrees >= 2, "LDE domain is too small for a partial tree");
        let mut upper = vec![H::Digest::default(); 2 * subtrees];
        let mut scratch = vec![H::Digest::default(); 2 << UNSTORED_LEVELS];
        for subtree in 0..subtrees {
            self.hash_subtree(subtree, &mut scratch);
            upper[subtrees + subtree] = scratch[1];
        }
        for node in (1..subtrees).rev() {
            upper[node] = H::merge(&[upper[2 * node], upper[2 * node + 1]]);
        }
        self.upper_nodes = upper;
    }

    /// Returns the batch opening proof for `positions`, in the format of
    /// `MerkleTree::prove_batch`: the verifier checks it with the ordinary `MerkleTree`.
    pub(crate) fn open(&self, positions: &[usize]) -> BatchMerkleProof<H> {
        let num_rows = self.num_rows();
        let depth = num_rows.ilog2() as usize;
        let subtrees = self.num_subtrees();

        let mut rebuilt: BTreeMap<usize, Vec<H::Digest>> = BTreeMap::new();
        for &position in positions {
            let subtree = position >> UNSTORED_LEVELS;
            rebuilt.entry(subtree).or_insert_with(|| {
                let mut nodes = vec![H::Digest::default(); 2 << UNSTORED_LEVELS];
                self.hash_subtree(subtree, &mut nodes);
                nodes
            });
        }
        // Looks up a node by its index in the full tree, where the row hashes follow the internal
        // nodes at `num_rows..2 * num_rows`.
        let node = |index: usize| -> H::Digest {
            if index < 2 * subtrees {
                return self.upper_nodes[index];
            }
            let levels_below_stored = index.ilog2() - subtrees.ilog2();
            let subtree = (index >> levels_below_stored) - subtrees;
            let in_subtree =
                (1 << levels_below_stored) | (index & ((1 << levels_below_stored) - 1));
            rebuilt[&subtree][in_subtree]
        };

        let mut pairs: Vec<usize> = positions.iter().map(|position| position & !1).collect();
        pairs.sort_unstable();
        pairs.dedup();

        // First layer: the sibling row hashes of opened rows, unless they are opened themselves.
        let mut nodes: Vec<Vec<H::Digest>> = pairs
            .iter()
            .map(|&left| {
                (left..left + 2)
                    .filter(|row| !positions.contains(row))
                    .map(|row| node(num_rows + row))
                    .collect()
            })
            .collect();

        // Higher layers: each sibling that the verifier cannot compute from lower layers.
        let mut layer: Vec<usize> = pairs.iter().map(|&left| (left + num_rows) >> 1).collect();
        for _ in 1..depth {
            let mut parents = Vec::with_capacity(layer.len());
            let mut i = 0;
            while i < layer.len() {
                let sibling = layer[i] ^ 1;
                if i + 1 < layer.len() && layer[i + 1] == sibling {
                    i += 1;
                } else {
                    nodes[i].push(node(sibling));
                }
                parents.push(sibling >> 1);
                i += 1;
            }
            layer = parents;
        }

        BatchMerkleProof {
            nodes,
            depth: depth as u8,
        }
    }

    fn query(&self, positions: &[usize]) -> Queries {
        let rows = positions
            .iter()
            .map(|&position| self.row(position).to_vec())
            .collect();
        Queries::new::<H, F, MerkleTree<H>>(self.open(positions), rows)
    }
}

/// Evaluates each polynomial over the LDE domain into rows of exact width.
///
/// Row `r` is the evaluation at `offset * g^r`. Rows with the same `r mod blowup` form a coset of
/// the trace domain, so each coset takes one trace-sized FFT per column, through one reused buffer.
fn evaluate_exact_rows<F: FieldElement>(
    polys: &ColMatrix<F>,
    domain: &StarkDomain<F::BaseField>,
) -> Vec<F> {
    let (width, length) = (polys.num_cols(), polys.num_rows());
    let blowup = domain.trace_to_lde_blowup();
    let lde_size = length * blowup;
    let generator = F::BaseField::get_root_of_unity(lde_size.ilog2());
    let twiddles = domain.trace_twiddles();

    let mut rows = vec![F::ZERO; lde_size * width];
    let mut shift_powers = vec![F::BaseField::ONE; length];
    let mut column = vec![F::ZERO; length];
    let mut coset_shift = domain.offset();

    for coset in 0..blowup {
        for power in 1..length {
            shift_powers[power] = shift_powers[power - 1] * coset_shift;
        }
        for col in 0..width {
            for ((value, &coefficient), &shift_power) in column
                .iter_mut()
                .zip(polys.get_column(col))
                .zip(&shift_powers)
            {
                *value = coefficient.mul_base(shift_power);
            }
            fft::evaluate_poly(&mut column, twiddles);
            for (step, &value) in column.iter().enumerate() {
                rows[(coset + step * blowup) * width + col] = value;
            }
        }
        coset_shift *= generator;
    }
    rows
}

/// Evaluates polynomials whose degree may exceed the trace length, as the zero-knowledge prover's
/// randomized columns do.
///
/// On a coset of the trace domain `X^n` is a constant, so the coefficients wrap around into `n`
/// slots after scaling. Each coset then takes one trace-sized FFT, as in [`evaluate_exact_rows`].
pub(crate) fn evaluate_long_columns<F: FieldElement>(
    columns: &[Vec<F>],
    domain: &StarkDomain<F::BaseField>,
) -> Vec<F> {
    let length = domain.trace_length();
    let blowup = domain.trace_to_lde_blowup();
    let lde_size = length * blowup;
    let width = columns.len();
    let generator = F::BaseField::get_root_of_unity(lde_size.ilog2());
    let twiddles = domain.trace_twiddles();
    let longest = columns.iter().map(Vec::len).max().unwrap_or(0);

    let mut rows = vec![F::ZERO; lde_size * width];
    let mut shift_powers = vec![F::BaseField::ONE; longest];
    let mut buffer = vec![F::ZERO; length];
    let mut coset_shift = domain.offset();

    for coset in 0..blowup {
        for power in 1..longest {
            shift_powers[power] = shift_powers[power - 1] * coset_shift;
        }
        for (col, coefficients) in columns.iter().enumerate() {
            buffer.fill(F::ZERO);
            for (index, (&coefficient, &shift_power)) in
                coefficients.iter().zip(&shift_powers).enumerate()
            {
                buffer[index % length] += coefficient.mul_base(shift_power);
            }
            fft::evaluate_poly(&mut buffer, twiddles);
            for (step, &value) in buffer.iter().enumerate() {
                rows[(coset + step * blowup) * width + col] = value;
            }
        }
        coset_shift *= generator;
    }
    rows
}

/// Evaluates trace columns with Winterfell's grouped FFT, which is faster than one column at a time
/// for seven columns, and drops the padding column from each row afterwards.
fn evaluate_grouped_rows<F: FieldElement>(
    polys: &ColMatrix<F>,
    domain: &StarkDomain<F::BaseField>,
) -> Vec<F> {
    let matrix = RowMatrix::evaluate_polys_over::<DEFAULT_SEGMENT_WIDTH>(polys, domain);
    let mut rows = Vec::with_capacity(matrix.num_rows() * matrix.num_cols());
    for index in 0..matrix.num_rows() {
        rows.extend_from_slice(matrix.row(index));
    }
    rows
}

/// Trace LDE and commitment. The Merkle AIR has one trace segment, so auxiliary segments are
/// unsupported.
pub struct CompactTraceLde<E: FieldElement, H: ElementHasher<BaseField = E::BaseField>> {
    oracle: RowOracle<E::BaseField, H>,
    blowup: usize,
    trace_info: TraceInfo,
}

impl<E: FieldElement, H: ElementHasher<BaseField = E::BaseField>> CompactTraceLde<E, H> {
    pub fn new(
        trace_info: &TraceInfo,
        main_trace: &ColMatrix<E::BaseField>,
        domain: &StarkDomain<E::BaseField>,
    ) -> (Self, TracePolyTable<E>) {
        let polys = main_trace.interpolate_columns();
        let lde = Self {
            oracle: RowOracle::commit(evaluate_grouped_rows(&polys, domain), polys.num_cols()),
            blowup: domain.trace_to_lde_blowup(),
            trace_info: trace_info.clone(),
        };
        (lde, TracePolyTable::new(polys))
    }
}

impl<E, H> TraceLde<E> for CompactTraceLde<E, H>
where
    E: FieldElement,
    H: ElementHasher<BaseField = E::BaseField> + Sync,
{
    type HashFn = H;
    type VC = MerkleTree<H>;

    fn get_main_trace_commitment(&self) -> H::Digest {
        self.oracle.commitment()
    }

    fn set_aux_trace(
        &mut self,
        _aux_trace: &ColMatrix<E>,
        _domain: &StarkDomain<E::BaseField>,
    ) -> (ColMatrix<E>, H::Digest) {
        unreachable!("the Merkle AIR has no auxiliary trace segment")
    }

    fn read_main_trace_frame_into(
        &self,
        lde_step: usize,
        frame: &mut EvaluationFrame<E::BaseField>,
    ) {
        let next_lde_step = (lde_step + self.blowup) % self.trace_len();
        frame
            .current_mut()
            .copy_from_slice(self.oracle.row(lde_step));
        frame
            .next_mut()
            .copy_from_slice(self.oracle.row(next_lde_step));
    }

    fn read_aux_trace_frame_into(&self, _lde_step: usize, _frame: &mut EvaluationFrame<E>) {
        unreachable!("the Merkle AIR has no auxiliary trace segment")
    }

    fn query(&self, positions: &[usize]) -> Vec<Queries> {
        vec![self.oracle.query(positions)]
    }

    fn trace_len(&self) -> usize {
        self.oracle.num_rows()
    }

    fn blowup(&self) -> usize {
        self.blowup
    }

    fn trace_info(&self) -> &TraceInfo {
        &self.trace_info
    }
}

/// Composition polynomial LDE and commitment.
pub struct CompactConstraintCommitment<E: FieldElement, H: ElementHasher<BaseField = E::BaseField>>
{
    oracle: RowOracle<E, H>,
}

impl<E: FieldElement, H: ElementHasher<BaseField = E::BaseField>>
    CompactConstraintCommitment<E, H>
{
    pub fn new(
        composition_poly_trace: CompositionPolyTrace<E>,
        num_columns: usize,
        domain: &StarkDomain<E::BaseField>,
    ) -> (Self, CompositionPoly<E>) {
        let composition_poly = CompositionPoly::new(composition_poly_trace, domain, num_columns);
        let columns = composition_poly.data();
        let oracle = RowOracle::commit(evaluate_exact_rows(columns, domain), columns.num_cols());
        (Self { oracle }, composition_poly)
    }
}

impl<E, H> ConstraintCommitment<E> for CompactConstraintCommitment<E, H>
where
    E: FieldElement,
    H: ElementHasher<BaseField = E::BaseField> + Sync,
{
    type HashFn = H;
    type VC = MerkleTree<H>;

    fn commitment(&self) -> H::Digest {
        self.oracle.commitment()
    }

    fn query(self, positions: &[usize]) -> Queries {
        self.oracle.query(positions)
    }
}
