use crate::balance_monitor::monitor_balance;
use crate::block_listener::listen_blocks_v2;
use crate::config::Network;
use crate::database::DatabaseEngine;
use crate::glitch::{ fee_payer_v2, run_network_listener };
use crate::Config;
use log::info;
use std::sync::Arc;
use tokio::time::{ sleep, Duration };

pub struct ScannerV2 {
    database_engine: Arc<DatabaseEngine>,
    networks: Vec<Network>,
    glitch_private_key: String,
    glitch_fee_address: String,
    interval_days_for_transfer: u32,
    business_fee: f64,
    glitch_gas: bool,
}

impl ScannerV2 {
    pub fn new(config: Config) -> Self {
        Self {
            database_engine: Arc::new(DatabaseEngine::new(config.db)),
            networks: config.networks,
            glitch_private_key: config.glitch_private_key.unwrap(),
            glitch_fee_address: config.glitch_fee_address,
            interval_days_for_transfer: config.interval_days_for_transfer,
            business_fee: config.business_fee,
            glitch_gas: config.glitch_gas,
        }
    }

    pub async fn run(config: Config) {
        info!("Scanner running...");

        info!("Found {} network{}to listen!", config.networks.len(), if config.networks.len() > 1 {
            "s "
        } else {
            " "
        });

        let database_engine = Arc::new(DatabaseEngine::new(config.db));

        config.networks.iter().for_each(|network_config| {
            tokio::task::spawn(listen_blocks_v2(network_config.clone(), database_engine.clone()));

            tokio::task::spawn(
                run_network_listener(
                    network_config.name.clone(),
                    config.glitch_private_key.clone().unwrap(),
                    network_config.ws_glitch_node.clone(),
                    config.business_fee,
                    config.glitch_gas,
                    database_engine.clone()
                )
            );

            tokio::task::spawn(
                fee_payer_v2(
                    database_engine.clone(),
                    config.interval_days_for_transfer,
                    network_config.ws_glitch_node.clone(),
                    network_config.name.clone(),
                    config.glitch_private_key.clone().unwrap(),
                    config.glitch_fee_address.clone()
                )
            );

            tokio::task::spawn(
                monitor_balance(
                    network_config.ws_glitch_node.clone(),
                    config.glitch_private_key.clone().unwrap(),
                    config.alert.clone()
                )
            );
        });

        loop {
            sleep(Duration::from_millis(1000)).await;
        }
    }
}
