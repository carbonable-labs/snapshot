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
    time::{Duration, Instant},
};
use tokio::time::sleep;

#[derive(Debug, serde::Serialize)]
struct TokenInfo {
    owner: String,
    value: String,
}

pub const RATE_LIMIT: Duration = Duration::from_millis(100);

async fn scan_slot_project(
    provider: &JsonRpcClient<HttpTransport>,
    contract_address: Felt,
) -> Result<HashMap<String, HashMap<String, TokenInfo>>, Box<dyn std::error::Error>> {
    println!(
        "\nScanning slot-based project: {}",
        contract_address.to_hex_string()
    );
    let mut slot_tokens = HashMap::new();

    // Get total slot count
    let slot_count_result = provider
        .call(
            FunctionCall {
                contract_address,
                entry_point_selector: selector!("slot_count"),
                calldata: vec![],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await?;

    sleep(RATE_LIMIT).await; // Rate limit delay

    let slot_count: usize = slot_count_result[0]
        .to_bigint()
        .to_string()
        .parse()
        .unwrap_or(0_usize);
    println!(
        "Total slots for {}: {}",
        contract_address.to_hex_string(),
        slot_count
    );

    // Iterate through each slot
    for slot in 1..slot_count + 1 {
        println!("Processing slot {}/{}", slot, slot_count);
        let mut slot_map = HashMap::new();

        // Get token supply in current slot
        let supply_result = provider
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector: selector!("token_supply_in_slot"),
                    calldata: vec![Felt::from(slot), Felt::ZERO],
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
        println!("Slot {} supply: {}", slot, slot_supply);

        // Process tokens sequentially with rate limiting
        for index in 0..slot_supply {
            if index % 10 == 0 {
                println!(
                    "Processing token {}/{} in slot {}",
                    index + 1,
                    slot_supply,
                    slot
                );
            }
            // let [slot, index] = [U256::from(slot), U256::from(index)];
            // pri
            // Get token
            let calldata = vec![Felt::from(slot), Felt::ZERO, Felt::from(index), Felt::ZERO];
            let token = provider
                .call(
                    FunctionCall {
                        contract_address,
                        entry_point_selector: selector!("token_in_slot_by_index"),
                        calldata,
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await?;

            sleep(RATE_LIMIT).await; // Rate limit delay

            let token_id = token[0].to_bigint().to_string();

            let owner_result = match provider
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
                Ok(res) => res[0].to_hex_string(),
                Err(e) => {
                    eprintln!(
                        "Error fetching owner for token {} in slot {}: {}",
                        token_id, slot, e
                    );
                    continue;
                }
            };

            sleep(RATE_LIMIT).await; // Rate LIMIT

            let value_result = match provider
                .call(
                    FunctionCall {
                        contract_address,
                        entry_point_selector: selector!("value_of"),
                        calldata: token.clone(),
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await
            {
                Ok(res) => res[0].to_bigint().to_string(),
                Err(e) => {
                    eprintln!(
                        "Error fetching value for token {} in slot {}: {}",
                        token_id, slot, e
                    );
                    continue;
                }
            };

            sleep(RATE_LIMIT).await; //

            slot_map.insert(
                token_id,
                TokenInfo {
                    owner: owner_result,
                    value: value_result,
                },
            );
        }

        slot_tokens.insert(slot.to_string(), slot_map);
    }

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
        felt!("0x0516d0acb6341dcc567e85dc90c8f64e0c33d3daba0a310157d6bba0656c8769"),
        felt!("0x05a667adc04676fba78a29371561a0bf91dab25847d5dc4709a93a4cfb5ff293"),
    ];

    let mut results = HashMap::new();

    for address in project_addresses {
        let token_data = scan_slot_project(&provider, address).await?;
        results.insert(address.to_hex_string(), token_data);
    }

    let json_output = json!(results);
    println!("\nResults JSON:");
    println!("{}", serde_json::to_string_pretty(&json_output)?);

    std::fs::write(
        "output/v2.json",
        serde_json::to_string_pretty(&json_output)?,
    )?;

    let duration = start_time.elapsed();
    println!("\nScan completed in {:?}", duration);

    Ok(())
}
