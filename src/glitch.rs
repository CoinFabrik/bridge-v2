use chrono::{NaiveDateTime, Utc};
use log::{error, info, warn};
use sp_core::{crypto::Pair, sr25519, sr25519::Public, H256, U256};
use std::{str::FromStr, sync::Arc};
use substrate_api_client::{
    rpc::WsRpcClient, AccountId, Api, BaseExtrinsicParams, GenericAddress, MultiAddress, PlainTip,
    PlainTipExtrinsicParams, XtStatus,
};
use tokio::time::{sleep, Duration};

use crate::database::DatabaseEngine;

async fn calculate_amount_to_transfer_and_business_fee_v2(
    scanner_name: String,
    api: &Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<PlainTip>>,
    glitch_gas: bool,
    amount: u128,
    business_fee: u128,
    public: Public,
    database_engine: Arc<DatabaseEngine>,
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

    database_engine
        .increment_fee_counter(scanner_name, business_fee_amount)
        .await;

    info!("Business fee amount is: {}", business_fee_amount);

    info!("Amount received in the ETH transaction: {}", amount);
    info!(
        "Estimated fee for the transaction on the Glitch network {}",
        fee
    );
    info!("Amount to be transferred {}", amount_to_transfer);

    return (amount_to_transfer, business_fee_amount);
}

pub async fn make_transfer(
    node: &str,
    glitch_pk: String,
    public: Public,
    amount: u128,
) -> Option<H256> {
    let client = WsRpcClient::new(node);
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let api = Api::<_, _, PlainTipExtrinsicParams>::new(client)
        .map(|api| api.set_signer(signer))
        .unwrap();
    let xt_to_send = api.balance_transfer(MultiAddress::Id(AccountId::from(public)), amount);
    match api.send_extrinsic(xt_to_send.hex_encode(), XtStatus::Finalized) {
        Ok(r) => r,
        Err(e) => {
            error!("Transfer error: {:?}", e);
            None
        }
    }
}

pub async fn run_network_listener(
    name: String,
    glitch_pk: String,
    glitch_node: String,
    business_fee: u128,
    glitch_gas: bool,
    database_engine: Arc<DatabaseEngine>,
) {
    let client = WsRpcClient::new(&glitch_node);
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let signer_account_id = AccountId::from(signer.public());
    let api: Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<_>> =
        Api::<_, _, PlainTipExtrinsicParams>::new(client)
            .map(|api| api.set_signer(signer))
            .unwrap();

    let mut interval = tokio::time::interval(Duration::from_millis(5000));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                info!("One iteration in runNetworkListener");

                let mut txs = database_engine.txs_to_process().await;

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
                            database_engine.update_tx_with_error(tx.id, format!("Error with address: {error:?}"))
                                .await;
                            continue;
                        }
                    };

                    let amount = match tx.amount.clone().parse::<u128>() {
                        Ok(a) => a,
                        Err(error) => {
                            database_engine
                                .update_tx_with_error(tx.id, format!("Error with amount: {error:?}"))
                                .await;
                            continue;
                        }
                    };
                    let (amount_to_transfer, business_fee_amount) = calculate_amount_to_transfer_and_business_fee_v2(name.clone(), &api, glitch_gas, amount, business_fee, public, database_engine.clone()).await;

                    let xt_result = make_transfer(glitch_node.as_str(), glitch_pk.clone(), public, amount_to_transfer - business_fee).await;

                    match xt_result {
                        Some(hash) => {
                            database_engine
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

                }
            }
        }
    }
}

// MEJORAS GLITCH:

// - ASINCRONISMO EN TRANSFERENCIAS EN GLITCH
// - ESTADO INTERMEDIO
// - FLOTANTE PARA EL FEE

async fn is_time_to_pay_fee_v2(last_time_fee: &str, interval_in_days: i64) -> bool {
    let last_day_payment =
        NaiveDateTime::parse_from_str(last_time_fee, "%Y-%m-%d %H:%M:%S").unwrap();
    Utc::now().timestamp() - last_day_payment.timestamp() >= (interval_in_days * 86000)
}

pub async fn fee_payer_v2(
    database_engine: Arc<DatabaseEngine>,
    interval_in_days: u8,
    scanner_name: String,
    glitch_pk: String,
    fee_address: String,
) {
    loop {
        sleep(Duration::from_secs(60)).await;
        let fee_last_time = database_engine.get_fee_last_time().await;
        if is_time_to_pay_fee_v2(fee_last_time.as_str(), interval_in_days as i64).await {
            let fee_to_send = database_engine.get_fee_counter(scanner_name.as_str()).await;

            if fee_to_send != 0 {
                let signer: sr25519::Pair = Pair::from_string(glitch_pk.as_str(), None).unwrap();
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
                        database_engine
                            .modify_fee_counter(0, scanner_name.as_str())
                            .await;
                        database_engine
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
