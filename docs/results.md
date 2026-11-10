# Results

Run `evaluation/results/20261109T134412Z/` (raw records, CSV, JSON, `report.md`, 23 charts). Machine:
Intel Core i5-8250U, WSL2 kernel 6.18, rustc 1.97.1, one pinned core. Earlier runs stay in
`evaluation/results/`; the previous revision's write-up is [`results-revision1.md`](results-revision1.md).

Reported security is 83 bits conjectured and 41 / 23 bits proven in every configuration at depth 7
and above (40 / 22 at depth 3, in all configurations including the original). Query count (28),
blowup (8), grinding (0), and field are the same everywhere.

**Original** is the pre-optimization implementation, rebuilt from `evaluation/reference/`.
**Current optimized** is the `optimized` mode. Every implementation is measured by the same code,
under the definitions in [`benchmarking.md`](benchmarking.md). Tables show medians. Bracketed values
are 95% bootstrap intervals for the median paired difference, and an interval that contains zero means
**no established difference**.

## Zero-knowledge cost (run `20261109T175338Z`)

The `zk` mode is the only one with zero knowledge; its construction and proof are in
[`zero-knowledge.md`](zero-knowledge.md). Same machine and method. Depth 7, medians. The sections
further down are the non-zero-knowledge results from the previous run.

| Metric | Original baseline | Optimized, no ZK | **Zero knowledge** | ZK vs optimized | ZK vs original baseline |
|---|---:|---:|---:|---|---|
| Trace build | 810.7 µs | 569.1 µs | 567.4 µs | 0.0% [−0.7, +2.4] | **−29.9%** [−31.2, −24.4] |
| Proving only | 1,875.4 µs | 1,675.0 µs | 2,061.3 µs | **+22.0%** [+21.0, +28.0] | +9.9% [+7.4, +27.0] |
| Trace build + proving | 2,686.0 µs | 2,244.3 µs | 2,628.8 µs | **+16.4%** [+15.5, +22.4] | −2.1% [−4.5, +9.1], not established |
| Verification (30 witnesses) | 214.7 µs | 122.7 µs | 158.8 µs | **+28.6%** [+27.2, +30.5] | **−25.1%** [−29.5, −23.1] |
| Proof size (30 witnesses) | 15,799 B | 12,824 B | 19,045 B | **+48.2%** [+45.3, +51.2] | +19.7% [+18.0, +23.6] |
| Peak proving heap | 235,519 B | 150,770 B | 214,083 B | **+42.0%** | **−9.1%** |
| Peak RSS after proving | 3,665,920 B | 3,452,928 B | 3,801,088 B | +9.7% [+8.1, +10.7] | +3.4% [+0.8, +4.5] |
| Reported security (conj / LDR / UDR bits) | 83 / 41 / 23 | 83 / 41 / 23 | 84 / 41 / 27 | | |

**Zero knowledge costs far less than the 2× previously estimated**, and the evaluation domain does
not change. Against the original baseline, the zero-knowledge prover is about the same speed end to
end, verifies 25% faster, and uses less heap, while adding zero knowledge. Its proof is 20% larger.

The cost comes from 37 queries instead of 28 (the masked code rate is higher, so more queries keep the
security target), three more composition columns plus the batching mask, one extra trace column, and
mask generation.

Proving stages for `zk` at depth 7 (mean of 2,011 µs): constraint evaluation 923 µs, composition LDE
304 µs, trace LDE 270 µs, composition commitment 138 µs, trace commitment 106 µs.

| Depth | Trace + prove | Verification | Proof size | Peak heap | Peak RSS | Queries | Security |
|---|---|---|---|---|---|---|---|
| 7 | 2.242 → 2.633 ms | 121.4 → 160.6 µs | 12,693 → 19,141 B | 150,770 → 214,691 B | 3,320 → 3,700 KiB | 37 | 84 / 41 / 27 |
| 15 | 4.412 → 4.817 ms | 158.7 → 170.8 µs | 15,769 → 19,805 B | 294,116 → 369,709 B | 3,520 → 3,728 KiB | 32 | 84 / 42 / 25 |
| 31 | 8.860 → 9.342 ms | 215.1 → 219.4 µs | 19,739 → 22,283 B | 586,486 → 687,503 B | 3,596 → 4,100 KiB | 30 | 83 / 42 / 24 |
| 63 | 18.650 → 18.973 ms | 255.5 → 322.9 µs | 26,113 → 27,655 B | 1,220,728 → 1,362,049 B | 4,368 → 4,744 KiB | 29 | 83 / 42 / 23 |

Each pair is optimized without ZK → zero knowledge, with five runs per depth and one witness per
depth. The masks are close to a fixed size, so the relative cost shrinks as the trace grows. Depths
below 7 are unsupported.

## Summary without zero knowledge (depth 7, 128 credentials; run `20261109T134412Z`)

| Metric | Original baseline | Original folded | Current optimized | vs original baseline | vs original folded |
|---|---:|---:|---:|---|---|
| Trace build + proving, warm | 2,873.8 µs | 2,727.5 µs | 2,411.5 µs | **−14.0%** [−17.4, −13.5] | **−11.2%** [−19.1, −9.6] |
| Proving only, warm | 2,012.1 µs | 1,970.3 µs | 1,806.7 µs | **−7.8%** [−12.1, −7.3] | **−7.9%** [−16.2, −6.3] |
| Verification (30 witnesses) | 229.6 µs | 414.2 µs | 131.1 µs | **−42.2%** [−44.7, −40.4] | **−66.8%** [−71.2, −64.0] |
| Proof size (30 witnesses) | 15,799 B | 17,344 B | 12,824 B | **−18.2%** [−19.7, −16.9] | **−25.6%** [−27.4, −23.9] |
| Peak proving heap (allocator) | 235,519 B | 237,019 B | 150,770 B | **−36.0%** | **−36.4%** |
| Peak RSS after proving, default build | 3,665,920 B | 3,674,112 B | 3,342,336 B | **−8.8%** [−9.7, −8.5] | **−9.1%** [−10.0, −8.1] |
| Peak RSS after proving, static build | | | 2,236,416 B | −39.0% (ratio of medians) | −39.1% (ratio of medians) |

Heap is deterministic: three runs give the same byte count.

### Against the previous revision

| Metric | Previous revision | Now | Change |
|---|---:|---:|---|
| Peak proving heap | 177,567 B (`folded`, run 20261105) | 150,770 B | **−15.1%** (improved) |
| Peak heap, ablation build in this run | 170,015 B | 150,770 B | **−11.3%** |
| Anonymous RSS after proving | 352,256 B | 331,776 B | **−5.8%** (improved) |
| File-backed RSS after proving | 2,805,760 B | 3,010,560 B | **+7.3%** (worse) |
| Total peak RSS after proving | 3,158,016 B | 3,342,336 B | **+5.8%** (worse) |
| Static-build peak RSS | 2,011,136 B | 2,236,416 B | **+11.2%** (worse) |

**The total RSS figure from the last report is not preserved.** The heap and the anonymous memory
that the prover controls both improved. File-backed memory, meaning mapped program code, grew: the
new routines and modes change which code pages get touched, and the kernel maps file pages in 64 KiB
groups. The original implementation's figures stayed the same across all runs (3.66-3.67 MB), so this
is a property of the build and not the machine. Allocator-tracked heap is the measure of the
algorithm; RSS at this scale is mostly the program image.

## Proving time

### Where the time goes (mean per proof)

| Stage | Baseline, depth 7 | Optimized, depth 7 | Optimized, depth 31 |
|---|---:|---:|---:|
| Trace build (Rescue) | 625 µs | 597 µs | 2,266 µs |
| Constraint evaluation | 862 µs | 806 µs | 3,055 µs |
| Trace LDE + commitment | 347 µs | 334 µs | 1,681 µs |
| Composition LDE + commitment | 358 µs | 320 µs | 1,393 µs |
| FRI layers / remainder | 58 µs | 50 µs | 212 µs |
| DEEP composition + evaluation | 91 µs | 84 µs | 328 µs |
| Queries + proof object | 40 µs | 66 µs | 81 µs |
| **`prove` total (excludes trace build)** | **1,856 µs** | **1,750 µs** | **6,919 µs** |

Rescue dominates: constraint evaluation is 46% of `prove`, and trace building adds about a third on
top. This table is why the work went into arithmetic and not FRI, which is under 3% of `prove`.

### What improved it

| Change | Effect (trace + prove) | Interval |
|---|---|---|
| Exact Rescue arithmetic (lazy MDS, inverse S-box addition chain) | **−14.0%** | [−15.2, −10.7] |
| FRI shape (optimized vs FRI 8, same storage) | −0.6% | [−2.4, +3.9], not established |
| Compact vs Winterfell storage (prove only) | +3.0% | [−0.5, +4.6], not established |
| `x86-64-v3` code generation | −4.9% | [−9.0, +0.2], not established |
| Static linking | −2.9% | [−6.2, +0.1], not established |

**The folding factor does not speed up proving at these sizes.** Prove-only, folded vs baseline is
−0.5% [−2.7, +2.4] in the original code and +2.3% [−3.7, +6.3] in the current code. The original's
−4.2% end-to-end difference is in trace building, where its baseline kept a redundant branch vector.

Not adopted: the `concurrent` feature (Winterfell only parallelizes constraint evaluation above a
domain of 8192, and the largest here is 4096), and `x86-64-v3` (not established, and it reduces
portability).

## Verification

Verification depends on the witness, through the Fiat-Shamir query positions, so the fair comparison
is across 30 witnesses. Folded mode is 77-78% slower than baseline in both code bases, because the
verifier interpolates a polynomial per query, and that cost grows quadratically with the folding
factor. The sweep at depth 7 with remainder degree 31 shows it directly:

| FRI folding factor | 2 | 4 | 8 | 16 |
|---|---:|---:|---:|---:|
| Verification | 149.0 µs | 168.1 µs | 213.8 µs | 361.7 µs |

Removing the folding layer is better than any folding factor: about 118-143 µs across shapes with no
layer. It works because the DEEP polynomial is small enough to send directly, and it drops the
layer's Merkle openings and interpolation while keeping every query check.

**Decision:** factor 16 is not the recommended configuration. It offers no proving-time gain, and
costs about 78% more verification and about 9% more proof bytes. The `optimized` mode uses remainder degree
`min(trace length − 1, 255)`, which means no layer up to depth 31, and folding factor 4 beyond that.

## Proof size

| Change | Effect | Note |
|---|---|---|
| FRI shape | **−18.0%** [−19.0, −16.3] vs FRI 8, 30 witnesses | Adopted. Security unchanged. |
| Compact storage | 0 | Byte-identical. |
| Blake3-192 commitments | −10.4% [−14.3, −9.2] further | **Not adopted.** Reported security stays 83 bits, but commitment collision resistance falls from 128 to 96 bits. A decision for the team. |

Of the 12.7 KB, roughly 3.1 KB is opened trace rows, 2.2 KB opened composition rows, and 1 KB the
remainder polynomial. By subtraction, Merkle authentication data is about 5.8 KB, already compressed
by batch multiproofs. Going further would mean fewer queries, a smaller field, or a weaker hash. No
redundant proof field was found; serialization adds a few bytes per length prefix.

## Memory

| Peak heap per phase, depth 7 | Winterfell storage | Compact storage |
|---|---:|---:|
| Whole proof, FRI 16 | 236,543 B | 156,135 B (**−34.0%**) |
| Whole proof, optimized | | 150,770 B |

Two changes were added to the compact storage: trace rows at exact width, and one more unstored
Merkle level. The extra level costs 28 × 4 row rehashes per commitment at query time, which is within
measurement noise.

Declined trade-offs are recorded in [`methodology.md`](methodology.md): statement splitting (about
k× proof and verification cost for 1/k memory) and coset-partitioned evaluation (an estimated 20%
more proving time for about 60 KB less heap).

## Scalability (original baseline → current optimized)

Five runs per depth, one witness per depth, so proof size carries witness variation.

| Depth (credentials) | Trace + prove | Verification | Proof size | Peak heap | Peak RSS |
|---|---|---|---|---|---|
| 3 (8) | 1.550 → 1.241 ms | 116.0 → 103.7 µs | 11,225 → 10,323 B | 128,591 → 84,642 B | 3,484 → 3,180 KiB |
| 7 (128) | 2.894 → 2.363 ms | 230.6 → 125.8 µs | 15,542 → 12,693 B | 235,519 → 150,770 B | 3,588 → 3,236 KiB |
| 15 (32,768) | 5.979 → 4.711 ms | 295.8 → 171.1 µs | 19,133 → 15,769 B | 458,393 → 294,116 B | 3,628 → 3,428 KiB |
| 31 (synthetic path) | 12.528 → 10.693 ms | 302.0 → 257.7 µs | 20,475 → 19,739 B | 914,603 → 586,486 B | 4,176 → 3,524 KiB |
| 63 (synthetic path) | 23.311 → 19.254 ms | 408.4 → 273.3 µs | 27,389 → 26,113 B | 1,827,005 → 1,220,728 B | 5,088 → 4,284 KiB |

Proving and heap grow linearly with trace length. Verification and proof size grow slowly. The
proof-size saving shrinks at depth 63, where one folding layer returns. Depths 31 and 63 use a
generated path, because such trees cannot be built; proving cost depends only on the path.

Charts: `compare_*.svg` (configurations at depth 7), `scaling_*.svg` (time, verification, size, heap,
RSS vs depth), `sweep_*.svg` (FRI shapes), `stages_*.svg`, `builds_*.svg`.

## Comparison with published work

These systems prove different statements with different guarantees. **The table explains why the
numbers differ; it does not rank the systems.** Published values are as the papers report them.

| | This project, `optimized` | Ligerolight (Paper 8) | SoK STARK (Paper 15) |
|---|---|---|---|
| Statement | Merkle path, depth 31 | Merkle path, depth 32 | FibonacciSq, 1023 steps |
| Hash inside the proof | Rescue (algebraic) | SHA-256 (bitwise) | None; hashes only for commitments |
| Size of the statement | 256 rows × 7 columns; 32 hashes | 897,551 constraints; about 27,000 per hash | 1,024 trace elements |
| Proof system | Winterfell STARK (AIR + FRI) | Ligero-style IOP with batch zk-IPA | Tutorial-style STARK prototype |
| Zero knowledge | Only in `zk` mode (figures below are `optimized`) | Yes | Not evaluated |
| Field | 128-bit | 64-bit (2^64 − 2^32 + 1) | 32-bit (3,221,225,473) |
| Security | 83 bits conjectured (84 for `zk` at depth 7), 41 proven | 100 bits (conjectured FRI soundness), 33 queries | Not stated as a level |
| Commitment hash | Blake3-256 | Merkle trees (hash not compared here) | SHA3-256 |
| Language | Rust | C++ | Go |
| Hardware | i5-8250U, one core, WSL2 | Ryzen 3900X, one core, 64 GB | Apple M1 (also Pi, x86-64, emulated PPC64) |
| Averaging | 15 paired processes, 5 for scaling | Mean of 10 runs | Averages; run count not given |
| **Proving** | **10.7 ms** (depth 31, trace + prove) | **160 s** | **3.81 s** (M1); 26.5 s (x86-64) |
| **Verification** | **0.26 ms** | **1.75 s** | **472 ms** (M1); 3.08 s (x86-64) |
| **Proof size** | **19.7 KB** | **131 KB** | **68.6 KB** |

### Why ours is so much smaller and faster

- **The statement is hundreds of times smaller.** SHA-256 costs about 27,000 constraints per hash
  because it is bitwise. Rescue was designed for this: one hash is 8 rows of 7 columns. Ligerolight's
  depth-32 circuit has 897,551 constraints against our 1,785 transition checks over 255 steps. This
  is the main reason, and it reflects the choice of hash, not a better proof system.
- **The settings differ.** Ligerolight is zero knowledge at 100-bit security. The figures above are
  for our `optimized` mode at 83 bits, without zero knowledge. Our `zk` mode at depth 31 takes 9.3 ms
  and 22.3 KB, at 83 bits.
- **Paper 15 tested an unoptimized teaching prototype.** It states that its implementation does not
  use FFT-based optimizations for polynomial evaluation or Merkle tree construction, and that its
  AIR-like checking is limited. Its numbers are not a performance target for a production-grade
  library such as Winterfell.

### Is the measurement complete?

Yes, for the operations listed. Proving includes the execution trace, and everything in Winterfell's
`prove`: LDE, commitments, constraint evaluation, DEEP, FRI, and building the proof object. The stage
profile accounts for 95-98% of it. Verification is the full `winterfell::verify` with parameter
checks; the separately measured decode adds about 18 µs. Not included: reading the witness file and
writing proof bytes to disk. The system does perform the work: 139,761 corrupted proofs were all
rejected.

Fair to say: this is a fast prototype for its own statement, with an algebraic hash. Not fair to say:
that it outperforms Ligerolight.

## Correctness and security checks

All run on the final code:

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` with each
  measurement feature: clean.
- `cargo test --workspace`: 56 tests passed (5 issuer, 10 merkle-core, and in prover-verifier 5 unit,
  20 correctness, and 16 zero-knowledge).
- `npm test`: 7 passed.
- `bash evaluation/run_e2e.sh`: all checks passed, including the `zk` mode (`evaluation/results/e2e-20261109T174300Z/`). The
  root fetched from Ethereum is used; baseline, folded, and optimized proofs verify against it; wrong
  roots, swapped limbs, wrong depth, wrong mode, corrupted proofs, tampered witnesses, and non-canonical
  witnesses are rejected; a session-bound proof is rejected in another session; and an old batch's
  proof is rejected after a root rotation.
- Exhaustive corruption sweep (every byte × 3 bit patterns, 3 modes, 139,761 proofs): 0 accepted, 0
  aborts. This sweep found the partition-field malleability described in
  [`security-and-privacy.md`](security-and-privacy.md).

## Objective-wise assessment

| Objective | Status | Evidence | Limits |
|---|---|---|---|
| 1. Eliminate trusted setup | **Achieved** | Hash-based Winterfell STARK; no setup step | |
| 2. Reduce prover memory through witness partitioning and memory optimization | **Achieved for memory; partial for the method** | Heap −36.0%; RSS −8.8% default, −39.0% static | The witness itself is not partitioned; the saving comes from commitment storage. RSS is mostly program image at this scale. |
| 3. Improve proving time through workflow optimization and efficient batching | **Achieved for time; partial for batching** | Trace + prove −14.0% against the original baseline | The gain is arithmetic, not batching. Batching is Winterfell's existing multiproofs and DEEP, used equally in the baseline. |
| 4. Maintain efficient verification, security, correctness, transparency | **Achieved** | Verification −42.2% (`optimized`) or −25.1% (`zk`) against the original baseline; soundness bugs fixed; security held; replay protection added; zero knowledge in `zk` mode | Zero knowledge holds in the random-oracle model, only in `zk` mode |
| 5. Evaluate against a baseline | **Achieved** | Paired benchmarks, 5 depths, automatic CSV/JSON/graphs | Single machine; depths 31 and 63 use generated paths |

## Trade-offs

| Decision | Gain | Cost |
|---|---|---|
| Compact storage | −34% heap, byte-identical proofs | +3% prove time, not statistically established |
| No FRI folding layer | −18% proof, −39% verification | Remainder sent in the clear (about 1 KB at depth 7); no security change |
| Session binding | Replay protection when used | Two extra public inputs; no measurable cost |
| Column-6 blinding | Hides the leaf index from the column attack | Weaker constraint on free rows, by design |
| Blake3-192 (not adopted) | −10.4% proof | Collision resistance 128 → 96 bits |
| Zero knowledge (`zk` mode) | Transcript simulatable without the witness | Against `optimized`: +16.4% proving, +28.6% verification, +48.2% proof size, +42.0% heap |
