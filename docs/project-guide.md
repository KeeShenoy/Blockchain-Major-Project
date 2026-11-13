# Project Guide

## An Optimized Transparent Zero-Knowledge Proving Framework for Privacy-Preserving Credential Verification on Ethereum

A beginner's explanation of the whole project, from "what is a hash?" to the final results. Read it in
order: each section uses only ideas explained before it. Numbers come from the benchmark runs in
`evaluation/results/` (details in `docs/results.md`).

---

## 1. The project in one paragraph

A college (the **issuer**) gives 128 students digital certificates. It combines all 128 into one short
fingerprint, the **Merkle root**, and publishes only that root on the **Ethereum** blockchain. Later a
student (the **holder**) wants to convince a company (the **verifier**) that they own one of those
certificates, **without showing which certificate, which position, or anything else about it**. The
student's computer produces a **zero-knowledge STARK proof**. The company reads the root from Ethereum
and checks the proof. If it verifies, the company knows the student holds a valid certificate from that
batch, and learns nothing else.

---

## 2. Basic ideas

### 2.1 Credential and the three roles

A **credential** is a digital claim about someone: a degree, a membership, a licence.

| Role | Who | What they do in our project |
|---|---|---|
| Issuer | The college | Creates credentials, builds the Merkle tree, publishes the root |
| Holder (prover) | The student | Keeps their credential secret; produces the proof |
| Verifier | The company | Reads the root from Ethereum; checks the proof |

### 2.2 Verification, privacy, and the gap between them

**Verification** means checking a claim instead of trusting it. **Privacy** means not revealing more
than necessary. Normally these pull against each other: to verify a certificate you usually look at it.
Our project gives the verifier certainty while the holder reveals nothing beyond "I am a member".

---

## 3. Cryptography basics

### 3.1 Hash functions

A **hash function** turns any input into a short, fixed-size output called a **digest**, like a
fingerprint.

- **Deterministic:** the same input always gives the same digest.
- **One-way:** you cannot get the input back from the digest.
- **Collision-resistant:** you cannot find two inputs with the same digest.
- A tiny change in the input changes the digest completely.

A hash is **not encryption**: encryption can be undone with a key, a hash cannot be undone at all.

**We use two hash functions, for two different jobs. A common viva question.**

| Hash | Used for | Why |
|---|---|---|
| **Rescue** | Building the credential Merkle tree, and inside the proof | It is made of simple field arithmetic, so it is cheap to prove inside a STARK |
| **Blake3** | The STARK's own internal commitments and random challenges | It is very fast on a normal CPU |

### 3.2 Finite fields (only what you need)

STARKs do arithmetic **modulo a large prime number** `p`: after every addition or multiplication, take
the remainder after dividing by `p`. These numbers are called **field elements**. Our prime is
`p = 2^128 − 45·2^40 + 1`, so one field element is about 128 bits (16 bytes).

Every value in our system (a credential, a hash, a root) is one or two field elements. That is why you
see "**limb0**" and "**limb1**" everywhere: a Rescue digest is **two field elements**, and each is one
"limb".

---

## 4. Merkle trees

### 4.1 Building one

Take 4 credentials A, B, C, D:

```
                 Root = H( P1 , P2 )
                /                   \
        P1 = H(hA, hB)          P2 = H(hC, hD)
         /        \               /        \
     hA=H(A)   hB=H(B)        hC=H(C)    hD=H(D)
```

- **Leaf:** the hash of one credential (in our code, `Rescue(credential)`).
- **Parent:** the hash of its two children joined together.
- **Root:** the single digest at the top. Change any credential and the root changes.

### 4.2 Depth

**Depth** is the number of levels between a leaf and the root. Leaves = 2^depth. Our main experiments
use **depth 7, so 2^7 = 128 credentials**.

### 4.3 Authentication path and index

To show that C is in the tree, you need only the **siblings** on the way up: `hD`, then `P1`. That list
is the **authentication path**. Depth 7 means 7 siblings.

The **index** is the leaf's position (C is index 2, binary `10`). Each bit of the index tells you, at
each level, whether your running hash goes on the **left or the right** before hashing.

**Problem:** a normal Merkle proof hands over the credential, the path, and the index. That reveals
exactly which credential is yours. Our STARK proof removes this problem.

---

## 5. Blockchain and Ethereum

### 5.1 Blockchain

A **blockchain** is a shared record kept by many computers. Records are grouped into **blocks**, and each
block contains the hash of the previous one, so changing old data would change every later hash. That
makes the history **tamper-evident**. Note: a blockchain stores data honestly; it does not make the
data true.

### 5.2 Ethereum, smart contracts, Solidity

**Ethereum** is a blockchain that can run programs called **smart contracts**. **Solidity** is the
language they are written in. Each change to a contract's data is a **transaction**, which costs **gas**
(a fee for computation).

### 5.3 Our contract: `CredentialRegistry`

Location: `ethereum/contracts/CredentialRegistry.sol`. It stores **only the root**. It does **not**
check proofs.

| Part | Meaning |
|---|---|
| `issuer` | The address that deployed the contract; only it may publish |
| `publishRoot(bytes32 root)` | Stores a new root; reverts with `NotIssuer` for anyone else |
| `getRoot()` | Anyone can read the current root |
| `RootPublished` event | A public log entry each time a root is published |

The root (two 16-byte limbs) is packed into one 32-byte `bytes32`, each limb in little-endian order
(`ethereum/scripts/rootCodec.js`). Limbs must be valid field elements, otherwise the script refuses.

### 5.4 Hardhat and the local node

**Hardhat** is a toolkit for developing contracts. `npx hardhat node` runs a **local practice
blockchain** on our own machine. **We never deployed to the real Ethereum network.**

### 5.5 On-chain vs off-chain

| On-chain (Ethereum) | Off-chain (our Rust programs) |
|---|---|
| Deploy contract, publish root, read root | Create credentials, build the tree, create witnesses, prove, verify |

Proofs are checked off-chain because STARK verification would be expensive in gas. Ethereum is the
**trust anchor**: everyone agrees on which root is the official one.

---

## 6. Zero-knowledge proofs

### 6.1 The idea

A **zero-knowledge proof** lets a prover convince a verifier that a statement is true **without
revealing why it is true**.

Analogy: a friend is colour-blind and holds two balls, one red, one green. You want to prove they are
different colours without saying which is which. Your friend hides them behind their back, maybe swaps,
and shows them again. You say "swapped" or "not swapped" correctly every time. After 20 rounds they are
convinced you can tell the colours apart, but they still do not know which ball is red.

### 6.2 Statement, witness, proof

| Term | In our project |
|---|---|
| **Statement** (public) | "I know a credential and a Merkle path that hash to **this root**" |
| **Witness** (secret) | The credential (2 field elements), its index, and the 7 path siblings: `witness_<i>.json` |
| **Proof** (public) | About 13-19 KB of bytes that convince the verifier |

The witness **never leaves the holder's machine**.

### 6.3 The three required properties

1. **Completeness:** an honest holder with a real witness is always accepted.
2. **Soundness:** without a real witness, a cheater is rejected (except with probability around
   2^−83 in our configuration).
3. **Zero knowledge:** the proof reveals nothing about the witness.

### 6.4 SNARK vs STARK and "trusted setup"

**SNARKs** (for example Groth16) give tiny proofs, but most need a **trusted setup**: a secret
generated once and destroyed. Whoever kept it could forge proofs. **STARKs** need **no trusted setup**:
they rely only on hash functions and public randomness. This is what **transparent** means, and it is
objective 1 of our project. STARKs are also believed to resist quantum computers. Their downside is
bigger proofs (kilobytes rather than hundreds of bytes).

---

## 7. How a STARK works, using our project

We use **Winterfell**, a Rust STARK library, and write the project-specific parts on top of it.

### Step 1: Execution trace

The **trace** is a table recording every step of the computation "hash from the leaf up to the root".

- **7 columns:** 6 hold the Rescue hash state; column 6 holds the index bit (left or right).
- **8 rows per hash:** 7 Rescue rounds, plus 1 row to load the next path sibling.
- Depth 7 means 8 hashes (1 leaf + 7 levels), so **8 × 8 = 64 rows**.

### Step 2: AIR, the rules

The **AIR** (Algebraic Intermediate Representation, `prover-verifier/src/air.rs`) lists equations
every honest trace must satisfy.

- **Transition constraints** (between consecutive rows): "the next row is one Rescue round of this
  row", "at a load row, the hash goes left or right according to the bit", "the bit is 0 or 1".
- **Boundary constraints** (fixed rows): "the last row equals the public root", and "the first row is
  a real leaf".

### Step 3: Polynomials and the low-degree extension

Each column is turned into a **polynomial** passing through its 64 values. It is then evaluated at
**8 × 64 = 512** points: the **blowup factor is 8**. This is the **LDE** (low-degree extension). The
redundancy is what makes cheating detectable.

### Step 4: Commit with a Merkle tree

The 512 rows are hashed into a Merkle tree (Blake3) and only its root is placed in the proof: a
**commitment**. The prover cannot change the data afterwards.

### Step 5: Combine the constraints

All constraint equations are mixed with random weights into one **composition polynomial**. Key
fact: if the trace is honest, the constraints are zero on every trace row, so this polynomial
divides cleanly. If anyone cheated, it does not divide, and the result is not a low-degree polynomial.

### Step 6: Fiat-Shamir, randomness without a live verifier

The verifier's random questions are produced by **hashing everything committed so far**. The
prover cannot predict them before committing, and no back-and-forth messages are needed. This makes the
proof **non-interactive**.

### Step 7: Out-of-domain check (DEEP)

A random point `z` is picked, the prover gives all polynomial values at `z`, and the verifier checks
the constraint equation there. Everything is then folded into one **DEEP polynomial**.

### Step 8: Queries and the low-degree test (FRI)

The verifier picks **28 random rows** (37 in zero-knowledge mode). The prover opens them with Merkle
paths. The verifier checks that the DEEP polynomial is really of **low degree**, which is what proves
the constraints hold everywhere.

- **FRI** (Fast Reed-Solomon IOP of Proximity) does the low-degree test by **folding** the polynomial
  in steps. The **folding factor** is how much it shrinks per step (8 in baseline, 16 in folded).
- In our `optimized` mode the polynomial is small enough to be **sent in full** and checked directly
  at the 28 rows: FRI's own final step, with no folding layers. This made proofs smaller and
  verification faster, with the same security.

**Security level.** 28 queries with blowup 8 give **83 bits** of conjectured security: a cheater
succeeds with probability about 2^−83. Winterfell also reports provable bounds of 41 and 23 bits
(list-decoding and unique-decoding regimes).

---

## 8. Our complete system

```
 ISSUER (Rust: issuer)               ETHEREUM (Solidity, Hardhat)
 make 128 credentials
 build Rescue Merkle tree  ──root──▶  CredentialRegistry.publishRoot(root)
 write root.json (public)
 write witness_i.json (secret) ─────────┐
                                        ▼
 HOLDER (Rust: prover-verifier prove)
 witness + session challenge ──▶ trace ──▶ STARK proof (bytes)
                                        │
                                        ▼
 VERIFIER (Rust: prover-verifier verify)
 getRoot() from Ethereum ──▶ check proof against the root ──▶ ACCEPT / REJECT
```

### The folders

| Folder | What it contains |
|---|---|
| `merkle-core/` | Rescue hash, Merkle path checking, witness file format and validation |
| `issuer/` | Creates credentials, tree, `root.json`, and `witness_<i>.json` |
| `prover-verifier/` | AIR, provers, verifiers, zero-knowledge module, command-line tool |
| `ethereum/` | Contract, deploy/publish/fetch scripts, contract tests |
| `evaluation/` | Benchmark harness, end-to-end test script, fixtures, original code for comparison, results |
| `docs/` | Methodology, security, zero knowledge, benchmarking, results, this guide |

### The modes

| Mode | Zero knowledge? | FRI | Purpose |
|---|---|---|---|
| `baseline` | No | Folding factor 8 | The unmodified reference STARK |
| `folded` | No | Folding factor 16 | The earlier "optimized" setting, kept for comparison |
| `optimized` | No | Polynomial sent in full (depth ≤ 31) | Fastest; the reference non-ZK configuration |
| **`zk`** | **Yes** | Masked polynomial sent in full | **The privacy-preserving mode the title promises** |
| `split`, `folded-default` | No | 8 or 16 | Ablations only: test one change at a time |

---

## 9. What we improved

### 9.1 We fixed the measurements first

- **The "35 MB" memory figure was wrong.** Running through `cargo run` makes Linux report cargo's own
  memory. The original prover itself used about 3.7 MB. Memory is now read in a way that cannot be confused.
- **Proof size depends on the witness** (the random query rows differ), so sizes are averaged over 30
  witnesses.
- **The earlier "20% faster" was noise:** single runs, rounded to whole milliseconds. Every comparison
  now uses 15 paired runs with 95% confidence intervals.

### 9.2 Optimizations that survived measurement

| Optimization | What it does | Main effect |
|---|---|---|
| Compact storage | Merkle trees store fewer levels (rebuilt on demand); rows stored without padding | Proving memory −36% |
| Faster Rescue arithmetic | Fewer reductions in the matrix step; shortcut for the inverse S-box power | Proving time −14% |
| No folding layer | Send the small DEEP polynomial in full instead of folding it | Proof −18%, verification −42% |
| Whole-program compilation (LTO) | Smaller, faster program | RSS about −13% |

All of this is with **unchanged security** (83 bits). Compact storage produces **byte-for-byte the same
proof** as Winterfell's default, a test checks this, so it cannot affect correctness.

**An honest finding:** changing the folding factor from 8 to 16 did **not** speed up proving at our
sizes, and it made verification about 78% slower. That is why `optimized` replaced `folded` as the
recommended setting.

### 9.3 Security bugs we found and fixed

| Bug in the original verifier | Fix |
|---|---|
| Accepted whatever security settings the proof itself declared: a forger could choose 2 queries | The verifier now insists on its own settings |
| Did not check the tree depth | Depth is fixed by the verifier |
| The "credential" could be any hash input, not a real leaf | Added a boundary constraint |
| Root values above `p` were silently reduced | Rejected as non-canonical |
| One flipped byte could crash the verifier with a huge memory request | All lengths checked before decoding |
| The leaf index could be recovered from any proof | Column 6 blinded with random values |
| Proofs could be replayed elsewhere | **Session binding:** the verifier's challenge is part of the proof |

---

## 10. Zero knowledge in our project

### 10.1 Why the old modes were not zero knowledge

Winterfell does not add randomness. If the verifier **knows all 128 credentials** (for example, the
issuer itself), it can compute each student's exact trace and compare it with the opened rows: one
matches, so the holder is identified. A test in the code demonstrates this attack.

### 10.2 How the `zk` mode hides everything

The idea is to **add carefully chosen randomness that disappears where the rules are checked**.

1. Each trace column becomes `T(x) + Z_H(x)·r(x)`, where `r` is random and `Z_H` is zero on all 64
   trace rows. On the trace rows nothing changes, so the AIR rules still hold. Everywhere the verifier
   looks (the 512 extended points and `z`) the values look **completely random**.
2. One **extra random column** cancels a subtle leak: checking a row also involves the next row,
   which is never opened.
3. The composition polynomial's pieces get masks that **cancel when added back together**.
4. The final DEEP polynomial gets a fully random polynomial added, so the transmitted polynomial is
   pure noise to the verifier, yet still passes the check.
5. Every mask has **one more random coefficient than the number of values revealed**, so even the
   Merkle hashes of unopened rows reveal nothing.

**Why this is proof and not just a hope:** we wrote a **simulator**, a program that produces a valid
looking proof transcript **without any witness**. If fake transcripts are indistinguishable from real
ones, a real proof cannot be carrying secret information. A second test takes a real proof by one
student and finds masks for **a different student** that produce exactly the same transcript. The full
argument is in `docs/zero-knowledge.md`.

**Precise claim:** non-interactive zero knowledge in the random-oracle model (treating the hash as
perfectly random). It needs **fresh randomness for every proof** (so two proofs by the same student
cannot be linked), and it works for **depth 7 and above**. It is not "perfect" zero knowledge, which
nobody achieves in practice.

### 10.3 What zero knowledge costs (depth 7, same benchmark run)

| | Original baseline | Optimized (no ZK) | **Zero knowledge** |
|---|---:|---:|---:|
| Trace build + proving | 2,686 µs | 2,244 µs | **2,629 µs** |
| Verification | 215 µs | 123 µs | **159 µs** |
| Proof size | 15,799 B | 12,824 B | **19,045 B** |
| Peak proving memory (heap) | 235,519 B | 150,770 B | **214,083 B** |
| Security (conjectured bits) | 83 | 83 | **84** |

Compared with the optimized mode, zero knowledge costs **+16% proving time, +29% verification, +48%
proof size, and +42% memory**. Compared with the **original** code, the ZK mode is about as fast to
prove, **25% faster to verify, and uses less memory**, while adding full privacy. The extra cost comes
from more queries (37 instead of 28), a few more polynomials, and generating the random masks.

---

## 11. Measuring performance

| Metric | Meaning |
|---|---|
| Proving time | Time to build the trace and produce the proof (warm, median) |
| Verification time | Time to check one proof (median of 100 checks, after 10 warm-ups) |
| Proof size | Exact number of bytes sent to the verifier |
| Peak heap | Most memory the proving algorithm requested at once (from a counting allocator) |
| Peak RSS | Most physical memory the whole program used (program code + libraries + heap) |

- **Median:** the middle value; it ignores freak runs.
- **Paired runs:** each configuration runs in alternating order, so background noise affects all
  configurations equally.
- **95% confidence interval:** the range the true difference likely lies in. **If it includes zero,
  we do not claim a difference.**
- **Heap vs RSS:** heap is what our code controls; RSS is mostly the program itself (about 3 MB),
  which is why RSS barely changes when heap does.

One command builds everything, runs all experiments, and draws every chart:

```bash
python3 evaluation/run_benchmarks.py
```

---

## 12. Running the project

Use Linux or WSL (Winterfell does not build on plain Windows).

```bash
cargo build --release
```

```bash
target/release/issuer 7 0
```

This creates a batch of 2^7 = 128 credentials, then writes `issuer/output/root.json` (public) and
`issuer/output/witness_0.json` (secret, for holder index 0).

In a second terminal, start the local blockchain and leave it running:

```bash
cd ethereum && npx hardhat node
```

Back in the first terminal:

```bash
cd ethereum
npm run deploy:local
npm run publish:root
npm run fetch:root
cd ..
```

These deploy the contract, publish the root, and read it back. `fetch:root` prints `root_limb0` and
`root_limb1`. Then prove as the holder:

```bash
target/release/prover-verifier prove --witness issuer/output/witness_0.json --mode zk --proof-out proof.bin
```

Then verify as the verifier, pasting in the two limbs printed by `fetch:root`:

```bash
target/release/prover-verifier verify --proof proof.bin --mode zk --depth 7 --root-limb0 <limb0> --root-limb1 <limb1>
```

Exit code 0 means accepted, 1 rejected, 2 invalid input. Change a limb and verification fails.

Run every test:

```bash
cargo test --workspace
```

```bash
bash evaluation/run_e2e.sh
```

The second runs the complete protocol on a fresh local chain, including every rejection case.

---

## 13. Limitations, stated honestly

1. **Local blockchain only**: no deployment to the real Ethereum network.
2. **Ethereum stores the root but does not verify proofs**: verification is off-chain.
3. **Zero knowledge needs depth ≥ 7**, and holds in the random-oracle model with fresh randomness per proof.
4. **No FRI folding in `zk` mode**: proof size grows with the tree; fine up to depth 63, not for huge trees.
5. **Revoking one credential is not supported**: publishing a new root invalidates the whole old batch.
6. **The issuer must be trusted** to put only real credentials in the batch.
7. **Research names are inspiration only**: we did not implement eSTARK, Split, or Ligerolight themselves (see `docs/methodology.md`).
8. **One machine** (Intel i5 laptop, WSL) for all measurements.

---

## 14. Glossary

| Term | Meaning |
|---|---|
| AIR | The equations every valid trace must satisfy |
| Blowup factor | How much the trace is stretched for the LDE (8) |
| Commitment | A hash that locks in data so it cannot be changed later |
| DEEP | Checking polynomials at a random outside point `z` |
| Field element | A number modulo our prime `p`; 16 bytes |
| Fiat-Shamir | Making challenges by hashing, so no live verifier is needed |
| FRI / folding factor | The low-degree test, and how much it shrinks per step |
| Limb | One field element of a two-element digest |
| Merkle root / path / index | Top digest / siblings to the top / leaf position |
| Query | A randomly chosen row the prover must open |
| Session challenge | A verifier's random value that binds a proof to one check, preventing replay |
| Soundness bits | 83 bits means a cheater succeeds with probability about 2^−83 |
| Trace | The table of every step of the computation |
| Transparent | No trusted setup is needed |
| Witness | The holder's secret: credential, index, and path |
| Zero knowledge | The proof reveals nothing beyond the statement being true |

---

## 15. Questions juniors should be able to answer

1. **Why publish only the root on Ethereum?** It is 32 bytes, represents all 128 credentials, and
   reveals none of them.
2. **Why two hash functions?** Rescue is cheap to prove inside a STARK; Blake3 is fast for the
   STARK's own commitments.
3. **What are limb0 and limb1?** The two field elements of a Rescue digest.
4. **Why is depth 7 equal to 128 credentials?** 2^7 = 128.
5. **Witness vs proof?** The witness is secret and stays with the holder; the proof is public and is
   sent.
6. **What does "transparent" mean?** No trusted setup, unlike most SNARKs.
7. **Why 64 rows?** 8 hashes × 8 rows per hash.
8. **Why is FRI needed?** To prove the polynomial is low-degree, which proves the rules hold on
   every row, not just the queried ones.
9. **Why did `optimized` replace `folded`?** Folding by 16 did not speed up proving, and it slowed
   verification. Sending the small polynomial directly made proofs smaller and verification faster.
10. **Why were the old modes not zero knowledge?** Without randomness, a verifier who knows the batch
    can recompute each candidate's trace and match it to the opened rows.
11. **How does masking keep the proof valid?** The mask is multiplied by `Z_H`, which is zero on the
    trace rows, so the rules still hold there.
12. **How do we know ZK works?** A simulator creates convincing transcripts with no witness, and a
    test shows another student's masks reproduce the same transcript.
13. **What does the session challenge stop?** Reusing someone's proof at a different verifier.
14. **What does a wrong root do?** Verification fails, with exit code 1.
15. **Heap vs RSS?** Heap is what our algorithm uses; RSS includes the whole program.
16. **Why 15 paired runs and confidence intervals?** Single runs are noisy; we claim only differences
    whose interval excludes zero.
17. **Does Ethereum verify the proof?** No. It stores the root; verification is off-chain.
18. **What does ZK cost?** About 16% more proving time and 48% larger proofs than `optimized`, but it
    is still faster to verify than the original code.
