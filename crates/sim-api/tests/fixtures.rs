//! The committed fixtures must equal what the code produces now. To refresh them:
//! `cargo run -p sim-api --example fixtures`.
use std::path::Path;

#[test]
fn committed_fixtures_are_current_and_deterministic() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures");
    let first = sim_api::fixtures::generate().unwrap();
    let second = sim_api::fixtures::generate().unwrap();
    assert_eq!(first, second, "fixtures differ between two runs");
    assert_eq!(first.len(), 4);
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
    assert_eq!(turn["accepted"].as_array().unwrap().len(), 2);
    assert_eq!(turn["rejected"][0]["reason"], "lever_unavailable");
    assert_eq!(turn["months"].as_array().unwrap().len(), 3);
    let snap = read("snapshot-turn1.json");
    assert_eq!(snap["buildings"]["fossilPlant"], 5);
    assert_eq!(read("levers.json").as_array().unwrap().len(), 19);
    assert_eq!(read("actions.json").as_array().unwrap().len(), 7);
}
