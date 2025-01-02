use serde_json::{json, Value};
use starknet::{
    core::types::{BlockId, BlockTag, Felt, FunctionCall},
    macros::{felt, selector},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::{collections::HashMap, time::Instant};

async fn scan_project(
    provider: &JsonRpcClient<HttpTransport>,
    contract_address: Felt,
) -> Result<Value, Box<dyn std::error::Error>> {
    println!("\nScanning project: {}", contract_address.to_hex_string());
    let mut token_owners = HashMap::new();

    let name_call_result = provider
        .call(
            FunctionCall {
                contract_address,
                entry_point_selector: selector!("name"),
                calldata: vec![],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await?;
    let name = felt_to_string(name_call_result[0].clone());

    // Get total supply
    let total_supply_call_result = provider
        .call(
            FunctionCall {
                contract_address,
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

    println!("Total supply for {}: {}", name, total_supply);

    if total_supply == 0 {
        return Ok(json!({
            "address": contract_address.to_hex_string(),
            "token_owners": token_owners,
        }));
    }

    let mut proms = Vec::with_capacity(total_supply - 1);
    for i in 0..total_supply - 1 {
        proms.push(provider.call(
            FunctionCall {
                contract_address,
                entry_point_selector: selector!("tokenByIndex"),
                calldata: vec![Felt::from(i), Felt::ZERO],
            },
            BlockId::Tag(BlockTag::Latest),
        ));
    }

    let all_tokens: Vec<Vec<Felt>> = futures::future::join_all(proms)
        .await
        .into_iter()
        .map(|x| x.expect("Failed to fetch token index"))
        .collect();

    for token in all_tokens.iter() {
        let token_id = token[0].to_bigint().to_string();
        match provider
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector: selector!("ownerOf"),
                    calldata: token.clone(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
        {
            Ok(res) => {
                let owner = res[0].to_hex_string();
                token_owners.insert(token_id, owner);
            }
            Err(e) => {
                eprintln!(
                    "Error fetching owner for token ID {} in project {}: {}",
                    token[0].to_bigint(),
                    contract_address.to_hex_string(),
                    e
                );
            }
        }
    }

    // Return structured JSON object
    Ok(json!({
        "address": contract_address.to_hex_string(),
        "token_owners": token_owners,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Starknet multi-project token scanner...");
    let start_time = Instant::now();

    // Initialize provider
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(
        "https://starknet-mainnet.public.blastapi.io/rpc/v0_7",
    )?));
    println!("Provider initialized successfully");

    // Array of project addresses
    let project_addresses = vec![
        BANEGAS,
        LAS_DELICIAS,
        MANJARISO_BRONZE,
        MANJARISO_SILVER,
        MANJARISO_GOLD,
    ];

    let mut results = Vec::new();

    // Scan each project
    for address in project_addresses {
        if let Ok(project_result) = scan_project(&provider, address).await {
            results.push(project_result);
        }
    }

    // Convert results to JSON and print
    let json_output = json!(results);

    let duration = start_time.elapsed();
    println!("\nScan completed in {:?}", duration);

    // Save to file
    std::fs::write(
        "output/v1.json",
        serde_json::to_string_pretty(&json_output)?,
    )?;

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

fn felt_to_string(number: Felt) -> String {
    String::from_utf8(number.to_bytes_be().to_vec())
        .expect("Invalid UTF-8")
        .replace("\0", "")
}
