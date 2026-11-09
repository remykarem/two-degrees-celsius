//! Scenario runs, invariant tests, sign tests and the golden snapshot for the whole simulation.
//! Town v2: 20 people in 8 households, a 12 x 12 map owned by the simulation.

use sim_checks::check_state;
use sim_engine::{Plan, Sim};
use sim_model::class::classify;
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
const SCENARIO_REGULATED: &str = include_str!("../content/scenarios/regulated_green_deal.json");
const SCENARIO_COOP: &str = include_str!("../content/scenarios/worker_coops.json");

/// Run `n` ticks (rounded up to whole turns), collecting one value per tick with `f`.
fn collect(sim: &mut Sim, n: u32, script: &Plan, f: impl Fn(&Sim) -> f64) -> Vec<f64> {
    let mut v = Vec::new();
    sim_engine::run(sim, n.div_ceil(3), script, |s, _| v.push(f(s))).expect("checks pass");
    v.truncate(n as usize);
    v
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

fn cmd(c: Command) -> Plan {
    Plan {
        name: "test".into(),
        turns: vec![sim_engine::plan::PlanTurn {
            turn: 1,
            command: Some(c),
            ..Default::default()
        }],
    }
}

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

// ---------- invariants over whole runs ----------

#[test]
fn invariants_hold_over_full_runs_for_several_seeds() {
    // The engine runs sim-checks after every system: finite numbers, non-negative stocks,
    // exact money conservation, sector balances, people/home/tile structure, no unfunded
    // payments (strict flows). Any violation aborts the run.
    for seed in 1..=6 {
        let mut sim = new_sim(seed);
        let turns = sim.params.run.turns;
        sim_engine::run(&mut sim, turns, &Plan::default(), |_, _| {})
            .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert_eq!(sim.state.tick, 480);
        let s = &sim.state;
        assert_eq!(s.bank.equity, s.bank.loans - s.bank.deposits);
        assert_eq!(s.bank.deposits, s.total_deposits());
        assert_eq!(s.persons.len(), 20, "nobody joins or leaves the town");
        assert_eq!(s.households.len(), 8);
    }
}

#[test]
fn same_seed_gives_identical_state() {
    let mut a = new_sim(7);
    let mut b = new_sim(7);
    let sc = Plan::from_json(SCENARIO_REGULATED).unwrap();
    sim_engine::run(&mut a, 160, &sc, |_, _| {}).unwrap();
    sim_engine::run(&mut b, 160, &sc, |_, _| {}).unwrap();
    assert_eq!(
        serde_json::to_string(&a.state).unwrap(),
        serde_json::to_string(&b.state).unwrap()
    );
    assert_eq!(a.rng.parts(), b.rng.parts());
}

#[test]
fn class_labels_match_the_class_function_every_tick() {
    let mut sim = new_sim(42);
    for _ in 0..480 {
        sim_engine::step(&mut sim).unwrap();
        let s = &sim.state;
        for h in &s.households {
            assert_eq!(
                h.class,
                classify(s, &sim.params, h),
                "tick {} household {}",
                s.tick,
                h.label
            );
        }
    }
}

#[test]
fn bank_failure_is_resolved_not_hidden() {
    // Make the bank insolvent by giving a household deposits the bank has no assets for; the
    // engine must resolve it with a deposit haircut (no bailout) and keep every invariant.
    let mut sim = new_sim(1);
    sim.state.policy.bailout = false;
    let take = sim.state.bank.equity + Cents::from_dollars(20_000.0);
    sim.state.households[2].savings += take;
    sim.state.bank.deposits += take;
    sim.state.bank.equity -= take;
    assert!(
        sim.state.bank.equity.is_negative(),
        "setup makes the bank insolvent"
    );
    let out = sim_engine::step(&mut sim).expect("engine resolves the failure");
    assert!(out.events.contains(&Event::BankFailed));
    assert!(!sim.state.bank.equity.is_negative());
}

// ---------- S1: named people and jobs ----------

#[test]
fn hiring_names_the_cheapest_unemployed_person_and_firing_names_the_least_skilled() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p, 1).unwrap();
    // Free the destitute household's adult and make the Factory want more people.
    let jobless = sim.state.households[7].members[0];
    assert!(sim.state.person(jobless).unemployed());
    sim.state.firms[0].expected_sales = Units(400.0);
    sim.state.firms[0].inventory = Units(0.0);
    let out = sim_engine::step(&mut sim).unwrap();
    assert!(
        out.events.iter().any(|e| matches!(e, Event::PersonHired { person, firm } if *person == jobless && *firm == FirmId(0))),
        "the only unemployed adult is hired by the firm that needs people"
    );
    assert_eq!(sim.state.person(jobless).job, Some(FirmId(0)));
    // Now the opposite: demand collapses, the least skilled worker is let go first.
    let mut sim = sim_engine::init(
        {
            let mut p = params();
            p.run.warmup_ticks = 0;
            p
        },
        1,
    )
    .unwrap();
    sim.state.firms[1].expected_sales = Units(40.0);
    sim.state.firms[1].inventory = Units(5000.0);
    let farm_workers: Vec<PersonId> = sim
        .state
        .persons
        .iter()
        .filter(|x| x.job == Some(FirmId(1)))
        .map(|x| x.id)
        .collect();
    let lowest = *farm_workers
        .iter()
        .min_by(|a, b| {
            sim.state
                .person(**a)
                .skill
                .0
                .partial_cmp(&sim.state.person(**b).skill.0)
                .unwrap()
        })
        .unwrap();
    let mut fired = None;
    for _ in 0..8 {
        let out = sim_engine::step(&mut sim).unwrap();
        fired = out.events.iter().find_map(|e| match e {
            Event::PersonFired { person, firm } if *firm == FirmId(1) => Some(*person),
            _ => None,
        });
        if fired.is_some() {
            break;
        }
    }
    assert_eq!(fired, Some(lowest), "the least skilled worker goes first");
}

#[test]
fn pay_follows_skill_and_never_falls_below_the_minimum_wage() {
    let sim = new_sim(42);
    let s = &sim.state;
    let f = &s.firms[0];
    let employed = || s.persons.iter().filter(|x| x.job.is_some());
    let by_skill = |a: &&Person, b: &&Person| a.skill.0.partial_cmp(&b.skill.0).unwrap();
    let manager = employed().max_by(by_skill).unwrap();
    let worker = employed().min_by(by_skill).unwrap();
    assert!(manager.skill.0 > worker.skill.0);
    let pm = sim_model::util::person_pay(s, &sim.params, manager, f.wage);
    let pw = sim_model::util::person_pay(s, &sim.params, worker, f.wage);
    assert!(pm > pw);
    assert!(pw >= s.policy.min_wage);
}

// ---------- S3: homes, rent, evictions ----------

#[test]
fn unpaid_rent_for_three_months_loses_the_home() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p, 1).unwrap();
    // A rent nobody can pay.
    sim.state.housing.rent_private = Cents::from_dollars(50_000.0);
    let renter = sim.state.households[5].id;
    let home = sim.state.households[5].home.unwrap();
    let mut evicted_at = None;
    for t in 1..=8 {
        let out = sim_engine::step(&mut sim).unwrap();
        if out
            .events
            .contains(&Event::HouseholdEvicted { household: renter })
        {
            evicted_at = Some(t);
            break;
        }
    }
    let t = evicted_at.expect("evicted");
    assert!(
        (4..=5).contains(&t),
        "evicted on tick {t}, after 3 months of arrears"
    );
    assert_eq!(sim.state.households[5].home, None);
    assert_eq!(
        sim.state.home(home).unwrap().household,
        None,
        "the home is vacant again"
    );
    assert!(sim.state.housing.unhoused_households >= 2);
}

#[test]
fn owner_occupiers_are_never_evicted() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p, 1).unwrap();
    sim.state.housing.rent_private = Cents::from_dollars(50_000.0);
    let owner = sim.state.households[2].id; // Manager A owns the home they live in
    for _ in 0..12 {
        let out = sim_engine::step(&mut sim).unwrap();
        assert!(!out
            .events
            .contains(&Event::HouseholdEvicted { household: owner }));
    }
    assert!(sim.state.households[2].home.is_some());
}

#[test]
fn an_unhoused_household_moves_into_a_vacant_affordable_home() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p, 1).unwrap();
    let destitute = sim.state.households[7].id;
    assert!(sim.state.households[7].home.is_none());
    // A vacant private home at an affordable rent: the landlord's own (Worker C leaves it).
    let vacated = sim.state.households[6].home.unwrap();
    sim.state.households[6].home = None;
    sim.state
        .homes
        .iter_mut()
        .find(|h| h.id == vacated)
        .unwrap()
        .household = None;
    let out = sim_engine::step(&mut sim).unwrap();
    // The richer of the two households without a home moves first and takes the only vacancy.
    let moved: Vec<_> = out
        .events
        .iter()
        .filter(|e| matches!(e, Event::HouseholdMoved { .. }))
        .collect();
    assert_eq!(moved.len(), 1, "one vacancy, one move: {moved:?}");
    let first = sim
        .state
        .households
        .iter()
        .find(|h| h.home == Some(vacated))
        .unwrap();
    assert!(first.id == destitute || first.id == sim.state.households[6].id);
}

#[test]
fn an_unhoused_person_is_less_well_off_than_a_housed_one() {
    let sim = new_sim(42);
    let s = &sim.state;
    let unhoused = &s.households[7];
    let housed = &s.households[6];
    assert!(unhoused.home.is_none() && housed.home.is_some());
    assert!(
        unhoused.wellbeing.0 < housed.wellbeing.0,
        "unhoused {} vs housed {}",
        unhoused.wellbeing.0,
        housed.wellbeing.0
    );
}

// ---------- S4/S8: income and government by household ----------

#[test]
fn benefits_go_to_the_unemployed_and_the_retired() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p, 1).unwrap();
    sim_engine::step(&mut sim).unwrap();
    let h = &sim.state.households[7]; // an unemployed adult and a retired person
    let expected = sim.state.policy.benefit.scale(2.0);
    assert_eq!(h.income.benefits, expected);
    // A household whose adults work and whose only other member is a child gets none.
    assert_eq!(sim.state.households[4].income.benefits, Cents::ZERO);
}

#[test]
fn purchases_are_recorded_household_to_firm() {
    let mut sim = new_sim(42);
    let out = sim_engine::step(&mut sim).unwrap();
    let n = out
        .events
        .iter()
        .filter(|e| matches!(e, Event::Purchase { .. }))
        .count();
    assert!(
        n >= 8,
        "households buy from several firms: {n} purchase events"
    );
}

// ---------- sign tests (causal map) ----------

fn no_lobby(mut p: Params) -> Params {
    p.lobby.spend_rate = 0.0;
    p
}

fn gross_emissions(s: &Sim) -> f64 {
    s.state.nature.gross_emissions.0
}

#[test]
fn raising_carbon_price_lowers_emissions_after_it_takes_effect() {
    let make = |seed| sim_engine::init(no_lobby(params()), seed).unwrap();
    let hike = cmd(Command::SetPolicy(PolicyChange::CarbonPrice(
        Cents::from_dollars(150.0),
    )));
    let (mut lower, mut n) = (0, 0);
    for seed in 1..=4 {
        let (mut base, mut taxed) = (make(seed), make(seed));
        let b = collect(&mut base, 72, &Plan::default(), gross_emissions);
        let t = collect(&mut taxed, 72, &hike, gross_emissions);
        n += 1;
        if mean(&t[36..]) < mean(&b[36..]) {
            lower += 1;
        }
    }
    assert_eq!(
        lower, n,
        "a carbon price must lower emissions in every seed"
    );
}

#[test]
fn raising_minimum_wage_lifts_the_floor_and_does_not_lower_unemployment() {
    // The floor binds: after the rise, no employed person is paid less than the new minimum.
    // (Wage share is not asserted: in a town this small a higher floor costs jobs, and the net
    // effect on the wage share is ambiguous; see docs/deviations.md.)
    let up = cmd(Command::SetPolicy(PolicyChange::MinWage(
        Cents::from_dollars(1400.0),
    )));
    let un = |s: &Sim| s.state.indicators.unemployment.0;
    let (mut ub, mut uh) = (vec![], vec![]);
    for seed in 1..=4 {
        ub.push(mean(
            &collect(&mut new_sim(seed), 36, &Plan::default(), un)[12..],
        ));
        let mut sim = new_sim(seed);
        uh.push(mean(&collect(&mut sim, 36, &up, un)[12..]));
        let s = &sim.state;
        for x in s.persons.iter().filter(|x| x.job.is_some()) {
            let f = &s.firms[x.job.unwrap().idx()];
            assert!(
                sim_model::util::person_pay(s, &sim.params, x, f.wage)
                    >= Cents::from_dollars(1400.0)
            );
        }
    }
    assert!(
        mean(&uh) >= mean(&ub) - 0.02,
        "unemployment {} vs {}",
        mean(&uh),
        mean(&ub)
    );
}

#[test]
fn raising_base_rate_lowers_investment_and_raises_the_price_of_credit() {
    let invest =
        |s: &Sim| s.state.gdp_parts.investment.dollars() + s.state.firms[0].capex.dollars();
    let hike = cmd(Command::SetPolicy(PolicyChange::BaseRate(Rate(0.15))));
    let (mut a, mut b) = (new_sim(42), new_sim(42));
    let (ia, ib) = (
        collect(&mut a, 36, &Plan::default(), invest),
        collect(&mut b, 36, &hike, invest),
    );
    assert!(
        ib.iter().sum::<f64>() < ia.iter().sum::<f64>(),
        "investment {} vs {}",
        ib.iter().sum::<f64>(),
        ia.iter().sum::<f64>()
    );
    let rate = |s: &Sim| s.state.firms[0].loan_rate.0;
    let (mut a, mut b) = (new_sim(42), new_sim(42));
    let ra = collect(&mut a, 36, &Plan::default(), rate);
    let rb = collect(&mut b, 36, &hike, rate);
    assert!(rb.last().unwrap() > ra.last().unwrap());
}

#[test]
fn harvesting_above_sustainable_yield_eventually_lowers_the_commons() {
    let commons = |s: &Sim| s.state.nature.commons.0;
    let heavy = cmd(Command::SetPolicy(PolicyChange::CommonsHarvest(Rate(3.0))));
    let (mut a, mut b) = (new_sim(42), new_sim(42));
    let ca = collect(&mut a, 240, &Plan::default(), commons);
    let cb = collect(&mut b, 240, &heavy, commons);
    assert!(cb[239] < ca[239], "commons {} vs {}", cb[239], ca[239]);
    assert!(
        cb[239] < 0.5 * cb[0],
        "heavy harvest should shrink the commons: {} -> {}",
        cb[0],
        cb[239]
    );
}

#[test]
fn raising_the_lobbying_cap_lowers_the_effective_carbon_price() {
    let eff = |cap: f64| {
        let mut sim = new_sim(42);
        sim.state.policy.carbon_price = Cents::from_dollars(100.0);
        sim.state.policy.campaign_cap = Rate(cap);
        let v = collect(&mut sim, 12, &Plan::default(), |s| {
            s.state.effective.carbon_price.dollars()
        });
        *v.last().unwrap()
    };
    let (capped, open) = (eff(0.0), eff(1.0));
    assert!(
        open < capped,
        "effective carbon price: open {open:.2} vs capped {capped:.2}"
    );
}

// ---------- economic systems ----------

#[test]
fn market_cannot_set_campaign_cap_but_regulated_can() {
    let mut sim = new_sim(1);
    let out = sim_engine::submit_turn(
        &mut sim,
        &[Command::SetPolicy(PolicyChange::CampaignCap(Rate(0.01)))],
    )
    .unwrap();
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].reason, BlockReason::LeverUnavailable);
    assert_eq!(sim.state.policy.campaign_cap, Rate(1.0));
}

#[test]
fn switching_system_is_slow_and_changes_ownership() {
    let mut sim = new_sim(42);
    let sc = Plan::from_json(SCENARIO_COOP).unwrap();
    sim_engine::run(&mut sim, 6, &sc, |_, _| {}).unwrap();
    assert_eq!(
        sim.state.system,
        EconomicSystem::Market,
        "still in transition"
    );
    assert!(sim.state.transition.is_some());
    assert!(
        sim.state.output_penalty.0 > 0.0,
        "temporary productivity drop"
    );
    sim_engine::run(&mut sim, 12, &sc, |_, _| {}).unwrap();
    assert_eq!(sim.state.system, EconomicSystem::Coop);
    assert!(sim
        .state
        .firms
        .iter()
        .filter(|f| f.active)
        .all(|f| f.owner == Owner::Workers));
}

// ---------- turn layer ----------

fn carbon(d: f64) -> Command {
    Command::SetPolicy(PolicyChange::CarbonPrice(Cents::from_dollars(d)))
}

#[test]
fn queued_policy_is_pending_then_active_after_its_lead_time() {
    let mut sim = new_sim(1);
    let lead = sim.params.turn.lever("carbon_price").lead_time;
    assert_eq!(lead, 2);
    let pc0 = sim.state.political_capital;
    let out = sim_engine::submit_turn(&mut sim, &[carbon(80.0)]).unwrap();
    assert_eq!(out.accepted.len(), 1);
    assert_eq!(sim.state.pending.len(), 1);
    assert_eq!(out.accepted[0].effective_turn, 1 + lead);
    assert_eq!(
        sim.state.policy.carbon_price,
        Cents::ZERO,
        "systems read only active values"
    );
    assert!(
        out.pc_after < pc0 + sim.params.turn.pc_base,
        "cost was charged"
    );
    let snap = serde_json::to_value(sim_engine::snapshot(&sim)).unwrap();
    assert_eq!(snap["pending"].as_array().unwrap().len(), 1);
    sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert_eq!(
        sim.state.policy.carbon_price,
        Cents::ZERO,
        "still pending after one turn"
    );
    let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert_eq!(out.activated.len(), 1);
    assert_eq!(sim.state.policy.carbon_price, Cents::from_dollars(80.0));
    assert!(sim.state.pending.is_empty());
}

#[test]
fn preview_matches_submit_and_does_not_mutate() {
    let mut sim = new_sim(1);
    let before = serde_json::to_string(&sim.state).unwrap();
    let cmds = [
        carbon(100.0),
        Command::Place {
            kind: InfraKind::RenewablePlant,
            tile: Tile::new(11, 11),
        },
    ];
    let costs = sim_engine::preview(&sim, &cmds);
    assert_eq!(before, serde_json::to_string(&sim.state).unwrap());
    assert!(
        costs.iter().all(|c| c.blocked.is_none() && c.tile_valid),
        "{costs:?}"
    );
    let pc0 = sim.state.political_capital;
    let out = sim_engine::submit_turn(&mut sim, &cmds).unwrap();
    let charged: f64 = out.accepted.iter().map(|c| c.pc_cost).sum();
    assert!((charged - costs.iter().map(|c| c.pc).sum::<f64>()).abs() < 1e-9);
    let regen = sim.params.turn.pc_base * sim.state.gov.legitimacy.0;
    assert!((out.pc_after - (pc0 - charged + regen).min(sim.params.turn.pc_cap)).abs() < 0.5);
}

#[test]
fn unaffordable_commands_are_rejected_and_reported() {
    let mut sim = new_sim(1);
    sim.state.political_capital = 5.0;
    let out = sim_engine::submit_turn(
        &mut sim,
        &[Command::SwitchSystem(EconomicSystem::Regulated)],
    )
    .unwrap();
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(
        out.rejected[0].reason,
        BlockReason::InsufficientPoliticalCapital
    );
    assert!(sim.state.pending.is_empty());
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, Event::PolicyBlocked { .. })));
}

#[test]
fn cancelling_a_pending_command_refunds_half() {
    let mut sim = new_sim(1);
    let out = sim_engine::submit_turn(&mut sim, &[carbon(80.0)]).unwrap();
    let c = out.accepted[0];
    let pc = sim.state.political_capital;
    let refund = sim_engine::cancel_pending(&mut sim, c.id).unwrap();
    assert!((refund - 0.5 * c.pc_cost).abs() < 1e-9);
    assert!((sim.state.political_capital - (pc + refund)).abs() < 1e-9);
    assert!(sim.state.pending.is_empty());
    assert!(sim_engine::cancel_pending(&mut sim, c.id).is_err());
}

#[test]
fn net_lobbying_raises_the_political_cost_of_opposed_policies() {
    let mut sim = new_sim(1);
    sim.state.net_lobby = Rate(0.0);
    let base = sim_engine::preview(&sim, &[carbon(100.0)])[0].pc;
    sim.state.net_lobby = Rate(0.5);
    let opposed = sim_engine::preview(&sim, &[carbon(100.0)])[0].pc;
    assert!((opposed - 1.5 * base).abs() < 1e-9);
    let benefit = Command::SetPolicy(PolicyChange::Benefit(Cents::from_dollars(300.0)));
    let other = sim_engine::preview(&sim, &[benefit])[0].pc;
    sim.state.net_lobby = Rate(0.0);
    let other0 = sim_engine::preview(&sim, &[benefit])[0].pc;
    assert!(
        (other - other0).abs() < 1e-9,
        "benefits are not lobbied against"
    );
}

#[test]
fn political_capital_is_capped_and_regenerates_with_legitimacy() {
    let mut sim = new_sim(1);
    for _ in 0..20 {
        sim_engine::submit_turn(&mut sim, &[]).unwrap();
    }
    assert!(sim.state.political_capital <= sim.params.turn.pc_cap + 1e-9);
    sim.state.political_capital = 10.0;
    sim.state.gov.legitimacy = Rate(1.0);
    let hi = {
        let mut a = sim.clone();
        sim_engine::submit_turn(&mut a, &[]).unwrap();
        a.state.political_capital
    };
    sim.state.gov.legitimacy = Rate(0.1);
    sim.state.political_capital = 10.0;
    sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert!(hi > sim.state.political_capital);
}

// ---------- strict flows and the demand levers ----------

#[test]
fn household_credit_flag_must_be_off() {
    let bad = PARAMS.replace("\"household_credit\": false", "\"household_credit\": true");
    assert_ne!(bad, PARAMS, "flag present in content");
    assert!(load(&bad, TOWN).is_err());
}

#[test]
fn sector_balances_hold_in_every_tick_of_a_run_with_policy() {
    // The engine checks the identity after every system; this run exercises taxes, subsidies,
    // a job guarantee, a system switch and public spending.
    let mut sim = new_sim(3);
    let plan = Plan::from_json(
        r#"{"turns":[
            {"turn":1,"set":{"job_guarantee":0.5,"green_subsidy":0.3}},
            {"turn":3,"switch":"Regulated"},
            {"turn":6,"set":{"public_spending":12000,"benefit":1000}}]}"#,
    )
    .unwrap();
    sim_engine::run(&mut sim, 40, &plan, |_, _| {}).expect("balances hold");
    assert!(sim.state.policy.job_guarantee.0 > 0.0);
}

#[test]
fn job_guarantee_employs_people_and_lowers_unemployment() {
    let u = |jg: bool| {
        let mut all = vec![];
        for seed in 1..=4 {
            let plan = if jg {
                Plan::from_json(r#"{"turns":[{"turn":1,"set":{"job_guarantee":1.0}}]}"#).unwrap()
            } else {
                Plan::default()
            };
            let v = collect(&mut new_sim(seed), 36, &plan, |s| {
                s.state.indicators.unemployment.0
            });
            all.push(mean(&v[12..]));
        }
        mean(&all)
    };
    assert!(u(true) < u(false) - 0.02, "{} vs {}", u(true), u(false));
}

#[test]
fn government_spending_cannot_vanish_silently() {
    // With a G far beyond the treasury's means, the G lever is cut, and the cut is reported.
    let mut sim = new_sim(42);
    sim.state.policy.public_spending = Cents::from_dollars(500_000.0);
    let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, Event::SpendingCut { .. })));
    assert!(sim.state.gov.unfunded_total > Cents::ZERO);
}
