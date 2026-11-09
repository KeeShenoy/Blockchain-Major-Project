> **Historical record.** These measurements predate the corrections described in
> `docs/benchmarking.md` (millisecond rounding, single runs, witness-dependent proof sizes, and
> `ru_maxrss` inheritance). The script they used is archived in
> `evaluation/reference/original-implementation.tar.gz`. Current results: `docs/results.md`.

# Phase 6 measurement workflow

The benchmark compares the current `baseline` mode (FRI folding factor 8) and
`folded` mode (FRI folding factor 16) using one already-issued private witness
and the corresponding root fetched from the local Ethereum registry. It does
not regenerate issuer data while measuring.

## Changes measured

The witness loader now deserializes from a buffered file reader and consumes the
deserialized witness into field elements and Rescue hashes. This releases the
string-backed JSON witness representation before trace construction and proving.
The direct-path construction used by `split` and `folded` is unchanged.

Verification is measured with `Instant` over a warm-up phase and a timed batch.
Every iteration clones the proof and calls Winterfell verification; the reported
mean therefore measures an operational verification invocation, not a cached
success. The output includes total and mean nanoseconds, warm-up count, and
timed iteration count. Proof generation remains measured only around
`prover.prove(trace)`.

## Run the benchmark

Start Hardhat, deploy, issue one witness, publish its root, and fetch the root
as described in the project demonstration runbook. Then, from the project root:

```bash
cd ethereum
FETCHED="$(npm run fetch:root)"
ROOT_LIMB0="$(printf '%s\n' "$FETCHED" | sed -n 's/^root_limb0: //p')"
ROOT_LIMB1="$(printf '%s\n' "$FETCHED" | sed -n 's/^root_limb1: //p')"
cd ..

ROOT_LIMB0="$ROOT_LIMB0" ROOT_LIMB1="$ROOT_LIMB1" \
  bash evaluation/run_phase6_benchmarks.sh 7
```

The script compiles the release binary before measurement, alternates baseline
and folded samples, and writes `metadata.json`, per-run JSON and `/usr/bin/time
-v` output, JSONL, and CSV below `evaluation/output/`. Compilation time is not
included in any record. `whole_process_peak_rss_bytes` comes from `/usr/bin/time
-v`; the Rust `peak_memory_bytes` field is separately instrumented process peak
RSS and must not be substituted for the external measurement.

Use `VERIFY_WARMUP_ITERATIONS` and `VERIFY_TIMED_ITERATIONS` to configure
verification batches; defaults are 20 and 200. The benchmark records the
configuration in metadata and every result row.

## Interpretation

At depth 7, the released witness strings are tiny compared with Winterfell's
trace and proving allocations. A lower RSS sample is not a demonstrated memory
improvement unless the median difference exceeds run-to-run variability. Report
proof generation, verification latency, whole-process RSS, and proof size as
separate metrics.

## Fresh measurement snapshot

The following measurements were collected on the canonical WSL2 workspace
(`Linux ... microsoft-standard-WSL2`, Rust 1.97.1, release profile) using the
same depth-7 witness and public root for both modes. Pre-change measurements
used the prior single-verification implementation; their verification timing is
not comparable at sub-millisecond precision and is therefore unavailable.

| Metric | Baseline before (n=7) | Folded before (n=7) | Baseline after (n=7) | Folded after (n=7) |
| --- | ---: | ---: | ---: | ---: |
| Median proving time | 11 ms | 11 ms | 12 ms | 10 ms |
| Proving-time range | 8-35 ms | 7-29 ms | 10-28 ms | 10-17 ms |
| Whole-process peak RSS median | 3,837,952 B | 3,780,608 B | 3,731,456 B | 3,706,880 B |
| Whole-process RSS range | 3,768,320-3,911,680 B | 3,620,864-3,833,856 B | 3,567,616-3,883,008 B | 3,493,888-3,866,624 B |
| Proof size | 15,344 B | 17,393 B | 15,344 B | 17,393 B |
| Median verification latency | N/A | N/A | 459,068.855 ns | 802,766.740 ns |

The after-run RSS standard deviation was 117,550 B for baseline and 139,682 B
for folded. The 24,576 B folded-versus-baseline median RSS difference, and the
73,728 B folded before-versus-after difference, are smaller than that noise.
This optimization removes duplicate witness ownership but does **not** establish
a measurable whole-process peak-RSS reduction at depth 7. The fresh proving-time
medians retain a folded advantage (10 ms versus 12 ms); the individual samples
remain variable and should not be generalized beyond this benchmark.

Correctness checks completed after the changes: `cargo fmt --all -- --check`,
`cargo check --workspace`, `cargo test -p prover-verifier` (6 tests), `cargo
test -p issuer` (3 tests), valid baseline/folded Ethereum-root verification,
and wrong-root rejection.
