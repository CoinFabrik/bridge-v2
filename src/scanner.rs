use crate::block_listener::{listen_blocks, listen_blocks_v2};
use crate::config::{Database, Network};
use crate::database::DatabaseEngine;
use crate::glitch::{self, fee_payer, run_network_listener};
use crate::{Config, ScannerState};
use futures::executor::block_on;
use log::info;
use std::sync::Arc;
use tokio::spawn;
use tokio::task::spawn_blocking;

pub struct Scanner {
    pub config: Arc<Config>,
}

pub struct ScannerV2 {
    database_engine: Arc<DatabaseEngine>,
    networks: Vec<Network>,
    glitch_private_key: String,
    glitch_fee_address: String,
    interval_days_for_transfer: u8,
    business_fee: u128,
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

    pub fn run(&self) {
        info!("Scanner running...");

        info!(
            "Found {} network{}to listen!",
            self.networks.len(),
            if self.networks.len() > 1 { "s " } else { " " }
        );

        self.networks.iter().for_each(|network_config| {
            tokio::task::spawn(run_network_listener(
                network_config.name.clone(),
                network_config.network.clone(),
                network_config.monitor_address.clone(),
                self.glitch_private_key.clone(),
                network_config.ws_glitch_node.clone(),
                self.business_fee,
                self.glitch_gas,
                self.database_engine.clone(),
            ));

            tokio::task::spawn(listen_blocks_v2(
                network_config.clone(),
                self.database_engine.clone(),
            ));
        });

        loop {}
    }
}

impl Scanner {
    pub fn new(config: Arc<Config>) -> Self {
        Scanner { config }
    }

    pub fn run(&self) {
        info!("Scanner running...");

        info!(
            "Found {} network{}to listen!",
            self.config.networks.len(),
            if self.config.networks.len() > 1 {
                "s "
            } else {
                " "
            }
        );

        self.config
            .networks
            .clone()
            .into_iter()
            .for_each(|network_config| {
                let scanner_state = ScannerState::new(
                    network_config.name.clone(),
                    network_config.network.clone(),
                    network_config.monitor_address.clone(),
                    self.config.db.clone(),
                );

                let pk_clone = self.config.glitch_private_key.clone();
                let pk_clone2 = self.config.glitch_private_key.clone();
                let scanner_clone = scanner_state.clone();
                let scanner_clone2 = scanner_state.clone();
                let glitch_node_clone = network_config.ws_glitch_node.clone();
                let business_fee = self.config.business_fee;
                let interval_in_days = self.config.interval_days_for_transfer;
                let fee_address = self.config.glitch_fee_address.clone();
                let glitch_gas = self.config.glitch_gas;

                spawn_blocking(move || {
                    block_on(glitch::transfer(
                        scanner_clone,
                        pk_clone,
                        glitch_node_clone,
                        business_fee,
                        glitch_gas,
                    ));
                });
                spawn_blocking(move || block_on(listen_blocks(network_config, scanner_state)));
                spawn_blocking(move || {
                    block_on(fee_payer(
                        scanner_clone2,
                        interval_in_days,
                        pk_clone2,
                        fee_address,
                    ));
                });
            });

        loop {}
    }
}
