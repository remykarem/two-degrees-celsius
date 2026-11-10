//! The committed fixtures must equal what the code produces now. To refresh them:
//! `cargo run -p sim-api --example fixtures`.
use std::path::Path;

#[test]
fn committed_fixtures_are_current_and_deterministic() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures");
    let first = sim_api::fixtures::generate().unwrap();
    let second = sim_api::fixtures::generate().unwrap();
    assert_eq!(first, second, "fixtures differ between two runs");
    assert_eq!(first.len(), 7);
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
    assert_eq!(accepted[1]["command"]["tile"]["x"], 4);
    assert_eq!(accepted[2]["command"]["kind"], "renewable_plant");
    assert_eq!(turn["rejected"][0]["reason"], "lever_unavailable");
    assert_eq!(turn["months"].as_array().unwrap().len(), 3);
    assert!(!turn["months"][0]["flows"]["purchases"]
        .as_array()
        .unwrap()
        .is_empty());
    let snap = read("snapshot-turn1.json");
    assert_eq!(snap["schemaVersion"], 2);
    assert_eq!(snap["people"].as_array().unwrap().len(), 20);
    assert_eq!(snap["households"].as_array().unwrap().len(), 8);
    assert_eq!(snap["homes"].as_array().unwrap().len(), 7);
    assert_eq!(snap["map"]["tiles"].as_array().unwrap().len(), 144);
    assert_eq!(read("levers.json").as_array().unwrap().len(), 19);
    assert_eq!(read("actions.json").as_array().unwrap().len(), 9);
    let tiles = read("tiles.json");
    for k in ["public_housing", "fossil_plant", "renewable_plant", "road"] {
        assert!(!tiles[k].as_array().unwrap().is_empty(), "{k}");
    }
}
