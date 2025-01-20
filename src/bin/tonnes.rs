use serde_json::{from_reader, json, Value};
use starknet::{core::types::Felt, macros::felt};
use std::{collections::HashMap, fs::File, vec};

fn main() {
    let banegas = vec!["BanegasFarmv1.json".into(), "BanegasFarmv2.json".into()];
    let delicias = vec![
        "LasDeliciasv1.json".into(),
        "LasDeliciasv2.json".into(),
        "LasDelicias_old.json".into(),
    ];
    let manjarisoa: Vec<String> = vec![
        "ManjarisoBronzev1.json".into(),
        "ManjarisoSilverv1.json".into(),
        //"ManjarisoGoldv1.json".into(),
        "Manjarisoav2.json".into(),
    ];
    let karathuru = vec![
        "Karathuruv2.json".into(),
        "Karathuru_undeployed.json".into(),
    ];

    // remove farming contracts from snapshots
    let banegas_farming = vec![
        felt!("0x03d25473be5a6316f351e8f964d0c303357c006f7107779f648d9879b7c6d58a"),
        felt!("0x0324b531f731100b494e2f978a26b20b5870585dd96d9f1166b43a28ebbb8aba"),
    ];
    let delicias_farming = vec![
        felt!("0x00426d4e86913759bcc49b7f992b1fe62e6571e8f8089c23d95fea815dbad471"),
        felt!("0x022f40128af9798a0b734874fd993bbab6cf75845f26f844cb151b7041132c6d"),
        felt!("0x030172515f274abe1a5938108f083fc7da90481933e34458f0352eebea32560d"),
        felt!("0x04774636985a87ae06f9eab5d7d25fb75ef8be0e04fe3444b0277bad17465e51"),
    ];
    let manjarisoa_farming = vec![
        felt!("0x03afe61732ed9b226309775ac4705129319729d3bee81da5632146ffd72652ae"),
        felt!("0x04258037980fcc15083cde324abe1861ac00d4d48b7d60d76b5efd6f57e59e73"),
    ];
    let karathuru_farming = vec![];

    compute_amount(
        "Banegas".into(),
        banegas,
        banegas_farming,
        1573_000_000_000,
        17600_000_000,
    );
    compute_amount(
        "Delicias".into(),
        delicias,
        delicias_farming,
        3603_000_000_000,
        39600_000_000,
    );
    compute_amount(
        "Manjarisoa".into(),
        manjarisoa,
        manjarisoa_farming,
        8000_000_000_000,
        121099_000_000,
    );
    compute_amount(
        "Karathuru".into(),
        karathuru,
        karathuru_farming,
        70589_000_000_000,
        367909_870_000,
    );
}

fn compute_amount(
    project: String,
    snapshots: Vec<String>,
    blacklist: Vec<Felt>,
    total_tonnes: u128,
    total_value: u128,
) {
    let mut amount_map = HashMap::new();
    for snapshot in snapshots {
        let file =
            File::open("snapshot/".to_owned() + &snapshot).expect("file should open read only");
        let data: Value = from_reader(file).expect("file should be proper JSON");

        for (owner, token_info) in data.as_object().unwrap() {
            if blacklist.contains(&Felt::from_hex(owner).unwrap()) {
                continue;
            };
            let value: u128 = token_info["value"].as_u64().unwrap().into();
            let value_offset: u128 = token_info["value_offset"].as_u64().unwrap().into();
            let value_yielder: u128 = token_info["value_yielder"].as_u64().unwrap().into();

            let current = *amount_map.entry(owner.to_owned()).or_insert(0_u128);
            let additional = if current == 0 {
                value + value_offset + value_yielder
            } else {
                value
            };

            let amount: u128 = current + (additional * total_tonnes) / total_value;
            amount_map.insert(owner.to_owned(), amount);
        }
    }

    let mut total_value_handled = 0;
    let mut total_tonnes_handled = 0;
    for amount in amount_map.values() {
        total_value_handled += amount * total_value / total_tonnes;
        total_tonnes_handled += amount
    }

    let json_output = json!(amount_map);

    std::fs::write(
        "tonnes/".to_owned() + project.as_str() + ".json",
        serde_json::to_string_pretty(&json_output).unwrap(),
    )
    .unwrap();

    println!(
        "Total tonnes handled for {} is {} out of {} tonnes",
        project,
        total_tonnes_handled / 1,
        total_tonnes / 1
    );
    println!(
        "Total value handled for {} is ${} out of ${}",
        project,
        total_value_handled / 1,
        total_value / 1
    );
}
