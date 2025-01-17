use serde_json::{from_reader, json, Value};
use std::{collections::HashMap, fs::File};

#[derive(Debug, Default, serde::Serialize)]
struct ValueInfo {
    value: u64,
    value_offset: u64,
    value_yielder: u64,
}

fn main() {
    total("v2");
    total("v1");
}

fn total(version: &str) {
    println!("\nCalculating total value for version {}", version);
    let file =
        File::open("output/s".to_owned() + version + ".json").expect("file should open read only");
    let json: Value = from_reader(file).expect("file should be proper JSON");

    for project in json.as_object().unwrap() {
        for (_, tokens) in project.1.as_object().unwrap() {
            let mut owners_infos = HashMap::new();
            for (_, token_info) in tokens.as_object().unwrap() {
                let owner = token_info["owner"].as_str().unwrap();

                let new_value = token_info["value"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();

                let new_value_offset = token_info["value_in_offsetter"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();

                let new_value_yielder = token_info["value_in_yielder"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();

                let old = owners_infos.entry(owner).or_insert(ValueInfo::default());
                old.value += new_value;
                old.value_offset = new_value_offset;
                old.value_yielder = new_value_yielder;
            }

            let json_output = json!(owners_infos);
            //println!("\nResults JSON:");
            //println!("{}", serde_json::to_string_pretty(&json_output)?);

            std::fs::write(
                "snapshot/".to_owned() + project.0 + version + ".json",
                serde_json::to_string_pretty(&json_output).unwrap(),
            )
            .unwrap();

            let mut total_value = 0;
            let mut total_value_offset = 0;
            let mut total_value_yield = 0;
            for (_, value) in owners_infos.iter() {
                total_value += value.value;
                total_value_offset += value.value_offset;
                total_value_yield += value.value_yielder;
            }
            println!(
                "Total value for project {}: ${}",
                project.0,
                total_value / 1_000_000
            );
            println!(
                "Total value in offsetter for project {}: ${}",
                project.0,
                total_value_offset / 1_000_000
            );

            println!(
                "Total value in yielder for project {}: ${}",
                project.0,
                total_value_yield / 1_000_000
            );

            println!(
                "total - (offset + vielder) = ${}",
                (total_value - (total_value_offset + total_value_yield)) / 1_000_000
            );
        }
    }
}
