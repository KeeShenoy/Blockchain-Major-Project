#!/usr/bin/env python3
"""Paired benchmark of the original and current provers, in baseline and folded modes.

Run from the repository root on Linux or WSL (Winterfell does not build on native Windows):

    python3 evaluation/run_benchmarks.py                # full run, about 10 minutes
    python3 evaluation/run_benchmarks.py --quick        # smoke test, about 1 minute

Everything is written to evaluation/results/<UTC timestamp>/: raw records, CSV and JSON summaries,
paired comparisons, report.md, and charts/*.svg. See docs/benchmarking.md for what each metric
measures. Only the Python standard library is required.

Experiments
  main     depth 7, every configuration, --runs fresh processes each, in rotating order
  heap     depth 7, allocator-instrumented builds (deterministic, so few runs are needed)
  witness  depth 7, one process per witness for 30 witnesses: proof size and verification time
           depend on the Fiat-Shamir query positions, which depend on the witness
  scaling  depths 3 to 63, original, previous, and current
  stages   where proving time goes, from Winterfell's tracing spans
  sweep    FRI folding factor and remainder degree: the evidence behind the optimized mode

Implementations
  original  the code before any of this work (evaluation/reference/)
  previous  the prior revision's prover: compact storage with the shallower partial tree, and
            Winterfell's field operators for Rescue
  current   this revision
"""

import argparse
import csv
import json
import math
import os
import pathlib
import platform
import random
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone

REPO = pathlib.Path(__file__).resolve().parent.parent
FIXTURES = REPO / "evaluation" / "fixtures"
MAIN_DEPTH = 7
SCALING_DEPTHS = (3, 7, 15, 31, 63)
WITNESS_SEEDS = range(1001, 1031)

# (implementation, build, mode). Build "dynamic" is the default `cargo build --release`.
MAIN_CONFIGS = [
    ("original", "dynamic", "baseline"),
    ("original", "dynamic", "folded"),
    ("previous", "dynamic", "split"),
    ("previous", "dynamic", "folded"),
    ("current", "dynamic", "baseline"),
    ("current", "dynamic", "split"),
    ("current", "dynamic", "folded"),
    ("current", "dynamic", "optimized"),
    ("current", "dynamic", "zk"),
    ("current", "generic-arith", "optimized"),
    ("current", "v3", "optimized"),
    ("current", "static", "optimized"),
]
HEAP_CONFIGS = [
    ("original", "alloc-profile", "baseline"),
    ("original", "alloc-profile", "folded"),
    ("previous", "alloc-profile", "split"),
    ("previous", "alloc-profile", "folded"),
    ("current", "alloc-profile", "baseline"),
    ("current", "alloc-profile", "split"),
    ("current", "alloc-profile", "folded-default"),
    ("current", "alloc-profile", "folded"),
    ("current", "alloc-profile", "optimized"),
    ("current", "alloc-profile", "zk"),
]
COMPARED_CONFIGS = [
    ("original", "dynamic", "baseline"),
    ("original", "dynamic", "folded"),
    ("previous", "dynamic", "folded"),
    ("current", "dynamic", "baseline"),
    ("current", "dynamic", "split"),
    ("current", "dynamic", "folded"),
    ("current", "dynamic", "optimized"),
    ("current", "dynamic", "zk"),
]
SWEEP_FOLDING = (2, 4, 8, 16)
SWEEP_REMAINDER = (31, 63, 127, 255)
SWEEP_DEPTHS = (7, 15, 31)

# Metric name -> path into a bench record. Units are in the names.
METRICS = {
    "prove_end_to_end_ns": ("prove_end_to_end_ns",),
    "prove_first_ns": ("prove_first_ns",),
    "prove_warm_median_ns": ("prove_warm", "median_ns"),
    "prove_warm_mean_ns": ("prove_warm", "mean_ns"),
    "prove_warm_min_ns": ("prove_warm", "min_ns"),
    "trace_build_ns": ("trace_build_ns",),
    "trace_build_warm_median_ns": ("trace_build_warm", "median_ns"),
    "verify_first_ns": ("verify_first_ns",),
    "verify_median_ns": ("verify", "median_ns"),
    "verify_mean_ns": ("verify", "mean_ns"),
    "verify_min_ns": ("verify", "min_ns"),
    "verify_total_ns": ("verify", "total_ns"),
    "verify_iterations": ("verify", "iterations"),
    "proof_deserialize_ns": ("proof_deserialize_ns",),
    "proof_size_bytes": ("proof_size_bytes",),
    "peak_rss_after_prove_bytes": ("memory_after_first_prove", "peak_rss_bytes"),
    "rss_anon_after_prove_bytes": ("memory_after_first_prove", "rss_anon_bytes"),
    "rss_file_after_prove_bytes": ("memory_after_first_prove", "rss_file_bytes"),
    "peak_rss_process_bytes": ("memory_at_exit", "peak_rss_bytes"),
    "external_peak_rss_bytes": ("external_peak_rss_bytes",),
    "heap_peak_prove_bytes": ("heap_peak_first_prove_bytes",),
}


def log(message):
    print(message, file=sys.stderr, flush=True)


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, capture_output=True, **kwargs)


def target_dir():
    return pathlib.Path(os.environ.get("CARGO_TARGET_DIR", REPO / "target"))


# BUILDS
# ------------------------------------------------------------------------------------------------

def cargo_build(manifest_dir, target, features="", env_extra=None, triple=None):
    command = ["cargo", "build", "--release", "-p", "prover-verifier"]
    if features:
        command += ["--features", features]
    else:
        command += ["-p", "issuer"]
    if triple:
        command += ["--target", triple]
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), **(env_extra or {}))
    result = subprocess.run(command, cwd=manifest_dir, env=env, text=True, capture_output=True)
    if result.returncode:
        sys.exit(f"build failed in {manifest_dir}:\n{result.stderr[-3000:]}")
    return target / triple / "release" if triple else target / "release"


def build_all(binary_dir):
    """Builds every variant and copies the binaries to a local directory.

    Running copies from local storage avoids page faults on a slow mount (such as /mnt/c under
    WSL) inflating first-run timings."""
    base = target_dir() / "bench"
    triple = "x86_64-unknown-linux-gnu"
    previous = "generic-field-arithmetic,previous-compact-storage"
    specs = {
        ("current", "dynamic"): (REPO, "dynamic", "", {}, None),
        ("current", "alloc-profile"): (REPO, "alloc", "alloc-profile", {}, None),
        ("current", "stage-profile"): (REPO, "stage", "stage-profile", {}, None),
        ("current", "generic-arith"): (REPO, "generic", "generic-field-arithmetic", {}, None),
        ("current", "blake3-192"): (REPO, "blake3-192", "blake3-192", {}, None),
        ("current", "v3"): (REPO, "v3", "", {"RUSTFLAGS": "-C target-cpu=x86-64-v3"}, None),
        ("current", "static"): (
            REPO, "static", "", {"RUSTFLAGS": "-C target-feature=+crt-static"}, triple),
        ("previous", "dynamic"): (REPO, "previous", previous, {}, None),
        ("previous", "alloc-profile"): (
            REPO, "previous-alloc", f"{previous},alloc-profile", {}, None),
    }
    original_source = base / "original-source"
    run([sys.executable, str(REPO / "evaluation" / "reference" / "instrument_original.py"),
         str(original_source)])
    specs[("original", "dynamic")] = (original_source, "original", "", {}, None)
    specs[("original", "alloc-profile")] = (
        original_source, "original-alloc", "alloc-profile", {}, None)

    binaries = {}
    issuer = binary_dir / "issuer"
    for (implementation, build), (source, target, features, env, spec_triple) in specs.items():
        log(f"building {implementation} ({build})")
        directory = cargo_build(source, base / target, features, env, spec_triple)
        destination = binary_dir / f"{implementation}-{build}"
        shutil.copy(directory / "prover-verifier", destination)
        binaries[(implementation, build)] = destination
        if (implementation, build) == ("current", "dynamic"):
            shutil.copy(directory / "issuer", issuer)
    return binaries, issuer


# FIXTURES
# ------------------------------------------------------------------------------------------------

def fixture(issuer, depth, seed):
    """Returns a witness file, issuing it deterministically if it is not committed yet."""
    directory = FIXTURES / f"depth{depth}-seed{seed}"
    files = sorted(directory.glob("witness_*.json"))
    if not files:
        directory.mkdir(parents=True, exist_ok=True)
        if depth <= 15:
            # A real batch: every holder index exists. Holder 0 keeps fixtures comparable.
            run([str(issuer), str(depth), "0", "--seed", str(seed), "--output", str(directory)])
        else:
            run([str(issuer), "--synthetic", str(depth), "--seed", str(seed),
                 "--output", str(directory)])
        files = sorted(directory.glob("witness_*.json"))
    return files[0]


# MEASUREMENT
# ------------------------------------------------------------------------------------------------

def pinned(command):
    """Pins to one CPU core when possible, to reduce scheduler noise."""
    if shutil.which("taskset") and os.cpu_count() and os.cpu_count() > 2:
        return ["taskset", "-c", str(os.cpu_count() - 1)] + command
    return command


def bench(binary, witness, mode, prove_warm, verify_warmup, verify_iterations, extra=()):
    command = [str(binary), "bench", "--witness", str(witness), "--mode", mode,
               "--prove-warm", str(prove_warm), "--verify-warmup", str(verify_warmup),
               "--verify-iterations", str(verify_iterations), *extra]
    time_tool = shutil.which("time") and pathlib.Path("/usr/bin/time").exists()
    with tempfile.NamedTemporaryFile("r") as time_output:
        if time_tool:
            command = ["/usr/bin/time", "-f", "%M", "-o", time_output.name] + command
        result = subprocess.run(pinned(command), text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(f"{' '.join(command)} failed:\n{result.stderr}")
        record = json.loads(result.stdout)
        if time_tool:
            # /usr/bin/time reports the child's ru_maxrss in KiB. Its own resident set (about
            # 1.2 MiB) is a floor carried across exec, far below the measured peaks.
            record["external_peak_rss_bytes"] = int(time_output.read().strip()) * 1024
    return record


def metric(record, path):
    value = record
    for key in path:
        if value is None:
            return None
        value = value.get(key)
    return value


def flatten(record):
    row = {name: metric(record, path) for name, path in METRICS.items()}
    # Everything a holder runs between having a witness and having proof bytes.
    trace, prove = row["trace_build_warm_median_ns"], row["prove_warm_median_ns"]
    row["prove_end_to_end_ns"] = trace + prove if trace and prove else None
    for key in ("experiment", "implementation", "build", "mode", "run", "order", "witness_seed",
                "tree_depth", "fri_folding_factor", "fri_remainder_max_degree", "num_queries",
                "blowup_factor",
                "conjectured_security_bits", "proven_security_ldr_bits",
                "proven_security_udr_bits", "fri_layers", "proof_digest", "verified"):
        row[key] = record.get(key)
    return row


class Recorder:
    def __init__(self, directory):
        self.records = []
        self.raw = open(directory / "raw.jsonl", "w")

    def add(self, record, **labels):
        record.update(labels)
        record["timestamp_utc"] = datetime.now(timezone.utc).isoformat()
        if not record.get("verified"):
            raise RuntimeError(f"a valid proof failed to verify: {labels}")
        self.records.append(record)
        self.raw.write(json.dumps(record, sort_keys=True) + "\n")
        self.raw.flush()


def experiment_main(recorder, binaries, witness, runs, iterations):
    for run_index in range(runs):
        # Rotate the order every run, and reverse it on odd runs, so that no configuration is
        # systematically first, last, or after the same neighbour.
        order = MAIN_CONFIGS[run_index % len(MAIN_CONFIGS):] + MAIN_CONFIGS[:run_index % len(MAIN_CONFIGS)]
        if run_index % 2:
            order.reverse()
        for position, (implementation, build, mode) in enumerate(order):
            record = bench(binaries[(implementation, build)], witness, mode, *iterations)
            recorder.add(record, experiment="main", implementation=implementation, build=build,
                         mode=mode, run=run_index, order=position, witness_seed=7)
        log(f"  main: run {run_index + 1}/{runs}")


def experiment_heap(recorder, binaries, witness, runs):
    for run_index in range(runs):
        for implementation, build, mode in HEAP_CONFIGS:
            record = bench(binaries[(implementation, build)], witness, mode, 0, 0, 1)
            recorder.add(record, experiment="heap", implementation=implementation, build=build,
                         mode=mode, run=run_index, order=0, witness_seed=7)
    log("  heap: done")


def experiment_witnesses(recorder, binaries, issuer, verify_iterations):
    configs = COMPARED_CONFIGS + [("current", "blake3-192", "optimized")]
    for index, seed in enumerate(WITNESS_SEEDS):
        witness = fixture(issuer, MAIN_DEPTH, seed)
        order = configs if index % 2 == 0 else list(reversed(configs))
        for position, (implementation, build, mode) in enumerate(order):
            record = bench(binaries[(implementation, build)], witness, mode, 0, 5,
                           verify_iterations)
            recorder.add(record, experiment="witness", implementation=implementation,
                         build=build, mode=mode, run=index, order=position, witness_seed=seed)
    log("  witness: done")


def experiment_scaling(recorder, binaries, issuer, runs):
    for depth in SCALING_DEPTHS:
        witness = fixture(issuer, depth, 7)
        # Zero knowledge needs a trace longer than the masks: see ZkParameters::for_depth.
        configs = [c for c in COMPARED_CONFIGS if c[2] != "zk" or depth >= 7]
        for run_index in range(runs):
            order = configs if run_index % 2 == 0 else list(reversed(configs))
            for position, (implementation, build, mode) in enumerate(order):
                record = bench(binaries[(implementation, build)], witness, mode, 10, 5, 30)
                recorder.add(record, experiment="scaling", implementation=implementation,
                             build=build, mode=mode, run=run_index, order=position,
                             witness_seed=7)
        for implementation, _, mode in configs:
            record = bench(binaries[(implementation, "alloc-profile")], witness, mode, 0, 0, 1)
            recorder.add(record, experiment="scaling-heap", implementation=implementation,
                         build="alloc-profile", mode=mode, run=0, order=0, witness_seed=7)
        log(f"  scaling: depth {depth} done")


def experiment_stages(recorder, binaries, issuer, prove_warm):
    binary = binaries[("current", "stage-profile")]
    for depth in (MAIN_DEPTH, 31):
        witness = fixture(issuer, depth, 7)
        for mode in ("baseline", "optimized", "zk"):
            record = bench(binary, witness, mode, prove_warm, 0, 1)
            recorder.add(record, experiment="stages", implementation="current",
                         build="stage-profile", mode=mode, run=0, order=0, witness_seed=7)
    log("  stages: done")


def experiment_sweep(recorder, binaries, issuer, quick):
    binary = binaries[("current", "dynamic")]
    for depth in SWEEP_DEPTHS:
        seeds = list(WITNESS_SEEDS)[:8] if depth == MAIN_DEPTH and not quick else [7]
        repeats = 1 if depth == MAIN_DEPTH and not quick else 3
        for folding in SWEEP_FOLDING:
            for remainder in SWEEP_REMAINDER:
                for seed in seeds:
                    witness = fixture(issuer, depth, seed)
                    for repeat in range(repeats):
                        record = bench(binary, witness, "folded", 20, 5, 60, (
                            "--fri-folding", str(folding), "--fri-remainder", str(remainder)))
                        recorder.add(record, experiment="sweep", implementation="current",
                                     build="dynamic", mode="folded", run=repeat, order=0,
                                     witness_seed=seed)
    log("  sweep: done")


# STATISTICS
# ------------------------------------------------------------------------------------------------

def describe(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {
        "n": len(values),
        "median": statistics.median(values),
        "mean": statistics.fmean(values),
        "min": min(values),
        "max": max(values),
        "stdev": statistics.stdev(values) if len(values) > 1 else 0.0,
    }


def sign_test_p(differences):
    """Two-sided exact sign test of whether the median paired difference is zero."""
    nonzero = [difference for difference in differences if difference != 0]
    n = len(nonzero)
    if n == 0:
        return 1.0
    k = min(sum(d > 0 for d in nonzero), sum(d < 0 for d in nonzero))
    tail = sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n
    return min(1.0, 2 * tail)


def bootstrap_ci(differences, resamples=10000, seed=12345):
    """95% percentile bootstrap interval for the median paired difference."""
    generator = random.Random(seed)
    medians = sorted(
        statistics.median(generator.choices(differences, k=len(differences)))
        for _ in range(resamples))
    return medians[int(0.025 * resamples)], medians[int(0.975 * resamples) - 1]


def paired(rows, experiment, metric_name, treatment, control, key="run", depth=None):
    def by_key(config):
        return {row[key]: row[metric_name] for row in rows
                if row["experiment"] == experiment
                and (depth is None or row["tree_depth"] == depth)
                and (row["implementation"], row["build"], row["mode"]) == config
                and row[metric_name] is not None}
    treated, controlled = by_key(treatment), by_key(control)
    keys = sorted(set(treated) & set(controlled))
    if len(keys) < 2:
        return None
    differences = [treated[k] - controlled[k] for k in keys]
    control_median = statistics.median(controlled[k] for k in keys)
    low, high = bootstrap_ci(differences)
    return {
        "metric": metric_name,
        "experiment": experiment,
        "tree_depth": depth if depth is not None else MAIN_DEPTH,
        "treatment": "/".join(treatment),
        "control": "/".join(control),
        "pairs": len(keys),
        "treatment_median": statistics.median(treated[k] for k in keys),
        "control_median": control_median,
        "median_difference": statistics.median(differences),
        "median_difference_percent": (100 * statistics.median(differences) / control_median
                                      if control_median else None),
        "ci95_low": low,
        "ci95_high": high,
        "sign_test_p": sign_test_p(differences),
        "treatment_lower_in_pairs": sum(d < 0 for d in differences),
    }


OPTIMIZED = ("current", "dynamic", "optimized")
ZK = ("current", "dynamic", "zk")
ORIGINAL_BASELINE = ("original", "dynamic", "baseline")
ORIGINAL_FOLDED = ("original", "dynamic", "folded")
PREVIOUS_FOLDED = ("previous", "dynamic", "folded")
PREVIOUS_SPLIT = ("previous", "dynamic", "split")
COMPARISONS = [
    # (label, treatment, control)
    ("zero knowledge vs optimized", ZK, OPTIMIZED),
    ("zero knowledge vs original baseline", ZK, ORIGINAL_BASELINE),
    ("zero knowledge vs original folded", ZK, ORIGINAL_FOLDED),
    ("optimized vs original baseline", OPTIMIZED, ORIGINAL_BASELINE),
    ("optimized vs original folded", OPTIMIZED, ORIGINAL_FOLDED),
    ("optimized vs previous folded", OPTIMIZED, PREVIOUS_FOLDED),
    ("optimized vs previous split", OPTIMIZED, PREVIOUS_SPLIT),
    ("current folded vs original folded", ("current", "dynamic", "folded"), ORIGINAL_FOLDED),
    ("current baseline vs original baseline", ("current", "dynamic", "baseline"),
     ORIGINAL_BASELINE),
    ("folded vs baseline (original)", ORIGINAL_FOLDED, ORIGINAL_BASELINE),
    ("folded vs baseline (current)", ("current", "dynamic", "folded"),
     ("current", "dynamic", "baseline")),
    ("optimized vs folded (current)", OPTIMIZED, ("current", "dynamic", "folded")),
    ("optimized vs baseline (current)", OPTIMIZED, ("current", "dynamic", "baseline")),
    ("storage: compact vs Winterfell, FRI 8", ("current", "dynamic", "split"),
     ("current", "dynamic", "baseline")),
    ("FRI shape: optimized vs split", OPTIMIZED, ("current", "dynamic", "split")),
    ("Rescue arithmetic: exact routines vs field operators", OPTIMIZED,
     ("current", "generic-arith", "optimized")),
    ("trade-off: Blake3-192 vs Blake3-256", ("current", "blake3-192", "optimized"), OPTIMIZED),
    ("build: x86-64-v3 vs portable", ("current", "v3", "optimized"), OPTIMIZED),
    ("build: static vs dynamic", ("current", "static", "optimized"), OPTIMIZED),
]
HEAP_COMPARISONS = [
    ("heap: zero knowledge vs optimized", ("current", "alloc-profile", "zk"),
     ("current", "alloc-profile", "optimized")),
    ("heap: zero knowledge vs original baseline", ("current", "alloc-profile", "zk"),
     ("original", "alloc-profile", "baseline")),
    ("heap: optimized vs original baseline", ("current", "alloc-profile", "optimized"),
     ("original", "alloc-profile", "baseline")),
    ("heap: optimized vs original folded", ("current", "alloc-profile", "optimized"),
     ("original", "alloc-profile", "folded")),
    ("heap: optimized vs previous compact", ("current", "alloc-profile", "optimized"),
     ("previous", "alloc-profile", "folded")),
    ("heap: folded vs folded with Winterfell storage", ("current", "alloc-profile", "folded"),
     ("current", "alloc-profile", "folded-default")),
]
SCALING_COMPARISONS = [
    ("zero knowledge vs optimized", ZK, OPTIMIZED),
    ("zero knowledge vs original baseline", ZK, ORIGINAL_BASELINE),
    ("optimized vs original baseline", OPTIMIZED, ORIGINAL_BASELINE),
    ("optimized vs original folded", OPTIMIZED, ORIGINAL_FOLDED),
]
SCALING_METRICS = ("prove_end_to_end_ns", "verify_median_ns", "proof_size_bytes",
                   "peak_rss_after_prove_bytes")
PAIRED_METRICS = ("prove_end_to_end_ns", "prove_warm_median_ns", "prove_first_ns",
                  "trace_build_warm_median_ns", "verify_median_ns",
                  "peak_rss_after_prove_bytes", "peak_rss_process_bytes", "proof_size_bytes")


def summarize(rows):
    groups = {}
    for row in rows:
        key = (row["experiment"], row["tree_depth"], row["implementation"], row["build"],
               row["mode"], row["fri_folding_factor"], row["fri_remainder_max_degree"])
        groups.setdefault(key, []).append(row)
    summary = []
    for (experiment, depth, implementation, build, mode, folding, remainder), members in sorted(
            groups.items(), key=lambda item: tuple(str(part) for part in item[0])):
        entry = {"experiment": experiment, "tree_depth": depth,
                 "implementation": implementation, "build": build, "mode": mode,
                 "fri_folding_factor": folding, "fri_remainder_max_degree": remainder,
                 "fri_layers": members[0]["fri_layers"],
                 "conjectured_security_bits": members[0]["conjectured_security_bits"],
                 "proven_security_ldr_bits": members[0]["proven_security_ldr_bits"],
                 "proven_security_udr_bits": members[0]["proven_security_udr_bits"]}
        for name in METRICS:
            description = describe([row[name] for row in members])
            if description:
                entry[name] = description
        summary.append(entry)

    comparisons = []
    for label, treatment, control in COMPARISONS:
        for name in PAIRED_METRICS:
            result = paired(rows, "main", name, treatment, control)
            if result:
                comparisons.append(dict(result, label=label))
        for name in ("proof_size_bytes", "verify_median_ns"):
            result = paired(rows, "witness", name, treatment, control, key="witness_seed")
            if result:
                comparisons.append(dict(result, label=label))
    for label, treatment, control in HEAP_COMPARISONS:
        result = paired(rows, "heap", "heap_peak_prove_bytes", treatment, control)
        if result:
            comparisons.append(dict(result, label=label))
    for depth in SCALING_DEPTHS:
        for label, treatment, control in SCALING_COMPARISONS:
            for name in SCALING_METRICS:
                result = paired(rows, "scaling", name, treatment, control, depth=depth)
                if result:
                    comparisons.append(dict(result, label=f"depth {depth}: {label}"))
    return summary, comparisons


def write_outputs(directory, rows, summary, comparisons, metadata):
    columns = list(rows[0].keys())
    with open(directory / "runs.csv", "w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=columns)
        writer.writeheader()
        writer.writerows(rows)

    with open(directory / "summary.csv", "w", newline="") as output:
        writer = csv.writer(output)
        writer.writerow(["experiment", "tree_depth", "implementation", "build", "mode",
                         "fri_folding_factor", "fri_remainder_max_degree", "metric", "n", "median", "mean", "min", "max",
                         "stdev"])
        for entry in summary:
            for name in METRICS:
                if name in entry:
                    stats = entry[name]
                    writer.writerow([entry["experiment"], entry["tree_depth"],
                                     entry["implementation"], entry["build"], entry["mode"],
                                     entry["fri_folding_factor"],
                                     entry["fri_remainder_max_degree"], name, stats["n"],
                                     stats["median"], stats["mean"], stats["min"], stats["max"],
                                     stats["stdev"]])

    with open(directory / "comparisons.csv", "w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(comparisons[0].keys()))
        writer.writeheader()
        writer.writerows(comparisons)

    with open(directory / "summary.json", "w") as output:
        json.dump({"metadata": metadata, "summary": summary, "comparisons": comparisons},
                  output, indent=2)


def write_report(directory, summary, comparisons, info):
    """Writes report.md: the headline tables, generated from this run's data."""
    def find(experiment, config, metric_name, depth=MAIN_DEPTH):
        for entry in summary:
            if (entry["experiment"], entry["implementation"], entry["build"], entry["mode"]) == (
                    experiment, *config) and entry["tree_depth"] == depth and metric_name in entry:
                return entry[metric_name]["median"]
        return None

    columns = [("Original baseline", ORIGINAL_BASELINE), ("Original folded", ORIGINAL_FOLDED),
               ("Previous folded", PREVIOUS_FOLDED), ("Current optimized", OPTIMIZED),
               ("Zero knowledge", ZK)]
    def heap(config):
        return (config[0], "alloc-profile", config[2])
    rows = [
        ("Proving, trace + prove, warm (µs)", "main", "prove_end_to_end_ns", 1e3, None),
        ("Proving, prove only, warm (µs)", "main", "prove_warm_median_ns", 1e3, None),
        ("Trace build, warm (µs)", "main", "trace_build_warm_median_ns", 1e3, None),
        ("Verification, 30 witnesses (µs)", "witness", "verify_median_ns", 1e3, None),
        ("Proof size, 30 witnesses (B)", "witness", "proof_size_bytes", 1, None),
        ("Peak heap during proving (B)", "heap", "heap_peak_prove_bytes", 1, heap),
        ("Peak RSS after proving (B)", "main", "peak_rss_after_prove_bytes", 1, None),
    ]
    lines = [f"# Benchmark report {directory.name}", "",
             f"Revision {info['source_revision']}, {info['cpu']}, kernel {info['kernel']}, "
             f"{info['rustc']}. Depth {MAIN_DEPTH}, medians.", "",
             "| Metric | " + " | ".join(label for label, _ in columns) + " |",
             "|---|" + "---:|" * len(columns)]
    for label, experiment, name, divisor, transform in rows:
        cells = []
        for _, config in columns:
            value = find(experiment, transform(config) if transform else config, name)
            cells.append("n/a" if value is None else f"{value / divisor:,.1f}" if divisor != 1
                         else f"{value:,.0f}")
        lines.append(f"| {label} | " + " | ".join(cells) + " |")
    lines += ["", "## Paired comparisons", "",
              "An interval containing zero means no established difference.", "",
              "| Comparison | Metric | Experiment | Pairs | Control | Treatment | Change | 95% CI |",
              "|---|---|---|---:|---:|---:|---:|---|"]
    for item in comparisons:
        control = item["control_median"]
        percent = 100 / control if control else 0
        lines.append(
            f"| {item['label']} | {item['metric']} | {item['experiment']} | {item['pairs']} | "
            f"{control:,.0f} | {item['treatment_median']:,.0f} | "
            f"{item['median_difference_percent']:+.1f}% | "
            f"[{item['ci95_low'] * percent:+.1f}%, {item['ci95_high'] * percent:+.1f}%] |")
    (directory / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def finish(directory, records, info):
    """Derives every summary, chart, and report from the raw records."""
    rows = [flatten(record) for record in records]
    summary, comparisons = summarize(rows)
    write_outputs(directory, rows, summary, comparisons, info)
    with open(directory / "stages.json", "w") as output:
        json.dump([{key: record.get(key) for key in (
            "mode", "tree_depth", "prove_warm", "trace_build_warm", "prove_stages_mean_ns")}
            for record in records if record["experiment"] == "stages"], output, indent=2)
    run([sys.executable, str(REPO / "evaluation" / "plot_graphs.py"), str(directory)])
    write_report(directory, summary, comparisons, info)
    # On a Windows mount under WSL, data not yet flushed is lost as NUL bytes if WSL shuts down.
    for path in directory.rglob("*"):
        if path.is_file():
            with open(path, "rb") as handle:
                os.fsync(handle.fileno())
    os.sync()


def metadata(args):
    def output_of(command):
        try:
            return run(command, cwd=REPO).stdout.strip()
        except (subprocess.CalledProcessError, FileNotFoundError):
            return None
    cpu = None
    if pathlib.Path("/proc/cpuinfo").exists():
        for line in pathlib.Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    revision = output_of(["git", "rev-parse", "--short", "HEAD"])
    if output_of(["git", "status", "--porcelain"]):
        revision = f"{revision}-dirty"
    return {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "source_revision": revision,
        "rustc": output_of(["rustc", "--version"]),
        "kernel": platform.release(),
        "cpu": cpu,
        "logical_cpus": os.cpu_count(),
        "pinned_core": os.cpu_count() - 1 if shutil.which("taskset") else None,
        "runs_per_configuration": args.runs,
        "prove_warm_iterations": args.prove_warm,
        "verify_warmup_iterations": args.verify_warmup,
        "verify_timed_iterations": args.verify_iterations,
        "main_tree_depth": MAIN_DEPTH,
        "security_parameters": {"num_queries": 28, "blowup_factor": 8, "grinding_factor": 0,
                                "field_extension": "none"},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--runs", type=int, default=15)
    parser.add_argument("--prove-warm", type=int, default=30)
    parser.add_argument("--verify-warmup", type=int, default=10)
    parser.add_argument("--verify-iterations", type=int, default=100)
    parser.add_argument("--quick", action="store_true", help="few runs, to check the pipeline")
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--from-raw", type=pathlib.Path, metavar="DIR",
                        help="rebuild summaries, charts, and report from DIR/raw.jsonl")
    args = parser.parse_args()
    if args.from_raw:
        records = [json.loads(line) for line in (args.from_raw / "raw.jsonl").read_text().splitlines()]
        finish(args.from_raw, records, json.loads((args.from_raw / "metadata.json").read_text()))
        print(args.from_raw)
        return
    if args.quick:
        args.runs, args.prove_warm, args.verify_iterations = 3, 5, 20

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    directory = args.output or REPO / "evaluation" / "results" / stamp
    directory.mkdir(parents=True, exist_ok=True)
    started = time.time()
    with tempfile.TemporaryDirectory(prefix="zkp-bench-") as binary_dir:
        binaries, issuer = build_all(pathlib.Path(binary_dir))
        info = metadata(args)
        (directory / "metadata.json").write_text(json.dumps(info, indent=2))
        recorder = Recorder(directory)
        witness = fixture(issuer, MAIN_DEPTH, 7)
        log("measuring")
        experiment_main(recorder, binaries, witness, args.runs,
                        (args.prove_warm, args.verify_warmup, args.verify_iterations))
        experiment_heap(recorder, binaries, witness, 3)
        experiment_witnesses(recorder, binaries, issuer, 30 if not args.quick else 5)
        experiment_scaling(recorder, binaries, issuer, 5 if not args.quick else 1)
        experiment_stages(recorder, binaries, issuer, 200 if not args.quick else 20)
        experiment_sweep(recorder, binaries, issuer, args.quick)

    info["elapsed_seconds"] = round(time.time() - started, 1)
    (directory / "metadata.json").write_text(json.dumps(info, indent=2))
    finish(directory, recorder.records, info)
    log(f"results: {directory}")
    print(directory)


if __name__ == "__main__":
    main()
