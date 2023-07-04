use std::time::Instant;

use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    Message,
    SmtpTransport,
    Transport,
};
use log::info;
use sp_core::{ crypto::Pair, sr25519 };
use substrate_api_client::{ rpc::WsRpcClient, AccountId, Api, PlainTipExtrinsicParams };
use tokio::time::Duration;

use crate::config::Notification;

pub fn build_email(emails_to: Vec<String>, low_balance: f64) -> Message {
    let mut email_builder = Message::builder();

    for email_to in emails_to {
        email_builder = email_builder.to(email_to.parse().unwrap());
        info!("Notification will be sent to: {}", email_to);
    }

    email_builder
        .from("NoBody <nobody@glitch.finance>".parse().unwrap())
        .subject("[Important] GLCH allocation is bridge now is low!")
        .header(ContentType::TEXT_PLAIN)
        .body(
            format!("[Important] GLCH allocation is bridge now is lower than {} GLCH, please quickly top it up to prevent any delays in user journey.", low_balance)
        )
        .unwrap()
}

pub async fn monitor_balance(glitch_node: String, glitch_pk: String, smtp_config: Notification) {
    let client = WsRpcClient::new(&glitch_node);
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let signer_account_id = AccountId::from(signer.public());
    let api = Api::<_, _, PlainTipExtrinsicParams>
        ::new(client)
        .map(|api| api.set_signer(signer))
        .unwrap();

    let mut interval = tokio::time::interval(Duration::from_millis(5000));
    let mut last_email_sent = Instant::now();
    let email_delay = Duration::from_secs(1800);

    let email = build_email(smtp_config.send_to, smtp_config.low_balance);

    let creds = Credentials::new(smtp_config.user, smtp_config.password);

    let low_balance_in_wei = smtp_config.low_balance * (10_f64).powf(18.0);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let signer_free_balance = match api.get_account_data(&signer_account_id).unwrap() {
                    Some(data) => data.free,
                    None => 0_u128,
                };

                let now = Instant::now();

                if signer_free_balance as f64 <= low_balance_in_wei && now.duration_since(last_email_sent) > email_delay {
                        let mailer: SmtpTransport = SmtpTransport::relay(smtp_config.host.as_str())
                        .unwrap()
                        .credentials(creds.clone())
                        .build();

                        match mailer.send(&email) {
                            Ok(_) => {
                                info!("Email sent successfully!");
                                last_email_sent = now;
                            },
                            Err(e) => info!("Could not send email: {e:?}"),
                        };
                }
                
            }
        }
    }
}
