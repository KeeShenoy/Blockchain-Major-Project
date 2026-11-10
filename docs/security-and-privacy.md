# Security and privacy

**The `zk` mode provides zero knowledge; every other mode does not.** The construction, the
simulation argument, and the assumptions are in [`zero-knowledge.md`](zero-knowledge.md). The tables
below distinguish the modes.

What the proof system guarantees, what was fixed in this revision, and what it does not guarantee.
Each fix has a regression test in `prover-verifier/tests/correctness.rs` or
`merkle-core/src/witness.rs`. The end-to-end script `evaluation/run_e2e.sh` repeats the main
rejections against a live local chain.

## Statement and parameters

The holder proves: *"I know a two-element credential `v`, an index `i < 2^d`, and an authentication
path whose Rescue hashes lead from `Rescue(v)` to the public root."* The verifier supplies the root,
which it reads from the `CredentialRegistry` contract, and the tree depth `d`.

| Parameter | Baseline | Folded |
|---|---|---|
| Field | f128 (2^128 − 45·2^40 + 1), no extension | same |
| Queries | 28 | 28 |
| Blowup | 8 | 8 |
| Grinding | 0 | 0 |
| FRI remainder max degree | 31 | 31 |
| FRI folding factor | **8** | **16** |
| Conjectured security | 83 bits | 83 bits |
| Proven security (LDR / UDR) | 41 / 23 bits | 41 / 23 bits |

The `optimized` mode uses the same field, queries, blowup, and grinding. It sets the FRI folding
factor to 4 and the remainder degree to `min(trace length − 1, 255)`, and reports the same 83 / 41 /
23 bits at every tested depth (`optimized_mode_reports_unchanged_security_at_every_depth`). No query
count, blowup, grinding, or field parameter has changed. The 24-byte commitment option measured in
`results.md` lowers collision resistance and is off by default.

### Is it safe to skip the FRI folding layer?

For trees up to depth 31, `optimized` sends the DEEP polynomial directly as the FRI remainder. The
verifier then evaluates that explicit low-degree polynomial at all 28 query positions and compares
the results with the DEEP values it computes from the opened trace and composition rows. This is
FRI's final-layer check with no prior folding rounds. If the committed DEEP function is δ-far from
every degree-bounded polynomial, all 28 checks pass with probability at most `(1 − δ)^28`. There is
no folding error term, so the check is at least as strong as with a folding layer, and Winterfell's
own security estimate is unchanged. `optimized_proof_with_altered_remainder_is_rejected` verifies that
an altered remainder fails.

## Soundness fixes

### 1. The verifier accepted any proof parameters the prover chose (critical)

The original verifier passed `AcceptableOptions::OptionSet(vec![proof.options().clone()])`, which
means it accepted the parameters written in the proof itself. A forger could generate a proof with
2 queries (security under 10 bits) and it would verify. The verifier now takes the options from its
own configured mode, and rejects anything else with `UnexpectedParameters`. A baseline proof
presented as folded, or the reverse, is also rejected.
Test: `proof_with_weakened_parameters_is_rejected`. That test also shows that the original check
accepts the weak proof.

### 2. Tree depth was not enforced

Depth is not encoded in the AIR. The trace length is `8·(d+1)`, and the proof declares it. A proof
over a shorter trace proves something weaker than membership at depth `d`. The verifier now
requires the trace length implied by the registry's depth. Test:
`proof_for_a_different_tree_depth_is_rejected`.

### 3. The first hash was not constrained to be a leaf

Registers 2 and 3 were unconstrained at step 0, so the "credential" could have been any 4-element
preimage, for example the two children of an internal node. Two boundary assertions now fix them to
zero. With the depth fix, this pins the statement to an actual leaf at the actual depth.

### 4. Non-canonical field encodings were silently reduced

Root limbs and witness values were parsed with `BaseElement::new(u128)`, which reduces modulo `p`.
Two different decimal strings could therefore denote the same root. Inputs at or above the modulus
are now rejected, in Rust (`parse_element`) and in the Ethereum scripts (`rootCodec.js`). Witness
indices outside `[0, 2^d)` and unsupported depths are also rejected before proving.

### 5. Hostile proof bytes could crash the verifier

Exhaustive corruption of a proof (3 bit patterns × every byte, for each mode) found these
problems:

- **Process abort.** Winterfell's decoder calls `Vec::with_capacity(declared_length)` before any
  bounds check for query values, Merkle paths, and the node vectors inside them. One flipped bit
  requested up to 10 GB, and the failed allocation aborted the process, which `catch_unwind` cannot
  intercept. `verify_bytes` now walks the whole serialized proof and checks every length prefix
  against the bytes present before decoding (`check_declared_lengths`).
- **Panics** in `TraceInfo` decoding. These are now caught and reported as `Malformed`.
- **An unread field.** With no folding layers, Winterfell never reads the proof's FRI partition
  count, so changing that byte produced a second valid encoding. The statement was unaffected, but
  the verifier now requires the expected value.

`verify_bytes` also rejects trailing bytes, because the decoder ignores them and a proof should have
one encoding. After the fixes, the sweep over 139,761 corrupted proofs (baseline, folded, optimized) gives 0
accepted and 0 aborts.
Tests: `corrupted_proof_bytes_are_never_accepted` and
`corrupted_length_prefixes_do_not_abort_the_verifier`, which together cover every byte of a proof.

### 6. Rejections were panics

An invalid proof used to end in an `expect` panic (exit code 101). The CLI now prints a JSON record
with `"verified": false` and a reason, and exits with code 1. Invalid input exits with code 2.

## Privacy

### Fixed: the leaf index was recoverable from every proof

Trace column 6 held only the index bits, repeated across each 8-row cycle, plus a constant. The
proof opens 28 rows of the trace's low-degree extension. A verifier could take each of the `2^d`
candidate indices, compute the column's extension, and check which candidate matches the opened
values. For depth 7 that is 128 candidates, and it pins down **the holder's exact position in the
batch**, which the project describes as hidden.

`leaf_index_is_not_recoverable_from_the_index_column` runs this attack. On the original column
construction it returns exactly the true index.

The fix: the AIR now constrains column 6 only in the one row per cycle where the swap reads the bit.
The prover fills every other row with pseudorandom field elements, derived from the witness as in
RFC 6979, so proving stays deterministic. For depth 7 there are 57 random rows, and the proof opens
at most 30 evaluations of the column (28 queries plus 2 out-of-domain points). The opened values are
then statistically independent of the index, and the attack finds no candidate. This costs no
measurable proving time, and the constraint degree is unchanged.

The argument requires the number of free rows to exceed the number of openings. That holds for
depth ≥ 7, is marginal at depth 3 (29 free rows), and fails at depth 1.

### Implemented and tested: session binding

The verifier can issue a challenge (`--session-limb0`, `--session-limb1`) that becomes part of the
public inputs. Public inputs seed Fiat-Shamir, so a proof is valid only for the session it was made
for. `proof_replayed_in_another_session_is_rejected` and `run_e2e.sh` verify this. Replay protection
holds only when verifiers actually issue fresh challenges; without a session argument the proof is
not bound and can be replayed. The challenge also feeds the blinding derivation, so column 6 differs
per session.

### What can honestly be claimed

| Property | `zk` mode | Other modes |
|---|---|---|
| Transparent setup | **Yes** | **Yes** |
| Witness not disclosed directly | **Yes** | **Yes** |
| Zero knowledge (honest-verifier, and non-interactive in the random-oracle model) | **Yes**: statistical given uniform masks, computational with the ChaCha20 mask stream | **No** |
| Hiding from a verifier who knows the whole batch | **Yes** (`verifier_who_knows_the_batch_cannot_match_openings_to_a_holder`) | **No** (`verifier_who_knows_the_batch_can_identify_the_holder`) |
| Hiding of the leaf position | **Yes** | Against the column-6 attack only, for depth ≥ 7 |
| Unlinkability of a holder's proofs | **Yes**, given fresh masks per proof | **No** |
| Replay protection | **When a session challenge is used** | **When a session challenge is used** |
| Perfect zero knowledge | **No** | **No** |

These are not interchangeable properties: `zero-knowledge.md` defines each. Side channels on the
prover are out of scope.

### The non-zero-knowledge modes

`baseline`, `folded`, `split`, and `optimized` are kept as reference configurations and are unchanged.
Winterfell 0.13 does not randomize their trace, composition, or FRI data. A verifier who knows the
batch can identify the holder, because for a candidate credential every trace value is determined,
and the opened rows either lie on its extension or do not.

### Root rotation

Publishing a new root makes proofs for the old batch fail, as shown in `run_e2e.sh`. Revoking a
single credential within a batch is not supported.

## Hash roles

- **Rescue-XLIX** builds the credential Merkle tree and is the hash inside the AIR. Inputs are
  fixed-length (2 or 4 elements), zero-extended without length padding. The AIR enforces exactly
  this construction, and the leaf boundary assertion keeps leaf and node inputs apart.
- **Blake3-256** builds the STARK's own commitments and drives Fiat-Shamir.
