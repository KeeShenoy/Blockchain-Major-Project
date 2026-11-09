const { expect } = require("chai");
const { ethers } = require("hardhat");
const { FIELD_MODULUS, encodeRoot, decodeRoot } = require("../scripts/rootCodec");

describe("CredentialRegistry", function () {
  async function deploy() {
    const [issuer, other] = await ethers.getSigners();
    const registry = await ethers.deployContract("CredentialRegistry");
    await registry.waitForDeployment();
    return { registry, issuer, other };
  }

  // A root as printed by the issuer.
  const limbs = ["85745324007795138414594717867368673756", "91486109769402765214278414575974973857"];

  it("records the deployer as issuer and starts with no root", async function () {
    const { registry, issuer } = await deploy();
    expect(await registry.issuer()).to.equal(issuer.address);
    expect(await registry.getRoot()).to.equal(ethers.ZeroHash);
  });

  it("lets the issuer publish a root and emits RootPublished", async function () {
    const { registry } = await deploy();
    const root = encodeRoot(...limbs);
    await expect(registry.publishRoot(root)).to.emit(registry, "RootPublished");
    expect(await registry.getRoot()).to.equal(root);
  });

  it("rejects publication by anyone but the issuer", async function () {
    const { registry, other } = await deploy();
    await expect(
      registry.connect(other).publishRoot(encodeRoot(...limbs))
    ).to.be.revertedWithCustomError(registry, "NotIssuer");
  });

  it("replaces the previous root on republication", async function () {
    const { registry } = await deploy();
    await registry.publishRoot(encodeRoot("1", "2"));
    await registry.publishRoot(encodeRoot(...limbs));
    expect(decodeRoot(await registry.getRoot()).map(String)).to.deep.equal(limbs);
  });
});

describe("root encoding", function () {
  it("round-trips limbs through bytes32", function () {
    const root = encodeRoot(...limbs());
    expect(root).to.have.length(66);
    expect(decodeRoot(root).map(String)).to.deep.equal(limbs());
  });

  it("stores each limb little-endian", function () {
    expect(encodeRoot("1", "256")).to.equal("0x01" + "00".repeat(15) + "0001" + "00".repeat(14));
  });

  it("rejects limbs that are not canonical field elements", function () {
    expect(() => encodeRoot((FIELD_MODULUS - 1n).toString(), "0")).not.to.throw();
    expect(() => encodeRoot(FIELD_MODULUS.toString(), "0")).to.throw(/canonical/);
    expect(() => encodeRoot("-1", "0")).to.throw(/canonical/);
  });

  function limbs() {
    return ["85745324007795138414594717867368673756", "91486109769402765214278414575974973857"];
  }
});
