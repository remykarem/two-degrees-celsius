//! Scenario runs, invariant tests, sign tests and the golden snapshot for the whole simulation.
//! Town v2: 20 people in 8 households, a 12 x 12 map owned by the simulation.

use sim_checks::check_state;
use sim_engine::Sim;
use sim_params::{ContentJson, Params};
use sim_types::*;

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");

fn load(params: &str, town: &str) -> Result<Params, String> {
    Params::from_content(&ContentJson {
        params,
        initial_town: town,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
}

fn params() -> Params {
    load(PARAMS, TOWN).expect("content must load")
}

fn new_sim(seed: u64) -> Sim {
    sim_engine::init(params(), seed).expect("init")
}

/// The town exactly as `initial_town.json` describes it, before any settling.
fn fresh_sim() -> Sim {
    let mut p = params();
    p.run.warmup_ticks = 0;
    sim_engine::init(p, 1).expect("init")
}

// ---------- setup ----------

#[test]
fn content_loads_and_town_matches_the_design() {
    let sim = fresh_sim();
    let s = &sim.state;
    assert_eq!(s.persons.len(), 20, "20 individual people");
    assert_eq!(s.households.len(), 8, "8 households");
    for h in &s.households {
        assert!((2..=4).contains(&h.members.len()), "{} size", h.label);
    }
    assert_eq!(s.homes.len(), 7, "7 homes for 8 households");
    assert_eq!(s.firms.len(), 4);
    assert_eq!(
        s.firms.iter().filter(|f| f.active).count(),
        3,
        "Factory, Farm, Shop"
    );
    assert_eq!(
        s.firms.iter().filter(|f| !f.active).count(),
        1,
        "one dormant slot"
    );
    assert_eq!(s.map.width, 12);
    assert_eq!(s.map.height, 12);
    let count = |a: AgeBand| s.persons.iter().filter(|p| p.age_band == a).count();
    assert_eq!(
        (
            count(AgeBand::Adult),
            count(AgeBand::Child),
            count(AgeBand::Retired)
        ),
        (13, 4, 3)
    );
    assert_eq!(s.bank.equity, s.bank.loans - s.bank.deposits);
    // The destitute household has no home.
    let d = s.households.last().unwrap();
    assert!(d.home.is_none());
    assert_eq!(s.housing.unhoused_households, 1);
}

#[test]
fn starting_inequality_is_harsh() {
    let sim = fresh_sim();
    let i = &sim.state.indicators;
    assert!(
        (0.65..=0.75).contains(&i.gini.0),
        "start gini {} (target 0.65 to 0.75)",
        i.gini.0
    );
    assert!(
        (0.60..=0.70).contains(&i.top_wealth_share.0),
        "top household holds {} of wealth (target 60% to 70%)",
        i.top_wealth_share.0
    );
    let w = sim_metrics::household_wealth(&sim.state, &sim.params);
    assert!(
        *w.last().unwrap() < 0.0,
        "the destitute household has negative net worth"
    );
}

#[test]
fn settled_start_keeps_the_target_inequality() {
    // What the player sees at turn 1 is the town after the warm-up months.
    let sim = new_sim(42);
    let i = &sim.state.indicators;
    println!(
        "settled start: gini {:.3}, top share {:.3}, avg income {:.0}, median {:.0}",
        i.gini.0,
        i.top_wealth_share.0,
        i.avg_income_pp.dollars(),
        i.median_income_pp.dollars()
    );
    assert!((0.60..=0.78).contains(&i.gini.0), "gini {}", i.gini.0);
    assert!((0.55..=0.75).contains(&i.top_wealth_share.0));
}

#[test]
fn bad_content_is_rejected() {
    let bad = PARAMS.replace("\"forest_max\": 2000.0", "\"forest_max\": -1.0");
    assert_ne!(bad, PARAMS);
    assert!(load(&bad, TOWN).is_err());
    let unknown = PARAMS.replace("\"turns\": 160,", "\"turns\": 160, \"bogus\": 1,");
    assert!(load(&unknown, TOWN).is_err());
}

fn town_json() -> serde_json::Value {
    serde_json::from_str(TOWN).unwrap()
}

fn load_town(v: &serde_json::Value) -> Result<Params, String> {
    load(PARAMS, &v.to_string())
}

#[test]
fn init_rejects_two_households_in_one_home() {
    let mut t = town_json();
    t["households"][3]["home"] = t["households"][2]["home"].clone();
    let e = load_town(&t).unwrap_err();
    assert!(e.contains("holds households"), "{e}");
}

#[test]
fn init_rejects_two_things_on_one_tile() {
    let mut t = town_json();
    t["plants"][0]["tile"] = t["homes"][0]["tile"].clone();
    let e = load_town(&t).unwrap_err();
    assert!(e.contains("holds both"), "{e}");
}

#[test]
fn init_rejects_tiles_outside_the_map_and_bad_jobs() {
    let mut t = town_json();
    t["homes"][0]["tile"] = serde_json::json!([12, 0]);
    assert!(load_town(&t).unwrap_err().contains("outside"));
    let mut t = town_json();
    t["households"][7]["people"][0]["job"] = serde_json::json!(3);
    assert!(load_town(&t).unwrap_err().contains("inactive"));
}

#[test]
fn invariants_catch_broken_people_homes_and_tiles() {
    let sim = fresh_sim();
    let ok = |s: &State| check_state(s, 0, SystemId::Commands, 20.0);
    assert!(ok(&sim.state).is_ok());

    // A person listed in two households.
    let mut s = sim.state.clone();
    let m = s.households[2].members[0];
    s.households[3].members.push(m);
    assert!(ok(&s).unwrap_err().what.contains("belongs to"));

    // A person in no household.
    let mut s = sim.state.clone();
    s.households[2].members.pop();
    assert!(ok(&s).is_err());

    // Two households in one home.
    let mut s = sim.state.clone();
    let home = s.households[2].home;
    s.households[7].home = home;
    assert!(ok(&s).is_err());

    // Two things on one tile.
    let mut s = sim.state.clone();
    s.plants[0].tile = s.homes[0].tile;
    assert!(ok(&s).unwrap_err().what.contains("both"));

    // A job in a dormant firm.
    let mut s = sim.state.clone();
    s.persons[17].job = Some(FirmId(3));
    assert!(ok(&s).is_err());
}

#[test]
fn money_and_structure_hold_over_the_warm_up() {
    // The engine runs sim-checks after every system during the warm-up months; any
    // violation would make `init` fail.
    let sim = new_sim(42);
    assert_eq!(sim.state.tick, 0);
    assert_eq!(sim.state.persons.len(), 20);
}
