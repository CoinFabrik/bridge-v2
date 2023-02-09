mod args;
mod block_listener;
mod config;
mod database;
mod glitch;
mod js_call;
mod logger;
mod scanner;

use crate::args::Args;
use crate::config::Config;
use clap::Parser;
use database::ScannerState;
use scanner::Scanner;
use std::sync::Arc;

const TITLE: &str = r#"
                                                                                                              
   /$$$$$$  /$$ /$$   /$$               /$$             /$$$$$$$            /$$       /$$                     
  /$$__  $$| $$|__/  | $$              | $$            | $$__  $$          |__/      | $$                     
 | $$  \__/| $$ /$$ /$$$$$$    /$$$$$$$| $$$$$$$       | $$  \ $$  /$$$$$$  /$$  /$$$$$$$  /$$$$$$   /$$$$$$  
 | $$ /$$$$| $$| $$|_  $$_/   /$$_____/| $$__  $$      | $$$$$$$  /$$__  $$| $$ /$$__  $$ /$$__  $$ /$$__  $$ 
 | $$|_  $$| $$| $$  | $$    | $$      | $$  \ $$      | $$__  $$| $$  \__/| $$| $$  | $$| $$  \ $$| $$$$$$$$ 
 | $$  \ $$| $$| $$  | $$ /$$| $$      | $$  | $$      | $$  \ $$| $$      | $$| $$  | $$| $$  | $$| $$_____/ 
 |  $$$$$$/| $$| $$  |  $$$$/|  $$$$$$$| $$  | $$      | $$$$$$$/| $$      | $$|  $$$$$$$|  $$$$$$$|  $$$$$$$ 
  \______/ |__/|__/   \___/   \_______/|__/  |__/      |_______/ |__/      |__/ \_______/ \____  $$ \_______/ 
                                                                                         /$$  \ $$            
                                                                                        |  $$$$$$/            
                                                                                         \______/             

Welcome to Glitch Bridge!
"#;

#[tokio::main]
async fn main() -> web3::Result<()> {
    println!("{TITLE}");

    let args = Args::parse();

    logger::config(args.loglevel);

    let config: Arc<Config> = Arc::new(Config::new(args).check_private_keys());

    Scanner::new(config).run();

    Ok(())
}
