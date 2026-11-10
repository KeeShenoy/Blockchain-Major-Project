//! Command-line prover and verifier. See `README.md` for usage.
//!
//! Exit codes: 0 means success or a verified proof, 1 means the proof was rejected, and 2 means the
//! input was invalid or an operation could not run.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

use merkle_core::{parse_element, RootFile, Witness, WitnessFile};
use prover_verifier::{
    air::trace_length,
    build_trace,
    config::options_with_fri,
    heap,
    memory::{read_process_memory, ProcessMemory},
    prove, prove_trace_with, stages, verify_bytes, verify_with_options, CommitmentHasher,
    ProverMode, SessionId,
};
use prover_verifier::{
    config::BLOWUP_FACTOR,
    zk::{fresh_rng, prove_trace_zk, prove_zk, verify_zk, verify_zk_bytes, ZkParameters, ZkProof},
};
use serde::Serialize;
use winterfell::{
    crypto::{Digest, Hasher},
    math::fields::f128::BaseElement,
    Proof, ProofOptions,
};

const USAGE: &str = "usage:
  prover-verifier prove  --witness <file> --mode <mode> --proof-out <file>
  prover-verifier verify --proof <file> --mode <mode> --depth <depth>
                         --root-limb0 <decimal> --root-limb1 <decimal>
  prover-verifier bench  --witness <file> --mode <mode>
                         [--root-limb0 <decimal> --root-limb1 <decimal>]
                         [--prove-warm <n>] [--verify-warmup <n>] [--verify-iterations <n>]
  prover-verifier --witness <file> [--mode <mode>] <root_limb0> <root_limb1>
  prover-verifier [depth] [<root_limb0> <root_limb1>]

  prove and verify also take --session-limb0 and --session-limb1, the verifier's challenge

modes: baseline, split, folded-default, folded, optimized, zk";

const ZK_DEPTH_ERROR: &str = "zero-knowledge proofs need a tree depth of at least 7";
const DEFAULT_PROVE_WARM_ITERATIONS: usize = 30;
const DEFAULT_VERIFY_WARMUP_ITERATIONS: usize = 10;
const DEFAULT_VERIFY_TIMED_ITERATIONS: usize = 100;
const DEFAULT_SYNTHETIC_DEPTH: usize = 7;
const SYNTHETIC_SEED: u64 = 42;

/// Options that follow a subcommand.
#[derive(Default)]
struct Options {
    witness: Option<PathBuf>,
    proof: Option<PathBuf>,
    proof_out: Option<PathBuf>,
    mode: Option<ProverMode>,
    depth: Option<usize>,
    root_limb0: Option<String>,
    root_limb1: Option<String>,
    prove_warm: Option<usize>,
    verify_warmup: Option<usize>,
    verify_iterations: Option<usize>,
    zk: bool,
    fri_folding: Option<usize>,
    fri_remainder: Option<usize>,
    session_limb0: Option<String>,
    session_limb1: Option<String>,
    positional: Vec<String>,
}

fn parse_options(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut iter = arguments.iter();
    let count = |value: &String, name: &str| {
        value
            .parse::<usize>()
            .map_err(|_| format!("{name} must be a non-negative integer"))
    };
    while let Some(argument) = iter.next() {
        if !argument.starts_with("--") {
            options.positional.push(argument.clone());
            continue;
        }
        let value = iter
            .next()
            .ok_or_else(|| format!("{argument} requires a value"))?;
        match argument.as_str() {
            "--witness" => options.witness = Some(value.into()),
            "--proof" => options.proof = Some(value.into()),
            "--proof-out" => options.proof_out = Some(value.into()),
            "--mode" if value == "zk" => options.zk = true,
            "--mode" => {
                options.mode = Some(
                    ProverMode::parse(value)
                        .ok_or_else(|| format!("unknown mode {value}\n\n{USAGE}"))?,
                )
            }
            "--depth" => options.depth = Some(count(value, "--depth")?),
            "--root-limb0" => options.root_limb0 = Some(value.clone()),
            "--root-limb1" => options.root_limb1 = Some(value.clone()),
            "--fri-folding" => options.fri_folding = Some(count(value, "--fri-folding")?),
            "--fri-remainder" => options.fri_remainder = Some(count(value, "--fri-remainder")?),
            "--session-limb0" => options.session_limb0 = Some(value.clone()),
            "--session-limb1" => options.session_limb1 = Some(value.clone()),
            "--prove-warm" => options.prove_warm = Some(count(value, "--prove-warm")?),
            "--verify-warmup" => options.verify_warmup = Some(count(value, "--verify-warmup")?),
            "--verify-iterations" => {
                options.verify_iterations = Some(count(value, "--verify-iterations")?)
            }
            _ => return Err(format!("unknown option {argument}\n\n{USAGE}")),
        }
    }
    Ok(options)
}

fn parse_root(limb0: &str, limb1: &str) -> Result<[BaseElement; 2], String> {
    let parse = |value, name| parse_element(value, name).map_err(|error| error.to_string());
    Ok([parse(limb0, "root limb 0")?, parse(limb1, "root limb 1")?])
}

impl Options {
    /// The verifier's challenge, or no binding when neither limb is given.
    fn session(&self) -> Result<SessionId, String> {
        match (&self.session_limb0, &self.session_limb1) {
            (Some(limb0), Some(limb1)) => parse_root(limb0, limb1).map(SessionId),
            (None, None) => Ok(SessionId::NONE),
            _ => Err("supply both session limbs or neither".into()),
        }
    }

    fn root(&self) -> Result<Option<[BaseElement; 2]>, String> {
        match (&self.root_limb0, &self.root_limb1) {
            (Some(limb0), Some(limb1)) => parse_root(limb0, limb1).map(Some),
            (None, None) => Ok(None),
            _ => Err("supply both root limbs or neither".into()),
        }
    }
}

fn load_witness(path: &Path) -> Result<Witness, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read witness {}: {error}", path.display()))?;
    let file: WitnessFile = serde_json::from_str(&text)
        .map_err(|error| format!("witness {} is not valid: {error}", path.display()))?;
    Witness::try_from(&file).map_err(|error| format!("witness {}: {error}", path.display()))
}

fn proof_digest(bytes: &[u8]) -> String {
    let digest = CommitmentHasher::hash(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest.as_bytes() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn command_prove(options: Options) -> Result<ExitCode, String> {
    let witness = load_witness(options.witness.as_deref().ok_or("--witness is required")?)?;
    let session = options.session()?;
    if options.zk {
        return command_prove_zk(witness, session, options.proof_out);
    }
    let mode = options.mode.ok_or("--mode is required")?;
    let out = options.proof_out.ok_or("--proof-out is required")?;

    let started = Instant::now();
    let proof =
        prove(&witness, mode, session).map_err(|error| format!("proving failed: {error}"))?;
    let prove_ns = started.elapsed().as_nanos();
    let bytes = proof.to_bytes();
    fs::write(&out, &bytes).map_err(|error| format!("cannot write {}: {error}", out.display()))?;

    print_json(&serde_json::json!({
        "mode": mode.label(),
        "tree_depth": witness.depth(),
        "prove_ns": prove_ns,
        "proof_size_bytes": bytes.len(),
        "proof_digest": proof_digest(&bytes),
        "peak_rss_bytes": read_process_memory().map(|memory| memory.peak_rss_bytes),
    }));
    Ok(ExitCode::SUCCESS)
}

/// Masks come from a freshly seeded stream: reusing them across proofs would break zero knowledge.
fn command_prove_zk(
    witness: Witness,
    session: SessionId,
    out: Option<PathBuf>,
) -> Result<ExitCode, String> {
    let out = out.ok_or("--proof-out is required")?;
    let started = Instant::now();
    let proof = prove_zk(&witness, session, &mut fresh_rng()).ok_or(ZK_DEPTH_ERROR)?;
    let prove_ns = started.elapsed().as_nanos();
    let bytes = proof.to_bytes();
    fs::write(&out, &bytes).map_err(|error| format!("cannot write {}: {error}", out.display()))?;
    print_json(&serde_json::json!({
        "mode": "zk",
        "tree_depth": witness.depth(),
        "prove_ns": prove_ns,
        "proof_size_bytes": bytes.len(),
        "proof_digest": proof_digest(&bytes),
        "peak_rss_bytes": read_process_memory().map(|memory| memory.peak_rss_bytes),
    }));
    Ok(ExitCode::SUCCESS)
}

fn command_verify(options: Options) -> Result<ExitCode, String> {
    let path = options.proof.as_deref().ok_or("--proof is required")?;
    let zk = options.zk;
    let mode = options.mode;
    if !zk && mode.is_none() {
        return Err("--mode is required".into());
    }
    let depth = options.depth.ok_or("--depth is required")?;
    merkle_core::validate_depth(depth).map_err(|error| error.to_string())?;
    let root = options
        .root()?
        .ok_or("--root-limb0 and --root-limb1 are required")?;
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;

    let session = options.session()?;
    let result = match mode {
        Some(mode) if !zk => {
            verify_bytes(&bytes, root, mode, depth, session).map_err(|error| error.to_string())
        }
        _ => verify_zk_bytes(&bytes, root, depth, session).map_err(|error| error.to_string()),
    };
    let root_file = RootFile::from_elements(root);
    print_json(&serde_json::json!({
        "mode": mode.filter(|_| !zk).map_or("zk", ProverMode::label),
        "tree_depth": depth,
        "root_limb0": root_file.root_limb0,
        "root_limb1": root_file.root_limb1,
        "proof_size_bytes": bytes.len(),
        "verified": result.is_ok(),
        "reason": result.as_ref().err().map(ToString::to_string),
    }));
    Ok(if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// Aggregates of a batch of durations, in nanoseconds.
#[derive(Serialize)]
struct TimingSummary {
    iterations: usize,
    total_ns: u128,
    mean_ns: f64,
    median_ns: u128,
    min_ns: u128,
    max_ns: u128,
}

impl TimingSummary {
    fn from_samples(samples: &mut [Duration]) -> Option<Self> {
        samples.sort_unstable();
        let total: u128 = samples.iter().map(Duration::as_nanos).sum();
        Some(Self {
            iterations: samples.len(),
            total_ns: total,
            mean_ns: total as f64 / samples.len() as f64,
            median_ns: samples.get(samples.len() / 2)?.as_nanos(),
            min_ns: samples.first()?.as_nanos(),
            max_ns: samples.last()?.as_nanos(),
        })
    }
}

#[derive(Serialize)]
struct BenchRecord {
    mode: &'static str,
    uses_compact_storage: bool,
    tree_depth: usize,
    num_leaves: u128,
    trace_length: usize,
    lde_domain_size: usize,

    num_queries: usize,
    blowup_factor: usize,
    grinding_factor: u32,
    fri_folding_factor: usize,
    fri_remainder_max_degree: usize,
    fri_layers: usize,
    conjectured_security_bits: u32,
    proven_security_ldr_bits: u32,
    proven_security_udr_bits: u32,

    /// Building the execution trace of the first proof.
    trace_build_ns: u128,
    /// The first proof in a fresh process, including page faults on first-touched code and data.
    prove_first_ns: u128,
    /// Later proofs in the same process.
    prove_warm: Option<TimingSummary>,
    /// Trace building for those later proofs.
    trace_build_warm: Option<TimingSummary>,
    /// Mean time per later proof inside each Winterfell span; null without `stage-profile`.
    prove_stages_mean_ns: Option<BTreeMap<&'static str, u128>>,
    /// Process memory right after the first proof, before any verification code has run.
    memory_after_first_prove: Option<ProcessMemory>,
    /// Peak live heap up to the end of the first proof; null without `alloc-profile`.
    heap_peak_first_prove_bytes: Option<usize>,

    proof_size_bytes: usize,
    proof_digest: String,
    proof_serialize_ns: u128,
    proof_deserialize_ns: u128,

    /// The first verification in the process.
    verify_first_ns: Option<u128>,
    verify_warmup_iterations: usize,
    /// Each sample times `verify` alone. The proof clone it consumes is made outside the timer.
    verify: Option<TimingSummary>,

    /// Process memory at exit, covering the whole pipeline.
    memory_at_exit: Option<ProcessMemory>,
    heap: Option<heap::HeapReport>,

    root_limb0: String,
    root_limb1: String,
    root_source: &'static str,
    verified: bool,
    rejection_reason: Option<String>,
}

struct BenchPlan {
    witness: Witness,
    mode: ProverMode,
    proof_options: ProofOptions,
    session: SessionId,
    trusted_root: Option<[BaseElement; 2]>,
    root_source: &'static str,
    prove_warm: usize,
    verify_warmup: usize,
    verify_iterations: usize,
}

fn run_bench(plan: BenchPlan) -> Result<ExitCode, String> {
    let BenchPlan {
        witness,
        mode,
        proof_options,
        session,
        trusted_root,
        root_source,
        prove_warm,
        verify_warmup,
        verify_iterations,
    } = plan;
    let prove_error = |error| format!("proving failed: {error}");
    let compact = mode.uses_compact_storage();
    let run_prover = |trace| prove_trace_with(trace, proof_options.clone(), compact, session);
    stages::install();

    let started = Instant::now();
    let trace = build_trace(&witness, session);
    let trace_build_ns = started.elapsed().as_nanos();

    let started = Instant::now();
    let proof = run_prover(trace).map_err(prove_error)?;
    let prove_first_ns = started.elapsed().as_nanos();
    let memory_after_first_prove = read_process_memory();
    let heap_peak_first_prove_bytes = heap::report().map(|_| heap::peak_bytes());

    stages::reset();
    let mut warm_samples = Vec::with_capacity(prove_warm);
    let mut trace_samples = Vec::with_capacity(prove_warm);
    for _ in 0..prove_warm {
        let started = Instant::now();
        let trace = build_trace(&witness, session);
        trace_samples.push(started.elapsed());
        let started = Instant::now();
        let repeat = run_prover(trace).map_err(prove_error)?;
        warm_samples.push(started.elapsed());
        drop(repeat);
    }
    let prove_stages_mean_ns = stages::totals_ns().map(|totals| {
        totals
            .into_iter()
            .map(|(name, total)| (name, total / prove_warm.max(1) as u128))
            .collect::<BTreeMap<_, _>>()
    });

    let started = Instant::now();
    let bytes = proof.to_bytes();
    let proof_serialize_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let decoded = Proof::from_bytes(&bytes).map_err(|error| error.to_string())?;
    let proof_deserialize_ns = started.elapsed().as_nanos();
    drop(decoded);

    let depth = witness.depth();
    let root = trusted_root.unwrap_or_else(|| witness.root());
    let verify_once = || -> (Duration, Result<(), _>) {
        let copy = proof.clone();
        let expected = proof_options.clone();
        let started = Instant::now();
        let result = verify_with_options(copy, root, expected, depth, session);
        (started.elapsed(), result)
    };

    let (first_duration, first_result) = verify_once();
    let mut verify_samples = Vec::with_capacity(verify_iterations);
    if first_result.is_ok() {
        for _ in 0..verify_warmup {
            verify_once().1.map_err(|error| error.to_string())?;
        }
        for _ in 0..verify_iterations {
            let (duration, result) = verify_once();
            result.map_err(|error| error.to_string())?;
            verify_samples.push(duration);
        }
    }

    let fri_options = proof_options.to_fri_options();
    let lde_domain_size = trace_length(depth) * proof_options.blowup_factor();
    let proven = proof.proven_security::<CommitmentHasher>();
    let root_file = RootFile::from_elements(root);
    let verified = first_result.is_ok();
    print_json(&BenchRecord {
        mode: mode.label(),
        uses_compact_storage: mode.uses_compact_storage(),
        tree_depth: depth,
        num_leaves: 1u128 << depth,
        trace_length: trace_length(depth),
        lde_domain_size,
        num_queries: proof_options.num_queries(),
        blowup_factor: proof_options.blowup_factor(),
        grinding_factor: proof_options.grinding_factor(),
        fri_folding_factor: fri_options.folding_factor(),
        fri_remainder_max_degree: fri_options.remainder_max_degree(),
        fri_layers: fri_options.num_fri_layers(lde_domain_size),
        conjectured_security_bits: proof.conjectured_security::<CommitmentHasher>().bits(),
        proven_security_ldr_bits: proven.ldr_bits(),
        proven_security_udr_bits: proven.udr_bits(),
        trace_build_ns,
        prove_first_ns,
        prove_warm: TimingSummary::from_samples(&mut warm_samples),
        trace_build_warm: TimingSummary::from_samples(&mut trace_samples),
        prove_stages_mean_ns,
        memory_after_first_prove,
        heap_peak_first_prove_bytes,
        proof_size_bytes: bytes.len(),
        proof_digest: proof_digest(&bytes),
        proof_serialize_ns,
        proof_deserialize_ns,
        verify_first_ns: verified.then_some(first_duration.as_nanos()),
        verify_warmup_iterations: verify_warmup,
        verify: TimingSummary::from_samples(&mut verify_samples),
        memory_at_exit: read_process_memory(),
        heap: heap::report(),
        root_limb0: root_file.root_limb0,
        root_limb1: root_file.root_limb1,
        root_source,
        verified,
        rejection_reason: first_result.err().map(|error| error.to_string()),
    });
    Ok(if verified {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// The bench for the zero-knowledge mode. Same steps and fields as [`run_bench`].
fn run_bench_zk(plan: BenchPlan) -> Result<ExitCode, String> {
    let BenchPlan {
        witness,
        session,
        trusted_root,
        root_source,
        prove_warm,
        verify_warmup,
        verify_iterations,
        ..
    } = plan;
    let depth = witness.depth();
    let parameters = ZkParameters::for_depth(depth).ok_or(ZK_DEPTH_ERROR)?;
    stages::install();

    let started = Instant::now();
    let trace = build_trace(&witness, session);
    let trace_build_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let proof = prove_trace_zk(trace, &parameters, session, &mut fresh_rng());
    let prove_first_ns = started.elapsed().as_nanos();
    let memory_after_first_prove = read_process_memory();
    let heap_peak_first_prove_bytes = heap::report().map(|_| heap::peak_bytes());

    stages::reset();
    let mut warm_samples = Vec::with_capacity(prove_warm);
    let mut trace_samples = Vec::with_capacity(prove_warm);
    for _ in 0..prove_warm {
        let started = Instant::now();
        let trace = build_trace(&witness, session);
        trace_samples.push(started.elapsed());
        let started = Instant::now();
        let repeat = prove_trace_zk(trace, &parameters, session, &mut fresh_rng());
        warm_samples.push(started.elapsed());
        drop(repeat);
    }
    let prove_stages_mean_ns = stages::totals_ns().map(|totals| {
        totals
            .into_iter()
            .map(|(name, total)| (name, total / prove_warm.max(1) as u128))
            .collect::<BTreeMap<_, _>>()
    });

    let started = Instant::now();
    let bytes = proof.to_bytes();
    let proof_serialize_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let decoded = ZkProof::from_bytes(&bytes, &parameters).map_err(|error| error.to_string())?;
    let proof_deserialize_ns = started.elapsed().as_nanos();
    drop(decoded);

    let root = trusted_root.unwrap_or_else(|| witness.root());
    let verify_once = || -> (Duration, Result<(), _>) {
        let started = Instant::now();
        let result = verify_zk(&proof, root, &parameters, session);
        (started.elapsed(), result)
    };
    let (first_duration, first_result) = verify_once();
    let mut verify_samples = Vec::with_capacity(verify_iterations);
    if first_result.is_ok() {
        for _ in 0..verify_warmup {
            verify_once().1.map_err(|error| error.to_string())?;
        }
        for _ in 0..verify_iterations {
            let (duration, result) = verify_once();
            result.map_err(|error| error.to_string())?;
            verify_samples.push(duration);
        }
    }

    let security = parameters.security();
    let root_file = RootFile::from_elements(root);
    let verified = first_result.is_ok();
    print_json(&BenchRecord {
        mode: "zk",
        uses_compact_storage: true,
        tree_depth: depth,
        num_leaves: 1u128 << depth,
        trace_length: parameters.trace_length,
        lde_domain_size: parameters.domain_size,
        num_queries: parameters.num_queries,
        blowup_factor: BLOWUP_FACTOR,
        grinding_factor: 0,
        fri_folding_factor: 0,
        fri_remainder_max_degree: parameters.batched_length - 1,
        fri_layers: 0,
        conjectured_security_bits: security.conjectured_bits,
        proven_security_ldr_bits: security.proven_ldr_bits,
        proven_security_udr_bits: security.proven_udr_bits,
        trace_build_ns,
        prove_first_ns,
        prove_warm: TimingSummary::from_samples(&mut warm_samples),
        trace_build_warm: TimingSummary::from_samples(&mut trace_samples),
        prove_stages_mean_ns,
        memory_after_first_prove,
        heap_peak_first_prove_bytes,
        proof_size_bytes: bytes.len(),
        proof_digest: proof_digest(&bytes),
        proof_serialize_ns,
        proof_deserialize_ns,
        verify_first_ns: verified.then_some(first_duration.as_nanos()),
        verify_warmup_iterations: verify_warmup,
        verify: TimingSummary::from_samples(&mut verify_samples),
        memory_at_exit: read_process_memory(),
        heap: heap::report(),
        root_limb0: root_file.root_limb0,
        root_limb1: root_file.root_limb1,
        root_source,
        verified,
        rejection_reason: first_result.err().map(|error| error.to_string()),
    });
    Ok(if verified {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn bench_iterations(options: &Options) -> Result<(usize, usize), String> {
    // The two older environment variables remain accepted for existing runbooks.
    let environment = |name: &str, default: usize| match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("{name} must be a non-negative integer")),
        Err(_) => Ok(default),
    };
    let warmup = match options.verify_warmup {
        Some(value) => value,
        None => environment("VERIFY_WARMUP_ITERATIONS", DEFAULT_VERIFY_WARMUP_ITERATIONS)?,
    };
    let iterations = match options.verify_iterations {
        Some(value) => value,
        None => environment("VERIFY_TIMED_ITERATIONS", DEFAULT_VERIFY_TIMED_ITERATIONS)?,
    };
    Ok((warmup, iterations))
}

fn command_bench(options: Options) -> Result<ExitCode, String> {
    let witness = load_witness(options.witness.as_deref().ok_or("--witness is required")?)?;
    let trusted_root = options.root()?;
    let (verify_warmup, verify_iterations) = bench_iterations(&options)?;
    if options.zk {
        return run_bench_zk(BenchPlan {
            proof_options: ProverMode::Optimized.proof_options(witness.depth()),
            mode: ProverMode::Optimized,
            session: options.session()?,
            root_source: if trusted_root.is_some() {
                "supplied"
            } else {
                "witness"
            },
            witness,
            trusted_root,
            prove_warm: options.prove_warm.unwrap_or(DEFAULT_PROVE_WARM_ITERATIONS),
            verify_warmup,
            verify_iterations,
        });
    }
    let mode = options.mode.ok_or("--mode is required")?;
    let default_fri = mode.proof_options(witness.depth()).to_fri_options();
    let proof_options = options_with_fri(
        options.fri_folding.unwrap_or(default_fri.folding_factor()),
        options
            .fri_remainder
            .unwrap_or(default_fri.remainder_max_degree()),
    );
    run_bench(BenchPlan {
        proof_options,
        session: options.session()?,
        witness,
        mode,
        root_source: if trusted_root.is_some() {
            "supplied"
        } else {
            "witness"
        },
        trusted_root,
        prove_warm: options.prove_warm.unwrap_or(DEFAULT_PROVE_WARM_ITERATIONS),
        verify_warmup,
        verify_iterations,
    })
}

/// Forms used by the earlier runbooks: `--witness <file> [--mode <mode>] <limb0> <limb1>`, and
/// `[depth] [<limb0> <limb1>]` with a generated path in place of a witness file.
fn command_legacy(arguments: &[String]) -> Result<ExitCode, String> {
    let options = parse_options(arguments)?;
    let (witness, mode, roots) = match &options.witness {
        Some(path) => (
            load_witness(path)?,
            options.mode.unwrap_or(ProverMode::Baseline),
            options.positional.as_slice(),
        ),
        None => {
            let (depth, roots) = match options.positional.split_first() {
                Some((first, rest)) => (
                    first
                        .parse()
                        .map_err(|_| format!("unknown command {first}\n\n{USAGE}"))?,
                    rest,
                ),
                None => (DEFAULT_SYNTHETIC_DEPTH, &[][..]),
            };
            let witness =
                Witness::synthetic(depth, SYNTHETIC_SEED).map_err(|error| error.to_string())?;
            (witness, options.mode.unwrap_or(ProverMode::Baseline), roots)
        }
    };
    let trusted_root = match roots {
        [limb0, limb1] => Some(parse_root(limb0, limb1)?),
        [] if options.witness.is_none() => None,
        [] => return Err("witness mode requires the two root limbs".into()),
        _ => return Err("supply both root limbs or neither".into()),
    };
    let (verify_warmup, verify_iterations) = bench_iterations(&options)?;
    run_bench(BenchPlan {
        proof_options: mode.proof_options(witness.depth()),
        session: SessionId::NONE,
        witness,
        mode,
        root_source: if trusted_root.is_some() {
            "ethereum"
        } else {
            "witness"
        },
        trusted_root,
        prove_warm: 0,
        verify_warmup,
        verify_iterations,
    })
}

fn print_json(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string(value).expect("metric records always serialize")
    );
}

fn run(arguments: &[String]) -> Result<ExitCode, String> {
    match arguments.split_first() {
        Some((command, rest)) if command == "prove" => command_prove(parse_options(rest)?),
        Some((command, rest)) if command == "verify" => command_verify(parse_options(rest)?),
        Some((command, rest)) if command == "bench" => command_bench(parse_options(rest)?),
        Some((command, _)) if command == "--help" || command == "help" => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        _ => command_legacy(arguments),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("prover-verifier: {message}");
            ExitCode::from(2)
        }
    }
}
