//! Prints an FNV-1a 64 hash of the serialised model `State` after every turn, for a set of
//! seeds and plans. Used to prove that a change did not alter the simulation:
//! `cargo run -p sim-cli --example statehash > hashes.txt`, change code, run again, `diff`.
use sim_engine::Plan;
use sim_params::{ContentJson, Params};
use std::fs;

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn main() {
    let r = |n: &str| fs::read_to_string(format!("content/{n}")).unwrap();
    let params = Params::from_content(&ContentJson {
        params: &r("params.json"),
        initial_town: &r("initial_town.json"),
        market: &r("systems/market.json"),
        regulated: &r("systems/regulated.json"),
        coop: &r("systems/coop.json"),
    })
    .unwrap();
    let plans = [
        "",
        "scenarios/baseline_b_sensible.json",
        "scenarios/baseline_c_neglect.json",
        "scenarios/baseline_d_carbon_only.json",
        "scenarios/baseline_e_adaptation_only.json",
        "scenarios/worker_coops.json",
    ];
    for plan_file in plans {
        let plan = if plan_file.is_empty() {
            Plan::default()
        } else {
            Plan::from_json(&r(plan_file)).unwrap()
        };
        for seed in [7u64, 42, 99] {
            let mut sim = sim_engine::init(params.clone(), seed).unwrap();
            let mut line = format!("{plan_file:<45} seed {seed:>3}:");
            for _ in 0..40 {
                let cmds = plan.commands_at(sim.state.turn);
                if sim_engine::submit_turn(&mut sim, &cmds).is_err() {
                    line.push_str(" ERR");
                    break;
                }
                let json = serde_json::to_string(&sim.state).unwrap();
                line.push_str(&format!(" {:08x}", fnv(&json) & 0xffff_ffff));
            }
            println!("{line}");
        }
    }
}
