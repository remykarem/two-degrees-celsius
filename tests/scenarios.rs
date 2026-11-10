//! Scenario runs, invariant tests, sign tests and the golden snapshot for the whole simulation.
//! Town v2: 20 people in 8 households, a 12 x 12 map owned by the simulation.

use sim_checks::check_state;
use sim_engine::{series, Plan, Sim};
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
    let unknown = PARAMS.replace("\"turns\": 40,", "\"turns\": 40, \"bogus\": 1,");
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
        assert_eq!(sim.state.tick, turns * sim.params.run.ticks_per_turn);
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
    sim_engine::run(&mut a, 40, &sc, |_, _| {}).unwrap();
    sim_engine::run(&mut b, 40, &sc, |_, _| {}).unwrap();
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
    // Town v4: droughts (S11) lower farm output and so the harvest itself, so the decline is
    // shallower than before weather (it used to fall below half; it now ends at about 0.54).
    assert!(
        cb[239] < 0.6 * cb[0],
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

// ---------- space: tiles, roads, commutes ----------

fn place(kind: InfraKind, x: u8, y: u8) -> Command {
    Command::Place {
        kind,
        tile: Tile::new(x, y),
    }
}

#[test]
fn preview_reports_tile_validity_without_changing_state() {
    let sim = new_sim(1);
    let before = serde_json::to_string(&sim.state).unwrap();
    let cases = [
        // (command, tile valid, reason)
        (place(InfraKind::Road, 11, 11), true, None),
        (
            place(InfraKind::Road, 3, 2),
            false,
            Some(BlockReason::TileOccupied),
        ), // a home
        (
            place(InfraKind::Road, 9, 3),
            false,
            Some(BlockReason::TileOccupied),
        ), // the Factory
        (
            place(InfraKind::Road, 5, 6),
            false,
            Some(BlockReason::TileOccupied),
        ), // a road
        (
            place(InfraKind::PublicHousing, 0, 0),
            false,
            Some(BlockReason::TileInvalid),
        ), // forest
        (
            place(InfraKind::PublicHousing, 9, 9),
            false,
            Some(BlockReason::TileInvalid),
        ), // commons
        (
            place(InfraKind::Road, 12, 0),
            false,
            Some(BlockReason::TileInvalid),
        ), // off the map
        (
            Command::Remove {
                tile: Tile::new(11, 11),
            },
            false,
            Some(BlockReason::NothingToRemove),
        ),
        (
            Command::Remove {
                tile: Tile::new(3, 2),
            },
            false,
            Some(BlockReason::TileInvalid),
        ), // private home
        (
            Command::Remove {
                tile: Tile::new(10, 2),
            },
            true,
            None,
        ), // fossil plant
        (
            Command::Remove {
                tile: Tile::new(5, 6),
            },
            true,
            None,
        ), // road
    ];
    for (c, ok, reason) in cases {
        let cost = &sim_engine::preview(&sim, &[c])[0];
        assert_eq!(cost.tile_valid, ok, "{c:?}");
        assert_eq!(cost.tile_reason, reason, "{c:?}");
        if !ok {
            assert!(cost.blocked.is_some(), "{c:?} must be blocked");
        }
    }
    assert_eq!(before, serde_json::to_string(&sim.state).unwrap());
    // valid_tiles lists exactly the empty tiles.
    let valid = sim_engine::turn::valid_tiles(&sim, InfraKind::Road);
    assert!(valid.contains(&Tile::new(11, 11)) && !valid.contains(&Tile::new(3, 2)));
    let empties = sim
        .state
        .map
        .cells
        .iter()
        .filter(|c| c.kind == TileKind::Empty)
        .count();
    assert_eq!(valid.len(), empties);
}

#[test]
fn a_second_order_on_the_same_tile_is_rejected_and_rejections_are_reported() {
    let mut sim = new_sim(1);
    let out = sim_engine::submit_turn(
        &mut sim,
        &[
            place(InfraKind::Road, 11, 11),
            place(InfraKind::RenewablePlant, 11, 11),
            place(InfraKind::Road, 3, 2),
        ],
    )
    .unwrap();
    assert_eq!(out.accepted.len(), 1);
    assert_eq!(out.rejected.len(), 2);
    assert!(out
        .rejected
        .iter()
        .all(|r| r.reason == BlockReason::TileOccupied));
}

#[test]
fn roads_are_built_on_their_tile_cost_money_and_need_upkeep() {
    let mut sim = new_sim(1);
    let tile = Tile::new(11, 11);
    let cost = sim_engine::preview(&sim, &[place(InfraKind::Road, 11, 11)])[0];
    assert_eq!(
        cost.money,
        Cents::from_dollars(sim.params.government.road_cost_usd)
    );
    sim_engine::submit_turn(&mut sim, &[place(InfraKind::Road, 11, 11)]).unwrap();
    assert_eq!(
        sim.state.map.kind_at(tile),
        Some(TileKind::Empty),
        "not built before its lead time"
    );
    let lead = sim.params.turn.lever("place_road").lead_time;
    let mut built = false;
    for _ in 0..=lead + 1 {
        let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
        built |= out.events.contains(&Event::Built {
            kind: InfraKind::Road,
            tile,
        });
    }
    assert!(built);
    assert_eq!(sim.state.map.kind_at(tile), Some(TileKind::Road));
    // The tile is now occupied for the next order.
    assert_eq!(
        sim_engine::preview(&sim, &[place(InfraKind::Road, 11, 11)])[0].tile_reason,
        Some(BlockReason::TileOccupied)
    );
}

#[test]
fn a_road_shortens_a_commute_and_raises_effective_hours() {
    let mut sim = new_sim(1);
    // The Factory worker who lives at (2, 6): roads along row 6 and up column 9 shorten the route.
    let worker = sim
        .state
        .persons
        .iter()
        .find(|x| {
            x.job == Some(FirmId(0))
                && sim.state.households[x.household.idx()].home == Some(HomeId(3))
        })
        .expect("someone at the Factory lives in the home at (2, 6)");
    let (id, before) = (
        worker.id,
        (worker.commute_minutes, worker.effective_hours.0),
    );
    let roads = [(3, 6), (4, 6), (9, 6), (9, 5), (9, 4)];
    let cmds: Vec<Command> = roads
        .iter()
        .map(|&(x, y)| place(InfraKind::Road, x, y))
        .collect();
    sim_engine::submit_turn(&mut sim, &cmds).unwrap();
    for _ in 0..4 {
        sim_engine::submit_turn(&mut sim, &[]).unwrap();
    }
    for &(x, y) in &roads {
        assert_eq!(sim.state.map.kind_at(Tile::new(x, y)), Some(TileKind::Road));
    }
    let after = sim.state.person(id);
    println!(
        "road example: commute {} -> {} minutes, effective hours {:.3} -> {:.3}",
        before.0, after.commute_minutes, before.1, after.effective_hours.0
    );
    assert!(
        after.commute_minutes < before.0,
        "commute {} -> {} minutes",
        before.0,
        after.commute_minutes
    );
    assert!(after.effective_hours.0 > before.1);
    assert_eq!(
        after.commute_tiles, 10,
        "same distance in tiles, fewer minutes"
    );
}

#[test]
fn commutes_are_reported_per_person_each_month() {
    let mut sim = new_sim(1);
    let out = sim_engine::step(&mut sim).unwrap();
    let commuters = out
        .events
        .iter()
        .filter(|e| matches!(e, Event::Commuted { .. }))
        .count();
    let jobs = sim.state.persons.iter().filter(|x| x.job.is_some()).count();
    assert_eq!(commuters, jobs);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, Event::Commuted { minutes, tiles, .. } if *minutes >= *tiles)));
}

#[test]
fn public_housing_is_built_on_the_tile_and_the_unhoused_move_in() {
    let mut sim = new_sim(1);
    assert!(sim.state.households[7].home.is_none());
    let tile = Tile::new(4, 5);
    sim_engine::submit_turn(&mut sim, &[place(InfraKind::PublicHousing, 4, 5)]).unwrap();
    let mut moved = false;
    for _ in 0..8 {
        let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
        moved |= out
            .events
            .iter()
            .any(|e| matches!(e, Event::HouseholdMoved { household, .. } if *household == HouseholdId(7)));
    }
    assert_eq!(sim.state.map.kind_at(tile), Some(TileKind::PublicHousing));
    let home = sim
        .state
        .homes
        .iter()
        .find(|h| h.tile == tile)
        .expect("home exists");
    assert_eq!(home.owner, HomeOwner::Public);
    assert!(moved, "the destitute household takes the new home");
    assert_eq!(sim.state.households[7].home, Some(home.id));
    assert_eq!(sim.state.housing.unhoused_households, 0);
}

#[test]
fn removing_public_housing_evicts_its_household_and_clears_the_tile() {
    let mut sim = new_sim(1);
    sim_engine::submit_turn(&mut sim, &[place(InfraKind::PublicHousing, 4, 5)]).unwrap();
    for _ in 0..8 {
        sim_engine::submit_turn(&mut sim, &[]).unwrap();
    }
    assert!(sim.state.households[7].home.is_some());
    sim_engine::submit_turn(
        &mut sim,
        &[Command::Remove {
            tile: Tile::new(4, 5),
        }],
    )
    .unwrap();
    let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert_eq!(
        sim.state.map.kind_at(Tile::new(4, 5)),
        Some(TileKind::Empty)
    );
    assert!(sim.state.homes.iter().all(|h| h.tile != Tile::new(4, 5)));
    assert!(sim.state.households[7].home.is_none());
    let _ = out;
}

// ---------- plants: direct emissions, attribution, rationing ----------

#[test]
fn emissions_are_counted_once_and_attributed_to_plants() {
    let mut sim = new_sim(1);
    let out = sim_engine::step(&mut sim).unwrap();
    let s = &sim.state;
    let firms: f64 = s.firms.iter().map(|f| f.emissions.0).sum();
    let hh: f64 = s.households.len() as f64 * 0.0
        + s.persons.len() as f64 * sim.params.nature.household_footprint_base;
    let plants: f64 = s.plants.iter().map(|p| p.emissions.0).sum();
    assert!(plants > 0.0, "fossil plants emit");
    assert!(
        (s.nature.gross_emissions.0 - (firms + hh + plants)).abs() < 1e-9,
        "no double counting"
    );
    // The trace names every plant and its tonnes.
    let traced: f64 = out
        .trace
        .iter()
        .filter_map(|t| match t {
            TraceRecord::Emissions {
                source: EmissionSource::Plant(_),
                tonnes,
            } => Some(tonnes.0),
            _ => None,
        })
        .sum();
    assert!((traced - plants).abs() < 1e-9);
    // Fossil emissions equal the energy delivered times the fossil share times the factor.
    let delivered = s.energy.delivered.0;
    assert!((plants - delivered * s.energy.grid_intensity.0).abs() < 1e-9);
}

#[test]
fn renewable_plants_emit_nothing_and_a_cleaner_mix_cuts_emissions() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut dirty = sim_engine::init(p.clone(), 1).unwrap();
    let mut clean = sim_engine::init(p, 1).unwrap();
    // Swap both fossil plants for renewable ones (the same capacity, so no shortage).
    for pl in &mut clean.state.plants {
        pl.kind = PlantKind::Renewable;
    }
    for t in [Tile::new(10, 2), Tile::new(10, 4)] {
        clean.state.map.set(
            t,
            MapTile {
                kind: TileKind::RenewablePlant,
                id: clean.state.map.get(t).unwrap().id,
            },
        );
    }
    clean.state.energy.grid_intensity = Rate(0.0);
    let d = collect(&mut dirty, 6, &Plan::default(), gross_emissions);
    let c = collect(&mut clean, 6, &Plan::default(), gross_emissions);
    assert!(clean.state.plants.iter().all(|p| p.emissions.0 == 0.0));
    assert!(
        mean(&c) < 0.6 * mean(&d),
        "clean {} vs dirty {}",
        mean(&c),
        mean(&d)
    );
}

#[test]
fn removing_the_power_plants_rations_output() {
    let mut sim = new_sim(1);
    let supply0 = sim.state.energy.supply.0;
    assert!(
        sim.state.energy.demand.0 <= supply0,
        "no shortage at the start"
    );
    let rationed = |s: &Sim| s.state.energy.served.0;
    // Remove one of the two fossil plants: supply halves and demand exceeds it.
    sim_engine::submit_turn(
        &mut sim,
        &[Command::Remove {
            tile: Tile::new(10, 2),
        }],
    )
    .unwrap();
    let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert_eq!(sim.state.plants.len(), 1);
    assert!(sim.state.energy.supply.0 < supply0);
    if sim.state.energy.demand.0 > sim.state.energy.supply.0 {
        assert!(rationed(&sim) < 1.0);
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, Event::ElectricityRationed { .. })));
    }
    // With both gone nothing can be produced.
    sim_engine::submit_turn(
        &mut sim,
        &[Command::Remove {
            tile: Tile::new(10, 4),
        }],
    )
    .unwrap();
    sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert!(sim.state.plants.is_empty());
    assert_eq!(sim.state.energy.served, Rate(0.0));
    assert!(sim
        .state
        .firms
        .iter()
        .filter(|f| f.active)
        .all(|f| f.output.0 == 0.0));
}

#[test]
fn living_next_to_a_fossil_plant_or_the_factory_costs_wellbeing() {
    // Worker A lives at (8, 4), next to the Factory and a fossil plant; the effect is a param.
    let wb = |penalty: f64| {
        let mut p = params();
        p.wellbeing.w_local_pollution = penalty;
        let mut sim = sim_engine::init(p, 1).unwrap();
        for _ in 0..12 {
            sim_engine::step(&mut sim).unwrap();
        }
        sim.state.households[4].wellbeing.0
    };
    let (with, without) = (wb(0.04), wb(0.0));
    assert!(with < without, "with {with} vs without {without}");
}

#[test]
fn long_commutes_cost_wellbeing() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let mut near = sim_engine::init(p.clone(), 1).unwrap();
    let mut far = sim_engine::init(p, 1).unwrap();
    // Make every worker commute a long time by turning the free minutes and speed up or down.
    far.params.space.slow_minutes = 12;
    for _ in 0..8 {
        sim_engine::step(&mut near).unwrap();
        sim_engine::step(&mut far).unwrap();
    }
    let avg = |s: &Sim| s.state.persons.iter().map(|x| x.wellbeing.0).sum::<f64>();
    assert!(avg(&far) < avg(&near));
    let out = |s: &Sim| s.state.firms[0].output.0 + s.state.firms[1].output.0;
    assert!(
        out(&far) <= out(&near),
        "slower commutes never raise output"
    );
}

// ---------- baselines over many seeds (causal model section 9, Town v2) ----------

const SCENARIO_B: &str = include_str!("../content/scenarios/baseline_b_sensible.json");
const SCENARIO_C: &str = include_str!("../content/scenarios/baseline_c_neglect.json");
const SCENARIO_D: &str = include_str!("../content/scenarios/baseline_d_carbon_only.json");
const SEEDS: std::ops::RangeInclusive<u64> = 1..=20;

struct Run {
    outcome: Outcome,
    /// Tick of the first loss, if any.
    loss_tick: Option<u32>,
    /// Tick at which the cumulative emissions passed the budget, if they did.
    exhausted_tick: Option<u32>,
    /// Tick at which half of the budget was gone, if it was.
    half_tick: Option<u32>,
    legitimacy_min: f64,
    /// Lowest legitimacy before the budget ran out (the run keeps going afterwards).
    legitimacy_min_before_exhaustion: f64,
    budget_left_share: f64,
    gini_start: f64,
}

fn play(script: &Plan, seed: u64) -> Run {
    let mut sim = new_sim(seed);
    let gini_start = sim.state.indicators.gini.0;
    let mut exhausted = None;
    let mut half = None;
    let mut legit_min = 1.0f64;
    let mut legit_pre = 1.0f64;
    let turns = sim.params.run.turns;
    sim_engine::run(&mut sim, turns, script, |s, _| {
        legit_min = legit_min.min(s.state.gov.legitimacy.0);
        if exhausted.is_none() {
            legit_pre = legit_pre.min(s.state.gov.legitimacy.0);
        }
        if half.is_none() && s.state.nature.cumulative.0 >= 0.5 * s.params.nature.budget_b0 {
            half = Some(s.state.tick);
        }
        if exhausted.is_none() && s.state.nature.cumulative.0 >= s.params.nature.budget_b0 {
            exhausted = Some(s.state.tick);
        }
    })
    .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
    Run {
        outcome: sim_engine::final_outcome(&sim),
        loss_tick: sim.state.outcome.map(|(t, _)| t),
        exhausted_tick: exhausted,
        half_tick: half,
        legitimacy_min: legit_min,
        legitimacy_min_before_exhaustion: legit_pre,
        budget_left_share: sim.state.indicators.remaining_budget.0 / sim.params.nature.budget_b0,
        gini_start,
    }
}

fn runs(script: &Plan) -> Vec<Run> {
    SEEDS.map(|seed| play(script, seed)).collect()
}

fn share(rs: &[Run], f: impl Fn(&Run) -> bool) -> f64 {
    rs.iter().filter(|r| f(r)).count() as f64 / rs.len() as f64
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Baseline A: no action. Horizon: a 40-turn game (10 years, 2026 to 2036) and a 900 t budget
/// (`nature.budget_b0`): half is gone around turn 14 and all of it around turn 27.5, in every
/// seed. The loss is the budget, not an early collapse: legitimacy and wellbeing are still
/// middling when it ends. The start is as unequal as the design asks.
#[test]
fn baseline_a_no_action_runs_the_budget_out_by_turn_30() {
    let rs = runs(&Plan::default());
    let turns: Vec<f64> = rs
        .iter()
        .filter_map(|r| r.exhausted_tick)
        .map(|t| t as f64 / 3.0)
        .collect();
    let halves: Vec<f64> = rs
        .iter()
        .filter_map(|r| r.half_tick)
        .map(|t| t as f64 / 3.0)
        .collect();
    println!(
        "A: half turns {halves:?}, exhausted turns {turns:?}, outcomes {:?}, legitimacy min {:?}",
        rs.iter().map(|r| r.outcome).collect::<Vec<_>>(),
        rs.iter().map(|r| r.legitimacy_min).collect::<Vec<_>>()
    );
    assert!(
        share(&rs, |r| r.gini_start > 0.65 && r.gini_start < 0.78) >= 1.0,
        "start gini"
    );
    assert!(turns.len() >= 18, "the budget runs out in nearly every run");
    assert!(
        share(&rs, |r| r
            .exhausted_tick
            .is_some_and(|t| (78..=90).contains(&t)))
            >= 0.9,
        "exhausted between turns 26 and 30"
    );
    let m = median(turns);
    assert!((26.0..=30.0).contains(&m), "median exhaustion turn {m}");
    let h = median(halves);
    assert!((13.0..=15.0).contains(&h), "median half-budget turn {h}");
    // The climate-damage and legitimacy terms must not make A an early collapse (the budget,
    // not the town, ends it).
    assert!(
        share(&rs, |r| r.legitimacy_min_before_exhaustion > 0.3) >= 0.8,
        "legitimacy stays above 0.3 in A until the budget ends the game"
    );
}

/// Baseline B (sensible play) in the 40-turn, 900 t game. HONEST RESULT, not a win. By carbon
/// alone B has a margin: emissions fall with lags (a carbon price takes 2 turns, a plant 4), the
/// budget is never exhausted and about 18 to 20% of it is left when the game stops (it would need
/// more than about 800 t). But the town collapses first: climate damage to farm output
/// (`production.climate_damage` x (cumulative / b0)^2) is about 15% by turn 28, the farm fails,
/// unemployment jumps to 70% and the bank fails (`BankFailureCascade`; turn 30 with the weather
/// off, turns 22 to 26 with the Horizon weather, which stresses the farm from 25% of the budget
/// used). With
/// `climate_damage` = 0 the same script survives all 40 turns with 13% left. B survives only with
/// a budget of about 1,500 t or more, which lets A survive too. Not forced by tuning behaviour;
/// see docs/tuning-log.md ("Horizon") and docs/known-issues.md. When the model is rebalanced so B
/// wins, flip the collapse assertion.
#[test]
fn baseline_b_sensible_play_keeps_budget_margin_but_the_town_collapses_before_turn_40() {
    let rs = runs(&Plan::from_json(SCENARIO_B).unwrap());
    let a = runs(&Plan::default());
    println!(
        "B: survived {:.0}%, outcomes {:?}, median budget left {:.2}, median legitimacy min {:.2}",
        100.0 * share(&rs, |r| matches!(r.outcome, Outcome::Survived)),
        rs.iter().map(|r| r.outcome).collect::<Vec<_>>(),
        median(rs.iter().map(|r| r.budget_left_share).collect()),
        median(rs.iter().map(|r| r.legitimacy_min).collect())
    );
    assert!(
        share(&rs, |r| r.exhausted_tick.is_none()) >= 0.9,
        "B never exhausts the budget inside 40 turns"
    );
    let left = median(rs.iter().map(|r| r.budget_left_share).collect());
    assert!(
        (0.05..=0.35).contains(&left),
        "B keeps a positive carbon margin at the end: {left}"
    );
    assert!(
        a.iter().all(|r| !matches!(r.outcome, Outcome::Survived)),
        "A always fails"
    );
    assert!(
        share(&a, |r| matches!(r.outcome, Outcome::BudgetExhausted)) >= 0.9,
        "A fails by the budget (in a rare seed a bank failure at the same time)"
    );
    // Known problem (docs/known-issues.md): the economy fails before turn 40.
    assert!(
        share(&rs, |r| matches!(
            r.outcome,
            Outcome::Collapse(CollapseReason::BankFailureCascade)
        )) >= 0.9,
        "B ends in a bank-failure cascade (known issue, not a win)"
    );
}

/// Baseline C: neglect and extraction fails inside the budget horizon (by tick 84, turn 28) in most seeds, and
/// burns the budget no more slowly than doing nothing.
#[test]
fn baseline_c_neglect_fails_inside_the_budget_horizon_in_most_seeds() {
    let rs = runs(&Plan::from_json(SCENARIO_C).unwrap());
    println!(
        "C: loss ticks {:?}",
        rs.iter().map(|r| r.loss_tick).collect::<Vec<_>>()
    );
    assert!(share(&rs, |r| r.loss_tick.is_some_and(|t| t <= 84)) >= 0.7);
    let a = runs(&Plan::default());
    let a_exh = median(
        a.iter()
            .filter_map(|r| r.exhausted_tick)
            .map(|t| t as f64)
            .collect(),
    );
    let c_exh = rs
        .iter()
        .filter_map(|r| r.exhausted_tick)
        .map(|t| t as f64)
        .collect::<Vec<_>>();
    // Extraction burns the budget no more slowly than doing nothing.
    assert!(c_exh.is_empty() || median(c_exh) <= a_exh + 3.0);
}

/// Baseline D: one tool (a carbon tax) is not enough; it also fails inside the horizon (budget
/// exhausted at turn 28.6).
#[test]
fn baseline_d_carbon_tax_alone_fails_in_most_seeds() {
    let rs = runs(&Plan::from_json(SCENARIO_D).unwrap());
    println!(
        "D: outcomes {:?}, median budget left {:.2}",
        rs.iter().map(|r| r.outcome).collect::<Vec<_>>(),
        median(rs.iter().map(|r| r.budget_left_share).collect())
    );
    assert!(share(&rs, |r| !matches!(r.outcome, Outcome::Survived)) >= 0.7);
}

#[test]
fn start_gini_and_top_share_are_in_range_in_every_seed() {
    for seed in SEEDS {
        let i = new_sim(seed).state.indicators;
        assert!(
            (0.65..=0.78).contains(&i.gini.0),
            "seed {seed}: gini {}",
            i.gini.0
        );
        assert!(
            (0.55..=0.72).contains(&i.top_wealth_share.0),
            "seed {seed}: top share {}",
            i.top_wealth_share.0
        );
    }
}

#[test]
fn regulated_scenario_emits_less_than_market() {
    let reg = Plan::from_json(SCENARIO_REGULATED).unwrap();
    let (mut m, mut r) = (vec![], vec![]);
    for seed in 1..=4 {
        let (mut a, mut b) = (new_sim(seed), new_sim(seed));
        sim_engine::run(&mut a, 40, &Plan::default(), |_, _| {}).unwrap();
        sim_engine::run(&mut b, 40, &reg, |_, _| {}).unwrap();
        m.push(a.state.nature.cumulative.0);
        r.push(b.state.nature.cumulative.0);
    }
    assert!(
        mean(&r) < mean(&m),
        "regulated {} vs market {}",
        mean(&r),
        mean(&m)
    );
}

#[test]
fn coop_scenario_has_lower_inequality_than_market() {
    let coop = Plan::from_json(SCENARIO_COOP).unwrap();
    let (mut m, mut c) = (vec![], vec![]);
    for seed in 1..=4 {
        let (mut a, mut b) = (new_sim(seed), new_sim(seed));
        sim_engine::run(&mut a, 40, &Plan::default(), |_, _| {}).unwrap();
        sim_engine::run(&mut b, 40, &coop, |_, _| {}).unwrap();
        m.push(a.state.indicators.gini.0);
        c.push(b.state.indicators.gini.0);
    }
    assert!(
        mean(&c) < mean(&m),
        "coop gini {} vs market {}",
        mean(&c),
        mean(&m)
    );
}

// ---------- golden snapshot ----------

#[test]
fn golden_snapshot_seed_42() {
    let mut sim = new_sim(42);
    let mut out = String::new();
    out.push_str(series::CSV_HEADER);
    out.push('\n');
    sim_engine::run(&mut sim, 40, &Plan::default(), |s, o| {
        if s.state.tick % 24 == 0 {
            out.push_str(&series::csv_row(s, &o.trace));
            out.push('\n');
        }
    })
    .unwrap();
    insta::assert_snapshot!("market_seed_42", out);
}
