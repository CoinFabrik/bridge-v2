use chrono::{NaiveDateTime, Utc};
use log::{error, info, warn};
use sp_core::{crypto::Pair, sr25519, sr25519::Public, U256};
use std::str::FromStr;
use substrate_api_client::{
    rpc::WsRpcClient, AccountId, Api, BaseExtrinsicParams, GenericAddress, MultiAddress, PlainTip,
    PlainTipExtrinsicParams, XtStatus,
};
use tokio::{
    task::spawn_blocking,
    time::{sleep, Duration},
};
use web3::block_on;

use crate::database::ScannerState;
use crate::database::TxToProcess;

// MEJORAS GLITCH:

// - ASINCRONISMO EN TRANSFERENCIAS EN GLITCH
// - ESTADO INTERMEDIO
// - FLOTANTE PARA EL FEE

async fn is_time_to_pay_fee(scanner_state: &ScannerState, interval_in_days: i64) -> bool {
    let last_day_payment = NaiveDateTime::parse_from_str(
        scanner_state.get_fee_last_time().await.as_str(),
        "%Y-%m-%d %H:%M:%S",
    )
    .unwrap();

    Utc::now().timestamp() - last_day_payment.timestamp() >= (interval_in_days * 86000)
}

pub async fn fee_payer(
    scanner_state: ScannerState,
    interval_in_days: u8,
    glitch_pk: Option<String>,
    fee_address: String,
) {
    loop {
        sleep(Duration::from_secs(60)).await;
        if is_time_to_pay_fee(&scanner_state, interval_in_days as i64).await {
            let fee_to_send = scanner_state.get_fee_counter().await;

            if fee_to_send != 0 {
                // Transfer
                let signer: sr25519::Pair =
                    Pair::from_string(glitch_pk.as_ref().unwrap(), None).unwrap();

                let client = WsRpcClient::new("ws://13.212.108.116:9944");
                let api = Api::<_, _, PlainTipExtrinsicParams>::new(client)
                    .map(|api| api.set_signer(signer))
                    .unwrap();

                let account_id = AccountId::from(Public::from_str(fee_address.as_str()).unwrap());

                let xt = api.balance_transfer(GenericAddress::Id(account_id), fee_to_send);

                let xt_result = match api.send_extrinsic(xt.hex_encode(), XtStatus::Finalized) {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Transfer error: {:?}", e);
                        None
                    }
                };

                match xt_result {
                    Some(hash) => {
                        scanner_state.modify_fee_counter(0).await;
                        scanner_state
                            .insert_tx_fee(format!("{:#x}", hash), fee_to_send.to_string())
                            .await;
                        info!(
                            "The transfer of the business fee ({}) has been completed",
                            fee_to_send
                        );
                    }
                    None => {
                        info!(
                            "Transfer of the business fee not completed. It will be tried again."
                        );
                    }
                }
            }
        }
    }
}

async fn calculate_amount_to_transfer_and_business_fee(
    api: &Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<PlainTip>>,
    glitch_gas: bool,
    amount: u128,
    tx: &TxToProcess,
    scanner_state: &ScannerState,
    business_fee: u128,
    public: Public,
) -> (u128, u128) {
    let xt_to_send = api
        .balance_transfer(MultiAddress::Id(AccountId::from(public)), amount)
        .hex_encode();
    let fee = if glitch_gas {
        api.get_fee_details(xt_to_send.as_str(), None)
            .unwrap()
            .unwrap()
            .final_fee()
    } else {
        0_u128
    };

    let amount_to_transfer = amount - fee;
    let business_fee_amount =
        (U256::from(amount_to_transfer) * U256::from(business_fee) / 100).as_u128();

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

    scanner_state.update_tx_to_processing(tx.id).await;

    return (amount_to_transfer, business_fee_amount);
}

pub async fn transfer(
    scanner_state: ScannerState,
    glitch_pk: Option<String>,
    node_glitch: String,
    business_fee: u128,
    glitch_gas: bool,
) {
    let client = WsRpcClient::new(&node_glitch);
    let signer: sr25519::Pair = Pair::from_string(glitch_pk.as_ref().unwrap(), None).unwrap();
    let signer_account_id = AccountId::from(signer.public());
    let api: Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<_>> =
        Api::<_, _, PlainTipExtrinsicParams>::new(client)
            .map(|api| api.set_signer(signer))
            .unwrap();

    loop {
        sleep(Duration::from_millis(5000)).await;

        let mut txs = scanner_state.txs_to_process().await;

        txs.sort_by(|a, b| {
            a.amount
                .parse::<u128>()
                .unwrap()
                .cmp(&b.amount.parse::<u128>().unwrap())
        });

        for tx in txs {
            let signer_free_balance = match api.get_account_data(&signer_account_id).unwrap() {
                Some(data) => data.free,
                None => 0_u128,
            };

            if tx.amount.as_str().parse::<u128>().unwrap() > signer_free_balance {
                warn!("There is not enough balance to continue processing transactions. To continue reload the account used as a signer.");
                break;
            }

            let public = match Public::from_str(&tx.glitch_address) {
                Ok(p) => p,
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

            let (amount_to_transfer, business_fee_amount) =
                calculate_amount_to_transfer_and_business_fee(
                    &api,
                    glitch_gas,
                    amount,
                    &tx,
                    &scanner_state,
                    business_fee,
                    public,
                )
                .await;
            let scanner_state_clone = scanner_state.clone();

            let signer_per_tx: sr25519::Pair =
                Pair::from_string(glitch_pk.as_ref().unwrap(), None).unwrap();
            let node_per_tx = node_glitch.clone();

            spawn_blocking(move || {
                block_on(async {
                    let client = WsRpcClient::new(node_per_tx.as_str());
                    let api = Api::<_, _, PlainTipExtrinsicParams>::new(client)
                        .map(|api| api.set_signer(signer_per_tx))
                        .unwrap();
                    let xt_to_send = api.balance_transfer(
                        MultiAddress::Id(AccountId::from(public)),
                        amount_to_transfer - business_fee_amount,
                    );
                    let xt_result =
                        match api.send_extrinsic(xt_to_send.hex_encode(), XtStatus::Finalized) {
                            Ok(r) => r,
                            Err(e) => {
                                error!("Transfer error: {:?}", e);
                                None
                            }
                        };

                    match xt_result {
                        Some(hash) => {
                            scanner_state_clone
                                .update_tx(
                                    tx.id,
                                    format!("{:#x}", hash),
                                    business_fee_amount,
                                    business_fee,
                                )
                                .await;
                            info!("Trasfer to address {} completed!", tx.glitch_address);
                        }
                        None => info!(
                            "Transfer to address {} not completed. It will be tried again.",
                            tx.glitch_address
                        ),
                    };
                });
            });
        }
    }
}
