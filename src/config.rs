use crate::args::{request_private_keys, Args};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub glitch_private_key: Option<String>,
    pub glitch_fee_address: String,
    pub interval_days_for_transfer: u32,
    pub business_fee: f64,
    pub glitch_gas: bool,
    pub db: Database,
    pub networks: Vec<Network>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    pub host: String,
    pub port: u32,
    pub database: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Network {
    pub name: String,
    pub network: String,
    pub monitor_address: String,
    pub ws_node: String,
    pub ws_glitch_node: String,
    pub confirmations: i32,
}

impl Config {
    pub fn new(args: Args) -> Self {
        let mut file = File::open(&args.config).expect("File not found!");

        let mut data = String::new();

        file.read_to_string(&mut data)
            .expect("Error while reading file!");

        match serde_json::from_str(&data) {
            Ok(config) => config,
            Err(e) => panic!("Error parsing json: {e}"),
        }
    }

    pub fn check_private_keys(mut self) -> Self {
        if self.glitch_private_key.is_some() {
            info!("The Glitch private key from the configuration file will be used.");
        } else {
            let glitch_private_key_result = request_private_keys();
            match glitch_private_key_result {
                Ok(pk) => {
                    info!("Config private key from standar input!");
                    self.glitch_private_key = Some(pk);
                }
                Err(e) => error!("{}", e),
            }
        }

        self
    }
}
