import { createConnection } from "typeorm";
import * as express from "express";
import * as bodyParser from "body-parser";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Tx } from "./entity/Tx";
import { SignedBlock } from "@polkadot/types/interfaces";
import * as dotenv from "dotenv";
dotenv.config();

createConnection().then(async (connection) => {
  const txRepository = connection.getRepository(Tx);
  const app = express();
  app.use(bodyParser.json());

  const wsProvider = new WsProvider(process.env.WS_NODE);
  const api = await ApiPromise.create({ provider: wsProvider });

  app.get("/api/transactionInfo/:txId", async (request, response) => {
    console.info(
      `[${new Date().toLocaleString()}] - Getting information from transaction with id ${
        request.params.txId
      }`
    );
    const tx = await txRepository.findOne(request.params.txId);

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
      return response
        .status(400)
        .json({ error: `No transaction found with id ${request.params.txId}` });
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
