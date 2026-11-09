//! Scenario runs, invariant tests and golden snapshots for the whole simulation.

use sim_engine::{Script, Sim};
use sim_params::{ContentJson, Params};

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");

fn params() -> Params {
    Params::from_content(&ContentJson {
        params: PARAMS,
        initial_town: TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
    .expect("content must load")
}

fn new_sim(seed: u64) -> Sim {
    sim_engine::init(params(), seed).expect("init")
}

#[test]
fn content_loads_and_town_matches_decisions() {
    let sim = new_sim(1);
    let s = &sim.state;
    assert_eq!(s.households.len(), 6, "D1: six cohorts");
    assert_eq!(s.firms.len(), 4, "D2: four firm slots");
    assert_eq!(s.firms.iter().filter(|f| f.active).count(), 2);
    assert_eq!(s.total_people().0, 1000.0);
    assert_eq!(s.bank.equity, s.bank.loans - s.bank.deposits);
}

#[test]
fn empty_ticks_run_and_keep_checks_green() {
    let mut sim = new_sim(42);
    sim_engine::run(&mut sim, 24, &Script::default(), |_, _| {}).expect("checks pass");
    assert_eq!(sim.state.tick, 24);
}

#[test]
fn money_conserved_over_full_run() {
    // The engine runs sim-checks after every system: finite numbers, non-negative stocks,
    // and exact money conservation (deposits change only through credit operations).
    let mut sim = new_sim(42);
    let ticks = sim.params.run.ticks;
    sim_engine::run(&mut sim, ticks, &Script::default(), |_, _| {}).expect("no invariant violated");
    assert_eq!(sim.state.tick, 480);
    let s = &sim.state;
    assert_eq!(s.bank.equity, s.bank.loans - s.bank.deposits);
    assert_eq!(s.bank.deposits, s.total_deposits());
}
