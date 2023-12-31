import { createConnection } from "typeorm";
import * as express from "express";
import * as bodyParser from "body-parser";
import * as cors from "cors";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Tx } from "./entity/Tx";
import { SignedBlock } from "@polkadot/types/interfaces";
import * as dotenv from "dotenv";
import { ethers, logger } from "ethers";
import { getGlitchInfo } from "./glitch";
dotenv.config();

createConnection().then(async (connection) => {
  const txRepository = connection.getRepository(Tx);
  const app = express();
  app.use(bodyParser.json());
  app.use(cors());

  const wsProvider = new WsProvider(process.env.WS_NODE);
  const api = await ApiPromise.create({ provider: wsProvider });

  app.get("/api/validators", async (request, response) => {
    const currentEra = (await api.query.staking.currentEra()).toString();
    const totalStake = (
      await api.query.staking.erasTotalStake(currentEra)
    ).toString();

    const stakers = (await api.query.session.validators()).toHuman();
    let stakersCount: number;
    if (Array.isArray(stakers)) {
      stakersCount = stakers.length;
    }
    return response.json({
      currentEra,
      stakersCount,
      totalStake,
    });
  });

  app.get("/api/transactionHistory/:wallet", async (request, response) => {
    console.info(
      `[${new Date().toLocaleString()}] - Obtaining the transaction history of address ${
        request.params.wallet
      }`
    );

    const page = (request.query.page || 0) as number;
    const limit = (request.query.limit || 10) as number;

    console.info(
      `[${new Date().toLocaleString()}] - Searching ${limit} transaction on page ${page}.`
    );

    const txs = await txRepository.find({
      take: limit,
      skip: limit * page,
      where: [
        { from_eth_address: request.params.wallet },
        { to_glitch_address: request.params.wallet },
      ],
    });

    console.info(
      `[${new Date().toLocaleString()}] - ${txs.length} transactions was found!`
    );

    const txsWithInfo = txs.map(async (tx) => {
      let response: any = { ...tx };
      try {
        const glitchInfo = await getGlitchInfo(tx, api);

        if (!tx.extrinsic_hash && !tx.net_amount) {
          tx.extrinsic_hash = glitchInfo.extrinsicHash;
          tx.net_amount = glitchInfo.netAmount;
          txRepository.save(tx).then((result) => {
            console.info(
              "Transaction updated with extrinsic_hash and net_amount!1"
            );
          });
        }

        response = {
          ...response,
          glitch_fee: glitchInfo.glitchFee,
          glitch_timestamp: glitchInfo.timestamp,
        };
      } catch (error) {
        console.error(
          `[${new Date().toLocaleString()}] - No information could be obtained from the node for this transaction.: ${
            tx.id
          }`
        );
        console.error(`[${new Date().toLocaleString()}] - Error: ${error}`);
        console.error(error);
      }

      try {
        const provider = new ethers.providers.JsonRpcProvider(
          process.env.ETH_NODE
        );
        const eth_tx = await provider.getTransaction(tx.tx_eth_hash);
        const block = await provider.getBlock(eth_tx.blockNumber);

        response = { ...response, eth_timestamp: block.timestamp };
      } catch (error) {
        console.error(
          `[${new Date().toLocaleString()}] - No information could be obtained from the eth hash: ${
            tx.tx_eth_hash
          }`
        );
        console.error(`[${new Date().toLocaleString()}] - Error: ${error}`);
      }

      return response;
    });

    return response.json(await Promise.all(txsWithInfo));
  });

  app.get("/api/transactionInfo/:eth_tx", async (request, response) => {
    console.info(
      `[${new Date().toLocaleString()}] - Getting information from eth transaction with id ${
        request.params.eth_tx
      }`
    );
    const tx = await txRepository.findOne({
      tx_eth_hash: request.params.eth_tx,
    });

    if (tx.extrinsic_hash && tx.net_amount) {
      console.info(
        `[${new Date().toLocaleString()}] - The information is already in the database.`
      );
      return response.json({
        netAmount: tx.net_amount,
        extrinsicHash: tx.extrinsic_hash,
      });
    }

    if (!tx) {
      return response.status(400).json({
        error: `No transaction found with id ${request.params.eth_tx}`,
      });
    }

    let signedBlock: SignedBlock;
    try {
      console.log(
        `[${new Date().toLocaleString()}] - Asking the node for block information: ${
          tx.tx_glitch_hash
        }`
      );
      signedBlock = await api.rpc.chain.getBlock(tx.tx_glitch_hash);
    } catch (error) {
      console.error(`[${new Date().toLocaleString()}] - ${error}`);
      return response
        .status(400)
        .json({ error: `Error getting information from the block: ${error}` });
    }

    let netAmount: string;
    let extrinsicHash: string;

    signedBlock.block.extrinsics.forEach((ex, index) => {
      // the extrinsics are decoded by the API, human-like view
      //console.log(index, ex.toHuman());

      const {
        isSigned,
        meta,
        method: { args, method, section },
      } = ex;

      // explicit display of name, args & documentation
      console.info(
        `[${new Date().toLocaleString()}] - ${section}.${method}(${args
          .map((a) => a.toString())
          .join(", ")})`
      );

      const x = args.map((a) => a.toString());
      netAmount = x.at(1);
      extrinsicHash = ex.hash.toHex();

      // signer/nonce info
      if (isSigned) {
        console.info(meta);
        console.info(
          `[${new Date().toLocaleString()}] - signer=${ex.signer.toString()}, nonce=${ex.nonce.toString()}`
        );
      }
    });

    tx.net_amount = netAmount;
    tx.extrinsic_hash = extrinsicHash;
    await txRepository.save(tx);

    return response.json({
      netAmount,
      extrinsicHash,
    });
  });

  app.listen(3000, () => {
    console.info(
      `[${new Date().toLocaleString()}] - Server is running on port 3000.`
    );
  });
});
