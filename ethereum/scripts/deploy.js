const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
  const Registry = await hre.ethers.getContractFactory("CredentialRegistry");
  const registry = await Registry.deploy();
  await registry.waitForDeployment();

  const deployment = {
    address: await registry.getAddress(),
    network: hre.network.name,
  };
  fs.writeFileSync(
    path.join(__dirname, "..", "deployment.json"),
    JSON.stringify(deployment, null, 2)
  );
  console.log("CredentialRegistry deployed to:", deployment.address);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
