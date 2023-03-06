use log::{debug, error};
use mysql_async::prelude::{BatchQuery, Queryable, WithParams};
use mysql_async::{params, Conn, Pool, Row};
use sp_core::U256;
use web3::types::{Log, H160, H256};

use crate::config::Db;

const SELECT_TRANSACTIONS_TO_PROCESS: &str =
    r"SELECT id, to_glitch_address, amount FROM tx WHERE state = 'TO_PROCESS'";
const SELECT_NETWORK_STATE: &str =
    r"SELECT id, network, monitor_address, last_block FROM scanner_state WHERE name = :name ";
const INSERT_NETWORK_STATE: &str = r"INSERT INTO scanner_state (name, network, monitor_address) VALUES (:name, :network, :monitor_address)";
const INSERT_TX_FEE: &str =
    r"INSERT INTO fee_transaction (hash, amount) values (:tx_glitch_hash, :amount)";
const SELECT_LAST_BLOCK: &str = r"SELECT last_block FROM scanner_state WHERE name = :name";
const SELECT_FEE_ACCUMULATED: &str =
    r"SELECT accumulated_fees FROM scanner_state WHERE name = :name";
const UPDATE_LAST_BLOCK: &str = r"UPDATE scanner_state SET last_block = :block WHERE name = :name";
const UPDATE_FEE: &str =
    r"UPDATE scanner_state SET accumulated_fees = :accumulated_fees WHERE name = :name";
const UPDATE_TX_GLITCH: &str = r"UPDATE tx SET tx_glitch_hash = :glitch_tx_hash, state = 'PROCESSED', business_fee_amount = :business_fee_amount, business_fee_percentage = :business_fee_percentage WHERE id = :id";
const UPDATE_TX_GLITCH_TO_PROCESSING: &str = r"UPDATE tx SET state = 'PROCESSING' WHERE id = :id";
const INSERT_TXS: &str = r"INSERT INTO tx (tx_eth_hash, from_eth_address, amount, to_glitch_address) VALUES (:tx_eth_hash, :from_eth_address, :amount, :to_glitch_address)";
const SAVE_ERROR: &str = r"UPDATE tx SET error = :error WHERE id = :id";
const GET_LAST_FEE_TIME: &str = r"SELECT MAX(time) as time FROM fee_transaction";

#[derive(Clone)]
pub struct ScannerState {
    pub name: String,
    pub network: String,
    pub monitor_address: String,
    pub config: Db,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TxToProcess {
    pub id: u128,
    pub glitch_address: String,
    pub amount: String,
}

impl ScannerState {
    pub fn new(name: String, network: String, monitor_address: String, db_config: Db) -> Self {
        Self {
            name,
            network,
            monitor_address,
            config: db_config,
        }
    }

    pub async fn txs_to_process(&self) -> Vec<TxToProcess> {
        let mut conn = self.establish_connection().await;

        let txs_to_process = conn
            .query_map(
                SELECT_TRANSACTIONS_TO_PROCESS,
                |(id, glitch_address, amount)| TxToProcess {
                    id,
                    glitch_address,
                    amount,
                },
            )
            .await
            .unwrap();

        drop(conn);
        txs_to_process
    }

    pub async fn exists_network_state(&self) -> bool {
        let mut conn = self.establish_connection().await;

        let result: Option<Row> = conn
            .exec_first(
                SELECT_NETWORK_STATE,
                params! {
                    "name" => &self.name
                },
            )
            .await
            .unwrap();

        let ret = if result.is_some() {
            true
        } else {
            let params = params! {
                "name" => &self.name,
                "network" => &self.network,
                "monitor_address" => &self.monitor_address
            };
            let result = INSERT_NETWORK_STATE
                .with(vec![params])
                .batch(&mut conn)
                .await;

            match result {
                Ok(_) => debug!("New scanner state created!"),
                Err(e) => panic!("The scanner status could not be created in the database.: {e}"),
            }

            false
        };

        drop(conn);
        ret
    }

    pub async fn get_last_block(&self) -> u32 {
        let mut conn = self.establish_connection().await;

        let result: u32 = conn
            .exec_first(
                SELECT_LAST_BLOCK,
                params! {
                    "name" => &self.name
                },
            )
            .await
            .unwrap()
            .unwrap();

        drop(conn);
        result
    }

    pub async fn insert_tx_fee(&self, glitch_hash: String, amount: String) {
        let mut conn = self.establish_connection().await;

        let params = params! {
            "tx_glitch_hash" => glitch_hash,
            "amount" => amount,
        };
        let result = INSERT_TX_FEE.with(vec![params]).batch(&mut conn).await;

        match result {
            Ok(_) => debug!("New tx fee created!"),
            Err(e) => panic!("Fee tx could not be created in the database.: {e}"),
        }
    }

    pub async fn get_fee_counter(&self) -> u128 {
        let mut conn = self.establish_connection().await;

        let result: u128 = conn
            .exec_first(
                SELECT_FEE_ACCUMULATED,
                params! {
                    "name" => &self.name
                },
            )
            .await
            .unwrap()
            .unwrap();

        drop(conn);
        result
    }

    pub async fn modify_fee_counter(&self, fee_amount: u128) {
        let mut conn = self.establish_connection().await;
        let params = params! {
            "name" => &self.name,
            "accumulated_fees" => fee_amount
        };

        let result = conn.exec_drop(UPDATE_FEE, params).await;

        match result {
            Ok(_) => debug!("Fee increased successful!"),
            Err(e) => error!("Error in the fee increased: {}", e),
        }

        drop(conn);
    }

    pub async fn update_block(&self, block: u32) {
        let mut conn = self.establish_connection().await;
        let params = params! {
            "block" => block,
            "name" => &self.name
        };

        let result = conn.exec_drop(UPDATE_LAST_BLOCK, params).await;

        match result {
            Ok(_) => debug!("Block update successful!"),
            Err(e) => error!("Error in the block update: {}", e),
        }

        drop(conn);
    }

    pub async fn update_tx_to_processing(&self, id: u128) {
        let mut conn = self.establish_connection().await;
        let params = params! {
            "id" => id,
        };
        let result = conn.exec_drop(UPDATE_TX_GLITCH_TO_PROCESSING, params).await;
        match result {
            Ok(_) => debug!("Glitch tx updated!"),
            Err(e) => error!("Error in the glitch tx updated: {}", e),
        }
        drop(conn);
    }

    pub async fn update_tx(
        &self,
        id: u128,
        glitch_hash: String,
        business_fee_amount: u128,
        business_fee_percentage: u128,
    ) {
        let mut conn = self.establish_connection().await;
        let params = params! {
            "id" => id,
            "glitch_tx_hash" => glitch_hash,
            "business_fee_amount" => business_fee_amount,
            "business_fee_percentage" => business_fee_percentage
        };

        let result = conn.exec_drop(UPDATE_TX_GLITCH, params).await;

        match result {
            Ok(_) => debug!("Glitch tx updated!"),
            Err(e) => error!("Error in the glitch tx updated: {}", e),
        }
        drop(conn);
    }

    pub async fn save_with_error(&self, id: u128, error_message: String) {
        let mut conn = self.establish_connection().await;
        let params = params! {
            "id" => id,
            "error" => error_message,
        };

        let result = conn.exec_drop(SAVE_ERROR, params).await;

        match result {
            Ok(_) => debug!("Glitch tx updated!"),
            Err(e) => error!("Error in the glitch tx updated: {}", e),
        }
        drop(conn);
    }

    pub async fn insert_txs(&self, logs: Vec<Log>) {
        let mut conn = self.establish_connection().await;
        let result = INSERT_TXS
            .with(logs.iter().map(|tx| {
                let data: Vec<u8> = tx.data.0.clone();
                let data_chunks: Vec<&[u8]> = data.chunks(32).collect();
                let string_len = U256::from_big_endian(data_chunks[2]).as_usize();
                let glitch_address: Vec<u8> = [data_chunks[3], data_chunks[4]]
                    .concat()
                    .iter()
                    .copied()
                    .take(string_len)
                    .collect();

                params! {
                    "tx_eth_hash" => format!("{:#x}",tx.transaction_hash.unwrap()),
                    "from_eth_address" => h256_to_address(*tx.topics.get(1).unwrap()),
                    "amount" => U256::from_big_endian(data_chunks[1]).to_string(),
                    "to_glitch_address" => std::str::from_utf8(glitch_address.as_slice()).unwrap()
                }
            }))
            .batch(&mut conn)
            .await;

        match result {
            Ok(_) => debug!("Inserts successful!"),
            Err(e) => error!("Inserts with error: {}", e),
        }

        drop(conn);
    }

    pub async fn get_fee_last_time(&self) -> String {
        let mut conn = self.establish_connection().await;
        let result: String = conn.query_first(GET_LAST_FEE_TIME).await.unwrap().unwrap();
        drop(conn);
        result
    }

    async fn establish_connection(&self) -> Conn {
        let database_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            self.config.username,
            self.config.password,
            self.config.host,
            self.config.port,
            self.config.database
        );

        let pool = Pool::new(database_url.as_str());
        pool.get_conn().await.unwrap()
    }
}

fn h256_to_address(h: H256) -> String {
    format!("{:#x}", H160::from(h))
}
