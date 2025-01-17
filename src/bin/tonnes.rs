use serde_json::{from_reader, json, Value};
use std::{collections::HashMap, fs::File, vec};

fn main() {
    let banegas = vec!["BanegasFarmv1.json".into(), "BanegasFarmv2.json".into()];
    let delicias = vec!["LasDeliciasv1.json".into(), "LasDeliciasv2.json".into()];
    let manjarisoa: Vec<String> = vec![
        "ManjarisoBronzev1.json".into(),
        "ManjarisoSilverv1.json".into(),
        "ManjarisoGoldv1.json".into(),
        "Manjarisoav2.json".into(),
    ];
    let karathuru = vec!["Karathuruv2.json".into()];

    // remove farming contracts from snapshots

    compute_amount("Banegas".into(), banegas, 1573_000_000_000, 17600_000_000);
    compute_amount("Delicias".into(), delicias, 1573_000_000_000, 17600_000_000);
    compute_amount(
        "Manjarisoa".into(),
        manjarisoa,
        1573_000_000_000,
        17600_000_000,
    );
    compute_amount(
        "Karathuru".into(),
        karathuru,
        1573_000_000_000,
        17600_000_000,
    );
}

fn compute_amount(project: String, snapshots: Vec<String>, total_tonnes: u128, total_value: u128) {
    let mut amount_map = HashMap::new();
    let mut total_value_handled = 0;
    let mut total_tonnes_handled = 0;
    for snapshot in snapshots {
        let file =
            File::open("snapshot/".to_owned() + &snapshot).expect("file should open read only");
        let data: Value = from_reader(file).expect("file should be proper JSON");

        for (owner, token_info) in data.as_object().unwrap() {
            let value: u128 = token_info["value"].as_u64().unwrap().into();
            let value_offset: u128 = token_info["value_offset"].as_u64().unwrap().into();
            let value_yielder: u128 = token_info["value_yielder"].as_u64().unwrap().into();

            let current = amount_map.entry(owner.to_owned()).or_insert(0_u128);
            *current += value + value_offset + value_yielder;

            let amount: u128 = (*current * total_tonnes) / total_value;
            total_tonnes_handled += amount;
            total_value_handled += *current;

            amount_map.insert(owner.to_owned(), amount);
        }
    }

    let json_output = json!(amount_map);

    std::fs::write(
        "tonnes/".to_owned() + project.as_str() + ".json",
        serde_json::to_string_pretty(&json_output).unwrap(),
    )
    .unwrap();

    println!(
        "Total tonnes handled for {} is ${} out of {} tonnes",
        project,
        total_tonnes_handled / 1_000_000_000,
        total_tonnes / 1_000_000_000
    );
    println!(
        "Total value handled for {} is ${} out of ${}",
        project,
        total_value_handled / 1_000_000,
        total_value / 1_000_000
    );
}
