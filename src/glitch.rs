use std::str::FromStr;
use std::sync::Arc;

use hex::decode;
use kitchensink_runtime::Runtime;
use log::info;
use serde::Deserialize;
use serde::Serialize;
use substrate_api_client::rpc::Request;
use substrate_api_client::sp_core::Encode;
use substrate_api_client::sp_runtime::app_crypto::sr25519::Pair;
use substrate_api_client::sp_runtime::app_crypto::sr25519::Public;
use substrate_api_client::sp_runtime::app_crypto::Pair as CryptoPair;
use substrate_api_client::RpcParams;
use substrate_api_client::{
    rpc::WsRpcClient, AccountId, AssetTipExtrinsicParams, GenericAddress, SubmitAndWatch,
};
use substrate_api_client::{Api, XtStatus};
use tokio::task::spawn_blocking;
use tokio::time::{sleep, Duration};
use web3::block_on;

use crate::database::ScannerState;
use crate::js_call::{self, get_fee};

// MEJORAS GLITCH:

// - ASINCRONISMO EN TRANSFERENCIAS EN GLITCH
// - ESTADO INTERMEDIO
// - FLOTANTE PARA EL FEE

pub async fn fee_payer(
    scanner_state: ScannerState,
    interval_in_days: u8,
    glitch_pk: Option<String>,
    fee_address: String,
) {
    loop {
        sleep(Duration::from_secs(u64::from(interval_in_days) * 86400)).await;

        let fee_to_send = scanner_state.get_fee_counter().await;

        if fee_to_send != 0 {
            scanner_state.modify_fee_counter(0).await;

            // Transfer
            let seed: [u8; 32] = decode(glitch_pk.as_ref().unwrap())
                .unwrap()
                .try_into()
                .unwrap();

            let pair = Pair::from_seed(&seed);

            let client = WsRpcClient::new("ws://13.212.108.116:9944").unwrap();
            let mut api =
                Api::<_, _, AssetTipExtrinsicParams<Runtime>, Runtime>::new(client).unwrap();
            api.set_signer(pair);

            let account_id = AccountId::from(Public::from_str(fee_address.as_str()).unwrap());

            let xt = api.balance_transfer(GenericAddress::Id(account_id), fee_to_send);

            let xt_result =
                api.submit_and_watch_extrinsic_until(Encode::encode(&xt), XtStatus::Finalized);

            match xt_result {
                Ok(extrinsic_report) => {
                    scanner_state
                        .insert_tx_fee(
                            format!("{:#x}", extrinsic_report.extrinsic_hash),
                            fee_to_send.to_string(),
                        )
                        .await;
                    info!(
                        "The transfer of the business fee ({}) has been completed",
                        fee_to_send
                    );
                }
                Err(e) => {
                    info!(
                        "Transfer of the business fee not completed. It will be tried again.: {}",
                        e
                    );
                }
            }
        }
    }
}

pub async fn transfer(
    scanner_state: ScannerState,
    glitch_pk: Option<String>,
    node_glitch: String,
    business_fee: u128,
    glitch_gas: bool,
) {
    // This is now done from Javascript.
    /*
    let seed: [u8; 32] = decode(glitch_pk.unwrap()).unwrap().try_into().unwrap();
    let pair = Pair::from_seed(&seed);
    let client = WsRpcClient::new(node_glitch.as_str()).unwrap();
    let mut api =
        Api::<_, _, AssetTipExtrinsicParams<Runtime>, Runtime>::new(client.clone()).unwrap();
    api.set_signer(pair);
    */

    loop {
        let txs = scanner_state.txs_to_process().await;

        for tx in txs {
            match Public::from_str(&tx.glitch_address) {
                Ok(_) => (),
                Err(error) => {
                    scanner_state
                        .save_with_error(tx.id, format!("Error with address: {error:?}"))
                        .await;
                    continue;
                }
            };

            let amount = match tx.amount.clone().parse::<u128>() {
                Ok(a) => a,
                Err(error) => {
                    scanner_state
                        .save_with_error(tx.id, format!("Error with amount: {error:?}"))
                        .await;
                    continue;
                }
            };

            let fee = if glitch_gas {
                get_fee(amount, &tx.glitch_address)
            } else {
                0_u128
            };

            let amount_to_transfer = amount - fee;
            let business_fee_amount: u256 = amount_to_transfer;
            let business_fee_amount = business_fee_amount * business_fee.into() / 100;
            let business_fee_amount: u128 = business_fee_amount.into();

            let fee_counter = scanner_state.get_fee_counter().await;

            scanner_state
                .modify_fee_counter(fee_counter + business_fee_amount)
                .await;

            info!("Business fee amount is: {}", business_fee_amount);

            info!("Amount received in the ETH transaction: {}", amount);
            info!(
                "Estimated fee for the transaction on the Glitch network {}",
                fee
            );
            info!("Amount to be transferred {}", amount_to_transfer);

            let scanner_state_clone = scanner_state.clone();

            scanner_state.update_tx_to_processing(tx.id).await;

            spawn_blocking(move || {
                block_on(async {
                    let tx_hash = js_call::transfer(
                        amount_to_transfer - business_fee_amount,
                        &tx.glitch_address,
                    );

                    scanner_state_clone
                        .update_tx(tx.id, tx_hash, business_fee_amount, business_fee)
                        .await;

                    info!("Trasfer to address {} completed!", tx.glitch_address);
                });
            });
        }

        sleep(Duration::from_millis(5000)).await;
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeResult {
    pub class: String,
    pub partial_fee: String,
    pub weight: u128,
}
