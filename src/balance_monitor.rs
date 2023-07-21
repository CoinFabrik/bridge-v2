use std::time::Instant;

use chrono::Local;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    Message,
    SmtpTransport,
    Transport,
};
use log::info;
use serde_json::json;
use sp_core::{ crypto::Pair, sr25519 };
use substrate_api_client::{
    rpc::WsRpcClient,
    AccountId,
    Api,
    BaseExtrinsicParams,
    PlainTip,
    PlainTipExtrinsicParams,
};
use reqwest::Error;
use tokio::time::Duration;

use crate::config::Notification;

pub fn build_email(emails_to: Vec<String>, message: &str, from: &str, env: &str) -> Message {
    let mut email_builder = Message::builder();

    for email_to in emails_to {
        email_builder = email_builder.to(email_to.parse().unwrap());
        info!("Notification will be sent to: {}", email_to);
    }

    email_builder
        .from(from.parse().unwrap())
        .subject(format!("[{}] GLCH allocation is bridge now is low!", env))
        .header(ContentType::TEXT_PLAIN)
        .body(message.to_string())
        .unwrap()
}

pub async fn send_slack_notify(msg: &str, slack_webhook_url: &str, env: &str) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let body = json!({
        "text": format!("[{}] {}", env, msg)
    });

    client.post(slack_webhook_url).json(&body).send().await?;

    Ok(())
}

pub async fn check_balance_and_notify(
    api: &Api<sr25519::Pair, WsRpcClient, BaseExtrinsicParams<PlainTip>>,
    signer_account_id: &AccountId,
    smtp_config: Notification,
    creds: &Credentials,
    low_balance_in_wei: f64,
    last_email_sent: &mut Instant,
    email_delay: &Duration
) {
    let signer_free_balance = match api.get_account_data(signer_account_id).unwrap() {
        Some(data) => data.free,
        None => 0_u128,
    };

    let now = Instant::now();

    if
        (signer_free_balance as f64) <= low_balance_in_wei &&
        now.duration_since(*last_email_sent) > *email_delay
    {
        let message = format!(
            "GLCH allocation in the new bridge now is lower than {} GLCH, please quickly top it up to prevent any delays in user journey. The current balance is {} GLCH. Timestamp: {}",
            smtp_config.low_balance,
            signer_free_balance / (10_u128).pow(18),
            Local::now().format("%T %d/%m/%Y [%:z]").to_string()
        );
        let email = build_email(
            smtp_config.send_to.clone(),
            &message,
            smtp_config.from.as_str(),
            &smtp_config.env
        );

        let mailer: SmtpTransport = SmtpTransport::relay(smtp_config.host.as_str())
            .unwrap()
            .credentials(creds.clone())
            .build();

        match mailer.send(&email) {
            Ok(_) => {
                info!("Email sent successfully!");
                *last_email_sent = now;
            }
            Err(e) => info!("Could not send email: {e:?}"),
        }

        match send_slack_notify(&message, &smtp_config.slack_webhook, &smtp_config.env).await {
            Ok(_) => {
                info!("Slack notification sent successfully!");
            }
            Err(e) => info!("Could not send slack notification: {e:?}"),
        }
    }
}

pub async fn monitor_balance(glitch_node: String, glitch_pk: String, smtp_config: Notification) {
    info!("Balance monitoring system running now!");
    let client = WsRpcClient::new(&glitch_node);
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let signer_account_id = AccountId::from(signer.public());
    let api = Api::<_, _, PlainTipExtrinsicParams>
        ::new(client)
        .map(|api| api.set_signer(signer))
        .unwrap();

    let mut interval = tokio::time::interval(Duration::from_millis(5000));
    let mut last_email_sent = Instant::now();
    let email_delay = Duration::from_secs(60 * smtp_config.delay_in_minutes);

    let creds = Credentials::new(smtp_config.user.clone(), smtp_config.password.clone());

    let low_balance_in_wei = smtp_config.low_balance * (10_f64).powf(18.0);

    loop {
        tokio::select! {
            _ = interval.tick() => check_balance_and_notify(&api, &signer_account_id, smtp_config.clone(), &creds, low_balance_in_wei, &mut last_email_sent, &email_delay).await
        }
    }
}
