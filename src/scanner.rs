use crate::block_listener::listen_blocks;
use crate::glitch::{self, fee_payer};
use crate::{Config, ScannerState};
use futures::executor::block_on;
use log::info;
use std::sync::Arc;
use tokio::task::spawn_blocking;

pub struct Scanner {
    pub config: Arc<Config>,
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
