import { time, loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { expect } from "chai";
import { ethers } from "hardhat";
import { GlitchBridge, GlitchToken } from "../typechain-types";

describe("Bridge", function () {

  async function deploy() {
    const minAmount = ethers.utils.parseEther("100");
    const maxAmount = ethers.utils.parseEther("40000");

    // Contracts are deployed using the first signer/account by default
    const [owner, otherAccount] = await ethers.getSigners();

    const GlitchToken = await ethers.getContractFactory("GlitchToken");
    const glitchToken: GlitchToken = await GlitchToken.deploy();

    const GlitchBridge = await ethers.getContractFactory("GlitchBridge");
    const glitchBridge = await GlitchBridge.deploy(glitchToken.address, minAmount, maxAmount);

    return [glitchToken, glitchBridge];
  }

  describe("Deployment", function () {
    it("Test contract", async function () {
        const minAmount = ethers.utils.parseEther("100");
        const maxAmount = ethers.utils.parseEther("40000");
    
        // Contracts are deployed using the first signer/account by default
        const [owner, otherAccount] = await ethers.getSigners();
    
        const GlitchToken = await ethers.getContractFactory("GlitchToken");
        const glitchToken: GlitchToken = await GlitchToken.deploy();
    
        const GlitchBridge = await ethers.getContractFactory("GlitchBridge");
        const glitchBridge: GlitchBridge = await GlitchBridge.deploy(glitchToken.address, minAmount, maxAmount);

        await glitchToken.transfer(otherAccount.address, ethers.utils.parseEther("1000"));

        console.log( await glitchToken.balanceOf(owner.address) );
        console.log( await glitchToken.balanceOf(otherAccount.address) );

        await glitchToken.connect(otherAccount).approve(glitchBridge.address, ethers.utils.parseEther("150"));

        await glitchBridge.connect(otherAccount).transferToGlitch("brian", ethers.utils.parseEther("101"));
    });
  });

    
    
});
