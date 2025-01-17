use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use itertools::Itertools;

use starknet::{
    core::types::{BlockId, BlockTag, Felt, FunctionCall},
    macros::{felt, selector},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::{time::Duration, vec};
use tokio::time::sleep;

pub const RATE_LIMIT: Duration = Duration::from_millis(100);
pub const MULTICALL_CONTRACT: Felt =
    felt!("0x0038e22d0a15703176262dd457a56e5176d13acdfa206d8d397e405223552c9c");

async fn aggregate_calls(
    provider: &JsonRpcClient<HttpTransport>,
    calls: Vec<FunctionCall>,
) -> Result<Vec<Felt>, Box<dyn std::error::Error>> {
    let mut calldata = vec![calls.len().into()];
    for call in calls {
        calldata.push(call.contract_address);
        calldata.push(call.entry_point_selector);
        calldata.push(call.calldata.len().into());
        calldata.extend(call.calldata);
    }
    let res = provider.call(
        FunctionCall {
            contract_address: MULTICALL_CONTRACT,
            entry_point_selector: selector!("aggregate"),
            calldata,
        },
        BlockId::Tag(BlockTag::Latest),
    );
    let res = res.await?;
    sleep(RATE_LIMIT).await; // Rate limit delay
    Ok(res)
}

async fn get_deposited(provider: &JsonRpcClient<HttpTransport>, farm_address: &str) {
    let file = File::open("yielder_depositers/".to_owned() + farm_address + ".txt")
        .expect("file should open read only");
    let reader = BufReader::new(file);

    let addrs = reader
        .lines()
        .map(|line| line.unwrap())
        .unique()
        .collect::<Vec<String>>();

    let calls = addrs
        .iter()
        .map(|addr| FunctionCall {
            contract_address: Felt::from_hex(farm_address).unwrap(),
            entry_point_selector: selector!("get_deposited_of"),
            calldata: vec![Felt::from_hex(addr).unwrap()],
        })
        .collect::<Vec<FunctionCall>>();

    let result = aggregate_calls(provider, calls).await.unwrap();
    let mut total = 0;
    for (addr, res) in addrs.iter().zip(result[2..].chunks(2)) {
        total += res[0].to_bigint().to_string().parse::<u64>().unwrap();
    }
    println!(
        "Total deposited of {} is ${}",
        farm_address,
        total / 1_000_000
    );
}

#[tokio::main]
async fn main() {
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://rpc.starknet.lava.build:443").expect("error parsing URL"),
    ));
    println!("Provider initialized successfully");
    let addresses = vec![
        "0x03d25473be5a6316f351e8f964d0c303357c006f7107779f648d9879b7c6d58a",
        "0x0324b531f731100b494e2f978a26b20b5870585dd96d9f1166b43a28ebbb8aba",
        "0x00426d4e86913759bcc49b7f992b1fe62e6571e8f8089c23d95fea815dbad471",
        "0x022f40128af9798a0b734874fd993bbab6cf75845f26f844cb151b7041132c6d",
        "0x03d25473be5a6316f351e8f964d0c303357c006f7107779f648d9879b7c6d58a",
        "0x03afe61732ed9b226309775ac4705129319729d3bee81da5632146ffd72652ae",
    ];
    for addr in addresses {
        get_deposited(&provider, addr).await;
    }
}
