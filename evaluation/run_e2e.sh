#!/usr/bin/env bash
# End-to-end protocol run on a local Hardhat chain, with the negative cases.
#
# Starts a Hardhat node, deploys the registry, issues a credential batch, publishes its root,
# fetches the root back from the chain, then proves and verifies in baseline and folded modes
# against the fetched root. Every check prints PASS or FAIL; the script exits non-zero on any FAIL.
#
# Usage (from the repository root, on Linux or WSL):  bash evaluation/run_e2e.sh [depth]
# Output: evaluation/results/e2e-<timestamp>/ (log, proofs, JSON records).
set -uo pipefail

depth="${1:-7}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${repo}/evaluation/results/e2e-$(date -u +%Y%m%dT%H%M%SZ)"
work="$(mktemp -d)"
mkdir -p "${out}"
exec > >(tee "${out}/e2e.log") 2>&1

failures=0
pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; failures=$((failures + 1)); }
expect_exit() { # expected-code description command...
    local expected="$1" description="$2"
    shift 2
    "$@" > "${work}/last.json" 2> "${work}/last.err"
    local code=$?
    cat "${work}/last.json" >> "${out}/records.jsonl"
    if [[ "${code}" == "${expected}" ]]; then
        pass "${description} (exit ${code})"
    else
        fail "${description}: expected exit ${expected}, got ${code}: $(cat "${work}/last.err")"
    fi
}

node_pid=""
cleanup() {
    [[ -n "${node_pid}" ]] && kill "${node_pid}" 2> /dev/null
    rm -rf "${work}"
}
trap cleanup EXIT

echo "== build"
cd "${repo}"
cargo build --release -p prover-verifier -p issuer --quiet || exit 2
bin="${CARGO_TARGET_DIR:-${repo}/target}/release"
pv="${bin}/prover-verifier"

echo "== start Hardhat node"
cd "${repo}/ethereum"
if curl -s -o /dev/null http://127.0.0.1:8545; then
    echo "port 8545 is already in use; stop the other node first" >&2
    exit 2
fi
npx hardhat node > "${out}/hardhat-node.log" 2>&1 &
node_pid=$!
for _ in $(seq 1 60); do
    curl -s -o /dev/null -X POST -H 'Content-Type: application/json' \
        --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' \
        http://127.0.0.1:8545 && break
    sleep 1
done

echo "== deploy, issue, publish, fetch"
npm run --silent deploy:local || exit 2
"${bin}/issuer" "${depth}" 3 --output "${work}/issued" || exit 2
issued_witness="$(ls "${work}"/issued/witness_*.json)"
ROOT_FILE="${work}/issued/root.json" npm run --silent publish:root || exit 2
fetched="$(npm run --silent fetch:root)" || exit 2
echo "${fetched}"
limb0="$(sed -n 's/^root_limb0: //p' <<< "${fetched}")"
limb1="$(sed -n 's/^root_limb1: //p' <<< "${fetched}")"
cd "${repo}"

issued_limbs="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d["root_limb0"], d["root_limb1"])' "${work}/issued/root.json")"
if [[ "${issued_limbs}" == "${limb0} ${limb1}" ]]; then
    pass "root fetched from Ethereum equals the issuer's root"
else
    fail "fetched root ${limb0} ${limb1} differs from issued ${issued_limbs}"
fi

verify() { # mode proof depth limb0 limb1 [extra arguments]
    local mode="$1" proof="$2" tree_depth="$3" root0="$4" root1="$5"
    shift 5
    "${pv}" verify --mode "${mode}" --proof "${proof}" --depth "${tree_depth}" \
        --root-limb0 "${root0}" --root-limb1 "${root1}" "$@"
}

echo "== valid proofs against the Ethereum root"
for mode in baseline folded optimized; do
    expect_exit 0 "${mode}: prove" \
        "${pv}" prove --witness "${issued_witness}" --mode "${mode}" --proof-out "${out}/${mode}.proof"
    expect_exit 0 "${mode}: proof verifies against the fetched root" \
        verify "${mode}" "${out}/${mode}.proof" "${depth}" "${limb0}" "${limb1}"
done

echo "== rejections"
wrong_limb0="$(python3 -c 'import sys; print((int(sys.argv[1]) + 1) % (2**128 - 45 * 2**40 + 1))' "${limb0}")"
for mode in baseline folded optimized; do
    expect_exit 1 "${mode}: wrong root rejected" \
        verify "${mode}" "${out}/${mode}.proof" "${depth}" "${wrong_limb0}" "${limb1}"
    expect_exit 1 "${mode}: swapped root limbs rejected" \
        verify "${mode}" "${out}/${mode}.proof" "${depth}" "${limb1}" "${limb0}"
    expect_exit 1 "${mode}: wrong tree depth rejected" \
        verify "${mode}" "${out}/${mode}.proof" 15 "${limb0}" "${limb1}"
done
expect_exit 1 "baseline proof rejected in folded mode" \
    verify folded "${out}/baseline.proof" "${depth}" "${limb0}" "${limb1}"

expect_exit 1 "optimized proof rejected in baseline mode" \
    verify baseline "${out}/optimized.proof" "${depth}" "${limb0}" "${limb1}"

echo "== zero knowledge"
expect_exit 0 "zk: prove" \
    "${pv}" prove --witness "${issued_witness}" --mode zk --proof-out "${out}/zk.proof"
expect_exit 0 "zk: proof verifies against the fetched root" \
    verify zk "${out}/zk.proof" "${depth}" "${limb0}" "${limb1}"
expect_exit 0 "zk: a second proof of the same witness" \
    "${pv}" prove --witness "${issued_witness}" --mode zk --proof-out "${out}/zk2.proof"
if cmp -s "${out}/zk.proof" "${out}/zk2.proof"; then
    fail "zk: repeated proofs are identical"
else
    pass "zk: repeated proofs differ (fresh randomness)"
fi
expect_exit 1 "zk: wrong root rejected" \
    verify zk "${out}/zk.proof" "${depth}" "${wrong_limb0}" "${limb1}"
expect_exit 1 "zk: wrong tree depth rejected" \
    verify zk "${out}/zk.proof" 15 "${limb0}" "${limb1}"
expect_exit 1 "optimized proof rejected in zk mode" \
    verify zk "${out}/optimized.proof" "${depth}" "${limb0}" "${limb1}"
zk_session=(--session-limb0 777 --session-limb1 888)
expect_exit 0 "zk: prove bound to a session" \
    "${pv}" prove --witness "${issued_witness}" --mode zk --proof-out "${out}/zk-session.proof" \
    "${zk_session[@]}"
expect_exit 0 "zk: proof verifies in its own session" \
    verify zk "${out}/zk-session.proof" "${depth}" "${limb0}" "${limb1}" "${zk_session[@]}"
expect_exit 1 "zk: proof replayed in another session rejected" \
    verify zk "${out}/zk-session.proof" "${depth}" "${limb0}" "${limb1}" \
    --session-limb0 777 --session-limb1 889

echo "== session binding: a proof made for one verifier challenge fails for another"
session=(--session-limb0 1234567890 --session-limb1 987654321)
other_session=(--session-limb0 1234567890 --session-limb1 987654322)
expect_exit 0 "optimized: prove bound to a session" \
    "${pv}" prove --witness "${issued_witness}" --mode optimized \
    --proof-out "${out}/session.proof" "${session[@]}"
expect_exit 0 "optimized: proof verifies in its own session" \
    verify optimized "${out}/session.proof" "${depth}" "${limb0}" "${limb1}" "${session[@]}"
expect_exit 1 "optimized: proof replayed in another session rejected" \
    verify optimized "${out}/session.proof" "${depth}" "${limb0}" "${limb1}" "${other_session[@]}"
expect_exit 1 "optimized: session-bound proof rejected without the session" \
    verify optimized "${out}/session.proof" "${depth}" "${limb0}" "${limb1}"

python3 - "${out}/folded.proof" "${work}/corrupted.proof" <<'PY'
import sys
data = bytearray(open(sys.argv[1], "rb").read())
data[len(data) // 2] ^= 0x01
open(sys.argv[2], "wb").write(data)
PY
expect_exit 1 "corrupted proof rejected" \
    verify folded "${work}/corrupted.proof" "${depth}" "${limb0}" "${limb1}"

python3 - "${issued_witness}" "${work}/tampered.json" <<'PY'
import json, sys
witness = json.load(open(sys.argv[1]))
witness["path"][0]["limb0"] = str(int(witness["path"][0]["limb0"]) ^ 1)
json.dump(witness, open(sys.argv[2], "w"))
PY
expect_exit 0 "tampered witness still produces a proof (for a different root)" \
    "${pv}" prove --witness "${work}/tampered.json" --mode folded --proof-out "${work}/tampered.proof"
expect_exit 1 "tampered witness rejected against the Ethereum root" \
    verify folded "${work}/tampered.proof" "${depth}" "${limb0}" "${limb1}"

python3 - "${issued_witness}" "${work}/noncanonical.json" <<'PY'
import json, sys
witness = json.load(open(sys.argv[1]))
witness["value_limb0"] = str(2**128 - 45 * 2**40 + 1)
json.dump(witness, open(sys.argv[2], "w"))
PY
expect_exit 2 "non-canonical witness value refused before proving" \
    "${pv}" prove --witness "${work}/noncanonical.json" --mode folded --proof-out "${work}/x.proof"

echo "== root rotation: after the issuer publishes a new batch, old proofs stop verifying"
"${bin}/issuer" "${depth}" 0 --output "${work}/rotated" > /dev/null || exit 2
cd "${repo}/ethereum"
ROOT_FILE="${work}/rotated/root.json" npm run --silent publish:root > /dev/null || exit 2
rotated="$(npm run --silent fetch:root)" || exit 2
cd "${repo}"
expect_exit 1 "folded proof for the old batch rejected against the new on-chain root" \
    verify folded "${out}/folded.proof" "${depth}" \
    "$(sed -n 's/^root_limb0: //p' <<< "${rotated}")" "$(sed -n 's/^root_limb1: //p' <<< "${rotated}")"

echo
if [[ "${failures}" == 0 ]]; then
    echo "ALL CHECKS PASSED  (log: ${out}/e2e.log)"
else
    echo "${failures} CHECK(S) FAILED  (log: ${out}/e2e.log)"
    exit 1
fi
