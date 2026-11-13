//! Decision points on the wire (contract v2.7): the registry joined with `content/decisions.json`.
use sim_api::decisions::DECISIONS_JSON;
use sim_api::Api;
use sim_model::decisions::DECISIONS;
use sim_types::{DecisionId, QuantityId};
use std::collections::BTreeSet;

fn params_json() -> serde_json::Value {
    serde_json::from_str(sim_api::content::PARAMS).unwrap()
}

fn path_resolves(pj: &serde_json::Value, path: &str) -> bool {
    let mut cur = pj;
    for k in path.split('.') {
        match cur.get(k) {
            Some(v) => cur = v,
            None => return false,
        }
    }
    cur.is_number()
}

#[test]
fn every_listed_param_path_resolves_in_params() {
    let pj = params_json();
    for d in DECISIONS {
        for path in d.params {
            assert!(
                path_resolves(&pj, path),
                "{:?}: no numeric param {path}",
                d.id
            );
        }
    }
}

#[test]
fn catalog_serves_every_decision_and_quantity_with_text() {
    let c = Api::init(1).unwrap().catalog().unwrap();
    assert_eq!(c.decisions.len(), DecisionId::ALL.len());
    assert_eq!(c.quantities.len(), QuantityId::ALL.len());
    let ids: BTreeSet<_> = c.decisions.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids.len(), c.decisions.len(), "decision ids unique");
    for d in &c.decisions {
        assert!(!d.title.is_empty() && !d.decides.is_empty() && !d.rule.text.is_empty());
        assert!(!d.inputs.is_empty() && !d.outputs.is_empty());
        for q in d.inputs.iter().chain(&d.outputs) {
            assert!(!q.label.is_empty() && !q.unit.is_empty(), "{}", q.id);
            assert!(c.quantities.iter().any(|x| x.id == q.id));
        }
        for p in &d.params {
            assert!(!p.label.is_empty() && !p.unit.is_empty(), "{}", p.path);
        }
        let kind = match d.owner.as_str() {
            "firm" => "firms",
            "household" => "households",
            "bank" => "bank",
            o => panic!("owner {o} has no agent kind in the test"),
        };
        assert_eq!(d.agent_kind, kind);
    }
    assert!(c
        .quantities
        .iter()
        .all(|q| !q.label.is_empty() && !q.summary.is_empty()));
}

/// "Who decides" for the firm's labour decisions is generated: the lines are on the wire, and the
/// hand-written `content/assumptions.json` no longer holds them (nor any of their parameters).
#[test]
fn agents_who_decides_is_generated_and_not_hand_written() {
    let api = Api::init(1).unwrap();
    let a = api.agents().unwrap();
    let c = api.catalog().unwrap();
    for d in &c.decisions {
        let firms = a.kinds.iter().find(|k| k.id == d.agent_kind).unwrap();
        assert!(
            firms.decides.contains(&d.decides),
            "{} missing from decides",
            d.id
        );
        for p in &d.params {
            assert!(
                firms.params.iter().any(|x| x.path == p.path),
                "{} missing from params",
                p.path
            );
        }
        for q in &d.inputs {
            assert!(
                firms.observes.iter().any(|o| o.starts_with(&q.label)),
                "{}",
                q.id
            );
        }
    }
    // The authored file: nothing generated is hand-written, for any kind that has decisions.
    let raw: serde_json::Value = serde_json::from_str(sim_api::assumptions::CATALOG).unwrap();
    for kind in raw["agents"]["kinds"].as_array().unwrap() {
        let id = kind["id"].as_str().unwrap();
        let strs = |key: &str| -> Vec<String> {
            kind[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap().to_string())
                .collect()
        };
        let hand_params: Vec<String> = kind["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["path"].as_str().unwrap().to_string())
            .collect();
        let mine: Vec<_> = c.decisions.iter().filter(|d| d.agent_kind == id).collect();
        if mine.is_empty() {
            continue;
        }
        for d in &mine {
            for p in &d.params {
                assert!(!hand_params.contains(&p.path), "{} is hand-written", p.path);
            }
            assert!(!strs("decides").contains(&d.decides));
        }
        let served = a.kinds.iter().find(|k| k.id == id).unwrap();
        assert_eq!(served.decides.len(), mine.len() + strs("decides").len());
    }
}

#[test]
fn the_file_has_no_stray_entries() {
    // Unknown ids in the file are rejected at resolve time (see decisions::resolve); here we
    // check the shipped file parses and names each decision once.
    let v: serde_json::Value = serde_json::from_str(DECISIONS_JSON).unwrap();
    let ids: Vec<&str> = v["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    let set: BTreeSet<_> = ids.iter().collect();
    assert_eq!(set.len(), ids.len());
}
