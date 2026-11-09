// Reads the trusted Merkle root from the registry and prints it as the two decimal limbs the
// verifier takes (--root-limb0, --root-limb1).

const { decodeRoot } = require("./rootCodec");
const { loadRegistry } = require("./deployment");

async function main() {
  const registry = await loadRegistry();
  const rootHex = await registry.getRoot();
  if (/^0x0*$/.test(rootHex)) {
    throw new Error("the registry has no published root yet");
  }
  const [limb0, limb1] = decodeRoot(rootHex);

  console.log("root_hex:", rootHex);
  console.log("root_limb0:", limb0.toString());
  console.log("root_limb1:", limb1.toString());
}

main().catch((error) => {
  console.error(error.message || error);
  process.exitCode = 1;
});
