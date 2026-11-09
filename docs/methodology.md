# Methodology

This document separates what the approved synopsis **proposed** from what the code **implements**,
and records why they differ. Measured values are in [`results.md`](results.md).

## Proposed (approved synopsis)

The synopsis describes an off-chain, transparent zero-knowledge pipeline. The issuer organises
credentials in a Merkle tree and publishes the root to an Ethereum contract. The holder assembles a
witness from the credential and Merkle path, and processes it through "an eSTARK-based transparent
proving pipeline" using Extended AIR. "Split-inspired witness partitioning" reduces peak memory
during trace generation, and "Ligerolight-inspired batched query and opening techniques" optimize
proving. The verifier retrieves the root from Ethereum and checks the proof without accessing the
credential contents. The framework is evaluated against a baseline STARK on proving time, peak
memory, proof size, verification time, and scalability.

## Implemented

```
Issuer     generates credentials, builds a Rescue Merkle tree, publishes the root
Ethereum   CredentialRegistry stores the root (issuer-only publish, public read)
Holder     loads witness -> builds the 7-column execution trace, with the index column blinded
           -> AIR (Rescue rounds, Merkle swaps, leaf and root boundary conditions)
           -> compact commitment storage -> Fiat-Shamir -> DEEP composition -> FRI -> proof bytes
Verifier   reads the root from Ethereum, fixes the parameters and tree depth from its own
           configuration, optionally issues a session challenge, and verifies off-chain
```

Winterfell 0.13.1 provides the STARK. All project code reaches it through public extension points:
`Air`, `Prover`, `TraceLde`, and `ConstraintCommitment`.

| Mode | FRI folding | FRI remainder degree | Prover storage | Role |
|---|---|---|---|---|
| `baseline` | 8 | 31 | Winterfell default | Reference STARK |
| `folded` | 16 | 31 | Compact | The earlier optimized configuration, kept for comparison |
| `optimized` | 4 | `min(trace length − 1, 255)` | Compact | Recommended configuration |
| `split` | 8 | 31 | Compact | Ablation: storage change alone |
| `folded-default` | 16 | 31 | Winterfell default | Ablation: folding factor alone |

Every mode uses 28 queries, blowup 8, no grinding, and the 128-bit field, and reports the same
security: 83 conjectured, 41 proven (list decoding), 23 proven (unique decoding).

### Optimizations in the code, and what supports them

1. **Compact commitment storage** (`prover-verifier/src/compact.rs`). Merkle tree levels near the
   leaves are hashed but not stored, and are rehashed only for the rows a query touches. Rows are
   stored at their exact width instead of padded to multiples of 8. Output is byte-identical to
   Winterfell's defaults (enforced by a test), so the verifier and soundness are unaffected.
2. **Exact arithmetic in Rescue** (`merkle-core/src/rescue.rs`). The MDS matrix-vector product
   accumulates 256-bit products and reduces once per row instead of once per product, and the
   inverse S-box uses an addition chain that exploits the repeating pattern of its exponent. Both
   are checked against Winterfell's field operators on random and edge-case inputs.
3. **FRI shape chosen by measurement**. The DEEP polynomial at these trace sizes has degree at most
   254, so FRI's "send the remainder in the clear" step can cover it directly. Sending it removes
   the folding layer, its Merkle tree, and the per-query interpolation. The verifier still checks
   the explicit polynomial against the DEEP evaluations at all 28 query positions, which is FRI's own
   final check.
4. **Whole-program compilation** (`lto = "fat"`, `codegen-units = 1`).

## Mapping to the proposed techniques

### eSTARK / Extended AIR: not implemented

eSTARK adds lookup, permutation, and connection arguments to AIR. The Merkle-path statement has no
need for them: it is a fixed sequence of hash rounds and swaps with no tables or permutations, and
Winterfell's auxiliary segment would add a second commitment round for nothing. The project uses
plain AIR. The accurate description is "a STARK with AIR", not "eSTARK-based".

### Split: not reproduced; a smaller storage technique is used instead

Split divides a large computation into smaller components connected by hashes, to reduce peak
memory. Two readings were evaluated for this workload.

- **Splitting the proof statement** (one STARK per path segment, linked by hash commitments). This
  would divide peak memory by about the number of segments, but multiplies proof size and
  verification by about the same factor. At depth 7, two segments would roughly double a 12.7 KB
  proof to save about 75 KB of heap. It works against objectives 3 and 4 at these sizes, and is only
  attractive for very long traces on memory-limited devices. Not implemented.
- **Partitioning storage** (recomputing data from smaller retained state). This is what the compact
  storage does: Merkle levels are discarded and rebuilt from the rows on demand, a hash-based
  memory saving in the spirit of Split, though not its technique.

A further step was analysed and declined: evaluating constraints one coset at a time would avoid
holding the whole trace LDE. It needs a custom `ConstraintEvaluator`, a second trace FFT (Fiat-Shamir
requires the commitment before the constraint coefficients are drawn), and recomputing the 28
queried rows. From the measured stage costs that is roughly 20% more proving time for about 60 KB
less heap at depth 7, which the current priorities do not justify.

The accurate description is "compact commitment storage", optionally "Split-inspired" with the
qualification above.

### Ligerolight: its batch zk-IPA does not apply; its relevant ideas are already present

Ligerolight's batch zero-knowledge inner product argument proves inner-product relations over many
committed vectors with one sum-check and one FRI instance. It exists because Ligero's linear and
quadratic tests over an R1CS matrix reduce to inner products.

- **Relevance to this workload.** The statement is the one Ligerolight evaluates (a Merkle
  authentication path), but the arithmetization is different. An AIR has no inner-product relations
  to batch: constraints are enforced through the composition polynomial and DEEP.
- **Feasibility in Winterfell.** Winterfell's `generate_proof` is a fixed protocol sequence with no
  hook for extra rounds. Adding the IPA means replacing the arithmetization with Ligero's matrices
  and writing a new sum-check and FRI flow, in effect a different proof system. That is an estimated
  several thousand lines and contradicts the fixed direction of the project.
- **Expected benefit.** Unfavourable at this scale. Ligerolight reports 131 KB at depth 32 for a
  zero-knowledge SHA-256 circuit; this project's proof at depth 31 is about 20 KB, and an IPA adds
  sum-check rounds on top of FRI.

The ideas from the paper that transfer are already in the pipeline:

| Ligerolight idea | Where it already exists here |
|---|---|
| All evaluations at a location in one Merkle leaf | Winterfell hashes whole rows into one leaf |
| One low-degree test for many polynomials | DEEP combines all trace and composition columns into one polynomial |
| Compressing many authentication paths | Winterfell's batch Merkle multiproofs |

The folding-factor change that was previously labelled "Ligerolight-inspired" is an FRI parameter
and nothing more. A remaining idea from the paper is a smaller field: it reports FFTs 4-6× faster on
`2^64 − 2^32 + 1`. Winterfell supports that field, but moving to it means replacing Rescue with
`Rp64_256`, rewriting the AIR, and changing the root format. It is the highest-value future
performance step, but a rewrite of the statement rather than a tuning change.

## Recommended corrections to the synopsis

The synopsis is an approved document, so it has not been altered. To keep it consistent with the
implementation, the team should consider the following wording changes with the guide. No objective
needs to change.

| Current wording | Recommended | Reason |
|---|---|---|
| "eSTARK-based transparent proving pipeline" / "Extended AIR" | "transparent STARK proving pipeline (Winterfell) using AIR" | No eAIR arguments are used |
| "Split-inspired witness partitioning" | "compact commitment storage: partial Merkle trees and exact-width rows" | Split's circuit partitioning is not used |
| "Ligerolight-inspired batched query and opening" | "batched openings through Merkle multiproofs and DEEP composition, with a measured FRI configuration" | The zk-IPA is not used |
| "non-interactive zero-knowledge proof" | "non-interactive proof that does not reveal the witness directly; formal zero knowledge is not provided" | Winterfell 0.13 has no ZK; see `security-and-privacy.md` |
| Flow diagram stages "Extended AIR", "Split: memory paging", "Ligerolight: batched opening" | "AIR", "Trace + compact storage", "DEEP + FRI + multiproofs" | Same reasons |

Objective 2 mentions "witness partitioning". What is delivered is memory optimization of the
committed data rather than partitioning of the witness itself, and the report should say so.

## Hash roles

Rescue-XLIX builds the credential tree and is arithmetized in the AIR. Blake3-256 builds the STARK's
commitments and drives Fiat-Shamir.

## Scope

Ethereum stores the root and performs no proof verification. Proving and verification are
off-chain. Revocation of an individual credential is outside the scope; publishing a new root
invalidates all proofs for the old batch.
