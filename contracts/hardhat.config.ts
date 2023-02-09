import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

const GOERLI_PRIVATE_KEY = "698b7c0f189872b77abf81107003ed2ec1ff7060090dac0bbc84a1b012ececef";
const ALCHEMY_API_KEY = "XsoDxjFTZiqbFPK_1-nn7meNNzGq-rR5";

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.17",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200
      }
    }
  },
  networks: {
    goerli: {
      url: `https://eth-goerli.g.alchemy.com/v2/${ALCHEMY_API_KEY}`,
      accounts: [GOERLI_PRIVATE_KEY],
    },
    bsctestnet: {
      url: "https://data-seed-prebsc-1-s1.binance.org:8545/",
      chainId: 97,
      gasPrice: 20000000000,
      accounts: ["7269264f5b19119d3474a9e81ae0ff67f3ae8b396193e3e19be4a78873f0e971"]
    }
  },
  etherscan: {
    apiKey: "CIANIPEHWQPCMY19IWUMQAXAC84Y3F9MU7",
  },
};

export default config;
