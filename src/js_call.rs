use std::process::Command;

use log::info;

pub fn get_fee(amount: u128, address: &str) -> u128 {
    info!(
        "Calculating fee for a transfer of {} amount to {} address.",
        amount, address
    );

    let command = format!("node ./feescript/index.js {} {}", amount, address);
    let test = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("Error");

    let out = test.stdout;
    let fee = std::str::from_utf8(out.as_slice())
        .unwrap()
        .replace('\n', "");

    fee.parse().unwrap()
}

pub fn transfer(amount: u128, address: &str) -> String {
    info!(
        "Calling transfer from javascript. Amount: {}, Address: {}",
        amount, address
    );

    let command = format!("node ./feescript/transfer.js {} {}", amount, address);
    let test = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("Error");

    let out = test.stdout;
    let hash = std::str::from_utf8(out.as_slice())
        .unwrap()
        .replace('\n', "");

    hash
}
