> **Historical record** of the previous revision. Current results: [results.md](results.md).

# Results

Run `evaluation/results/20261105T191200Z/` (raw records, CSV, JSON, charts). Machine: Intel Core
i5-8250U, WSL2 kernel 6.18, rustc 1.97.1, one pinned core. Workload: tree depth 7 (128 credentials),
28 queries, blowup 8, no grinding, remainder degree 31. Conjectured security is 83 bits and proven
security 41 (LDR) / 23 (UDR) bits in every configuration.

**Original** is the pre-revision implementation, built from `evaluation/reference/`. **Current** is
this revision. Both are measured by the same code under the definitions in
[`benchmarking.md`](benchmarking.md). Medians come from 15 paired fresh processes unless stated
otherwise. Bracketed values are 95% bootstrap intervals for the median paired difference. An
interval that contains zero means **no established difference**.

## Summary

| Metric (folded mode) | Original | Current | Change |
|---|---:|---:|---|
| Peak proving heap (allocator) | 237,019 B | 177,567 B | **−25.1%** (deterministic, every run) |
| Peak RSS after proving, default build | 3,665,920 B (3.50 MiB) | 3,158,016 B (3.01 MiB) | **−13.6%** [−14.2, −13.4]% |
| Peak RSS after proving, static build | 3,665,920 B | 2,011,136 B (1.92 MiB) | **−45.1%** [−46.0, −44.6]% |
| Peak RSS of whole process (incl. verification) | 3,989,504 B | 3,690,496 B | −7.9% [−8.9, −6.9]% |
| Warm proving time | 1.949 ms | 1.952 ms | +2.9% [−3.3, +7.1]%: not established |
| First proof in process | 2.226 ms | 2.106 ms | −5.4% [−6.6, −2.1]% |
| Verification, 30 witnesses | 406.0 µs | 401.9 µs | +2.0% [−6.3, +6.0]%: not established |
| Proof size, 30 witnesses | 17,344 B | 17,263 B | −0.9% [−2.3, +1.5]%: not established |

The first two rows are the primary objective. The folded prover's own working memory is a quarter
smaller at the same proving time, and its resident memory is 13.6% smaller with the default build,
or 45% smaller with a static build.

## Memory

### Where the memory goes

At depth 7, the proving algorithm's heap is about 6% of the process's resident memory. The rest is
mapped code: the program, libc, libm, and the dynamic loader.

| Original folded, after proving | Bytes |
|---|---:|
| Program code and data (file-backed) | about 1.3 MiB |
| libc, libm, libgcc, ld.so (file-backed) | about 2.1 MiB |
| Heap mapping | 264 KiB |
| Allocator-tracked peak during the proof | 237,019 B |

That split is why two separate optimizations were needed, and why each is reported on its own.

### 1. Algorithmic: compact commitment storage (−25% heap)

`prover-verifier/src/compact.rs` replaces Winterfell's trace and constraint commitment storage
through its `TraceLde` and `ConstraintCommitment` traits:

- **Partial Merkle trees.** Row hashes are computed while building the tree, but not kept. A query
  opening rehashes only the rows it needs, 28 queries × 2 rows per tree.
- **Exact-width composition rows.** Winterfell evaluates columns in groups of 8, so the 5
  composition columns occupied 8 columns of storage. They are now evaluated one column and one coset
  at a time into rows of exact width, which also skips 3 padding columns' worth of FFTs.

Proofs are **byte-identical** to Winterfell's default storage (`compact_prover_output_is_byte_identical`,
depths 3-31). The verifier and soundness are therefore unaffected.

| Heap per phase (current code, depth 7) | Winterfell storage | Compact storage |
|---|---:|---:|
| Trace commitment | 123,147 B | 106,883 B |
| Constraint evaluation | 180,523 B | 164,131 B |
| Composition commitment | 219,387 B | 162,163 B |
| **Peak over the whole proof** | **235,903 B** | **177,567 B** (−24.7%) |

The compact storage adds no measurable proving time: compact vs Winterfell storage, warm, +0.17%
[−6.2, +4.4]%.

The saving holds at every depth:

| Depth (credentials) | Original folded heap | Current folded heap | Change |
|---|---:|---:|---:|
| 3 (8) | 128,739 B | 94,143 B | −26.9% |
| 7 (128) | 237,019 B | 177,567 B | −25.1% |
| 15 (32,768) | 452,309 B | 337,585 B | −25.4% |
| 31 (2^31, synthetic path) | 902,375 B | 672,963 B | −25.4% |
| 63 (2^63, synthetic path) | 1,802,489 B | 1,343,701 B | −25.5% |

**Trade-off not taken.** Dropping one more tree level reaches −31% heap at depth 7 but costs about
1.5% proving time, measured in tuning. Discarding the trace LDE after constraint evaluation, and
recomputing the 28 queried rows from the polynomials, would remove another ~65 KB. That would cost
roughly 15% proving time, so it was not done.

### 2. Build: link-time optimization (−13% RSS)

The release profile now uses `lto = "fat"` and `codegen-units = 1`. This removes about 0.5 MiB of
touched code pages: LTO vs no LTO in folded mode, peak RSS after proving −13.2% [−14.8, −12.7]%. As
a side effect, the first proof is 9.5% faster [−18.1, −2.3]%.

### 3. Optional: static linking (−45% RSS)

Linking statically (`RUSTFLAGS="-C target-feature=+crt-static"`, built with `--target
x86_64-unknown-linux-gnu`) maps no dynamic loader and no shared libraries. Peak RSS after proving
drops to 2,011,136 B, −36.3% against the dynamic current build, at unchanged proving time (−0.8%,
not established). This is a genuine reduction of the process's resident memory. Shared library pages
are, however, shared with other processes, so the saving system-wide is smaller than per-process
RSS suggests. It is therefore reported separately and not made the default. Static builds also change
the output directory, and the team's commands assume `target/release`.

## Proving time: baseline vs folded

| Warm proving, depth 7 | Baseline (FRI 8) | Folded (FRI 16) | Folded vs baseline |
|---|---:|---:|---|
| Original | 1.979 ms | 1.949 ms | −3.0% [−7.4, +1.9]%: not established |
| Current | 1.920 ms | 1.952 ms | +3.8% [+1.0, +4.9]% |

| Warm proving, folded vs baseline (current, 5 runs) | Baseline | Folded |
|---|---:|---:|
| Depth 3 | 0.980 ms | 0.996 ms |
| Depth 15 | 3.937 ms | 3.848 ms |
| Depth 31 | 8.062 ms | 8.212 ms |
| Depth 63 | 15.768 ms | 16.301 ms |

**The folding factor does not measurably speed up proving at these sizes**, in either the original
or the current code. At depth 7, FRI has a single layer in both modes (512 → 64 or 512 → 32
evaluations). Folding by 16 only changes how that one layer is hashed, which is a small part of a
proof dominated by Rescue trace generation, LDE, and constraint evaluation. The run-to-run spread is
about ±10%: the standard deviation of warm medians is about 200 µs on a shared laptop CPU.

The earlier "5 ms → 4 ms (20%)" came from single cold runs recorded in whole milliseconds. It falls
within that spread and that rounding, and it does not reproduce. The current folded prover is not
slower than the original folded prover (+2.9%, not established), so the memory work did not cost
proving time.

## Verification time

Verification time depends on the witness, because the Fiat-Shamir query positions do. Over 30
witnesses at depth 7:

| Median verification | Baseline | Folded |
|---|---:|---:|
| Original | 224.8 µs | 406.0 µs |
| Current | 215.5 µs | 401.9 µs |
| Current vs original | −4.1% [−6.7, −1.2]% | +2.0% [−6.3, +6.0]%: not established |

Folded verification is about 80% slower than baseline in both implementations. This comes from the
folding factor: the verifier interpolates a degree-15 polynomial per query instead of a degree-7
one. That code is inside Winterfell and was not changed. The parameter checks added to the verifier
cost nothing measurable. A single-witness comparison would mislead here: the fixed depth-7 witness
of the `main` experiment shows folded verification 17% faster, which is not borne out across 30
witnesses.

## Proof size

| Median over 30 witnesses | Baseline | Folded |
|---|---:|---:|
| Original | 15,799 B | 17,344 B |
| Current | 15,974 B | 17,263 B |
| Current vs original | +0.8% (not established) | −0.9% (not established) |

Proof size is unchanged. The proof format is Winterfell's, and no security parameter was changed.
The two added boundary assertions and the column blinding do not change the proof's structure. Folded
proofs are 8% larger than baseline: more values per FRI query (16 instead of 8) outweigh the smaller
FRI tree.

Smaller proofs would need a parameter change, for example Blake3-192 commitments (24-byte digests)
or fewer queries with grinding. Either changes the security margin, so neither was done silently.
See "Future work".

## Correctness and security checks run

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (with and
  without `alloc-profile`): clean.
- `cargo test --workspace`: 27 tests passed (5 issuer, 5 merkle-core, 1 unit and 16 integration
  tests in prover-verifier).
- `npm test` in `ethereum/`: 7 tests passed.
- `bash evaluation/run_e2e.sh`: 18 checks passed on a fresh Hardhat node. Covered: the root fetched
  from Ethereum matches the issuer's root; baseline and folded proofs verify against it; wrong root,
  swapped limbs, wrong depth, wrong mode, corrupted proof, tampered witness, and non-canonical
  witness are all rejected; and after a root rotation the old proof is rejected. Log:
  `evaluation/results/e2e-20261105T190805Z/e2e.log`.
- Exhaustive corruption sweep (every byte × 3 bit patterns × 2 modes, 97,164 proofs): 0 accepted,
  0 aborts.

## Future work

- Zero knowledge needs a Winterfell version that supports it, or trace randomization in every
  column plus FRI masking (see `security-and-privacy.md`).
- A verifier nonce in the public inputs would bind proofs to a session and prevent replay.
- Proof size: Blake3-192 commitments would cut about 15-20% of the bytes while keeping the reported
  83-bit conjectured security, but they lower the hash collision resistance from 128 to 96 bits.
  That is a parameter decision for the team.
- Lower memory at a time cost: recomputing queried trace rows instead of storing the trace LDE.
- Larger batches would make the folding-factor comparison more meaningful. At depth 7, FRI has one
  layer in both modes.
