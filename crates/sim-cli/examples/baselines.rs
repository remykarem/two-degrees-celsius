//! Baselines A to F over seeds 1..=20: `cargo run --release -p sim-cli --example baselines`.
//! Prints, per baseline, how many seeds survive, the outcome tally, the median turn on which the
//! carbon budget runs out (the first tick the cumulative total reaches `budget_b0`, over 3) and
//! the median share of the budget left. Developer probe for the tuning log, like `tune_probe`.
use sim_engine::Plan;
use sim_params::{ContentJson, Params};
use sim_types::*;
use std::collections::BTreeMap;
use std::fs;

fn main() {
    let r = |n: &str| fs::read_to_string(format!("content/{n}")).expect("content file");
    let p = Params::from_content(&ContentJson {
        params: &r("params.json"),
        initial_town: &r("initial_town.json"),
        market: &r("systems/market.json"),
        regulated: &r("systems/regulated.json"),
        coop: &r("systems/coop.json"),
    })
    .expect("content loads");
    let scripts: [(&str, Option<&str>); 6] = [
        ("A none", None),
        ("B sensible", Some("baseline_b_sensible.json")),
        ("C neglect", Some("baseline_c_neglect.json")),
        ("D carbon tax", Some("baseline_d_carbon_only.json")),
        ("E adaptation", Some("baseline_e_adaptation_only.json")),
        ("F renewables", Some("baseline_f_renewables_only.json")),
    ];
    let med = |mut v: Vec<f64>| -> Option<f64> {
        v.sort_by(|a, b| a.total_cmp(b));
        v.get(v.len() / 2).copied()
    };
    for (name, file) in scripts {
        let plan = match file {
            Some(f) => Plan::from_json(&r(&format!("scenarios/{f}"))).expect("plan"),
            None => Plan::default(),
        };
        let mut tally: BTreeMap<String, u32> = BTreeMap::new();
        let (mut exhausted, mut left, mut survived) = (vec![], vec![], 0);
        for seed in 1..=20u64 {
            let mut sim = sim_engine::init(p.clone(), seed).expect("init");
            let mut ex = None;
            sim_engine::run(&mut sim, 40, &plan, |s, _| {
                if ex.is_none() && s.state.nature.cumulative.0 >= p.nature.budget_b0 {
                    ex = Some(f64::from(s.state.tick) / 3.0);
                }
            })
            .expect("checks pass");
            let o = sim_engine::final_outcome(&sim);
            if matches!(o, Outcome::Survived) {
                survived += 1;
            }
            *tally.entry(format!("{o:?}")).or_default() += 1;
            if let Some(t) = ex {
                exhausted.push(t);
            }
            left.push(sim.state.indicators.remaining_budget.0 / p.nature.budget_b0);
        }
        println!(
            "{name:<13} survived {survived:>2}/20  outcomes {tally:?}  budget run-out turn median {:?} ({} seeds)  budget left median {:.2}",
            med(exhausted.clone()),
            exhausted.len(),
            med(left).unwrap_or(0.0)
        );
    }
}
