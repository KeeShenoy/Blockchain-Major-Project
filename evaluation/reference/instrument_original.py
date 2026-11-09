"""Builds the original (pre-optimization) implementation with the current measurement code.

The archive `original-implementation.tar.gz` is the prover/verifier exactly as it was before this
revision. Its own reporting is not comparable: it read peak memory through `ru_maxrss`, which
inherits the parent's peak across exec, and it timed verification including a proof clone. This
script replaces only the original `main()` with a `bench` entry point that follows the same steps
and prints the same fields as the current `prover-verifier bench`, using the same `memory.rs` and
`heap.rs`. The original trace builder, prover, AIR, options, and verifier run unmodified.

Usage: python3 instrument_original.py <output-dir>
Then:  cargo build --release [--features alloc-profile] --manifest-path <output-dir>/Cargo.toml
"""

import pathlib
import shutil
import sys
import tarfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent

BENCH_MAIN = r'''
mod heap;
mod memory;

fn bench_proof_digest(bytes: &[u8]) -> String {
    use winterfell::crypto::{Digest, Hasher};
    StarkCommitmentHasher::hash(bytes)
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn bench_summary(samples: &mut [Duration]) -> serde_json::Value {
    if samples.is_empty() {
        return serde_json::Value::Null;
    }
    samples.sort_unstable();
    let total: u128 = samples.iter().map(Duration::as_nanos).sum();
    serde_json::json!({
        "iterations": samples.len(),
        "total_ns": total,
        "mean_ns": total as f64 / samples.len() as f64,
        "median_ns": samples[samples.len() / 2].as_nanos(),
        "min_ns": samples[0].as_nanos(),
        "max_ns": samples[samples.len() - 1].as_nanos(),
    })
}

/// Same steps and fields as the current `prover-verifier bench`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value_of = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|index| args[index + 1].clone())
    };
    let count = |name: &str, default: usize| {
        value_of(name).map_or(default, |value| value.parse().unwrap())
    };
    let witness_path = value_of("--witness").expect("--witness is required");
    let mode_label = value_of("--mode").expect("--mode is required");
    let mode = ProverMode::parse(&mode_label);
    let prove_warm = count("--prove-warm", 30);
    let verify_warmup = count("--verify-warmup", 10);
    let verify_iterations = count("--verify-iterations", 100);

    let (value, index, path) = into_witness_parts(load_witness(Path::new(&witness_path)));
    let depth = path.len();
    let witness_root = root_from_path(value, &path, index).to_elements();
    let root = match (value_of("--root-limb0"), value_of("--root-limb1")) {
        (Some(limb0), Some(limb1)) => [
            BaseElement::new(limb0.parse().unwrap()),
            BaseElement::new(limb1.parse().unwrap()),
        ],
        _ => witness_root,
    };
    let prover = MerkleProver::<StarkCommitmentHasher>::new(proof_options(mode));
    let build = || match mode {
        ProverMode::Baseline => {
            let mut branch = Vec::with_capacity(path.len() + 1);
            branch.push(Rescue128::digest(&value));
            branch.extend_from_slice(&path);
            prover.build_trace(value, &branch, index)
        }
        _ => prover.build_trace_from_path(value, &path, index),
    };

    let started = Instant::now();
    let trace = build();
    let trace_build_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let proof = prover.prove(trace).expect("proof generation");
    let prove_first_ns = started.elapsed().as_nanos();
    let memory_after_first_prove = memory::read_process_memory();
    let heap_peak_first_prove_bytes = heap::report().map(|_| heap::peak_bytes());

    let mut warm = Vec::with_capacity(prove_warm);
    let mut trace_samples = Vec::with_capacity(prove_warm);
    for _ in 0..prove_warm {
        let started = Instant::now();
        let trace = build();
        trace_samples.push(started.elapsed());
        let started = Instant::now();
        let repeat = prover.prove(trace).expect("proof generation");
        warm.push(started.elapsed());
        drop(repeat);
    }

    let started = Instant::now();
    let bytes = proof.to_bytes();
    let proof_serialize_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let decoded = Proof::from_bytes(&bytes).expect("decode");
    let proof_deserialize_ns = started.elapsed().as_nanos();
    drop(decoded);

    let verify_once = || {
        let copy = proof.clone();
        let started = Instant::now();
        let result = verify_proof(copy, root);
        (started.elapsed(), result)
    };
    let (first_duration, first_result) = verify_once();
    let mut samples = Vec::new();
    if first_result.is_ok() {
        for _ in 0..verify_warmup {
            verify_once().1.unwrap();
        }
        for _ in 0..verify_iterations {
            let (duration, result) = verify_once();
            result.unwrap();
            samples.push(duration);
        }
    }

    let security = proof_security(&proof);
    let fri = proof.options().to_fri_options();
    let trace_length = (depth + 1) * HASH_CYCLE_LEN;
    let lde_domain_size = trace_length * proof.options().blowup_factor();
    let verified = first_result.is_ok();
    println!(
        "{}",
        serde_json::json!({
            "mode": mode_label,
            "uses_compact_storage": false,
            "tree_depth": depth,
            "num_leaves": 1u128 << depth,
            "trace_length": trace_length,
            "lde_domain_size": lde_domain_size,
            "num_queries": proof.options().num_queries(),
            "blowup_factor": proof.options().blowup_factor(),
            "grinding_factor": proof.options().grinding_factor(),
            "fri_folding_factor": fri.folding_factor(),
            "fri_remainder_max_degree": fri.remainder_max_degree(),
            "fri_layers": fri.num_fri_layers(lde_domain_size),
            "conjectured_security_bits": security.conjectured_bits,
            "proven_security_ldr_bits": security.proven_ldr_bits,
            "proven_security_udr_bits": security.proven_udr_bits,
            "trace_build_ns": trace_build_ns,
            "prove_first_ns": prove_first_ns,
            "prove_warm": bench_summary(&mut warm),
            "trace_build_warm": bench_summary(&mut trace_samples),
            "memory_after_first_prove": memory_after_first_prove,
            "heap_peak_first_prove_bytes": heap_peak_first_prove_bytes,
            "proof_size_bytes": bytes.len(),
            "proof_digest": bench_proof_digest(&bytes),
            "proof_serialize_ns": proof_serialize_ns,
            "proof_deserialize_ns": proof_deserialize_ns,
            "verify_first_ns": verified.then(|| first_duration.as_nanos()),
            "verify_warmup_iterations": verify_warmup,
            "verify": bench_summary(&mut samples),
            "memory_at_exit": memory::read_process_memory(),
            "heap": heap::report(),
            "root_limb0": root[0].as_int().to_string(),
            "root_limb1": root[1].as_int().to_string(),
            "root_source": "witness",
            "verified": verified,
            "rejection_reason": first_result.err().map(|error| error.to_string()),
        })
    );
    std::process::exit(if verified { 0 } else { 1 });
}
'''

CHECKPOINTS = {
    "        DefaultTraceLde::new(trace_info, main_trace, domain, partition_option)\n":
        "        crate::heap::checkpoint(\"prover_setup\");\n"
        "        let result = DefaultTraceLde::new(trace_info, main_trace, domain, partition_option);\n"
        "        crate::heap::checkpoint(\"trace_commitment\");\n"
        "        result\n",
    "        DefaultConstraintCommitment::new(\n":
        "        crate::heap::checkpoint(\"constraint_evaluation\");\n"
        "        let result = DefaultConstraintCommitment::new(\n",
}


def main() -> None:
    destination = pathlib.Path(sys.argv[1]).resolve()
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    with tarfile.open(HERE / "original-implementation.tar.gz") as archive:
        archive.extractall(destination, filter="data")

    crate = destination / "prover-verifier"
    main_rs = crate / "src" / "main.rs"
    source = main_rs.read_text()
    start, end = source.index("fn main() {"), source.index("#[cfg(test)]")
    main_rs.write_text(source[:start] + BENCH_MAIN.lstrip() + "\n" + source[end:])

    prover_rs = crate / "src" / "prover.rs"
    source = prover_rs.read_text()
    for old, new in CHECKPOINTS.items():
        assert old in source, old
        source = source.replace(old, new)
    commitment_end = "            partition_options,\n        )\n    }\n}"
    assert commitment_end in source
    source = source.replace(
        commitment_end,
        "            partition_options,\n        );\n"
        "        crate::heap::checkpoint(\"composition_commitment\");\n        result\n    }\n}",
    )
    prover_rs.write_text(source)

    for module in ("memory.rs", "heap.rs"):
        shutil.copy(REPO / "prover-verifier" / "src" / module, crate / "src" / module)

    manifest = crate / "Cargo.toml"
    manifest.write_text(manifest.read_text() + "\n[features]\nalloc-profile = []\n")
    print(destination)


if __name__ == "__main__":
    main()
