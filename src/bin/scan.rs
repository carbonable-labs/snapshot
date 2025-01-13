use serde_json::json;
use starknet::{
    core::types::{BlockId, BlockTag, Felt, FunctionCall},
    macros::{felt, selector},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::{
    collections::HashMap,
    str::FromStr,
    time::{Duration, Instant},
    vec,
};
use tokio::time::sleep;

#[derive(Debug, serde::Serialize)]
struct TokenInfo {
    owner: String,
    value: String,
    account: String,
    value_in_yielder: String,
    value_in_offsetter: String,
}

#[derive(Debug, serde::Serialize)]
struct ProjectInfo {
    name: String,
    slot: Felt,
    address: Felt,
    yielder: Felt,
    offsetter: Felt,
}

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

async fn scan_project(
    provider: &JsonRpcClient<HttpTransport>,
    project: &ProjectInfo,
) -> Result<HashMap<String, HashMap<String, TokenInfo>>, Box<dyn std::error::Error>> {
    println!("\nScanning project: {}", project.name);
    let mut slot_tokens = HashMap::new();
    let mut slot_map = HashMap::new();

    // Get total supply
    let total_supply_call_result = provider
        .call(
            FunctionCall {
                contract_address: project.address,
                entry_point_selector: selector!("totalSupply"),
                calldata: vec![],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await?;
    let total_supply: usize = total_supply_call_result[0]
        .to_bigint()
        .to_string()
        .parse()
        .unwrap_or(0_usize);

    // get token ids
    let mut calls: Vec<FunctionCall> = vec![];
    for index in 0..total_supply {
        let calldata = vec![Felt::from(index), Felt::ZERO];

        calls.push(FunctionCall {
            contract_address: project.address,
            entry_point_selector: selector!("tokenByIndex"),
            calldata,
        });
    }

    println!("Fetching token ids...");
    let token_ids = aggregate_calls(provider, calls).await?;

    let token_ids: Vec<Felt> = token_ids[2..]
        .chunks(3)
        .map(|chunk| chunk[1].clone())
        .filter(|id| !vec!["2055"].contains(&id.to_bigint().to_string().as_str()))
        .collect();

    println!("Token ids: {:?}", token_ids);

    let calls: Vec<FunctionCall> = token_ids
        .iter()
        .flat_map(|id| {
            vec![FunctionCall {
                contract_address: project.address,
                entry_point_selector: selector!("ownerOf"),
                calldata: vec![*id, Felt::ZERO],
            }]
        })
        .collect();

    println!("Fetching token data...");
    let results = aggregate_calls(provider, calls).await?;

    for (i, data) in results[2..].chunks(2).enumerate() {
        let owner_result = data[1].to_hex_string();
        let token_id = token_ids[i].to_bigint().to_string();

        let token = TokenInfo {
            owner: owner_result,
            value: project.slot.to_string(),
            account: "".to_string(),
            value_in_yielder: "0".to_string(),
            value_in_offsetter: "0".to_string(),
        };

        slot_map.insert(token_id, token);
    }

    for token_id in token_ids.iter() {
        let owner = Felt::from_str(&slot_map[&token_id.to_bigint().to_string()].owner).unwrap();
        let s1 = match provider
            .call(
                FunctionCall {
                    contract_address: owner,
                    entry_point_selector: selector!("supportsInterface"),
                    calldata: vec![felt!(
                        "0x2ceccef7f994940b3962a6c67e0ba4fcd37df7d131417c604f91e03caecc1cd"
                    )],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
        {
            Ok(res) => res[0].to_bigint().to_string(),
            Err(_) => "0".to_string(),
        };
        sleep(RATE_LIMIT).await; // Rate limit delay

        let s2 = match provider
            .call(
                FunctionCall {
                    contract_address: owner,
                    entry_point_selector: selector!("supportsInterface"),
                    calldata: vec![felt!("0xa66bd575")],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
        {
            Ok(res) => res[0].to_bigint().to_string(),
            Err(_) => "0".to_string(),
        };
        sleep(RATE_LIMIT).await; // Rate limit delay

        let s3 = match provider
            .call(
                FunctionCall {
                    contract_address: owner,
                    entry_point_selector: selector!("supportsInterface"),
                    calldata: vec![felt!("0xf10dbd44")],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
        {
            Ok(res) => res[0].to_bigint().to_string(),
            Err(_) => "0".to_string(),
        };
        sleep(RATE_LIMIT).await; // Rate limit delay

        let support = s1 == "1" || s2 == "1" || s3 == "1";
        slot_map.insert(
            token_id.to_bigint().to_string(),
            TokenInfo {
                owner: slot_map[&token_id.to_bigint().to_string()].owner.clone(),
                value: slot_map[&token_id.to_bigint().to_string()].value.clone(),
                account: support.to_string(),
                value_in_yielder: "0".to_string(),
                value_in_offsetter: "0".to_string(),
            },
        );
    }

    slot_tokens.insert(project.slot.to_string(), slot_map);

    Ok(slot_tokens)
}

async fn scan_slot_project(
    provider: &JsonRpcClient<HttpTransport>,
    project: &ProjectInfo,
) -> Result<HashMap<String, HashMap<String, TokenInfo>>, Box<dyn std::error::Error>> {
    println!("\nScanning slot-based project: {}", project.name);
    let mut slot_tokens = HashMap::new();

    let mut slot_map = HashMap::new();

    // Get token supply in current slot
    let supply_result = provider
        .call(
            FunctionCall {
                contract_address: project.address,
                entry_point_selector: selector!("token_supply_in_slot"),
                calldata: vec![Felt::from(project.slot), Felt::ZERO],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await?;

    sleep(RATE_LIMIT).await; // Rate limit delay

    let slot_supply: usize = supply_result[0]
        .to_bigint()
        .to_string()
        .parse()
        .unwrap_or(0_usize);
    println!("Slot {} supply: {}", project.slot, slot_supply);

    // get token ids
    let mut calls: Vec<FunctionCall> = vec![];
    for index in 0..slot_supply {
        let calldata = vec![
            Felt::from(project.slot),
            Felt::ZERO,
            Felt::from(index),
            Felt::ZERO,
        ];

        calls.push(FunctionCall {
            contract_address: project.address,
            entry_point_selector: selector!("token_in_slot_by_index"),
            calldata,
        });
    }

    println!("Fetching token ids...");
    let token_ids = aggregate_calls(provider, calls).await?;

    let token_ids: Vec<Felt> = token_ids[2..]
        .chunks(3)
        .map(|chunk| chunk[1].clone())
        .filter(|id| {
            !vec!["2055", "2056", "2057", "2059"].contains(&id.to_bigint().to_string().as_str())
        })
        .collect();

    let calls: Vec<FunctionCall> = token_ids
        .iter()
        .flat_map(|id| {
            vec![
                FunctionCall {
                    contract_address: project.address,
                    entry_point_selector: selector!("owner_of"),
                    calldata: vec![*id, Felt::ZERO],
                },
                FunctionCall {
                    contract_address: project.address,
                    entry_point_selector: selector!("value_of"),
                    calldata: vec![*id, Felt::ZERO],
                },
            ]
        })
        .collect();

    println!("Fetching token data...");
    let results = aggregate_calls(provider, calls).await?;

    for (i, data) in results[2..].chunks(5).enumerate() {
        let owner_result = data[1].to_hex_string();
        let value_result = data[3].to_bigint().to_string();
        let token_id = token_ids[i].to_bigint().to_string();

        let token = TokenInfo {
            owner: owner_result,
            value: value_result,
            account: "".to_string(),
            value_in_yielder: "".to_string(),
            value_in_offsetter: "".to_string(),
        };

        slot_map.insert(token_id, token);
    }

    let mut calls1: Vec<FunctionCall> = vec![];
    let mut calls2: Vec<FunctionCall> = vec![];
    let mut calls3: Vec<FunctionCall> = vec![];
    let mut calls4: Vec<FunctionCall> = vec![];
    let mut calls5: Vec<FunctionCall> = vec![];

    for token_id in &token_ids {
        let owner = Felt::from_str(&slot_map[&token_id.to_bigint().to_string()].owner).unwrap();

        calls1.push(FunctionCall {
            contract_address: owner,
            entry_point_selector: selector!("supportsInterface"),
            calldata: vec![felt!(
                "0x2ceccef7f994940b3962a6c67e0ba4fcd37df7d131417c604f91e03caecc1cd"
            )],
        });
        calls2.push(FunctionCall {
            contract_address: owner,
            entry_point_selector: selector!("supportsInterface"),
            calldata: vec![felt!("0xa66bd575")],
        });
        calls3.push(FunctionCall {
            contract_address: owner,
            entry_point_selector: selector!("supportsInterface"),
            calldata: vec![felt!("0xf10dbd44")],
        });

        calls4.push(FunctionCall {
            contract_address: project.offsetter,
            entry_point_selector: selector!("get_deposited_of"),
            calldata: vec![owner],
        });
        calls5.push(FunctionCall {
            contract_address: project.yielder,
            entry_point_selector: selector!("get_deposited_of"),
            calldata: vec![owner],
        });
    }

    println!("Fetching token account data...");
    let results1 = aggregate_calls(provider, calls1).await?;
    let results2 = aggregate_calls(provider, calls2).await?;
    let results3 = aggregate_calls(provider, calls3).await?;

    let results4 = if project.offsetter != felt!("0x0") {
        println!("Fetching token offsetter data...");
        aggregate_calls(provider, calls4).await?
    } else {
        vec![]
    };

    let results5 = if project.yielder != felt!("0x0") {
        println!("Fetching token yielder data...");
        aggregate_calls(provider, calls5).await?
    } else {
        vec![]
    };

    for (i, token_id) in token_ids.iter().enumerate() {
        let token_id = &token_id.to_bigint().to_string();
        let support_result = results1[2 + 2 * i + 1].to_bigint().to_string();
        let support_result2 = results2[2 + 2 * i + 1].to_bigint().to_string();
        let support_result3 = results3[2 + 2 * i + 1].to_bigint().to_string();
        let value_in_offsetter = if project.offsetter != felt!("0x0") {
            results4[2 + 3 * i + 1].to_bigint().to_string()
        } else {
            "0".to_string()
        };
        let value_in_yielder = if project.yielder != felt!("0x0") {
            results5[2 + 3 * i + 1].to_bigint().to_string()
        } else {
            "0".to_string()
        };

        let support = support_result == "1" || support_result2 == "1" || support_result3 == "1";
        let token = TokenInfo {
            owner: slot_map[token_id].owner.clone(),
            value: slot_map[token_id].value.clone(),
            account: support.to_string(),
            value_in_yielder,
            value_in_offsetter,
        };

        if !support {
            println!("Token {:?}", token);
        }

        slot_map.insert(token_id.to_string(), token);
    }
    slot_tokens.insert(project.slot.to_string(), slot_map);

    Ok(slot_tokens)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Starknet slot-based token scanner...");
    let start_time = Instant::now();

    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(
        "https://rpc.starknet.lava.build:443",
    )?));
    println!("Provider initialized successfully");

    let project_addresses = vec![
        ProjectInfo {
            name: "BanegasFarm".to_string(),
            slot: felt!("0x1"),
            address: felt!("0x0516d0acb6341dcc567e85dc90c8f64e0c33d3daba0a310157d6bba0656c8769"),
            yielder: felt!("0x03d25473be5a6316f351e8f964d0c303357c006f7107779f648d9879b7c6d58a"),
            offsetter: felt!("0x0324b531f731100b494e2f978a26b20b5870585dd96d9f1166b43a28ebbb8aba"),
        },
        ProjectInfo {
            name: "LasDelicias".to_string(),
            slot: felt!("0x2"),
            address: felt!("0x0516d0acb6341dcc567e85dc90c8f64e0c33d3daba0a310157d6bba0656c8769"),
            yielder: felt!("0x00426d4e86913759bcc49b7f992b1fe62e6571e8f8089c23d95fea815dbad471"),
            offsetter: felt!("0x022f40128af9798a0b734874fd993bbab6cf75845f26f844cb151b7041132c6d"),
        },
        ProjectInfo {
            name: "Manjarisoa".to_string(),
            slot: felt!("0x3"),
            address: felt!("0x0516d0acb6341dcc567e85dc90c8f64e0c33d3daba0a310157d6bba0656c8769"),
            yielder: felt!("0x03d25473be5a6316f351e8f964d0c303357c006f7107779f648d9879b7c6d58a"),
            offsetter: felt!("0x0324b531f731100b494e2f978a26b20b5870585dd96d9f1166b43a28ebbb8aba"),
        },
        ProjectInfo {
            name: "Karathuru".to_string(),
            slot: felt!("0x1"),
            address: felt!("0x05a667adc04676fba78a29371561a0bf91dab25847d5dc4709a93a4cfb5ff293"),
            yielder: felt!("0x0"),
            offsetter: felt!("0x0"),
        },
    ];

    // let mut results = HashMap::new();

    // for project in project_addresses {
    //     let token_data = scan_slot_project(&provider, &project).await?;
    //     results.insert(project.address.to_hex_string(), token_data);
    // }

    // let json_output = json!(results);
    // //println!("\nResults JSON:");
    // //println!("{}", serde_json::to_string_pretty(&json_output)?);

    // std::fs::write(
    //     "output/sv2.json",
    //     serde_json::to_string_pretty(&json_output)?,
    // )?;

    // v1 projects
    let project_addresses = vec![
        ProjectInfo {
            name: "BanegasFarm".to_string(),
            slot: Felt::from(110000000),
            address: BANEGAS,
            yielder: felt!("0x0"),
            offsetter: felt!("0x0"),
        },
        ProjectInfo {
            name: "LasDelicias".to_string(),
            slot: Felt::from(110000000),
            address: LAS_DELICIAS,
            yielder: felt!("0x0"),
            offsetter: felt!("0x0"),
        },
        ProjectInfo {
            name: "ManjarisoBronze".to_string(),
            slot: Felt::from(54500000),
            address: MANJARISO_BRONZE,
            yielder: felt!("0x0"),
            offsetter: felt!("0x0"),
        },
        ProjectInfo {
            name: "ManjarisoSilver".to_string(),
            slot: Felt::from(272500000),
            address: MANJARISO_SILVER,
            yielder: felt!("0x0"),
            offsetter: felt!("0x0"),
        },
        ProjectInfo {
            name: "ManjarisoGold".to_string(),
            slot: Felt::from(817500000),
            address: MANJARISO_GOLD,
            yielder: felt!("0x0"),
            offsetter: felt!("0x0"),
        },
    ];

    let mut results = HashMap::new();

    for project in project_addresses {
        let token_data = scan_project(&provider, &project).await?;
        results.insert(project.address.to_hex_string(), token_data);
    }

    let json_output = json!(results);
    //println!("\nResults JSON:");
    //println!("{}", serde_json::to_string_pretty(&json_output)?);

    std::fs::write(
        "output/sv1.json",
        serde_json::to_string_pretty(&json_output)?,
    )?;

    let duration = start_time.elapsed();
    println!("\nScan completed in {:?}", duration);

    Ok(())
}

const BANEGAS: Felt = felt!("0x04047810e4f759336f941a16b6de9d8d2f934e976b9a9431a2964646df9025c6");
const LAS_DELICIAS: Felt =
    felt!("0x00ebf4bbab9c934fa0212c9358331d4a9543ad66a7e0007d4a720cfa6b56061e");
const MANJARISO_BRONZE: Felt =
    felt!("0x0541b5dd5fae206ceccaf4eeb0642e4c04d456c5bc296eab047c9414bdad4f09");
const MANJARISO_SILVER: Felt =
    felt!("0x06191013bbd6bcf11d69f3d60d20f7aa03439d7011bd511d13122de2275dce21");
const MANJARISO_GOLD: Felt =
    felt!("0x061bcc33b0469cd072ad47813a1efd250fd36a28425d774c31c8f33c87306e8e");
