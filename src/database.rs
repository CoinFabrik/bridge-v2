use std::process;

use log::{debug, error, info};
use mysql_async::prelude::{BatchQuery, Queryable, WithParams};
use mysql_async::{params, Conn, Pool, Row, TxOpts, Params, OptsBuilder};
use sp_core::U256;
use web3::types::{Log, H160, H256};
use tokio::time::{Duration, sleep};

use crate::config::{self, Database};

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
const INSERT_TXS: &str = r"INSERT INTO tx (tx_eth_hash, from_eth_address, amount, to_glitch_address) VALUES (:tx_eth_hash, :from_eth_address, :amount, :to_glitch_address)";
const SAVE_ERROR: &str = r"UPDATE tx SET error = :error WHERE id = :id";
const GET_LAST_FEE_TIME: &str = r"SELECT time FROM fee_transaction ft ORDER BY time DESC LIMIT 1";
const UPDATE_TX_WITH_TRANSACTION_FEE_ID: &str = r"UPDATE tx t SET t.wich_transaction_fee = :transaction_fee_id WHERE t.wich_transaction_fee is NULL  AND t.state = 'PROCESSED';";

#[derive(Clone)]
pub struct ScannerState {
    pub name: String,
    pub network: String,
    pub monitor_address: String,
    pub config: Database,
    pub connection_pool: Pool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TxToProcess {
    pub id: u128,
    pub glitch_address: String,
    pub amount: String,
}

pub struct DatabaseEngine {
    pub host: String,
    pub user: String,
    pub password: String,
    pub port: u32,
    pub database: String,
}

impl DatabaseEngine {
    pub async fn establish_connection(&self) -> Conn {
        const MAX_RETRIES: u8 = 5;
        for i in 1..=MAX_RETRIES {
            let database_url = format!(
                "mysql://{}:{}@{}:{}/{}",
                self.user,
                self.password,
                self.host,
                self.port,
                self.database
            );
            let opts = OptsBuilder::from_opts(database_url.as_str());
            match mysql_async::Conn::new(opts).await {
                Ok(conn) => return conn,
                Err(e) => {
                    error!("Error establishing connection (attempt {} of {}): {}", i, MAX_RETRIES, e);
                    if i < MAX_RETRIES {
                        sleep(Duration::from_secs(5)).await;
                    } else {
                        error!("The connection could not be established after {} attempts, terminating the program.", MAX_RETRIES);
                        process::exit(1);
                    }
                }
            }
        }
        unreachable!()
    }
}

impl DatabaseEngine {
    pub fn new(db_config: config::Database) -> Self {
        Self {
            host: db_config.host,
            user: db_config.username,
            password: db_config.password,
            port: db_config.port,
            database: db_config.database,
        }
    }

    pub async fn get_fee_last_time(&self) -> Option<String> {
        let mut conn = self.establish_connection().await;
        let result: Option<String> = conn.query_first(GET_LAST_FEE_TIME).await.unwrap();
        drop(conn);
        result
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

    pub async fn update_tx_with_error(&self, id: u128, error_message: String) {
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

    pub async fn increment_fee_counter(&self, scanner_name: String, amount: u128) {
        let mut conn = self.establish_connection().await;

        let current_fee_counter: u128 = conn
            .exec_first(
                SELECT_FEE_ACCUMULATED,
                params! {
                    "name" => &scanner_name
                },
            )
            .await
            .unwrap()
            .unwrap();

        let params = params! {
            "name" => scanner_name,
            "accumulated_fees" => current_fee_counter + amount
        };

        let result = conn.exec_drop(UPDATE_FEE, params).await;

        match result {
            Ok(_) => debug!("Fee increased successful!"),
            Err(e) => error!("Error in the fee increased: {}", e),
        }
    }

    pub async fn update_tx(
        &self,
        id: u128,
        glitch_hash: String,
        business_fee_amount: u128,
        business_fee_percentage: String,
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

    pub async fn get_last_block(&self, scanner_name: &str) -> u32 {
        let mut conn = self.establish_connection().await;

        let result: u32 = conn
            .exec_first(
                SELECT_LAST_BLOCK,
                params! {
                    "name" => scanner_name
                },
            )
            .await
            .unwrap()
            .unwrap();

        drop(conn);
        result
    }

    pub async fn update_block_and_insert_txs(
        &self,
        scanner_name: String,
        block: u32,
        logs: Vec<Log>,
    ) {
        let mut conn = self.establish_connection().await;
        let mut tx = conn.start_transaction(TxOpts::new()).await.unwrap();

        let params = params! {
            "block" => block,
            "name" => scanner_name
        };

        let update_block_result = tx.exec_drop(UPDATE_LAST_BLOCK, params).await;
        match update_block_result {
            Ok(_) => debug!("Block update successful!"),
            Err(e) => error!("Error in the block update: {}", e),
        }

        if !logs.is_empty() {
            let insert_logs_result = tx.exec_batch(
                INSERT_TXS,
                logs.iter().map(|tx| {
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
                }),
            )
            .await;

            match insert_logs_result {
                Ok(_) => debug!("Inserts successful!"),
                Err(e) => error!("Inserts with error: {}", e),
            }
        }

        if tx.affected_rows() > 0 {
            tx.commit().await.unwrap()
        } else {
            tx.rollback().await.unwrap()
        }
    }

    pub async fn get_fee_counter(&self, scanner_name: &str) -> u128 {
        let mut conn = self.establish_connection().await;

        let result: u128 = conn
            .exec_first(
                SELECT_FEE_ACCUMULATED,
                params! {
                    "name" => scanner_name
                },
            )
            .await
            .unwrap()
            .unwrap();

        drop(conn);
        result
    }

    pub async fn modify_fee_counter(&self, fee_amount: u128, scanner_name: &str) {
        let mut conn = self.establish_connection().await;
        let params = params! {
            "name" => scanner_name,
            "accumulated_fees" => fee_amount
        };

        let result = conn.exec_drop(UPDATE_FEE, params).await;

        match result {
            Ok(_) => debug!("Fee increased successful!"),
            Err(e) => error!("Error in the fee increased: {}", e),
        }

        drop(conn);
    }

    pub async fn insert_tx_fee(&self, glitch_hash: String, amount: String) {
        let mut conn = self.establish_connection().await;

        let params = params! {
            "tx_glitch_hash" => glitch_hash,
            "amount" => amount,
        };
        let result = INSERT_TX_FEE.with(vec![params]).batch(&mut conn).await;

        match result {
            Ok(_) => {
                debug!("New tx fee created!");
                let last_id: u64 = conn.exec_first("SELECT LAST_INSERT_ID()", Params::Empty).await.unwrap().unwrap();

                let result = conn.exec_drop(UPDATE_TX_WITH_TRANSACTION_FEE_ID, params!{"transaction_fee_id" => last_id}).await;

                match result {
                    Ok(_) => info!("Tx updated with transaction fee id!"),
                    Err(e) => error!("Error when updating the transaction fee id of the tx {e}")
                }
            },
            Err(e) => error!("Fee tx could not be created in the database.: {e}"),
        }
    }

    pub async fn exists_network_state(&self, scanner_name: &str, network: &str, monitor_address: &str) -> bool {
        let mut conn = self.establish_connection().await;

        let result: Option<Row> = conn
            .exec_first(
                SELECT_NETWORK_STATE,
                params! {
                    "name" => scanner_name
                },
            )
            .await
            .unwrap();

        let ret = if result.is_some() {
            true
        } else {
            let params = params! {
                "name" => scanner_name,
                "network" => network,
                "monitor_address" => monitor_address
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
}

fn h256_to_address(h: H256) -> String {
    format!("{:#x}", H160::from(h))
}
