// Publishes the issuer's Merkle root to the registry.
//
// The root comes from ROOT_LIMB0 and ROOT_LIMB1 (decimal), or, when those are unset, from the
// issuer's root.json (ROOT_FILE, default ../issuer/output/root.json).

const fs = require("fs");
const path = require("path");
const { encodeRoot } = require("./rootCodec");
const { loadRegistry } = require("./deployment");

function rootLimbs() {
  const { ROOT_LIMB0: limb0, ROOT_LIMB1: limb1 } = process.env;
  if (limb0 !== undefined || limb1 !== undefined) {
    if (limb0 === undefined || limb1 === undefined) {
      throw new Error("set both ROOT_LIMB0 and ROOT_LIMB1, or neither");
    }
    return [limb0, limb1];
  }
  const rootFile =
    process.env.ROOT_FILE || path.join(__dirname, "..", "..", "issuer", "output", "root.json");
  const { root_limb0: fileLimb0, root_limb1: fileLimb1 } = JSON.parse(
    fs.readFileSync(rootFile, "utf8")
  );
  return [fileLimb0, fileLimb1];
}

async function main() {
  const rootHex = encodeRoot(...rootLimbs());
  const registry = await loadRegistry();
  const transaction = await registry.publishRoot(rootHex);
  const receipt = await transaction.wait();

  console.log("published_root:", rootHex);
  console.log("transaction_hash:", receipt.hash);
  console.log("gas_used:", receipt.gasUsed.toString());
}

main().catch((error) => {
  console.error(error.message || error);
  process.exitCode = 1;
});
