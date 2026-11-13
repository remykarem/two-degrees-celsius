//! Decision points on the wire (contract v2.7): the registry joined with `content/decisions.json`.
use sim_api::decisions::DECISIONS_JSON;
use sim_api::Api;
use sim_model::decisions::{DECISIONS, MECHANISMS};
use sim_types::{DecisionId, MechanismId, QuantityId};
use std::collections::{BTreeMap, BTreeSet};

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

/// Contract v2.11: every `drives` edge carries the declared effect, and its sentence is generated
/// from the sign and lag, so it cannot disagree with them.
#[test]
fn every_drives_edge_has_a_sign_a_lag_and_a_sentence_that_agrees() {
    let api = Api::init(1).unwrap();
    let c = api.catalog().unwrap();
    let g = &c.causal;
    let mut by_sign = std::collections::BTreeMap::new();
    for e in &g.edges {
        if e.kind != "drives" {
            assert!(
                e.effect.is_none(),
                "{} -> {} is not a drives edge",
                e.from,
                e.to
            );
            continue;
        }
        let fx = e.effect.as_ref().expect("every drives edge has an effect");
        *by_sign.entry(fx.sign.as_str()).or_insert(0) += 1;
        let from = g.nodes.iter().find(|n| n.id == e.from).unwrap();
        let to = g.nodes.iter().find(|n| n.id == e.to).unwrap();
        let lower = |s: &str| {
            let mut cs = s.chars();
            match cs.next() {
                Some(a) if cs.next().is_some_and(|b| !b.is_uppercase()) => {
                    a.to_lowercase().collect::<String>() + &s[a.len_utf8()..]
                }
                _ => s.to_string(),
            }
        };
        let s = fx.sentence.to_lowercase();
        match fx.sign.as_str() {
            "up" => assert!(
                s.contains(&format!("{} goes up", lower(&to.label).to_lowercase())),
                "{}",
                fx.sentence
            ),
            "down" => assert!(
                s.contains(&format!("{} goes down", lower(&to.label).to_lowercase())),
                "{}",
                fx.sentence
            ),
            "varies" => assert!(
                s.contains("up or down") && fx.reason.is_some(),
                "{}",
                fx.sentence
            ),
            "none" => assert!(
                s.contains("does not directly change") && fx.reason.is_some(),
                "{}",
                fx.sentence
            ),
            other => panic!("unknown sign {other}"),
        }
        assert!(
            s.contains(&lower(&from.label).to_lowercase()),
            "{}",
            fx.sentence
        );
        if fx.sign != "none" {
            assert!(s.contains(&fx.lag_label.to_lowercase()), "{}", fx.sentence);
        }
        assert!(matches!(
            fx.basis.as_str(),
            "perturbed" | "system" | "declared_unverified"
        ));
    }
    // The wire carries exactly the registry's effects.
    let drives = g.edges.iter().filter(|e| e.kind == "drives").count();
    let declared: usize = DECISIONS.iter().map(|d| d.effects.len()).sum::<usize>()
        + MECHANISMS.iter().map(|m| m.effects.len()).sum::<usize>();
    assert_eq!(drives, declared);
    assert!(
        by_sign["up"] > 0 && by_sign["down"] > 0 && by_sign["varies"] > 0 && by_sign["none"] > 0
    );
    // Lags: a stock reaches itself next month; a lever arrives at once.
    let lag_of = |via: &str, from: &str, to: &str| {
        g.edges
            .iter()
            .find(|e| {
                e.kind == "drives" && e.via.as_deref() == Some(via) && e.from == from && e.to == to
            })
            .and_then(|e| e.effect.as_ref())
            .map(|f| f.lag_months)
    };
    assert_eq!(
        lag_of(
            "mechanism:capital_accumulation",
            "quantity:capital",
            "quantity:capital"
        ),
        Some(1)
    );
    assert_eq!(
        lag_of(
            "mechanism:tax_collection",
            "quantity:wage_tax_rate",
            "quantity:treasury_taxes"
        ),
        Some(0)
    );
}

/// A hand-written reason names a real signed effect (a typo would fail at load).
#[test]
fn because_overrides_are_used_in_sentences() {
    let api = Api::init(1).unwrap();
    let c = api.catalog().unwrap();
    let e = c
        .causal
        .edges
        .iter()
        .find(|e| {
            e.via.as_deref() == Some("mechanism:food_cap") && e.from == "quantity:commons_harvest"
        })
        .unwrap();
    let fx = e.effect.as_ref().unwrap();
    assert_eq!(fx.sign, "down");
    assert!(
        fx.sentence.contains("commons run out sooner"),
        "{}",
        fx.sentence
    );
}

/// Contract v2.12 (causality graph stage 3): the feedback loops on the wire.
#[test]
fn loops_are_served_with_sentences_that_agree_with_their_kind() {
    let c = Api::init(1).unwrap().catalog().unwrap();
    let g = &c.causal;
    assert_eq!(g.loops.len(), 203);
    assert_eq!(g.loop_max_len, 8);
    let kinds: Vec<&str> = g.loop_kinds.iter().map(|k| k.id.as_str()).collect();
    assert_eq!(kinds, vec!["R", "B", "depends"]);
    let node_ids: BTreeSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut ids = BTreeSet::new();
    for l in &g.loops {
        assert!(ids.insert(l.id.as_str()), "duplicate loop id {}", l.id);
        assert!(kinds.contains(&l.kind.as_str()));
        assert_eq!(l.members.len(), l.edges.len());
        assert!(l.delay_months >= 1);
        assert_eq!(
            l.delay_months,
            l.edges.iter().map(|e| e.lag_months).sum::<u32>()
        );
        assert!(l.sentence.starts_with("This is a"), "{}", l.sentence);
        assert!(l.sentence.contains(": when "), "{}", l.sentence);
        // The kind in words matches the id, and the last clause says how the loop closes.
        let phrase = match l.kind.as_str() {
            "R" => "reinforcing",
            "B" => "balancing",
            _ => "mixed",
        };
        assert!(l.sentence.contains(phrase), "{}", l.sentence);
        let minus = l.edges.iter().filter(|e| e.sign == "down").count();
        let varies = l.edges.iter().any(|e| e.sign == "varies");
        match l.kind.as_str() {
            "depends" => assert!(varies),
            "B" => assert!(!varies && minus % 2 == 1 && l.sentence.contains("again")),
            _ => assert!(!varies && minus % 2 == 0),
        }
        if l.kind == "R" {
            assert!(l.sentence.ends_with("goes up again."), "{}", l.sentence);
        }
        if l.kind == "B" {
            assert!(l.sentence.ends_with("goes down again."), "{}", l.sentence);
        }
        for (i, e) in l.edges.iter().enumerate() {
            let (from, to) = (&l.members[i], &l.members[(i + 1) % l.members.len()]);
            assert!(node_ids.contains(e.via.as_str()));
            // The loop's edge is a real signed drives edge of the graph with the same sign.
            assert!(g.edges.iter().any(|x| x.kind == "drives"
                && &x.from == from
                && &x.to == to
                && x.via.as_deref() == Some(e.via.as_str())
                && x.effect.as_ref().map(|f| f.sign.as_str()) == Some(e.sign.as_str())));
        }
        assert!(l.scc < g.sccs.len());
        assert_eq!(l.incomplete, !l.incomplete_reasons.is_empty());
    }
    // inLoops is the inverse relation, for quantities and for the decisions and mechanisms that carry the edges.
    for n in &g.nodes {
        let want: Vec<&str> = g
            .loops
            .iter()
            .filter(|l| l.members.contains(&n.id) || l.edges.iter().any(|e| e.via == n.id))
            .map(|l| l.id.as_str())
            .collect();
        assert_eq!(
            n.in_loops.iter().map(String::as_str).collect::<Vec<_>>(),
            want,
            "{}",
            n.id
        );
    }
    // Teaching names are attached to real loops.
    let named: Vec<&str> = g.loops.iter().filter_map(|l| l.name.as_deref()).collect();
    assert_eq!(named.len(), 7, "{named:?}");
    assert!(g
        .loops
        .iter()
        .all(|l| l.name.is_some() == l.story.is_some()));
    assert!(named.contains(&"Reserve army"));
}

/// Contract v2.13 (causality graph stage 4): the causes, net signs, short loop ids and entry points.
#[test]
fn loop_ids_are_short_unique_and_stable() {
    let a = Api::init(1).unwrap().catalog().unwrap();
    let b = Api::init(2).unwrap().catalog().unwrap();
    let ids: Vec<&str> = a.causal.loops.iter().map(|l| l.id.as_str()).collect();
    let set: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(set.len(), ids.len());
    assert!(
        ids.iter().all(|i| i.len() <= 5 && i.starts_with('l')),
        "{ids:?}"
    );
    // The same loops in the same order, and the id is the position: deterministic across runs.
    let again: Vec<&str> = b.causal.loops.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, again);
    for (i, l) in a.causal.loops.iter().enumerate() {
        assert_eq!(l.id, format!("l{i}"));
    }
    // No dangling id: every inLoops entry names a loop.
    for n in &a.causal.nodes {
        assert!(
            n.in_loops.iter().all(|l| set.contains(l.as_str())),
            "{}",
            n.id
        );
    }
}

#[test]
fn causes_match_causes_of_and_net_signs_match_the_paths() {
    use sim_model::causal::{self, NetSign};
    let g = Api::init(1).unwrap().catalog().unwrap().causal;
    let depth = g.upstream_depth;
    let words: BTreeSet<&str> = g.net_signs.iter().map(|w| w.id.as_str()).collect();
    assert_eq!(
        words,
        BTreeSet::from(["up", "down", "varies", "mixed", "none"])
    );
    let nodes: BTreeSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut mixed = 0;
    for n in g.nodes.iter().filter(|n| n.kind == "quantity") {
        let q = *QuantityId::ALL
            .iter()
            .find(|q| q.as_str() == n.ref_id)
            .unwrap();
        let want = causal::causes_of(q, depth).quantities;
        assert_eq!(n.causes.len(), want.len(), "{}", n.id);
        // The same set and order as `upstream`.
        assert_eq!(
            n.causes.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
            n.upstream
        );
        let mut last = 0;
        for (c, (u, d)) in n.causes.iter().zip(&want) {
            assert_eq!(c.id, format!("quantity:{}", u.as_str()));
            assert!(nodes.contains(c.id.as_str()));
            assert_eq!(c.depth, *d);
            assert!(c.depth >= last && (1..=depth).contains(&c.depth));
            last = c.depth;
            // Net sign is the one `sign_of` gives, and Mixed means paths of both signs.
            let paths = causal::paths(*u, q, depth);
            let expect = match causal::sign_of(*u, q, depth) {
                NetSign::Up => "up",
                NetSign::Down => "down",
                NetSign::Varies => "varies",
                NetSign::Mixed => "mixed",
                NetSign::NoPath => "none",
            };
            assert_eq!(c.net_sign, expect, "{} -> {}", c.id, n.id);
            if c.net_sign == "mixed" {
                mixed += 1;
                assert!(paths.iter().any(|p| p.sign == NetSign::Up));
                assert!(paths.iter().any(|p| p.sign == NetSign::Down));
            }
            assert!(words.contains(c.net_sign.as_str()));
        }
        assert!(n.causes.iter().all(|c| c.id != n.id));
    }
    assert!(mixed > 0, "expected some mixed net signs in the registry");
    for n in g.nodes.iter().filter(|n| n.kind != "quantity") {
        assert!(n.causes.is_empty());
    }
}

#[test]
fn entry_points_name_real_quantities_once_each() {
    let g = Api::init(1).unwrap().catalog().unwrap().causal;
    assert!(!g.entry_points.is_empty());
    let nodes: BTreeSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    let anchors: BTreeSet<&str> = g.entry_points.iter().map(|e| e.anchor.as_str()).collect();
    assert_eq!(anchors.len(), g.entry_points.len());
    assert!(g
        .entry_points
        .iter()
        .all(|e| nodes.contains(e.quantity.as_str())));
}

// ---- stage 5: strength (contract 2.14) ----

#[test]
fn every_verified_signed_effect_has_a_strength_and_the_others_have_none() {
    let g = Api::init(1).unwrap().catalog().unwrap().causal;
    let (mut sized, mut depends, mut none, mut kinds) = (0, 0, 0, BTreeMap::new());
    for e in g.edges.iter().filter(|e| e.kind == "drives") {
        let fx = e.effect.as_ref().unwrap();
        let label = format!("{} {} -> {}", e.via.as_deref().unwrap(), e.from, e.to);
        match (fx.sign.as_str(), fx.basis.as_str()) {
            ("up" | "down", "perturbed" | "system") => {
                let s = fx
                    .strength
                    .as_ref()
                    .unwrap_or_else(|| panic!("{label}: no strength"));
                *kinds.entry(format!("{:?}", s.kind)).or_insert(0) += 1;
                assert!(s.live <= s.total && s.samples <= s.live, "{label}");
                // The sentence is the template for the kind filled with these parts (the web does it).
                assert_eq!(s.pct.is_some(), s.word.is_some(), "{label}");
                assert_eq!(s.low_label.is_some(), s.value.is_some(), "{label}");
                if let Some(v) = s.value {
                    assert!(v.is_finite(), "{label}");
                    // The measured slope has the declared sign (the sign test's rule, reused).
                    assert_eq!(v > 0.0, fx.sign == "up", "{label}: {v} against {}", fx.sign);
                    assert!(s.low.unwrap() <= v && v <= s.high.unwrap(), "{label}");
                    assert!(s.samples >= 1, "{label}");
                    sized += 1;
                }
                if let Some(w) = s.weight {
                    assert!((0.0..=1.0).contains(&w), "{label}");
                    assert!(s.word.is_some(), "{label}");
                }
            }
            ("varies", _) => {
                assert!(
                    fx.strength.is_none(),
                    "{label}: varies has no single strength"
                );
                depends += 1;
            }
            _ => {
                assert!(fx.strength.is_none(), "{label}: {} {}", fx.sign, fx.basis);
                none += 1;
            }
        }
    }
    println!("sized {sized}, depends {depends}, none or unverified {none}, kinds {kinds:?}");
    assert!(sized > 200, "only {sized} sized links");
}

#[test]
fn strengths_file_has_no_stray_entries() {
    use sim_api::strength::Strengths;
    let s = Strengths::load().unwrap();
    let g = Api::init(1).unwrap().catalog().unwrap().causal;
    let real: BTreeSet<(String, String, String)> = g
        .edges
        .iter()
        .filter(|e| e.kind == "drives")
        .map(|e| {
            let via = e.via.as_deref().unwrap().split(':').nth(1).unwrap();
            let from = e.from.split(':').nth(1).unwrap();
            let to = e.to.split(':').nth(1).unwrap();
            (via.into(), from.into(), to.into())
        })
        .collect();
    let mut seen = BTreeSet::new();
    for e in s.entries() {
        let k = (e.via.clone(), e.from.clone(), e.to.clone());
        assert!(real.contains(&k), "{k:?} is not an edge of the registry");
        assert!(seen.insert(k.clone()), "{k:?} appears twice");
        assert!(e.median.is_finite() && e.low <= e.median && e.median <= e.high);
    }
}

#[test]
fn strength_is_deterministic_and_net_strength_agrees_with_the_signs() {
    let a = Api::init(1).unwrap().catalog().unwrap().causal;
    let b = Api::init(2).unwrap().catalog().unwrap().causal;
    assert_eq!(a.edges, b.edges, "strength must not depend on the seed");
    assert_eq!(a.nodes, b.nodes);
    let mut with = 0;
    let mut cancelled = 0;
    for n in a.nodes.iter().filter(|n| n.kind == "quantity") {
        for c in &n.causes {
            let Some(ns) = &c.net_strength else { continue };
            with += 1;
            assert!(
                ns.value.is_finite() && ns.gross.is_finite(),
                "{} -> {}",
                c.id,
                n.id
            );
            assert!(ns.value.abs() <= ns.gross + 1e-9 && ns.routes >= 1);
            assert!((0.0..=1.0).contains(&ns.weight));
            assert!(a.strength.words.iter().any(|w| w.id == ns.word));
            if !ns.partial {
                match c.net_sign.as_str() {
                    "up" => assert!(ns.value > 0.0, "{} -> {}", c.id, n.id),
                    "down" => assert!(ns.value < 0.0, "{} -> {}", c.id, n.id),
                    _ => {}
                }
            }
            if ns.value.abs() + 1e-9 < ns.gross {
                cancelled += 1;
            }
        }
    }
    println!("{with} causes have a net strength, {cancelled} of them cancel in part");
    assert!(with > 500);
    assert!(cancelled > 0, "no cause where routes cancel");
    // The thresholds come from content and rise.
    let mins: Vec<f64> = a.strength.words.iter().map(|w| w.min).collect();
    assert!(mins.windows(2).all(|w| w[0] < w[1]));
}
