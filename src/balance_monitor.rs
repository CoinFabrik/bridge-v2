use crate::config::Config;
use substrate_api_client::{
    rpc::WsRpcClient, AccountId, Api, BaseExtrinsicParams, GenericAddress, MultiAddress, PlainTip,
    PlainTipExtrinsicParams, XtStatus,
};

pub async fn monitor_balance(config: Config){
    let client = WsRpcClient::new(&config. glitch_node);
    let signer: sr25519::Pair = Pair::from_string(&glitch_pk, None).unwrap();
    let signer_account_id = AccountId::from(signer.public());
    let api = Api::<_, _, PlainTipExtrinsicParams>::new(client)
        .map(|api| api.set_signer(signer))
        .unwrap();

    let interval = tokio::time::interval(Duration::from_millis(5000));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let signer_free_balance = match api.get_account_data(&signer_account_id).unwrap() {
                    Some(data) => data.free,
                    None => 0_u128,
                };

            }
        }
    }
}
