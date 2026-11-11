//! Throwaway-style probe kept for the Town v7 balance log: `cargo run --release -p sim-cli --example tune_probe -- key=value ...`
//! Runs A and B over seeds 1..=20 with the given parameter overrides (any `section.field=number`).
use sim_engine::Plan;
use sim_params::{ContentJson, Params};
use sim_types::*;
use std::fs;
fn main() {
    let r = |n: &str| fs::read_to_string(format!("content/{n}")).unwrap();
    let p = Params::from_content(&ContentJson {
        params: &r("params.json"),
        initial_town: &r("initial_town.json"),
        market: &r("systems/market.json"),
        regulated: &r("systems/regulated.json"),
        coop: &r("systems/coop.json"),
    })
    .unwrap();
    let mut j = serde_json::to_value(&p).unwrap();
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap();
        let mut cur = &mut j;
        let parts: Vec<&str> = k.split('.').collect();
        for part in &parts[..parts.len() - 1] {
            cur = cur.get_mut(*part).unwrap();
        }
        cur[parts[parts.len() - 1]] = serde_json::json!(v.parse::<f64>().unwrap());
    }
    let p: Params = serde_json::from_value(j).unwrap();
    let b = Plan::from_json(&r("scenarios/baseline_b_sensible.json")).unwrap();
    let med = |mut v: Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    };
    let (mut ex, mut lg_a) = (vec![], vec![]);
    for seed in 1..=20u64 {
        let mut sim = sim_engine::init(p.clone(), seed).unwrap();
        let mut done = None;
        let mut lg = 1.0f64;
        sim_engine::run(&mut sim, 40, &Plan::default(), |s, _| {
            if done.is_none() {
                lg = lg.min(s.state.gov.legitimacy.0);
                if s.state.nature.cumulative.0 >= p.nature.budget_b0 {
                    done = Some(s.state.tick as f64 / 3.0);
                }
            }
        })
        .unwrap();
        ex.push(done.unwrap_or(40.0));
        lg_a.push(lg);
    }
    let (mut surv, mut left, mut lg_b) = (0, vec![], vec![]);
    for seed in 1..=20u64 {
        let mut sim = sim_engine::init(p.clone(), seed).unwrap();
        let mut lg = 1.0f64;
        sim_engine::run(&mut sim, 40, &b, |s, _| {
            lg = lg.min(s.state.gov.legitimacy.0);
        })
        .unwrap();
        if matches!(sim_engine::final_outcome(&sim), Outcome::Survived) {
            surv += 1;
        }
        left.push(sim.state.indicators.remaining_budget.0 / p.nature.budget_b0);
        lg_b.push(lg);
    }
    println!("A: exhausted turn median {:.2}, legitimacy min median {:.2} | B: survived {surv}/20, budget left median {:.2}, legitimacy min median {:.2} (lowest {:.2})", med(ex), med(lg_a), med(left.clone()), med(lg_b.clone()), lg_b.iter().cloned().fold(1.0, f64::min));
}
