use chrono::{Days, NaiveDateTime, Utc};
use log::{error, info, warn};
use sp_core::{crypto::Pair, sr25519, sr25519::Public};
use std::{str::FromStr, sync::Arc};
use substrate_api_client::{
    rpc::WsRpcClient, AccountId, Api, BaseExtrinsicParams, GenericAddress, MultiAddress, PlainTip,
    PlainTipExtrinsicParams, XtStatus,
};
use tokio::time::Duration;

use crate::database::DatabaseEngine;

async fn calculate_amount_to_transfer_and_business_fee_v2(
    api: &Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<PlainTip>>,
    glitch_gas: bool,
    amount: u128,
    business_fee: f64,
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
    let business_fee_amount = ((amount_to_transfer as f64 * business_fee) / 100.0) as u128;

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
    scanner_name: String,
    tx_ix: u128,
    tx_glitch_address: String,
    node: &str,
    glitch_pk: String,
    public: Public,
    amount_to_transfer: u128,
    amount_business_fee: u128,
    database_engine: Arc<DatabaseEngine>,
    business_fee_percentage: f64,
) {
    let client = WsRpcClient::new(node);
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let api = Api::<_, _, PlainTipExtrinsicParams>::new(client)
        .map(|api| api.set_signer(signer))
        .unwrap();
    let xt_to_send = api.balance_transfer(
        MultiAddress::Id(AccountId::from(public)),
        amount_to_transfer - amount_business_fee,
    );

    let xt_result = match api.send_extrinsic(xt_to_send.hex_encode(), XtStatus::Finalized) {
        Ok(r) => r,
        Err(e) => {
            error!("Transfer error: {:?}", e);
            None
        }
    };

    match xt_result {
        Some(hash) => {
            database_engine
                .update_tx(
                    tx_ix,
                    format!("{:#x}", hash),
                    amount_business_fee,
                    business_fee_percentage.to_string(),
                )
                .await;
            database_engine
                .increment_fee_counter(scanner_name, amount_business_fee)
                .await;
            info!("Trasfer to address {} completed!", tx_glitch_address);
        }
        None => info!(
            "Transfer to address {} not completed. It will be tried again.",
            tx_glitch_address
        ),
    };
}

pub async fn run_network_listener(
    name: String,
    glitch_pk: String,
    glitch_node: String,
    business_fee: f64,
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

    let interval = tokio::time::interval(Duration::from_millis(5000));

    loop {
        tokio::select! {
            _ = interval.tick() => {

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
                    let (amount_to_transfer, business_fee_amount) = calculate_amount_to_transfer_and_business_fee_v2(&api, glitch_gas, amount, business_fee, public).await;

                    make_transfer(name.clone(),tx.id, tx.glitch_address, glitch_node.as_str(), glitch_pk.clone(), public, amount_to_transfer, business_fee_amount, database_engine.clone(), business_fee).await;

                }
            }
        }
    }
}

pub async fn fee_payer_v2(
    database_engine: Arc<DatabaseEngine>,
    interval_in_days: u32,
    glitch_node: String,
    scanner_name: String,
    glitch_pk: String,
    fee_address: String,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let signer_account_id = AccountId::from(signer.public());
    let client = WsRpcClient::new(&glitch_node); // Before "ws://13.212.108.116:9944"
    let api = Api::<_, _, PlainTipExtrinsicParams>::new(client)
        .map(|api| api.set_signer(signer))
        .unwrap();

    loop {
        interval.tick().await;
        make_fee_transfer(
            database_engine.clone(),
            interval_in_days,
            &scanner_name,
            &api,
            &signer_account_id,
            &fee_address,
        )
        .await;
    }
}

async fn is_time_to_pay_fee_v2(last_time_fee: Option<String>, interval_in_secs: i64) -> bool {
    let last_day_payment = match last_time_fee {
        Some(time) => NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S").unwrap(),
        None => NaiveDateTime::from_timestamp_millis(
            Utc::now()
                .checked_sub_days(Days::new(2))
                .unwrap()
                .timestamp(),
        )
        .unwrap(),
    };

    Utc::now().timestamp() - last_day_payment.timestamp() >= interval_in_secs
}

async fn make_fee_transfer(
    database_engine: Arc<DatabaseEngine>,
    interval_in_secs: u32,
    scanner_name: &str,
    api: &Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<PlainTip>>,
    signer_account_id: &AccountId,
    fee_address: &str,
) {
    let fee_last_time = database_engine.get_fee_last_time().await;
    info!("Fee last time: {:?}", fee_last_time);
    if !is_time_to_pay_fee_v2(fee_last_time, interval_in_secs as i64).await {
        return;
    }
    let fee_to_send = database_engine.get_fee_counter(scanner_name).await;
    if fee_to_send == 0 {
        return;
    }

    info!("It's time to pay business fee!");
    info!("Executing transfer of {} as business fee.", fee_to_send);

    let signer_free_balance = match api.get_account_data(&signer_account_id).unwrap() {
        Some(data) => {
            warn!("Signer balance is: {:?}", data);
            data.free
        }
        None => 0_u128,
    };

    if fee_to_send > signer_free_balance {
        warn!("There are not enough funds to send the business fee.");
        return;
    }

    let account_id = AccountId::from(Public::from_str(fee_address).unwrap());

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
            database_engine.modify_fee_counter(0, scanner_name).await;
            database_engine
                .insert_tx_fee(format!("{:#x}", hash), fee_to_send.to_string())
                .await;
            info!(
                "The transfer of the business fee ({}) has been completed",
                fee_to_send
            );
        }
        None => {
            info!("Transfer of the business fee not completed. It will be tried again.");
        }
    }
}
