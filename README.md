# credential-zkp-project

Privacy-preserving credential membership with a transparent STARK, and Ethereum as the trust anchor.

An issuer commits a batch of credentials to a Rescue Merkle tree and publishes only the root to a
`CredentialRegistry` contract. A holder proves, with a Winterfell STARK, that their credential is a
leaf of that tree, without revealing the credential or its authentication path. The verifier reads
the root from the contract and checks the proof off-chain. Ethereum stores the root and does no
proof verification.

- Architecture diagrams: `docs/architecture-diagrams/`
- Methodology (as proposed): `docs/methodology.md`
- Guarantees, fixes, and limits: [`docs/security-and-privacy.md`](docs/security-and-privacy.md)
- Metric definitions and how to reproduce them: [`docs/benchmarking.md`](docs/benchmarking.md)
- Latest results: [`docs/results.md`](docs/results.md)

## What is implemented

| Component | Implementation |
|---|---|
| Proof system | Winterfell 0.13.1 STARK: AIR, FRI, and Fiat-Shamir with Blake3-256. Transparent, with no trusted setup. |
| Statement | Knowledge of a credential and a Merkle path that hash to the public root (`prover-verifier/src/air.rs`). |
| Credential tree | Rescue-XLIX Merkle tree (`merkle-core`), shared by the issuer and the prover. |
| `baseline` mode | FRI folding factor 8, Winterfell's default storage. The reference STARK. |
| `optimized` mode (recommended) | Compact storage, and an FRI shape chosen by measurement. Same query count, blowup, grinding, field, and reported security. |
| `zk` mode | **Zero knowledge**: masked trace, quotient, and batched polynomial, with a separate transcript and proof format. Built on Winterfell's public crates. Depth 7 and above. |
| `folded` mode | FRI folding factor 16 with compact storage. The earlier optimized configuration, kept for comparison. |
| Compact storage | Partial Merkle trees and exact-width rows (`prover-verifier/src/compact.rs`). Output is byte-identical to Winterfell's default storage. |
| Session binding | An optional verifier challenge in the public inputs (`--session-limb0`, `--session-limb1`), which prevents replay. |
| Trust anchor | `ethereum/contracts/CredentialRegistry.sol`: issuer-only `publishRoot`, public `getRoot`. |

The proposal's eSTARK extended AIR, Split's circuit partitioning, and Ligerolight's batch zk-IPA are
**not** implemented. `docs/methodology.md` explains why, and what is used instead. Only the `zk` mode
is zero knowledge: `docs/zero-knowledge.md` gives the construction and security argument, and
`docs/security-and-privacy.md` lists what each mode can claim.

## Requirements

Winterfell 0.13.1 does not build from a native Windows checkout (a dependency contains a
Windows-reserved path), so use Linux or WSL for everything below.

- Rust stable (tested with 1.97.1)
- Node.js 18 or later with npm (Hardhat prints a warning on Node 18; it works)
- Python 3.10 or later (standard library only)

```bash
cd ethereum && npm install && cd ..
```

## Usage

```bash
cargo build --release
```

Issue a batch of 2^7 credentials and the witness for holder 3:

```bash
target/release/issuer 7 3
```

This writes `issuer/output/root.json` (public) and `issuer/output/witness_3.json` (private, file
mode 0600). Then publish and fetch the root on a local chain. Run `npx hardhat node` in a second
terminal first.

```bash
cd ethereum
npm run deploy:local
npm run publish:root
npm run fetch:root
cd ..
```

`publish:root` reads `issuer/output/root.json`, or `ROOT_LIMB0` and `ROOT_LIMB1` when set.
`fetch:root` prints `root_limb0` and `root_limb1`.

Prove as the holder, then verify against the fetched root as the verifier:

```bash
target/release/prover-verifier prove --witness issuer/output/witness_3.json --mode optimized --proof-out proof.bin
```

```bash
target/release/prover-verifier verify --proof proof.bin --mode optimized --depth 7 --root-limb0 <limb0> --root-limb1 <limb1>
```

`verify` prints a JSON record and exits with code 0 when the proof is accepted, 1 when it is
rejected, and 2 for invalid input. The modes are `baseline`, `folded`, `optimized`, `zk`, and, for
ablation only, `split` (folding factor 8 with compact storage) and `folded-default` (folding factor 16 with
Winterfell's storage). To bind a proof to a session, pass the verifier's challenge to both `prove`
and `verify` with `--session-limb0` and `--session-limb1`.

`prover-verifier bench --witness <file> --mode <mode>` proves and verifies in one process and
prints every metric. The earlier form `prover-verifier --witness <file> --mode <mode> <limb0>
<limb1>` still works.

## Tests

```bash
cargo test --workspace
```

```bash
cd ethereum && npm test
```

```bash
bash evaluation/run_e2e.sh
```

The Rust suite covers acceptance in every mode and depth, rejection of wrong roots, tampered
witnesses, other modes, weakened parameters, other depths, replayed sessions, and corrupted bytes,
byte-identity of the compact prover, the exact arithmetic, determinism, and the index-recovery
attack. `run_e2e.sh` runs the whole protocol on
a fresh Hardhat node, including all rejections and a root rotation.

## Benchmarks

One command builds everything, runs every experiment, and generates all outputs:

```bash
python3 evaluation/run_benchmarks.py
```

Each run creates a new `evaluation/results/<timestamp>/` holding `raw.jsonl`, `runs.csv`,
`summary.csv`, `summary.json`, `comparisons.csv`, `stages.json`, `report.md`, and `charts/*.svg`.
Earlier runs are never overwritten. The original implementation is rebuilt from
`evaluation/reference/` so that it is measured under identical definitions. Use `--quick` for a
one-minute check of the pipeline. See `docs/benchmarking.md`.

## Layout

```
merkle-core/      Rescue hash, Merkle root reconstruction, witness format and validation
issuer/           batch issuance: root.json (public) and witness_<i>.json (private)
prover-verifier/  AIR, provers (default and compact storage), verifier, CLI
ethereum/         CredentialRegistry contract, deploy/publish/fetch scripts, contract tests
evaluation/       benchmark harness, end-to-end script, fixtures, reference original, results
docs/             methodology, security notes, benchmarking method, results
```
