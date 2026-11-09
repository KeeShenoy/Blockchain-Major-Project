// Locates the deployed registry recorded by deploy.js, and checks that it still exists. A restarted
// Hardhat node forgets its contracts, but deployment.json survives.

const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

const DEPLOYMENT_FILE = path.join(__dirname, "..", "deployment.json");

async function loadRegistry() {
  if (!fs.existsSync(DEPLOYMENT_FILE)) {
    throw new Error("no deployment.json: run `npm run deploy:local` first");
  }
  const { address } = JSON.parse(fs.readFileSync(DEPLOYMENT_FILE, "utf8"));
  const code = await hre.ethers.provider.getCode(address);
  if (code === "0x") {
    throw new Error(
      `no contract at ${address} on this network; the node was probably restarted, so run ` +
        "`npm run deploy:local` again"
    );
  }
  return hre.ethers.getContractAt("CredentialRegistry", address);
}

module.exports = { DEPLOYMENT_FILE, loadRegistry };
