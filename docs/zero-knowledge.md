# Zero-knowledge construction

The `zk` mode proves, in zero knowledge, the statement *"I know a credential and an authentication
path that hash to this Ethereum-anchored root."* This document gives the construction, the
simulation argument, the parameters, and the limits. Code: `prover-verifier/src/zk/`. Measured cost:
[`results.md`](results.md).

**Source note.** The construction below was derived and proved directly against this codebase. The
paper suggested as the starting point (Haböck and Al Kindi, ePrint 2024/1037) could not be read in
full: its host refused automated retrieval, and only its abstract-level description was available.
That description confirms randomization of witness polynomials outside the trace domain, a code
`RS[D, |H| + h]`, and a decomposed quotient. Nothing below is attributed to it beyond that. The team
should compare this construction with the paper, especially the amount of randomness required and
the treatment of the quotient at `g·x`.

## What is claimed

| Property | Status here | Precise meaning |
|---|---|---|
| Hiding the witness directly | Yes | Implied by zero knowledge, and much weaker. Not what is claimed. |
| Honest-verifier zero knowledge | Yes | The interactive protocol's transcript is simulated exactly from public data, up to the negligible events below. |
| Non-interactive zero knowledge | Yes, in the random-oracle model | Fiat-Shamir over Blake3 modelled as a random oracle. The simulator programs the oracle. |
| Statistical or computational | Computational in practice | Statistical given truly uniform masks. Masks come from ChaCha20 seeded by the OS, so against a real attacker it rests on that PRG. |
| Perfect zero knowledge | **No** | There are negligible failure events and random-oracle query bounds. |
| Unlinkability of proofs | Follows from zero knowledge | Each proof can be simulated from public data with fresh masks, so two proofs of one holder share nothing beyond the statement. Depends on fresh randomness per proof. |
| Replay resistance | **Only with a session challenge** | A separate property. Provided by `SessionId`, not by zero knowledge. |

Security model: a verifier that follows or deviates arbitrarily, sees the whole proof and knows the
whole credential batch, makes at most `Q` random-oracle queries, and has no side channel into the
prover. Timing and cache side channels are out of scope.

## Why the earlier design leaked

Winterfell 0.13.1's transcript reveals, per column: LDE values at each query, values at `z` and
`g·z`; composition columns at the same; and the whole DEEP-based FRI data. None of it is masked. Each
value is a deterministic function of the witness, so a verifier who knows the batch recomputes a
candidate's trace and tests it. `leaf_index_is_not_recoverable` and the batch-identification test
demonstrate this for the non-zero-knowledge modes.

Three less obvious sources also need treatment:

1. **The quotient at a query depends on an unopened point.** `Q(x)` is computed from the trace at
   `x` **and at `g·x`**, because of the AIR's next-row references. Opening the composition at `x`
   therefore leaks a function of the trace at `g·x`, which is never opened.
2. **The batched polynomial.** In `optimized` mode it is sent whole. If unmasked, it publishes every
   coefficient of a witness-dependent polynomial.
3. **Merkle commitments.** Unopened rows appear as sibling digests in authentication paths. If a row
   were a deterministic function of what the verifier can compute, a batch-aware verifier could
   test candidates against those digests. Every row must keep conditional entropy.

Winterfell's composition columns are also opened at `g·z`, which the protocol does not need. The ZK
mode opens them only at `z`.

## The construction

Notation. `n` is the trace length, `H` the trace domain, `g` its generator, `Z_H = X^n − 1`, and `D`
the evaluation domain, a coset of size `8n` disjoint from `H`. `s` is the number of queries.

**Prover.**

1. Build the execution trace. Interpolate each of the 7 columns to `T_i`, of degree below `n`.
2. Sample `r_i` uniformly with `deg r_i < h`, and set `T̃_i = T_i + Z_H · r_i`. Sample a mask column
   `m` uniformly with `deg m < h_m`.
3. Commit to the rows of `(T̃_0, …, T̃_6, m)` over `D` (one Merkle tree).
4. Draw constraint coefficients. Compute the quotient `Q` from the `T̃_i` on `D`, as coefficients.
   Set `Q'(X) = Q(X) + m(gX)`.
5. Cut `Q'` into chunks of `n` coefficients, `Q'_0 … Q'_{k−1}`. Sample `ρ_0 … ρ_{k−2}` uniformly of
   degree below `h_ρ`, and set `Q̃_0 = Q'_0 + X^n ρ_0`, `Q̃_j = Q'_j + X^n ρ_j − ρ_{j−1}`, and
   `Q̃_{k−1} = Q'_{k−1} − ρ_{k−2}`. The masks telescope, so `Σ_j X^{jn} Q̃_j = Q'`.
6. Sample the batching mask `R` uniformly with `L` coefficients, `L = n + h − 1`. Commit to the rows
   of `(Q̃_0, …, Q̃_{k−1}, R)` over `D` (second Merkle tree).
7. Draw `z`. Send `T̃_i(z)`, `T̃_i(gz)`, `m(z)`, `m(gz)`, `Q̃_j(z)`.
8. Draw batching weights `a_i` and `c_j`. Compute
   `B = Σ_i a_i[(T̃_i − T̃_i(z))/(X − z) + (T̃_i − T̃_i(gz))/(X − gz)] + Σ_j c_j (Q̃_j − Q̃_j(z))/(X − z)`
   (the sum over `i` includes `m`), and send `f = R + B` in full, as `L` coefficients.
9. Draw `s` query positions in `D`. Open the rows of both trees at them.

**Verifier.** Replays the transcript. Checks the Merkle openings, then
`N(z)/Z(z) + m(gz) = Σ_j z^{jn} Q̃_j(z)` (where `N/Z` is the constraint evaluation at the opened
frame), then, at every query `x`, that `f(x)` equals `R(x)` plus the batched openings. Because `f`
has exactly `L` coefficients, the degree bound is enforced by the proof format.

### Masking degrees

| Mask | Coefficients | Covers | At depth 7 (s = 37) |
|---|---|---|---|
| `r_i` (each trace column) | `h = s + 3` | `s` queries, `z`, `gz`, one spare | 40 |
| `m` | `h_m = 2s + 3` | `s` queries, `s` shifted pads at `g·x`, `z`, `gz`, one spare | 77 |
| `ρ_j` (each chunk mask) | `h_ρ = s + 2` | `s` queries, `z`, one spare | 39 |
| `R` | `L = n + h − 1` | The whole batched polynomial | 103 |

`m` lets the trace masks stay at `s + 3`, instead of `2s + 3` for all seven columns. That choice is
what avoids enlarging the domain.

## Simulation argument

Fix the verifier's coins. Write `X` for the set of queried points, assumed to avoid `H`, `z`, and
`gz` (the failure probability is below `(|D| + 2n + 2s + 2)/|F|`, under `2^−114` here).

**Lemma 1 (masked columns).** For any `T_i` and any `h − 1` distinct points outside `H`, the values
`T̃_i(p)` are uniform and independent. *Proof.* A polynomial of degree below `h` can take any values at `h`
distinct points, so `r_i ↦ (r_i(p))` is onto, and `Z_H(p) ≠ 0` outside `H`. So `T̃_i(p) = T_i(p) + Z_H(p) r_i(p)` is uniform. The opened
points number at most `s + 2 ≤ h − 1`. Trace columns are masked independently. ∎

**Lemma 2 (mask column and quotient).** Conditioned on all opened `T̃`, the values `m(p)` for the
opened `p ∈ X ∪ {z, gz}` and the pads `m(g x)` for queries `x` with `g x ∉ X` are uniform and
independent, since there are at most `2s + 2 ≤ h_m − 1` such points. Now consider `Q'(x) = Q(x) +
m(gx)` at a query `x`. If `gx ∉ X`, `m(gx)` is a fresh one-time pad, so `Q'(x)` is uniform and
independent of the witness. If `gx ∈ X`, `Q(x)` is a function of rows that are all opened and `m(gx)`
is opened, so `Q'(x)` is a public function of them. ∎

**Lemma 3 (chunks).** Fix `T̃` and `m`, hence `Q'`. For an opened point `p`, the map from
`(ρ_0(p), …, ρ_{k−2}(p))` to `(Q̃_0(p), …, Q̃_{k−2}(p))` is triangular with diagonal `p^n ≠ 0`, so
those `k − 1` values are uniform. `Q̃_{k−1}(p)` is then fixed by `Σ_j p^{jn} Q̃_j(p) = Q'(p)`. The
masks have degree below `s + 2`, enough for the `s + 1` points `X ∪ {z}`. At `z`, `Q'(z)` is a public
function of the out-of-domain frame by the verifier's own check. ∎

**Lemma 4 (batched polynomial).** `R` is uniform over polynomials with `L` coefficients and
independent of everything else, so `f = R + B` is uniform and independent of `B`. Given `f`, each
`R(x) = f(x) − B(x)` is determined by opened values. ∎

**Lemma 5 (commitments).** A row at an unopened position is one that the verifier never receives.
Each trace column keeps one free coefficient beyond the opened points, and each `ρ_j` likewise, so
such a row has at least 128 bits of conditional entropy, even to a verifier who knows the witness.
Modelling the hash as a random oracle, the verifier learns nothing from a digest unless it queries
the oracle on the exact row, which it does with probability at most `Q · 2^−128` per row. Sibling
digests and roots can therefore be simulated by uniform values, with the oracle programmed at the
opened rows.

**Simulator.** On input the statement and coins: sample the opened `T̃` and `m` uniformly; sample
each `Q'(x)` uniformly when `gx ∉ X`, and compute it from opened rows otherwise; compute `Q'(z)` from
the out-of-domain frame; sample chunks uniformly on the hyperplane given by Lemma 3; sample `f`
uniformly; set `R(x) = f(x) − B(x)`. The output has exactly the distribution given by Lemmas 1-4,
and contains no witness. `prover-verifier/src/zk/simulator.rs` implements this.

**From interactive to non-interactive.** Commitments have high entropy (Lemma 5), so the simulator
programs the random oracle at transcript prefixes the adversary has not queried, except with
probability about `Q · 2^−128`. The total distance from the real distribution is at most
`(|D| + 2n + 2s + 2)/|F| + O(Q · 2^−128)`.

## What the tests add

They support the argument and do not replace it.

| Test | What it establishes |
|---|---|
| `another_holders_masks_reproduce_the_same_view` | Executes the argument: given a real proof by one holder, it constructs another holder's masks that produce exactly the same opened values under the same challenges. Passes across trials. |
| `simulated_views_pass_the_verifier_without_a_witness` | The simulator's output passes every algebraic check, without a witness. |
| `verifier_who_knows_the_batch_cannot_match_openings_to_a_holder` | The attack that identifies holders in the non-ZK modes finds nobody here. |
| `repeated_proofs_use_fresh_masks_everywhere` | No opened value repeats between proofs. |
| `every_corrupted_byte_is_rejected_without_a_crash` | Every byte, two bit patterns. |
| `proof_replayed_in_another_session_is_rejected`, wrong root, wrong credential, tampered witness, wrong depth, altered `f` | Soundness and binding. |
| `opened_values_have_balanced_bits_across_holders` | A weak statistical check, and only that. |

## Parameters and soundness

Masking raises the degree bound, so the rate rises from `1/8` and the query count must rise to hold
security. `s` is the smallest value that reaches the 83-bit target of the other modes:
`⌊s · log₂(|D|/L)⌋ − 1 ≥ 83`, which is Winterfell's conjectured formula with the true rate. Proven
levels use Winterfell's own theorems with the rate replaced.

| Depth | `n` | `|D|` | `s` | `h` | `h_m` | `L` | Rate | Chunks | Conjectured | Proven LDR | Proven UDR |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 7 | 64 | 512 | 37 | 40 | 77 | 103 | 0.201 | 8 | 84 | 41 | 27 |
| 15 | 128 | 1,024 | 32 | 35 | 67 | 162 | 0.158 | 7 | 84 | 42 | 25 |
| 31 | 256 | 2,048 | 30 | 33 | 63 | 288 | 0.141 | 6 | 83 | 42 | 24 |
| 63 | 512 | 4,096 | 29 | 32 | 61 | 543 | 0.133 | 6 | 83 | 42 | 23 |

The non-zero-knowledge modes report 83 / 41 / 23 bits, for comparison. The test
`security_meets_the_target_after_masking` confirms that each depth reaches the target. Depths below 7
are refused, since the masks do not fit in the trace.

Soundness is unchanged in structure. `T̃_i` still equals the trace on `H`, so the AIR is enforced as
before. `Z_H · m(gX)` is a multiple of the divisor, so `m` cannot make unsatisfied constraints pass.
`R` is committed before the batching weights are drawn. With no folding rounds, the only
low-degree check is the comparison of `f` with the batched openings, and `f` has a fixed length.

### Does zero knowledge cost 2×?

An earlier report estimated about 2×, by assuming the trace had to double. That is not the case for
this construction:

| Approach | Depth 7 domain | Depth 7 queries | Effect |
|---|---|---|---|
| Masking all columns for `2s + 3` | 1024 (doubled), because at 512 the quotient does not fit | 28 | LDE, hashing, and evaluation all double |
| **This construction** | **512 (unchanged)** | **37** | Measured against `optimized`: +16.4% proving, +42.0% heap (`results.md`) |

## Optimizations carried over

| Optimization | Status in the ZK mode |
|---|---|
| Compact storage (partial trees, exact-width rows) | **Retained**, same code. A masked oracle's rows are simply wider. |
| Rescue lazy MDS, inverse S-box chain | **Retained**, unchanged. |
| No-folding low-degree check | **Retained**, and is the natural fit: masking `f` makes the whole polynomial uniform. |
| Session binding | **Retained.** Now also separates the ZK transcript. |
| Malformed-proof protections | **Retained**: canonical fixed-layout format, every length checked. |
| Winterfell `Air`, `MerkleTree`, FFT, coin | **Reused.** |
| Winterfell `DefaultConstraintEvaluator` | **Replaced in ZK only.** Its debug check assumes an unmasked trace degree. |

The non-zero-knowledge modes are untouched and remain byte-identical to before.

## Why not fork Winterfell, or move to Plonky3

Winterfell's `Prover::generate_proof` has no extension point for extra committed polynomials,
masking `B`, or dropping the `g·z` composition opening, so changing it means patching
`winter-prover`, `winter-verifier`, and the proof format. The same result was reached with no fork,
by building a separate protocol on Winterfell's public crates. Maintaining a fork would risk
disturbing the reference modes.

Plonky3 was not adopted, because no limitation of Winterfell blocks the construction. Porting would
mean rewriting the AIR and arithmetic for a different framework, losing the compact storage work.
Its documentation describes a hiding polynomial commitment (`HidingFriPcs`); a hiding commitment
alone is not a complete zero-knowledge argument, and this review did not establish more than that
from its source.

## Limits

- **No FRI folding.** `f` is sent whole, so proof size grows linearly with the trace. This is the
  right choice up to depth 63 but is not succinct. Folding with the same `R` is compatible with the
  argument (the folded layers are functions of a uniform `f`), but is not implemented.
- **Minimum depth 7.** Below that the trace is shorter than the masks.
- **The random-oracle model** is an idealization of Blake3.
- **Fresh randomness is essential.** A prover that reuses masks loses zero knowledge. The CLI seeds a
  new ChaCha20 stream from the OS for each proof.
- **Side channels** are not addressed, and the proof does not hide the tree depth or the root.
