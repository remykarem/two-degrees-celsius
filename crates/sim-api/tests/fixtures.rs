//! The committed fixtures must equal what the code produces now. To refresh them:
//! `cargo run -p sim-api --example fixtures`.
use std::path::Path;

#[test]
fn committed_fixtures_are_current_and_deterministic() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures");
    let first = sim_api::fixtures::generate().unwrap();
    let second = sim_api::fixtures::generate().unwrap();
    assert_eq!(first, second, "fixtures differ between two runs");
    assert_eq!(first.len(), 18);
    for (name, body) in first {
        let committed = std::fs::read_to_string(dir.join(&name))
            .unwrap_or_else(|e| panic!("{name}: {e} (run the fixtures example)"));
        assert!(
            committed == body,
            "docs/fixtures/{name} is stale: run `cargo run -p sim-api --example fixtures`"
        );
    }
}

#[test]
fn fixtures_have_the_documented_content() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures");
    let read = |n: &str| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(n)).unwrap()).unwrap()
    };
    let turn = read("turn-result.json");
    assert_eq!(turn["turn"], 1);
    // A policy, a road and a plant are queued, each with its tile; the Market refuses the cap.
    let accepted = turn["accepted"].as_array().unwrap();
    assert_eq!(accepted.len(), 3);
    assert_eq!(accepted[1]["command"]["type"], "place");
    assert_eq!(accepted[1]["command"]["kind"], "road");
    assert_eq!(accepted[1]["command"]["tile"]["x"], 6);
    assert_eq!(accepted[2]["command"]["kind"], "renewable_plant");
    assert_eq!(turn["rejected"][0]["reason"], "lever_unavailable");
    assert_eq!(turn["months"].as_array().unwrap().len(), 3);
    assert!(!turn["months"][0]["flows"]["purchases"]
        .as_array()
        .unwrap()
        .is_empty());
    let snap = read("snapshot-turn1.json");
    assert_eq!(snap["schemaVersion"], 6);
    // Contract v1.4: finance and energy are on every snapshot.
    assert_eq!(snap["energy"]["unit"], "MWh");
    assert!(snap["finance"]["government"]["revenue"]["wageTax"].is_number());
    // The new fixtures: ledgers reconcile; finance and energy are the snapshot parts.
    for n in ["ledger-household.json", "ledger-government.json"] {
        let l = read(n);
        assert_eq!(l["reconciles"], true, "{n}");
        assert!(!l["entries"].as_array().unwrap().is_empty(), "{n}");
    }
    assert_eq!(read("finance.json")["government"]["basis"], "month");
    assert_eq!(read("energy.json")["unit"], "MWh");
    assert_eq!(snap["people"].as_array().unwrap().len(), 22);
    assert_eq!(snap["households"].as_array().unwrap().len(), 9);
    assert_eq!(snap["homes"].as_array().unwrap().len(), 8);
    // Contract v1.9: the estate, its tag and wealth band, the pilot's employer.
    assert_eq!(snap["estate"]["household"], 8);
    assert_eq!(snap["households"][8]["tag"], "Estate owner");
    assert_eq!(snap["households"][8]["wealthBand"]["id"], "ultra_rich");
    assert_eq!(snap["people"][18]["employer"]["household"], 8);
    assert_eq!(snap["map"]["tiles"].as_array().unwrap().len(), 144);
    assert_eq!(read("levers.json").as_array().unwrap().len(), 20);
    assert_eq!(read("actions.json").as_array().unwrap().len(), 11);
    let tiles = read("tiles.json");
    for k in [
        "public_housing",
        "fossil_plant",
        "renewable_plant",
        "road",
        "fire_station",
    ] {
        assert!(!tiles[k].as_array().unwrap().is_empty(), "{k}");
    }
    // Contract v1.2: the weather block, and a weather turn with a burning tile and a drought.
    assert_eq!(snap["weather"]["stress"], 0.0);
    assert_eq!(snap["weather"]["forecast"]["fireRisk"], "low");
    assert!(snap["nature"]["farmHealth"].as_f64().unwrap() > 0.0);
    let w = read("turn-result-weather.json");
    let weather = &w["snapshot"]["weather"];
    assert!(!weather["fires"].as_array().unwrap().is_empty());
    assert!(weather["stress"].as_f64().unwrap() > 0.0);
    let states: Vec<&str> = w["snapshot"]["map"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["state"].as_str())
        .collect();
    assert!(states.contains(&"burning") && states.contains(&"standing"));
    let kinds: Vec<&str> = w["months"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| m["events"].as_array().unwrap())
        .filter_map(|e| e["type"].as_str())
        .collect();
    assert!(kinds
        .iter()
        .any(|k| k.starts_with("fire_") || k.starts_with("drought_")));
}
