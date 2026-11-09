// Encoding of the credential Merkle root as the contract's bytes32.
//
// The root is two elements of the prover's 128-bit field. Each element is stored as 16 bytes,
// little-endian, which is the byte order Winterfell uses for field elements, so the bytes32 equals
// the Rust digest's byte representation.

// Modulus of Winterfell's f128 field: 2^128 - 45 * 2^40 + 1.
const FIELD_MODULUS = (1n << 128n) - 45n * (1n << 40n) + 1n;

function limbToBytes(value) {
  let remaining = BigInt(value);
  if (remaining < 0n || remaining >= FIELD_MODULUS) {
    throw new Error("each root limb must be a canonical field element (0 <= limb < modulus)");
  }
  const bytes = Buffer.alloc(16);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number(remaining & 0xffn);
    remaining >>= 8n;
  }
  return bytes;
}

function bytesToLimb(bytes) {
  let value = 0n;
  for (let index = bytes.length - 1; index >= 0; index -= 1) {
    value = (value << 8n) | BigInt(bytes[index]);
  }
  return value;
}

function encodeRoot(limb0, limb1) {
  return "0x" + Buffer.concat([limbToBytes(limb0), limbToBytes(limb1)]).toString("hex");
}

function decodeRoot(rootHex) {
  const bytes = Buffer.from(rootHex.replace(/^0x/, ""), "hex");
  if (bytes.length !== 32) {
    throw new Error("a root must be exactly 32 bytes");
  }
  return [bytesToLimb(bytes.subarray(0, 16)), bytesToLimb(bytes.subarray(16, 32))];
}

module.exports = { FIELD_MODULUS, encodeRoot, decodeRoot };
