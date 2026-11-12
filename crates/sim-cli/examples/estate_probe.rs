//! Developer probe for the Town v7 balance log (see docs/how-to.md section 18). Not shipped, not tested:
//! it unwraps freely and exits with a panic on bad input, which is fine for a throwaway measuring tool.
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
    let mut p = p;
    if let Ok(w) = std::env::var("WAGE") {
        p.estate.pilot_wage_usd = w.parse().unwrap();
    }
    for (name, file) in [
        ("A", None),
        ("B", Some("baseline_b_sensible.json")),
        ("C", Some("baseline_c_neglect.json")),
        ("D", Some("baseline_d_carbon_only.json")),
        ("E", Some("baseline_e_adaptation_only.json")),
        ("F", Some("baseline_f_renewables_only.json")),
    ] {
        let plan = match file {
            Some(f) => Plan::from_json(&r(&format!("scenarios/{f}"))).unwrap(),
            None => Plan::default(),
        };
        let (mut dismissed, mut hired, mut flights, mut minsav, mut tonnes, mut g) =
            (0, 0, 0u32, i64::MAX, 0.0, 0.0);
        let mut n = 0.0;
        let mut turns_flying = 0;
        let mut firsts: Vec<u32> = Vec::new();
        for seed in 1..=20u64 {
            let mut sim = sim_engine::init(p.clone(), seed).unwrap();
            let mut dis = 0;
            let mut first: Option<u32> = None;
            let mut lowest = i64::MAX;
            sim_engine::run(&mut sim, 40, &plan, |s, o| {
                for e in &o.events {
                    match e {
                        Event::PilotLetGo { .. } => {
                            dis += 1;
                            first.get_or_insert(s.state.turn);
                        }
                        Event::PilotHired { .. } => hired += 1,
                        _ => {}
                    }
                }
                lowest = lowest.min(s.state.households[8].savings.0);
            })
            .unwrap();
            if dis > 0 {
                dismissed += 1;
                firsts.push(first.unwrap());
            }
            let e = sim.state.estate.as_ref().unwrap();
            flights += e.flights_total;
            tonnes += e.tonnes_total.0;
            minsav = minsav.min(lowest);
            g += sim.state.indicators.gini.0;
            n += 1.0;
            if e.flights_total > 0 {
                turns_flying += 1;
            }
        }
        println!("{name}: seeds with a dismissal {dismissed}/20, rehires {hired}, flights/seed {:.1}, tonnes/seed {:.1}, min estate savings {}, final gini {:.3}, seeds with flights {turns_flying}, first dismissal turns {:?}", flights as f64 / n, tonnes / n, minsav / 100, g / n, firsts);
    }
}
