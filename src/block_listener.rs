use std::sync::Arc;

use crate::config;
use crate::database::{DatabaseEngine, ScannerState};
use futures::executor::block_on;
use futures::StreamExt;
use log::{error, info, warn};
use regex::Regex;
use tokio::task::spawn_blocking;
use web3::api::{Eth, EthSubscribe, Namespace};
use web3::signing::keccak256;
use web3::transports::WebSocket;
use web3::types::{BlockNumber, FilterBuilder, Log, H160, H256, U64};

pub async fn listen_blocks_v2(
    network_config: config::Network,
    database_engine: Arc<DatabaseEngine>,
) {
    info!(
        "Running block listener to network {}",
        network_config.network
    );

    loop {
        match WebSocket::new(&network_config.ws_node).await {
            Ok(transport) => {
                info!(
                    "WebSocket connection for {} is now open!",
                    &network_config.network
                );

                let subscribe = EthSubscribe::new(transport);

                let mut subscription = subscribe.subscribe_new_heads().await.unwrap();

                while let Some(b) = subscription.next().await {
                    let block: U64 =
                        b.as_ref().unwrap().number.unwrap() - network_config.confirmations;
                    let last_block: U64 = U64::from(
                        database_engine
                            .get_last_block(network_config.name.clone())
                            .await,
                    );
                    info!(
                        "New block in {}: {:?}",
                        &network_config.network,
                        b.unwrap().number.unwrap()
                    );

                    let eth = Eth::new(subscribe.transport());

                    let address: H160 = network_config.monitor_address.parse().unwrap();
                    let topic_bytes =
                        keccak256("TransferToGlitch(address,string,uint256)".as_bytes());

                    let filter = FilterBuilder::default()
                        .address(vec![address])
                        .from_block(BlockNumber::Number(last_block))
                        .to_block(BlockNumber::Number(block))
                        .topics(Some(vec![H256::from(topic_bytes)]), None, None, None)
                        .build();

                    match eth.logs(filter).await {
                        Ok(logs) => {
                            info!("{} transactions found in block {}", logs.len(), block);

                            database_engine
                                .update_block_and_insert_txs(
                                    network_config.name.clone(),
                                    block.as_u32(),
                                    logs,
                                )
                                .await;
                        }
                        Err(e) => {
                            error!("Error obtaining contract logs on the Ethereum network: {e}")
                        }
                    };
                }
            }
            Err(e) => error!(
                "Error connecting with {} network: {:?}",
                network_config.network, e
            ),
        }

        warn!(
            "Restarting the {} network listening.",
            network_config.network
        );
    }
}

pub async fn listen_blocks(network_config: config::Network, scanner_state: ScannerState) {
    info!(
        "Running block listener to network {}",
        network_config.network
    );

    loop {
        match WebSocket::new(&network_config.ws_node).await {
            Ok(transport) => {
                info!(
                    "WebSocket connection for {} is now open!",
                    &network_config.network
                );

                let subscribe = EthSubscribe::new(transport);

                let transport_clone = subscribe.transport().clone();
                let ss = scanner_state.clone();

                spawn_blocking(move || block_on(catch_up(transport_clone, ss)));

                let mut subscription = subscribe.subscribe_new_heads().await.unwrap();

                while let Some(b) = subscription.next().await {
                    let block: U64 =
                        b.as_ref().unwrap().number.unwrap() - network_config.confirmations;

                    scanner_state.update_block(block.as_u32()).await;

                    info!(
                        "New block in {}: {:?}",
                        &network_config.network,
                        b.unwrap().number.unwrap()
                    );

                    let eth = Eth::new(subscribe.transport());

                    let address: H160 = network_config.monitor_address.parse().unwrap();
                    let topic_bytes =
                        keccak256("TransferToGlitch(address,string,uint256)".as_bytes());

                    let filter = FilterBuilder::default()
                        .address(vec![address])
                        .from_block(BlockNumber::Number(block))
                        .to_block(BlockNumber::Number(block))
                        .topics(Some(vec![H256::from(topic_bytes)]), None, None, None)
                        .build();

                    match eth.logs(filter).await {
                        Ok(logs) => {
                            if !logs.is_empty() {
                                info!("{} transactions found in block {}", logs.len(), block);

                                ScannerState::insert_txs(&scanner_state, logs).await;
                            }
                        }
                        Err(e) => {
                            error!("Error obtaining contract logs on the Ethereum network: {e}")
                        }
                    };
                }
            }
            Err(e) => error!(
                "Error connecting with {} network: {:?}",
                network_config.network, e
            ),
        }

        warn!(
            "Restarting the {} network listening.",
            network_config.network
        );
    }
}

pub async fn catch_up(ws: WebSocket, scanner_state: ScannerState) {
    let eth = Eth::new(ws);

    if !scanner_state.exists_network_state().await {
        return;
    }

    let last_scanned_block = scanner_state.get_last_block().await;

    let address: H160 = scanner_state.monitor_address.parse().unwrap();
    let topic_bytes = keccak256("TransferToGlitch(address,string,uint256)".as_bytes());
    let from_block = BlockNumber::Number(U64::from(last_scanned_block + 1));

    info!(
        "Starting catch up from block {} to current block.",
        last_scanned_block + 1
    );

    let filter = FilterBuilder::default()
        .address(vec![address])
        .from_block(from_block)
        .topics(Some(vec![H256::from(topic_bytes)]), None, None, None)
        .to_block(BlockNumber::Latest)
        .build();

    let result_logs: Result<Vec<Log>, web3::Error> = eth.logs(filter).await;
    let mut logs_to_persist: Vec<Log> = Vec::new();

    match result_logs {
        Ok(mut result) => {
            if result.is_empty() {
                info!("No past transactions were found for processing.");
            } else {
                info!("{} transactions were found.", result.len());

                logs_to_persist.append(&mut result);
            }
        }
        Err(e) => match e {
            web3::Error::Rpc(error) => {
                println!("{:?}", error.code);

                let regex = Regex::new("0[xX][0-9a-fA-F]+").unwrap();

                let result: Vec<String> = regex
                    .find_iter(error.message.as_str())
                    .map(|mat| mat.as_str().to_string())
                    .collect();

                let without_prefix = result[0].trim_start_matches("0x");
                println!("{:?}", u64::from_str_radix(without_prefix, 16));
            }
            _ => panic!("{e:?}"),
        },
    }

    scanner_state.insert_txs(logs_to_persist).await;

    info!("Finish catch up.");
}
