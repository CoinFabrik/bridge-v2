mod args;
mod balance_monitor;
mod block_listener;
mod config;
mod database;
mod glitch;
mod logger;
mod scanner;

use crate::args::Args;
use crate::config::Config;
use clap::Parser;
use scanner::ScannerV2;

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

    let config: Config = Config::new(args).check_private_keys();

    ScannerV2::new(config).run();

    Ok(())
}
