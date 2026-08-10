# credential-zkp-project

Transparent hybrid-STARK proving pipeline (eSTARK-style AIR/FRI + Split-inspired
memory optimization + Ligerolight-inspired batched opening) for privacy-preserving
credential verification, with Ethereum as trust anchor only.

Architecture: see `docs/architecture-diagrams/`. Methodology: see `docs/methodology.md`.

## Prerequisites (install locally — not available in this environment)

| Tool | Version | Install |
|---|---|---|
| Rust | stable, ≥1.75 | https://rustup.rs |
| Node.js | ≥18 | https://nodejs.org |
| Python | ≥3.10 | system package manager |

## Setup

```bash
# Rust workspace
cargo check                     # verifies winterfell + deps resolve

# Ethereum toolchain
cd ethereum
npm install
npm install --save-dev @nomicfoundation/hardhat-toolbox
npx hardhat node                # separate terminal, keep running
npm run deploy:local            # second terminal

# Evaluation toolchain
cd ../evaluation
python3 -m venv venv && source venv/bin/activate
pip install -r requirements.txt
```

## Acceptance check for Phase 1

- [ ] `cargo check` completes with no errors (confirms `winterfell = "0.13"` resolves
      and the toolchain compiles it — this was not run in the environment that
      generated this scaffold; run it first on your machine and report any
      version conflicts before starting Phase 2)
- [ ] `npx hardhat node` starts a local chain on `127.0.0.1:8545`
- [ ] `npm run deploy:local` prints a deployed contract address
- [ ] `python3 -c "import pandas, matplotlib, numpy"` runs with no import errors

## Phase status

1. Repo scaffold, toolchains — **done, this response**
2. Baseline STARK pipeline (Winterfell `merkle` example, unmodified) — pending
3. Ethereum stub wiring to prover/verifier — pending
4. Issuer module — pending
5. Split-inspired memory optimization — pending
6. Ligerolight-inspired batched opening — pending
7. Evaluation harness — pending
8. Graphs + final report artifacts — pending

Folder structure, interfaces, and module names above are frozen per project rules —
do not rename `issuer/`, `prover-verifier/`, `ethereum/`, or `evaluation/` in later
phases.
