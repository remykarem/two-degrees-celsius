//! Contract v2.9 (causality graph stage 0): every graph quantity has a live-value binding or an
//! explicit reason it has none, and every bound path resolves on a real snapshot.
use serde_json::Value;
use sim_api::live::{self, Binding};
use sim_api::Api;
use sim_types::QuantityId;

/// All values a path reaches in the snapshot JSON. `a.b` descends objects, `xs[]` fans out over an
/// array (firms: active only, like the live value), `markets[food]` picks the entry with that id.
fn resolve(root: &Value, path: &str) -> Vec<Value> {
    let mut cur = vec![root.clone()];
    for seg in path.split('.') {
        let (key, sel) = match seg.split_once('[') {
            Some((k, rest)) => (k, Some(rest.trim_end_matches(']'))),
            None => (seg, None),
        };
        let mut next = Vec::new();
        for v in &cur {
            let Some(child) = v.get(key) else { continue };
            match sel {
                None => next.push(child.clone()),
                Some("") => {
                    for item in child.as_array().cloned().unwrap_or_default() {
                        // Firms carry an `active` flag; inactive ones are not counted.
                        if item.get("active") != Some(&Value::Bool(false)) {
                            next.push(item);
                        }
                    }
                }
                Some(id) => {
                    next.extend(
                        child
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|m| m.get("id").and_then(Value::as_str) == Some(id)),
                    );
                }
            }
        }
        cur = next;
    }
    cur
}

fn api_after_turns(n: u32) -> Api {
    let mut api = Api::init(7).unwrap();
    for _ in 0..n {
        api.submit_turn(&[]).unwrap();
    }
    api
}

#[test]
fn every_quantity_is_bound_or_has_a_reason_and_the_catalog_says_which() {
    let api = Api::init(1).unwrap();
    let c = api.catalog().unwrap();
    let unbound_ids: Vec<&str> = c
        .causal
        .live
        .unbound
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    let agg_ids: Vec<&str> = c
        .causal
        .live
        .aggregates
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(c.quantities.len(), QuantityId::ALL.len());
    let (mut bound, mut unbound) = (0, 0);
    for q in &c.quantities {
        let l = &q.live;
        if l.bound {
            bound += 1;
            assert!(l.path.is_some(), "{} bound without a path", q.id);
            assert!(
                agg_ids.contains(&l.aggregate.as_deref().unwrap_or("")),
                "{}",
                q.id
            );
            assert!(l.unbound.is_none(), "{}", q.id);
        } else {
            unbound += 1;
            assert!(l.path.is_none() && l.aggregate.is_none() && !l.has_trend);
            assert!(
                unbound_ids.contains(&l.unbound.as_deref().unwrap_or("")),
                "{} is unbound without a listed reason",
                q.id
            );
        }
    }
    // Both lists are non-trivial: this is a sanity bound, not a target.
    assert!(
        bound >= 30 && unbound >= 30,
        "{bound} bound, {unbound} unbound"
    );
    // Every reason and aggregate word is used (no dead content).
    for r in &unbound_ids {
        assert!(
            c.quantities
                .iter()
                .any(|q| q.live.unbound.as_deref() == Some(*r)),
            "reason {r} is never used"
        );
    }
    for a in &agg_ids {
        assert!(
            c.quantities
                .iter()
                .any(|q| q.live.aggregate.as_deref() == Some(*a)),
            "aggregate {a} is never used"
        );
    }
}

#[test]
fn every_bound_path_resolves_on_a_real_snapshot_and_the_value_matches() {
    // Turn 3 so the history exists (trends) and the markets have a month.
    let api = api_after_turns(3);
    let snap = api.snapshot();
    let json = serde_json::to_value(&snap).unwrap();
    let mut seen = 0;
    for q in QuantityId::ALL {
        let Binding::Bound(b) = live::binding(*q) else {
            assert!(
                snap.quantities.iter().all(|l| l.id != q.as_str()),
                "{} is unbound but served a value",
                q.as_str()
            );
            continue;
        };
        let reached = resolve(&json, b.path);
        assert!(
            !reached.is_empty(),
            "{}: path {} does not resolve",
            q.as_str(),
            b.path
        );
        let served = snap
            .quantities
            .iter()
            .find(|l| l.id == q.as_str())
            .unwrap_or_else(|| panic!("{} is bound but has no live value", q.as_str()));
        assert!(served.value.is_finite());
        seen += 1;
        // A yes/no lever holds a bool; its live value is 1 or 0.
        let nums: Vec<f64> = reached
            .iter()
            .filter_map(|v| {
                v.as_f64()
                    .or_else(|| v.as_bool().map(|b| f64::from(u8::from(b))))
            })
            .collect();
        match b.aggregate {
            live::Aggregate::Town | live::Aggregate::Lever => {
                assert_eq!(nums.len(), 1, "{}", q.as_str());
                assert_eq!(nums[0], served.value, "{} vs {}", q.as_str(), b.path);
            }
            live::Aggregate::Sum => {
                let t: f64 = nums.iter().sum();
                assert!(
                    (t - served.value).abs() <= 1e-9 * t.abs().max(1.0),
                    "{}",
                    q.as_str()
                );
            }
            live::Aggregate::Mean => {
                let m = nums.iter().sum::<f64>() / nums.len() as f64;
                assert!(
                    (m - served.value).abs() <= 1e-9 * m.abs().max(1.0),
                    "{}",
                    q.as_str()
                );
            }
            // The path holds a bool, a status or an access mode, not the 1 or 0.
            live::Aggregate::Flag => {
                assert!(served.value == 0.0 || served.value == 1.0, "{}", q.as_str());
                assert_eq!(reached.len(), 1);
            }
        }
        // A trend needs the history and a series for it.
        assert_eq!(
            served.trend.is_some(),
            b.previous.is_some(),
            "{}: trend only where the history keeps the series",
            q.as_str()
        );
    }
    assert_eq!(seen, snap.quantities.len());
}

#[test]
fn live_values_are_deterministic_and_the_trend_follows_the_history() {
    let a = api_after_turns(2).snapshot().quantities;
    let b = api_after_turns(2).snapshot().quantities;
    assert_eq!(a, b);
    // `up` / `down` / `steady` rule: relative band, not absolute.
    assert_eq!(live::trend(1.0, 1.0, 0.005), "steady");
    assert_eq!(live::trend(1.1, 1.0, 0.005), "rising");
    assert_eq!(live::trend(0.9, 1.0, 0.005), "falling");
    assert_eq!(live::trend(0.0, 0.0, 0.005), "steady");
    // At a turn boundary the latest history point is now, so the baseline is the one before it.
    let api = api_after_turns(3);
    let snap = api.snapshot();
    assert_eq!(snap.turn_phase.months_done, 0);
    let h = &snap.history;
    let u = snap
        .quantities
        .iter()
        .find(|l| l.id == "unemployment")
        .unwrap();
    assert_eq!(h.last().unwrap().unemployment, u.value);
    assert_eq!(u.previous, Some(h[h.len() - 2].unemployment));
    // Mid-turn the baseline is the end of the previous turn.
    let mut api = api_after_turns(2);
    api.begin_turn(&[]).unwrap();
    api.advance_month().unwrap();
    let snap = api.snapshot();
    assert_eq!(snap.turn_phase.months_done, 1);
    let u = snap
        .quantities
        .iter()
        .find(|l| l.id == "unemployment")
        .unwrap();
    assert_eq!(u.previous, Some(snap.history.last().unwrap().unemployment));
}

#[test]
fn the_first_snapshot_has_values_but_no_trends() {
    let snap = api_after_turns(1).snapshot();
    assert!(!snap.quantities.is_empty());
    assert!(snap.quantities.iter().all(|l| l.trend.is_none()));
}

/// Every `quantityId` anywhere under `v`, with the path it was found at.
fn quantity_ids(v: &Value, at: &str, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if k == "quantityId" || k == "priceQuantity" {
                    out.push((at.to_string(), x.as_str().unwrap_or("").to_string()));
                } else {
                    quantity_ids(x, &format!("{at}.{k}"), out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| quantity_ids(x, at, out)),
        _ => {}
    }
}

#[test]
fn breakdown_inputs_and_market_drivers_link_to_real_quantities() {
    let api = api_after_turns(3);
    let c = api.catalog().unwrap();
    let known: Vec<&str> = c.quantities.iter().map(|q| q.id.as_str()).collect();
    let json = serde_json::to_value(api.snapshot()).unwrap();
    for part in ["breakdowns", "markets"] {
        let mut ids = Vec::new();
        quantity_ids(&json[part], part, &mut ids);
        assert!(!ids.is_empty(), "{part} links nothing");
        for (at, id) in ids {
            assert!(
                known.contains(&id.as_str()),
                "{at}: dangling quantity id {id:?}"
            );
        }
    }
}

#[test]
fn legitimacy_breakdown_inputs_are_what_the_legitimacy_decision_reads() {
    let api = api_after_turns(2);
    let json = serde_json::to_value(api.snapshot()).unwrap();
    let spec = sim_model::decisions::DECISIONS
        .iter()
        .find(|d| d.id == sim_types::DecisionId::GovernmentLegitimacy)
        .unwrap();
    let linked: Vec<&str> = json["breakdowns"]["legitimacy"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["quantityId"].as_str())
        .collect();
    assert!(linked.len() >= 4);
    for id in linked {
        assert!(
            spec.inputs.iter().any(|q| q.as_str() == id),
            "legitimacy breakdown input {id} is not read by the legitimacy decision"
        );
    }
}
