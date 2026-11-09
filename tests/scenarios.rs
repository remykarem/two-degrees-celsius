//! Scenario runs, invariant tests, sign tests and the golden snapshot for the whole simulation.

use sim_engine::{series, Script, Sim};
use sim_model::class::classify;
use sim_params::{ContentJson, Params};
use sim_types::*;

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");
const SCENARIO_REGULATED: &str = include_str!("../content/scenarios/regulated_green_deal.json");
const SCENARIO_COOP: &str = include_str!("../content/scenarios/worker_coops.json");
const SCENARIO_CARBON: &str = include_str!("../content/scenarios/carbon_price_year5.json");

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

/// Run `n` ticks, collecting one value per tick with `f`.
fn collect(sim: &mut Sim, n: u32, script: &Script, f: impl Fn(&Sim) -> f64) -> Vec<f64> {
    let mut v = Vec::new();
    sim_engine::run(sim, n, script, |s, _| v.push(f(s))).expect("checks pass");
    v
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

fn cmd(c: Command) -> Script {
    Script {
        name: "test".into(),
        actions: vec![sim_engine::script::Action { tick: 0, command: Some(c), ..Default::default() }],
    }
}

// ---------- setup ----------

#[test]
fn content_loads_and_town_matches_decisions() {
    let mut p = params();
    p.run.warmup_ticks = 0;
    let sim = sim_engine::init(p, 1).unwrap();
    let s = &sim.state;
    assert_eq!(s.households.len(), 6, "D1: six cohorts");
    assert_eq!(s.firms.len(), 4, "D2: four firm slots");
    assert_eq!(s.firms.iter().filter(|f| f.active).count(), 2, "D2: two active, two dormant");
    assert_eq!(s.total_people().0, 1000.0);
    assert_eq!(s.bank.equity, s.bank.loans - s.bank.deposits);
    assert!((s.indicators.gini.0 - 0.40).abs() < 0.05, "initial gini {}", s.indicators.gini.0);
}

#[test]
fn bad_content_is_rejected() {
    let bad = PARAMS.replace("\"forest_max\": 100000.0", "\"forest_max\": -1.0");
    let r = Params::from_content(&ContentJson {
        params: &bad,
        initial_town: TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    });
    assert!(r.is_err());
    let unknown = PARAMS.replace("\"ticks\": 480,", "\"ticks\": 480, \"bogus\": 1,");
    assert!(Params::from_content(&ContentJson {
        params: &unknown,
        initial_town: TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
    .is_err());
}

// ---------- invariants ----------

#[test]
fn money_conserved_and_stocks_sane_over_full_run() {
    // The engine runs sim-checks after every system: finite numbers, non-negative stocks,
    // exact money conservation, constant population. Any violation aborts the run.
    let mut sim = new_sim(42);
    let ticks = sim.params.run.ticks;
    sim_engine::run(&mut sim, ticks, &Script::default(), |_, _| {}).expect("no invariant violated");
    assert_eq!(sim.state.tick, 480);
    let s = &sim.state;
    assert_eq!(s.bank.equity, s.bank.loans - s.bank.deposits);
    assert_eq!(s.bank.deposits, s.total_deposits());
    assert!((s.total_people().0 - 1000.0).abs() < 1e-6);
}

#[test]
fn same_seed_gives_identical_state() {
    let mut a = new_sim(7);
    let mut b = new_sim(7);
    let sc = Script::from_json(SCENARIO_REGULATED).unwrap();
    sim_engine::run(&mut a, 480, &sc, |_, _| {}).unwrap();
    sim_engine::run(&mut b, 480, &sc, |_, _| {}).unwrap();
    assert_eq!(serde_json::to_string(&a.state).unwrap(), serde_json::to_string(&b.state).unwrap());
    assert_eq!(a.rng.parts(), b.rng.parts());
}

#[test]
fn class_labels_match_the_class_function_every_tick() {
    let mut sim = new_sim(42);
    for _ in 0..480 {
        sim_engine::step(&mut sim, &[]).unwrap();
        let s = &sim.state;
        for h in &s.households {
            assert_eq!(h.class, classify(s, &sim.params, h), "tick {} cohort {}", s.tick, h.label);
        }
    }
}

#[test]
fn bank_failure_is_resolved_not_hidden() {
    // Destroy the bank's equity by writing off loans; the engine must resolve it with a
    // deposit haircut (no bailout) and keep every invariant.
    let mut sim = new_sim(1);
    sim.state.policy.bailout = false;
    let loss = sim.state.bank.equity + Cents::from_dollars(500_000.0);
    // Force the loss through the same path a default uses: a firm debt written off.
    sim.state.firms[0].debt -= Cents::ZERO;
    sim.state.bank.equity -= loss;
    sim.state.bank.loans -= loss;
    sim.state.firms[0].debt -= loss.min(sim.state.firms[0].debt);
    sim.state.households[1].debt -= (loss - loss.min(Cents::from_dollars(3_000_000.0))).min(sim.state.households[1].debt);
    // Re-derive the bank books so the balance sheet is consistent but equity is negative.
    sim.state.bank.loans = sim.state.total_debts();
    sim.state.bank.deposits = sim.state.total_deposits();
    sim.state.bank.equity = sim.state.bank.loans - sim.state.bank.deposits;
    // Make the bank clearly insolvent.
    let take = sim.state.households[2].savings.scale(0.5);
    sim.state.households[2].savings += take;
    sim.state.bank.deposits += take;
    sim.state.bank.equity -= take;
    assert!(sim.state.bank.equity.is_negative(), "setup must make the bank insolvent");
    let out = sim_engine::step(&mut sim, &[]).expect("engine resolves the failure");
    assert!(out.events.contains(&Event::BankFailed));
    assert!(!sim.state.bank.equity.is_negative());
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
fn raising_carbon_price_lowers_emissions_within_12_months() {
    let make = || sim_engine::init(no_lobby(params()), 42).unwrap();
    let mut base = make();
    let mut taxed = make();
    let none = Script::default();
    let hike = cmd(Command::SetPolicy(PolicyChange::CarbonPrice(Cents::from_dollars(400.0))));
    let b = collect(&mut base, 12, &none, gross_emissions);
    let t = collect(&mut taxed, 12, &hike, gross_emissions);
    let (b2, t2) = (mean(&b[6..]), mean(&t[6..]));
    assert!(t2 < b2, "emissions months 7-12: taxed {t2:.1} vs baseline {b2:.1}");
}

#[test]
fn raising_minimum_wage_raises_wage_share_and_does_not_lower_unemployment() {
    let mut base = new_sim(42);
    let mut high = new_sim(42);
    let wage = base.state.firms[0].wage;
    let up = cmd(Command::SetPolicy(PolicyChange::MinWage(wage.scale(1.5))));
    let none = Script::default();
    let ws = |s: &Sim| s.state.indicators.wage_share.0;
    let un = |s: &Sim| s.state.indicators.unemployment.0;
    let (wb, wh) = (collect(&mut base, 24, &none, ws), collect(&mut high, 24, &up, ws));
    let mut base = new_sim(42);
    let mut high = new_sim(42);
    let (ub, uh) = (collect(&mut base, 24, &none, un), collect(&mut high, 24, &up, un));
    assert!(mean(&wh[6..]) > mean(&wb[6..]), "wage share {} vs {}", mean(&wh[6..]), mean(&wb[6..]));
    assert!(mean(&uh[6..]) >= mean(&ub[6..]) - 0.01, "unemployment {} vs {}", mean(&uh[6..]), mean(&ub[6..]));
}

#[test]
fn raising_base_rate_lowers_credit_and_investment() {
    let invest = |s: &Sim| s.state.gdp_parts.investment.dollars();
    let lent = |s: &Sim| s.state.bank.lent_this_tick.dollars();
    let none = Script::default();
    let hike = cmd(Command::SetPolicy(PolicyChange::BaseRate(Rate(0.15))));
    let (mut a, mut b) = (new_sim(42), new_sim(42));
    let (ia, ib) = (collect(&mut a, 36, &none, invest), collect(&mut b, 36, &hike, invest));
    let (mut a, mut b) = (new_sim(42), new_sim(42));
    let (la, lb) = (collect(&mut a, 36, &none, lent), collect(&mut b, 36, &hike, lent));
    assert!(ib.iter().sum::<f64>() < ia.iter().sum::<f64>(), "investment {} vs {}", ib.iter().sum::<f64>(), ia.iter().sum::<f64>());
    assert!(lb.iter().sum::<f64>() <= la.iter().sum::<f64>(), "lending {} vs {}", lb.iter().sum::<f64>(), la.iter().sum::<f64>());
}

#[test]
fn harvesting_above_sustainable_yield_eventually_lowers_the_commons() {
    let commons = |s: &Sim| s.state.nature.commons.0;
    let none = Script::default();
    let heavy = cmd(Command::SetPolicy(PolicyChange::CommonsHarvest(Rate(3.0))));
    let (mut a, mut b) = (new_sim(42), new_sim(42));
    let ca = collect(&mut a, 240, &none, commons);
    let cb = collect(&mut b, 240, &heavy, commons);
    assert!(cb[239] < ca[239], "commons {} vs {}", cb[239], ca[239]);
    assert!(cb[239] < 0.5 * cb[0], "heavy harvest should shrink the commons: {} -> {}", cb[0], cb[239]);
}

#[test]
fn raising_the_lobbying_cap_lowers_the_effective_carbon_price() {
    let eff = |cap: f64| {
        let mut sim = new_sim(42);
        sim.state.policy.carbon_price = Cents::from_dollars(100.0);
        sim.state.policy.campaign_cap = Rate(cap);
        let v = collect(&mut sim, 12, &Script::default(), |s| s.state.effective.carbon_price.dollars());
        *v.last().unwrap()
    };
    let (capped, open) = (eff(0.0), eff(1.0));
    assert!(open < capped, "effective carbon price: open {open:.2} vs capped {capped:.2}");
}

// ---------- economic systems ----------

#[test]
fn market_cannot_set_campaign_cap_but_regulated_can() {
    let mut sim = new_sim(1);
    let out = sim_engine::step(&mut sim, &[Command::SetPolicy(PolicyChange::CampaignCap(Rate(0.01)))]).unwrap();
    assert!(out.events.iter().any(|e| matches!(e, Event::PolicyBlocked { .. })));
    assert_eq!(sim.state.policy.campaign_cap, Rate(1.0));
}

#[test]
fn switching_system_is_slow_and_changes_ownership() {
    let mut sim = new_sim(42);
    let sc = Script::from_json(SCENARIO_COOP).unwrap();
    sim_engine::run(&mut sim, 6, &sc, |_, _| {}).unwrap();
    assert_eq!(sim.state.system, EconomicSystem::Market, "still in transition");
    assert!(sim.state.transition.is_some());
    assert!(sim.state.output_penalty.0 > 0.0, "temporary productivity drop");
    sim_engine::run(&mut sim, 40, &sc, |_, _| {}).unwrap();
    assert_eq!(sim.state.system, EconomicSystem::Coop);
    assert!(sim.state.firms.iter().filter(|f| f.active).all(|f| f.owner == Owner::Workers));
}

// ---------- calibration targets and scenario summaries ----------

struct Summary {
    unemployment_mean: f64,
    unemployment_max: f64,
    gini_start: f64,
    gini_end: f64,
    gdp_cagr: f64,
    exhausted_year: Option<f64>,
    final_outcome: Outcome,
    emissions_total: f64,
    legitimacy_end: f64,
}

fn summarise(script: &Script, seed: u64) -> Summary {
    let mut sim = new_sim(seed);
    let mut u = Vec::new();
    let mut g = Vec::new();
    let mut gdp = Vec::new();
    let mut exhausted = None;
    sim_engine::run(&mut sim, 480, script, |s, _| {
        u.push(s.state.indicators.unemployment.0);
        g.push(s.state.indicators.gini.0);
        gdp.push(s.state.indicators.gdp_real_annual);
        if exhausted.is_none() {
            if let Some((t, Outcome::BudgetExhausted)) = s.state.outcome {
                exhausted = Some(t as f64 / 12.0);
            }
        }
    })
    .unwrap();
    let first = mean(&gdp[..12]);
    let last = mean(&gdp[468..]);
    Summary {
        unemployment_mean: mean(&u[12..]),
        unemployment_max: u.iter().cloned().fold(0.0, f64::max),
        gini_start: g[0],
        gini_end: g[479],
        gdp_cagr: (last / first).powf(12.0 / 468.0) - 1.0,
        exhausted_year: exhausted,
        final_outcome: sim_engine::final_outcome(&sim),
        emissions_total: sim.state.nature.cumulative.0,
        legitimacy_end: sim.state.gov.legitimacy.0,
    }
}

#[test]
fn default_market_run_lands_near_the_design_targets() {
    let s = summarise(&Script::default(), 42);
    println!(
        "market: unemployment mean {:.3} max {:.3}, gini {:.3} -> {:.3}, gdp cagr {:.4}, exhausted {:?}, outcome {:?}",
        s.unemployment_mean, s.unemployment_max, s.gini_start, s.gini_end, s.gdp_cagr, s.exhausted_year, s.final_outcome
    );
    assert!((0.06..=0.12).contains(&s.unemployment_mean), "unemployment mean {}", s.unemployment_mean);
    assert!((s.gini_start - 0.40).abs() < 0.06, "gini start {}", s.gini_start);
    assert!((0.48..=0.62).contains(&s.gini_end), "gini end {}", s.gini_end);
    assert!((0.01..=0.03).contains(&s.gdp_cagr), "gdp growth {}", s.gdp_cagr);
    let y = s.exhausted_year.expect("budget should run out in the default run");
    assert!((23.0..=33.0).contains(&y), "budget exhausted in year {y}");
}

#[test]
fn regulated_scenario_emits_less_than_market() {
    let market = summarise(&Script::default(), 42);
    let reg = summarise(&Script::from_json(SCENARIO_REGULATED).unwrap(), 42);
    println!(
        "regulated: unemployment mean {:.3}, gini end {:.3}, gdp cagr {:.4}, emissions {:.0} vs market {:.0}, exhausted {:?}, outcome {:?}",
        reg.unemployment_mean, reg.gini_end, reg.gdp_cagr, reg.emissions_total, market.emissions_total, reg.exhausted_year, reg.final_outcome
    );
    assert!(reg.emissions_total < market.emissions_total);
    assert!(reg.legitimacy_end >= 0.0);
}

#[test]
fn coop_scenario_has_lower_inequality_than_market() {
    let market = summarise(&Script::default(), 42);
    let coop = summarise(&Script::from_json(SCENARIO_COOP).unwrap(), 42);
    println!(
        "coop: unemployment mean {:.3}, gini end {:.3} vs market {:.3}, gdp cagr {:.4} vs {:.4}, emissions {:.0} vs {:.0}, exhausted {:?}",
        coop.unemployment_mean, coop.gini_end, market.gini_end, coop.gdp_cagr, market.gdp_cagr, coop.emissions_total, market.emissions_total, coop.exhausted_year
    );
    assert!(coop.gini_end < market.gini_end, "coop gini {} vs market {}", coop.gini_end, market.gini_end);
    assert!(coop.gdp_cagr <= market.gdp_cagr + 0.002, "coops should not grow faster");
    // Not a free pass on emissions: a coop run without a carbon price still burns most of its budget.
    assert!(coop.emissions_total > 0.5 * market.emissions_total);
}

#[test]
fn carbon_price_script_runs() {
    let s = summarise(&Script::from_json(SCENARIO_CARBON).unwrap(), 42);
    let market = summarise(&Script::default(), 42);
    assert!(s.emissions_total < market.emissions_total);
}

// ---------- golden snapshot ----------

#[test]
fn golden_snapshot_seed_42() {
    let mut sim = new_sim(42);
    let mut out = String::new();
    out.push_str(series::CSV_HEADER);
    out.push('\n');
    sim_engine::run(&mut sim, 480, &Script::default(), |s, o| {
        if s.state.tick % 24 == 0 {
            out.push_str(&series::csv_row(s, &o.trace));
            out.push('\n');
        }
    })
    .unwrap();
    insta::assert_snapshot!("market_seed_42", out);
}
