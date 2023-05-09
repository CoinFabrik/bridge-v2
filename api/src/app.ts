import { createConnection } from "typeorm";
import * as express from "express";
import * as bodyParser from "body-parser";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Tx } from "./entity/Tx";

createConnection().then(async (connection) => {
  const txRepository = connection.getRepository(Tx);
  const app = express();
  app.use(bodyParser.json());

  const wsProvider = new WsProvider("ws://13.212.197.46:9944");
  const api = await ApiPromise.create({ provider: wsProvider });

  app.get("/api/getNetAmount/:txId", async (request, response) => {
    const signedBlock = await api.rpc.chain.getBlock(
      "0xe6dac7cc27e7870dcd68bd98d9f85faf0ee31bedef3bda03ccdf2f168e2216f4"
    );

    const tx = await txRepository.findOne(request.params.txId);
    if (!tx) {
      response
        .status(400)
        .json({ error: `No transaction found with id ${request.params.txId}` });
    }

    signedBlock.block.extrinsics.forEach((ex, index) => {
      // the extrinsics are decoded by the API, human-like view
      //console.log(index, ex.toHuman());

      const {
        isSigned,
        meta,
        method: { args, method, section },
      } = ex;

      // explicit display of name, args & documentation
      console.log(
        `${section}.${method}(${args.map((a) => a.toString()).join(", ")})`
      );

      // signer/nonce info
      if (isSigned) {
        console.log(
          `signer=${ex.signer.toString()}, nonce=${ex.nonce.toString()}`
        );
      }
    });

    response.json({
      msg: "hola",
    });
  });

  app.get("/api/getExtrinsicHash", async (request, response) => {
    response.json({ msg: "chau" });
  });

  app.listen(3000, () => {
    console.log(
      "Server is running on port 3000. Open http://localhost:3000/users to see results"
    );
  });
});
