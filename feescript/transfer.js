const { ApiPromise, WsProvider } = require('@polkadot/api');
const { 
    mnemonicGenerate, 
    mnemonicValidate 
} = require('@polkadot/util-crypto');
const { Keyring } = require('@polkadot/keyring');
const BN = require('bn.js');

const connect = async () => {
    const wsProvider = new WsProvider('ws://13.212.197.46:9944'); 
    //const wsProvider = new WsProvider('ws://127.0.0.1:9890'); 
    const api = new ApiPromise({ provider: wsProvider });
    return api.isReady;
};

const keyring = new Keyring({type: 'sr25519'});

const ROOT_SEED = '0x5c1f0018ddb973564910ce1af1daab89cfdc3f49e01570704041e877cffae30a';

const retrieveAccount = (rawSeed) => {
    const account = keyring.addFromSeed(rawSeed);
    return { account };
}

const main = async (api) => {
    const amountArg = process.argv[2];
    const addressArg = process.argv[3];

    const { account: root_account } = retrieveAccount(ROOT_SEED);
    const transfer = api.tx.balances.transfer(addressArg, amountArg);

    const hash = await transfer.signAndSend(root_account);
    console.log(`${hash}`);

    var start = new Date().getTime();
    while (new Date().getTime() < start + 10000);
    return api;
};

connect()
        .then((api) => main(api))
         .catch((err) => {
    console.error(err)
}).finally(() => process.exit());
