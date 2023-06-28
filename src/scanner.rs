use crate::block_listener::listen_blocks_v2;
use crate::config::Network;
use crate::database::DatabaseEngine;
use crate::glitch::{fee_payer_v2, run_network_listener};
use crate::Config;
use log::info;
use std::sync::Arc;

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

    pub fn run(&self) {
        info!("Scanner running...");

        info!(
            "Found {} network{}to listen!",
            self.networks.len(),
            if self.networks.len() > 1 { "s " } else { " " }
        );

        self.networks.iter().for_each(|network_config| {
            tokio::task::spawn(listen_blocks_v2(
                network_config.clone(),
                self.database_engine.clone(),
            ));

            tokio::task::spawn(run_network_listener(
                network_config.name.clone(),
                self.glitch_private_key.clone(),
                network_config.ws_glitch_node.clone(),
                self.business_fee,
                self.glitch_gas,
                self.database_engine.clone(),
            ));

            tokio::task::spawn(fee_payer_v2(
                self.database_engine.clone(),
                self.interval_days_for_transfer,
                network_config.ws_glitch_node.clone(),
                network_config.name.clone(),
                self.glitch_private_key.clone(),
                self.glitch_fee_address.clone(),
            ));
        });
        // TODO: change because it consumes the entire cpu
        loop {}
    }
}
