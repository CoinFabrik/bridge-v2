import { ethers } from "hardhat";

async function main() {
  /*const TokenFactory = await ethers.getContractFactory("GlitchToken");
  const token = await TokenFactory.deploy();

  await token.deployed();

  console.log(token.address);*/

  const minAmount = ethers.utils.parseEther("100");
  const maxAmount = ethers.utils.parseEther("40000");

  const GlitchBridgeFactory = await ethers.getContractFactory("GlitchBridge");
  const bridge = await GlitchBridgeFactory.deploy(
    "0x7428417089727238b4B3BA5933c77357Af9B56f5",
    minAmount,
    maxAmount
  );

  await bridge.deployed();

  console.log(bridge.address);
  console.log(minAmount.toString());
  console.log(maxAmount.toString());
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
