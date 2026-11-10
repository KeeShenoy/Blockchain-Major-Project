# Benchmarking method

How every reported number is produced, what it means, and why earlier numbers disagreed with each
other.

## Running it

```bash
python3 evaluation/run_benchmarks.py
```

This single command, on Linux or WSL:

1. Builds the binaries: the current code as released (`dynamic`), allocator-instrumented
   (`alloc-profile`), stage-instrumented (`stage-profile`), Winterfell's field operators in Rescue
   (`generic-arith`), `x86-64-v3` code generation (`v3`), Blake3-192 commitments (`blake3-192`),
   statically linked (`static`), the **previous** revision's prover
   (`generic-field-arithmetic,previous-compact-storage`), and the **original** implementation. The
   original is extracted from `evaluation/reference/original-implementation.tar.gz`, and
   `instrument_original.py` replaces only its `main()` with the same measurement code. Its trace
   builder, prover, AIR, options, and verifier run unmodified.
2. Copies the binaries to local storage. Running them from `/mnt/c` under WSL adds page-fault
   latency to first runs.
3. Runs every configuration in a **fresh process**, pinned to one CPU core, in an order that rotates
   every run and reverses on odd runs. Each process is also wrapped in `/usr/bin/time` as an
   external RSS check.
4. Writes `evaluation/results/<timestamp>/`: `raw.jsonl` (every record), `runs.csv` (one flattened
   row per process), `summary.csv` and `summary.json` (median, mean, min, max, and standard
   deviation per configuration and metric), `comparisons.csv` (paired differences), `stages.json`,
   `metadata.json` (revision, compiler, kernel, CPU, parameters), `report.md` (headline tables), and
   `charts/*.svg`. Earlier result directories are never modified.

The `zk` mode draws fresh masks for every proof, so its proof size varies from proof to proof as the
query positions do, independently of the witness. Its proving time includes mask generation.

Charts and `report.md` are generated from that run's data; nothing is entered by hand.
`python3 evaluation/run_benchmarks.py --from-raw <directory>` rebuilds every derived file from a
run's `raw.jsonl`, without measuring again. `plot_graphs.py <directory>` regenerates just the charts.

**"Previous"** is an ablation of the prior revision's two performance choices that the code still
expresses: Winterfell's field operators in Rescue, and the shallower partial tree. It keeps this
revision's exact-width trace rows, so its heap (170,015 B) is below the 177,567 B the prior revision
recorded in `20261105T191200Z`. `docs/results.md` quotes both.

All configurations use the same release workload, witnesses, roots, and security parameters (28
queries, blowup 8, no grinding, no field extension). Only the FRI shape and prover storage differ.

| Experiment | What varies | Runs |
|---|---|---|
| `main` | 12 configurations at depth 7, one fixed witness | 15 processes each |
| `heap` | allocator-instrumented builds at depth 7 | 3 each (deterministic) |
| `witness` | 30 different depth-7 witnesses | 1 process per witness and configuration |
| `scaling` | depths 3, 7, 15, 31, 63 | 5 processes each, 1 heap run |
| `stages` | where proving time goes, depths 7 and 31 | 200 proofs per configuration |
| `sweep` | FRI folding 2/4/8/16 × remainder 31/63/127/255, depths 7, 15, 31 | 8 witnesses at depth 7, 3 repeats otherwise |

## Metric definitions

Every metric carries its unit in its name. Times are measured with `std::time::Instant` and kept in
integer nanoseconds. Means are floating point. Nothing is rounded to milliseconds before
aggregation.

| Metric | Definition |
|---|---|
| `prove_end_to_end_ns` | `trace_build_warm_median_ns + prove_warm_median_ns`: everything a holder runs to turn a parsed witness into a proof object. This is the fairest proving figure to compare with published end-to-end numbers. |
| `prove_first_ns` | Wall time of `Prover::prove` for the first proof in a fresh process. Includes first-touch page faults on code and data. Excludes trace building. |
| `prove_warm_median_ns` | Median of 30 further `prove` calls (trace building excluded) in the same process. This is the steady-state proving cost. |
| `trace_build_ns`, `trace_build_warm_median_ns` | Building the execution trace from the witness: for the first proof, and the median over the warm proofs. Rescue's inverse S-box dominates this. |
| `verify_median_ns`, `verify_mean_ns`, `verify_total_ns` | Each sample times one call of `verify` (Winterfell verification plus the parameter and depth checks). There are 10 warm-up and 100 timed samples. The proof copy that `verify` consumes is made **outside** the timer. |
| `verify_first_ns` | The first verification in the process. |
| `proof_deserialize_ns` | `Proof::from_bytes` on the serialized proof. |
| `proof_size_bytes` | Length of `Proof::to_bytes()`, the exact bytes sent to a verifier. |
| `peak_rss_after_prove_bytes` | `VmHWM` from `/proc/self/status` right after the first proof: the peak resident set of the proving process, before any verification code runs. |
| `peak_rss_process_bytes` | `VmHWM` at exit: the whole process, including verification. |
| `rss_anon_after_prove_bytes`, `rss_file_after_prove_bytes` | Current anonymous resident memory (heap, stack) and file-backed resident memory (mapped code and libraries) after the first proof. |
| `external_peak_rss_bytes` | `ru_maxrss` of the child as reported by `/usr/bin/time -f %M`. A cross-check on `peak_rss_process_bytes`. |
| `prove_stages_mean_ns` | Mean time per warm proof inside each Winterfell tracing span, from the `stage-profile` build. Spans nest; the top-level ones cover 95-98% of `prove`, the rest being Winterfell's setup between spans. |
| `heap_peak_prove_bytes` | Peak live bytes through Rust's global allocator during the first proof, from the `alloc-profile` build. This is **not** RSS: it excludes code, libraries, stack, and allocator overhead, and it isolates the memory the proving algorithm itself holds. |

RSS is reported in bytes, at kernel page granularity (4 KiB). 1 MiB = 1,048,576 bytes.

### Paired comparisons

Each comparison pairs two configurations by run index, or by witness for the `witness`
experiment, and reports:

- the median of the per-pair differences, and that median as a percentage of the control's median;
- a 95% percentile bootstrap interval for the median difference (10,000 resamples, fixed seed);
- a two-sided exact sign test p-value;
- the number of pairs in which the treatment was lower.

A difference counts as established only when the interval excludes zero.

## Why earlier numbers disagreed

### The 35 MB "peak memory" was cargo's memory

`peak_memory_bytes` came from `getrusage(RUSAGE_SELF).ru_maxrss`. On Linux, `exec` carries the old
address space's high-water mark into the new program. Under `cargo run`, the old address space is
cargo's own, so the prover reported cargo's peak. Measured on the original binary:

| How it was started | Reported `peak_memory_bytes` |
|---|---|
| `cargo run --release -- ...` | 35,303,424 (baseline), 35,004,416 (folded) |
| directly | about 3.7 million |
| from a Python `subprocess` (`os.wait4`) | 11,808,768: Python's own RSS |

The 34,996,224 and 34,988,032 figures in the latest terminal run are therefore cargo's RSS, not the
prover's. The roughly 3.7 MB of the Phase 6 runs were genuine. The near-equality of baseline and
folded in both runs is consistent with this: neither measured the prover's own difference. The
current code reads `VmHWM`, which belongs to the current address space and is unaffected.

### Proof size depends on the witness

Proof size is deterministic for a given witness and mode, but Fiat-Shamir derives the 28 query
positions from the transcript, and the transcript depends on the witness and root. Coinciding
positions are deduplicated in Merkle multi-proofs, so the size varies by roughly ±1 KB between
witnesses. The reported sizes 16,246/17,199, 15,344/17,393, and 16,470/17,233 came from three
different issued witnesses: the issuer draws fresh random credentials on every run. Proof sizes are
therefore compared over 30 fixed witnesses, never one.

### Millisecond rounding and single runs

Proving was recorded as integer milliseconds of a single cold run. A proof takes about 2 ms at
depth 7, so a 1 ms rounding step is a 50% quantum, and the "5 ms → 4 ms" (20%) result in the five-run
table is within that quantization and run-to-run noise. The first proof in a process also pays
page faults, which were larger when the binary ran from `/mnt/c` through cargo. Proving is now
recorded in nanoseconds, cold and warm, over 15 processes.

### Verification timing included a proof clone

Each timed iteration cloned the proof inside the timer, and verification was reported as `0 ms` in
the earliest table because the duration was printed as integer milliseconds. Clones are now outside
the timer, and the median, mean, minimum, and total are kept in nanoseconds. Verification time also
varies by witness, for the same reason proof size does. The fair comparison is therefore the
30-witness experiment.
