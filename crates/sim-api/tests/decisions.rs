//! Decision points on the wire (contract v2.7): the registry joined with `content/decisions.json`.
use sim_api::decisions::DECISIONS_JSON;
use sim_api::Api;
use sim_model::decisions::{DECISIONS, MECHANISMS};
use sim_types::{DecisionId, MechanismId, QuantityId};
use std::collections::BTreeSet;

/// The loaded `Params` as JSON: includes `systems.*`, which come from the system files.
fn params_json() -> serde_json::Value {
    let p = sim_params::Params::from_content(&sim_api::content::default_content()).unwrap();
    serde_json::to_value(p).unwrap()
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
fn every_mechanism_param_path_resolves_in_params() {
    let pj = params_json();
    for m in MECHANISMS {
        for path in m.params {
            assert!(
                path_resolves(&pj, path),
                "{:?}: no numeric param {path}",
                m.id
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
            "government" => "government",
            "estate" => "estate",
            "nature" => "nature",
            "weather" => "weather",
            o => panic!("owner {o} has no agent kind in the test"),
        };
        assert_eq!(d.agent_kind, kind);
    }
    assert!(c
        .quantities
        .iter()
        .all(|q| !q.label.is_empty() && !q.summary.is_empty()));
}

/// "Who decides" is generated for every kind and none of it is hand-written: the lines are on the
/// wire, each registered decision appears in exactly one kind, and `content/assumptions.json`
/// holds no decides, observes, params or rule next to registered decisions. The only authored text
/// is the documented exceptions: `kind_notes` in `content/decisions.json`, and the mechanism lines
/// (observes, params, rule) of the two kinds with no decision (lobbyists, weather).
#[test]
fn agents_who_decides_is_generated_and_not_hand_written() {
    let api = Api::init(1).unwrap();
    let a = api.agents().unwrap();
    let c = api.catalog().unwrap();
    for d in &c.decisions {
        let owners: Vec<_> = a.kinds.iter().filter(|k| k.id == d.agent_kind).collect();
        assert_eq!(owners.len(), 1, "{} is in exactly one kind", d.id);
        let kind = owners[0];
        assert_eq!(
            kind.decides.iter().filter(|x| **x == d.decides).count(),
            1,
            "{} missing or repeated in decides",
            d.id
        );
        assert!(kind.rule.contains(&d.rule.text), "{} rule", d.id);
        for p in &d.params {
            assert!(kind.params.iter().any(|x| x.path == p.path), "{}", p.path);
        }
        for q in &d.inputs {
            assert!(
                kind.observes.iter().any(|o| o.starts_with(&q.label)),
                "{}",
                q.id
            );
        }
    }
    let raw: serde_json::Value = serde_json::from_str(sim_api::assumptions::CATALOG).unwrap();
    let mut without = Vec::new();
    for kind in raw["agents"]["kinds"].as_array().unwrap() {
        let id = kind["id"].as_str().unwrap();
        let empty = |key: &str| kind[key].as_array().is_none_or(|v| v.is_empty());
        // Nothing is hand-written under `decides` for any kind.
        assert!(empty("decides"), "{id}: hand-written decides");
        let n = c.decisions.iter().filter(|d| d.agent_kind == id).count();
        let served = a.kinds.iter().find(|k| k.id == id).unwrap();
        assert_eq!(served.decides.len(), n, "{id}");
        if n > 0 {
            assert!(
                empty("observes") && empty("params"),
                "{id}: hand-written lines"
            );
            assert!(
                kind["rule"].as_str().is_none_or(str::is_empty),
                "{id}: hand-written rule"
            );
            assert!(!served.rule.is_empty() && !served.observes.is_empty());
        } else {
            assert!(
                !served.notes.is_empty(),
                "{id}: no decision and no documented note"
            );
            without.push(id.to_string());
        }
    }
    assert_eq!(
        without,
        ["lobbyists", "weather"],
        "the documented no-decision kinds"
    );
    // Every decision is served: the kinds' decides lines add up to the registry.
    let total: usize = a.kinds.iter().map(|k| k.decides.len()).sum();
    assert_eq!(total, DecisionId::ALL.len());
}

#[test]
fn every_quantity_has_label_unit_owner_and_text() {
    let c = Api::init(1).unwrap().catalog().unwrap();
    let owners: Vec<&str> = sim_types::OwnerKind::ALL
        .iter()
        .map(|o| o.as_str())
        .collect();
    for q in &c.quantities {
        assert!(!q.label.trim().is_empty(), "{} label", q.id);
        assert!(!q.unit.trim().is_empty(), "{} unit", q.id);
        assert!(!q.summary.trim().is_empty(), "{} text", q.id);
        assert!(owners.contains(&q.owner.as_str()), "{} owner", q.id);
    }
}

/// The causal graph on the wire is the registry, joined with labels.
#[test]
fn causal_graph_matches_the_registry() {
    let api = Api::init(1).unwrap();
    let c = api.catalog().unwrap();
    let g = &c.causal;
    let ids: BTreeSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids.len(), g.nodes.len(), "node ids unique");
    assert_eq!(
        g.nodes.len(),
        QuantityId::ALL.len() + DECISIONS.len() + MECHANISMS.len()
    );
    // Order: quantities in vocabulary order, then decisions, then mechanisms in registry order.
    let want: Vec<String> = QuantityId::ALL
        .iter()
        .map(|q| format!("quantity:{}", q.as_str()))
        .chain(
            DECISIONS
                .iter()
                .map(|d| format!("decision:{}", d.id.as_str())),
        )
        .chain(
            MECHANISMS
                .iter()
                .map(|m| format!("mechanism:{}", m.id.as_str())),
        )
        .collect();
    assert_eq!(
        g.nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>(),
        want
    );
    for n in &g.nodes {
        assert!(!n.label.is_empty());
        assert!(g.roles.iter().any(|r| r.id == n.role));
        for u in &n.upstream {
            assert!(ids.contains(u.as_str()), "dangling upstream {u}");
        }
    }
    for e in &g.edges {
        assert!(
            ids.contains(e.from.as_str()) && ids.contains(e.to.as_str()),
            "dangling edge"
        );
        if let Some(v) = &e.via {
            assert!(
                (v.starts_with("decision:") || v.starts_with("mechanism:"))
                    && ids.contains(v.as_str())
            );
        }
    }
    // Every edge comes from a spec, and every spec input and output has its reads / writes edge.
    for s in DECISIONS {
        let d = format!("decision:{}", s.id.as_str());
        for q in s.inputs {
            let from = format!("quantity:{}", q.as_str());
            assert!(g
                .edges
                .iter()
                .any(|e| e.kind == "reads" && e.from == from && e.to == d));
        }
        for q in s.outputs {
            let to = format!("quantity:{}", q.as_str());
            assert!(g
                .edges
                .iter()
                .any(|e| e.kind == "writes" && e.from == d && e.to == to));
        }
    }
    for s in MECHANISMS {
        let m = format!("mechanism:{}", s.id.as_str());
        for q in s.inputs {
            let from = format!("quantity:{}", q.as_str());
            assert!(g
                .edges
                .iter()
                .any(|e| e.kind == "reads" && e.from == from && e.to == m));
        }
        for q in s.outputs {
            let to = format!("quantity:{}", q.as_str());
            assert!(g
                .edges
                .iter()
                .any(|e| e.kind == "writes" && e.from == m && e.to == to));
        }
    }
    for e in g.edges.iter().filter(|e| e.kind == "reads") {
        let inputs: Vec<&str> = if let Some(id) = e.to.strip_prefix("decision:") {
            DECISIONS
                .iter()
                .find(|s| s.id.as_str() == id)
                .unwrap()
                .inputs
                .iter()
                .map(|q| q.as_str())
                .collect()
        } else {
            let id = e.to.strip_prefix("mechanism:").unwrap();
            MECHANISMS
                .iter()
                .find(|s| s.id.as_str() == id)
                .unwrap()
                .inputs
                .iter()
                .map(|q| q.as_str())
                .collect()
        };
        assert!(inputs.contains(&e.from.strip_prefix("quantity:").unwrap()));
    }
    // Source and sink markers.
    let role = |q: &str| {
        g.nodes
            .iter()
            .find(|n| n.id == format!("quantity:{q}"))
            .unwrap()
    };
    for q in ["markup", "carbon_price", "treasury_cash"] {
        assert_eq!(role(q).role, "source", "{q}");
        assert!(role(q).upstream.is_empty());
    }
    assert_eq!(role("legitimacy").role, "sink");
    assert!(role("legitimacy").self_loop);
    assert!(g
        .edges
        .iter()
        .any(|e| e.kind == "drives" && e.self_loop && e.from == "quantity:legitimacy"));
    assert_eq!(role("wanted_workers").role, "derived");
    assert!(!role("wanted_workers").upstream.is_empty());
    // The same name for a quantity and a decision does not collide.
    assert!(ids.contains("quantity:bank_dividend") && ids.contains("decision:bank_dividend"));
    // Deterministic.
    assert_eq!(api.catalog().unwrap().causal, c.causal);
    // Counts the docs quote (change them with the registry).
    let sources = g.nodes.iter().filter(|n| n.role == "source").count();
    let sinks = g.nodes.iter().filter(|n| n.role == "sink").count();
    println!(
        "decisions {} mechanisms {} quantities {} edges {} sources {sources} sinks {sinks}",
        DECISIONS.len(),
        MECHANISMS.len(),
        QuantityId::ALL.len(),
        g.edges.len()
    );
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

/// Stage 1: mechanisms are served next to decisions, each in exactly one owner kind that the
/// Agents explorer lists, with its words from content and its reads and writes from the spec.
#[test]
fn mechanisms_are_served_with_text_and_one_agents_kind() {
    let api = Api::init(1).unwrap();
    let c = api.catalog().unwrap();
    let a = api.agents().unwrap();
    assert_eq!(c.mechanisms.len(), MechanismId::ALL.len());
    let ids: BTreeSet<_> = c.mechanisms.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids.len(), c.mechanisms.len(), "mechanism ids unique");
    for m in &c.mechanisms {
        assert!(!m.title.is_empty() && !m.explains.is_empty() && !m.rule.text.is_empty());
        assert!(!m.inputs.is_empty() && !m.outputs.is_empty());
        // Exactly one Agents kind lists it, and that kind exists.
        assert_eq!(a.kinds.iter().filter(|k| k.id == m.agent_kind).count(), 1);
        // No clash with a decision of the same name.
        assert!(!c.decisions.iter().any(|d| d.id == m.id));
        for q in m.inputs.iter().chain(&m.outputs) {
            assert!(c.quantities.iter().any(|x| x.id == q.id), "{}", q.id);
        }
        for p in &m.params {
            assert!(!p.label.is_empty() && !p.unit.is_empty(), "{}", p.path);
        }
        assert!(
            c.causal
                .nodes
                .iter()
                .any(|n| n.id == format!("mechanism:{}", m.id) && n.kind == "mechanism"),
            "{} is not a graph node",
            m.id
        );
    }
    // Every param label in content is used by something (no dead entries).
    let raw: serde_json::Value = serde_json::from_str(DECISIONS_JSON).unwrap();
    for p in raw["params"].as_array().unwrap() {
        let path = p["path"].as_str().unwrap();
        let used = DECISIONS.iter().any(|d| d.params.contains(&path))
            || MECHANISMS.iter().any(|m| m.params.contains(&path));
        assert!(
            used,
            "params entry {path} is used by no decision or mechanism"
        );
    }
    // Every quantity is read or written by a decision or a mechanism, and every mechanism
    // output is a quantity of the vocabulary: the graph has no dangling id.
    let g = &c.causal;
    let node_ids: BTreeSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    for e in &g.edges {
        assert!(node_ids.contains(e.from.as_str()) && node_ids.contains(e.to.as_str()));
    }
}

/// The DP-13 chain on the wire: hiring's upstream causes reach emissions through the weather.
#[test]
fn the_emissions_to_hiring_chain_is_connected_on_the_wire() {
    let c = Api::init(1).unwrap().catalog().unwrap();
    let node = |id: &str| {
        c.causal
            .nodes
            .iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("no node {id}"))
    };
    // The one-step upstream of the weather factor is the three weather mechanisms' inputs.
    let wf = node("quantity:weather_factor");
    assert!(wf.upstream.contains(&"quantity:drought".to_string()));
    assert!(wf
        .upstream
        .contains(&"quantity:cumulative_emissions".to_string()));
    assert_eq!(wf.role, "derived");
    // Following `drives` edges from gross emissions reaches the weather factor and then output.
    let mut frontier = vec!["quantity:gross_emissions".to_string()];
    let mut seen = BTreeSet::new();
    while let Some(n) = frontier.pop() {
        for e in c
            .causal
            .edges
            .iter()
            .filter(|e| e.kind == "drives" && e.from == n)
        {
            if seen.insert(e.to.clone()) {
                frontier.push(e.to.clone());
            }
        }
    }
    for want in [
        "climate_stress",
        "drought",
        "weather_factor",
        "output",
        "wanted_workers",
    ] {
        assert!(
            seen.contains(&format!("quantity:{want}")),
            "{want} not reached"
        );
    }
}
