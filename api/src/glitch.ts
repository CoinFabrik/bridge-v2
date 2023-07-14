import { ApiPromise } from "@polkadot/api";
import { SignedBlock } from "@polkadot/types/interfaces";
import { Tx } from "./entity/Tx";

export const getGlitchInfo = async (tx: Tx, api: ApiPromise) => {
  let signedBlock: SignedBlock;
  let allRecords;
  try {
    console.log(
      `[${new Date().toLocaleString()}] - Asking the node for block information: ${
        tx.tx_glitch_hash
      }`
    );
    signedBlock = await api.rpc.chain.getBlock(tx.tx_glitch_hash);

    const apiAt = await api.at(signedBlock.block.header.hash);
    allRecords = await apiAt.query.system.events();
  } catch (error) {
    console.error(`[${new Date().toLocaleString()}] - ${error}`);
  }

  let netAmount: string;
  let extrinsicHash: string;
  let glitchFee: string;
  let timestamp: string;

  signedBlock.block.extrinsics.forEach((ex, index) => {
    // the extrinsics are decoded by the API, human-like view
    //console.log(index, ex.toHuman());

    const {
      isSigned,
      method: { args, method, section },
    } = ex;

    const extrinsicSuccessEvent = JSON.parse(
      allRecords
        .filter(
          ({ phase }) =>
            phase.isApplyExtrinsic && phase.asApplyExtrinsic.eq(index)
        )
        .filter(
          ({ event }) =>
            event.section === "system" && event.method === "ExtrinsicSuccess"
        )
        .map(({ event }) => event.data.toString())[0]
    );

    glitchFee = extrinsicSuccessEvent[0].weight;

    // explicit display of name, args & documentation
    console.info(
      `[${new Date().toLocaleString()}] - ${section}.${method}(${args
        .map((a) => a.toString())
        .join(", ")})`
    );

    if (section === "timestamp" && method === "set") {
      timestamp = args[0].toString();
    }

    if (
      section === "balances" &&
      method === "transfer" &&
      args[0].toString() === tx.to_glitch_address
    ) {
      const x = args.map((a) => a.toString());
      netAmount = x.at(1);
      extrinsicHash = ex.hash.toHex();
    }

    // signer/nonce info
    if (isSigned) {
      console.info(
        `[${new Date().toLocaleString()}] - signer=${ex.signer.toString()}, nonce=${ex.nonce.toString()}`
      );
    }
  });

  return { netAmount, extrinsicHash, glitchFee, timestamp };
};
