use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input};
use log::LevelFilter;
use std::{self, fmt::Debug, io::Error};

/// Glitch blockchain bridge.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Configuration file to use
    #[clap(short, long, value_parser, default_value = "config.json")]
    pub config: std::path::PathBuf,
    /// Level of logs, can be (OFF, ERROR, WARN, INFO, DEBUG, TRACE)
    #[clap(short, long, default_value = "INFO")]
    pub loglevel: LevelFilter,
}

pub fn request_private_keys() -> Result<String, Error> {
    Input::with_theme(&ColorfulTheme::default())
        .allow_empty(true)
        .with_prompt("Enter the private key of the Glitch account.")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                Ok(())
            } else {
                Err("Glitch private key is not valid!")
            }
        })
        .interact_text()
}
