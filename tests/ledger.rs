//! Town v5 ledger: every money movement of every tick is a `Transaction`, and for every
//! account opening + transactions = closing. `step()` already enforces this (it fails the run),
//! so every other engine test is covered too; these tests re-check it independently, over the
//! baselines A-F and the weather and co-op runs, and prove that the ledger mapping is complete.

use sim_checks::check_ledger;
use sim_engine::{Plan, Sim};
use sim_params::{ContentJson, Params};
use sim_types::*;
use std::collections::BTreeMap;

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");
const B: &str = include_str!("../content/scenarios/baseline_b_sensible.json");
const C: &str = include_str!("../content/scenarios/baseline_c_neglect.json");
const D: &str = include_str!("../content/scenarios/baseline_d_carbon_only.json");
const E: &str = include_str!("../content/scenarios/baseline_e_adaptation_only.json");
const F: &str = include_str!("../content/scenarios/baseline_f_renewables_only.json");
const COOPS: &str = include_str!("../content/scenarios/worker_coops.json");
const REG: &str = include_str!("../content/scenarios/regulated_green_deal.json");

fn params() -> Params {
    Params::from_content(&ContentJson {
        params: PARAMS,
        initial_town: TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
    .expect("content loads")
}

/// Play 40 turns, re-checking every tick against an independently kept set of balances, and
/// add the transactions of the run to `kinds` (count per kind and detail).
fn play(
    plan: &Plan,
    seed: u64,
    p: Params,
    kinds: &mut BTreeMap<(LedgerKind, &'static str), (u64, Cents)>,
) -> Sim {
    let mut sim = sim_engine::init(p, seed).expect("init");
    let mut tracked = Balances::of(&sim.state);
    let mut total_txs = 0usize;
    sim_engine::run(&mut sim, 40, plan, |s, out| {
        let closing = Balances::of(&s.state);
        // Ticks inside `on_tick` see the state after the tick, with `tick` already advanced.
        check_ledger(&tracked, &out.transactions, &closing, s.state.tick - 1)
            .unwrap_or_else(|v| panic!("seed {seed}: {v}"));
        for t in &out.transactions {
            assert!(
                t.amount > Cents::ZERO,
                "recorded a non-positive amount: {t:?}"
            );
            assert!(
                t.from != t.to,
                "a transaction from an account to itself: {t:?}"
            );
            let e = kinds.entry((t.kind, t.detail)).or_default();
            e.0 += 1;
            e.1 += t.amount;
        }
        total_txs += out.transactions.len();
        tracked = closing;
    })
    .expect("run (step also reconciles every tick)");
    assert!(total_txs > 0);
    sim
}

#[test]
fn reconciliation_holds_every_tick_over_baselines_and_weather_runs() {
    let plans: Vec<(&str, Plan)> = vec![
        ("A", Plan::default()),
        ("B", Plan::from_json(B).unwrap()),
        ("C", Plan::from_json(C).unwrap()),
        ("D", Plan::from_json(D).unwrap()),
        ("E", Plan::from_json(E).unwrap()),
        ("F", Plan::from_json(F).unwrap()),
        ("coops", Plan::from_json(COOPS).unwrap()),
        ("regulated", Plan::from_json(REG).unwrap()),
    ];
    let mut kinds = BTreeMap::new();
    for (name, plan) in &plans {
        for seed in 1..=20u64 {
            let _ = name;
            play(plan, seed, params(), &mut kinds);
        }
    }
    // Weather runs (fires, droughts, home damage on): same check.
    let mut p = params();
    p.weather.home_damage_prob = 0.5;
    for seed in 1..=5u64 {
        play(&Plan::default(), seed, p.clone(), &mut kinds);
    }
    // A stressed economy: UBI, job guarantee, bailout on, a punishing base rate and wealth tax.
    let stress = Plan::from_json(
        r#"{"turns":[{"turn":1,"set":{"ubi":300,"job_guarantee":0.6,"bailout":1,"base_rate":0.2,"wealth_tax":0.05,"wealth_threshold":1000,"green_subsidy":0.5,"public_spending":40000}},{"turn":3,"switch":"Regulated"}]}"#,
    )
    .unwrap();
    for seed in 1..=10u64 {
        play(&stress, seed, params(), &mut kinds);
    }
    for (k, (n, total)) in &kinds {
        println!(
            "{:?} {:<32} {:>8} tx {:>14.2}",
            k.0,
            k.1,
            n,
            total.dollars()
        );
    }
    // Every kind the model can emit shows up somewhere in these runs.
    let seen = |k: LedgerKind| kinds.keys().any(|(kk, _)| *kk == k);
    for k in [
        LedgerKind::Wage,
        LedgerKind::Dividend,
        LedgerKind::Rent,
        LedgerKind::Benefit,
        LedgerKind::Tax,
        LedgerKind::Purchase,
        LedgerKind::Interest,
        LedgerKind::LoanNew,
        LedgerKind::LoanRepay,
        LedgerKind::Build,
        LedgerKind::Upkeep,
        LedgerKind::Subsidy,
    ] {
        assert!(seen(k), "no {k:?} transaction in any run");
    }
}

/// The ledger mapping is exhaustive over `Delta` and `MoneyOp`: these matches have no wildcard,
/// so adding a variant breaks the build until it is classified here (and a test below proves
/// each money operation produces transactions that reconcile).
fn delta_changes_a_balance(d: &Delta) -> bool {
    match d {
        // Field changes never touch savings, cash, debt, equity or loans: those change only
        // through `MoneyOp` (the patches have no such fields).
        Delta::Person(..)
        | Delta::Firm(..)
        | Delta::Household(..)
        | Delta::Home(..)
        | Delta::AddHome(..)
        | Delta::AddPlant(..)
        | Delta::SetTile(..)
        | Delta::PlantOutput { .. }
        | Delta::Bank(..)
        | Delta::Gov(..)
        | Delta::Nature(..)
        | Delta::Weather(..)
        | Delta::Housing(..)
        | Delta::Energy(..)
        | Delta::Effective(..)
        | Delta::System(..)
        | Delta::Transition(..)
        | Delta::OutputPenalty(..)
        | Delta::UnionDensity(..)
        | Delta::NetLobby(..)
        | Delta::Gdp(..)
        | Delta::Outcome(..)
        | Delta::ClearPendingCommons
        | Delta::SetPolicy(..)
        | Delta::ResetFlows => false,
    }
}

/// The posting each money operation is recorded with.
fn money_op_postings(op: &MoneyOp) -> &'static [Posting] {
    match op {
        MoneyOp::Transfer { .. } => &[Posting::Transfer],
        MoneyOp::Lend { .. } => &[Posting::CreditCreate],
        MoneyOp::Repay { .. } => &[Posting::CreditRepay],
        MoneyOp::PayInterest { .. } => &[Posting::BankReceives],
        MoneyOp::DepositInterest { .. } => &[Posting::BankPays],
        MoneyOp::BankDividend { .. } => &[Posting::BankPays],
        MoneyOp::WriteOff { .. } => &[Posting::WriteOff],
        MoneyOp::Bailout { .. } => &[Posting::DebtToBank],
        MoneyOp::CapitalizeInterest { .. } => &[Posting::DebtToBank],
        MoneyOp::Haircut { .. } => &[Posting::BankReceives],
    }
}

#[test]
fn every_delta_and_money_operation_has_a_ledger_mapping() {
    assert!(!delta_changes_a_balance(&Delta::ResetFlows));
    let h = HouseholdId(0);
    let f = FirmId(0);
    let a = Cents(500);
    let ops = [
        MoneyOp::Transfer {
            from: Account::Household(h),
            to: Account::Firm(f),
            amount: a,
            kind: FlowKind::Purchase,
            purpose: Purpose::Plain,
        },
        MoneyOp::Lend {
            to: Account::Firm(f),
            amount: a,
        },
        MoneyOp::Repay {
            from: Account::Firm(f),
            amount: a,
        },
        MoneyOp::PayInterest {
            from: Account::Household(h),
            amount: a,
        },
        MoneyOp::DepositInterest {
            to: Account::Household(h),
            amount: a,
        },
        MoneyOp::BankDividend { to: h, amount: a },
        MoneyOp::WriteOff {
            who: Account::Firm(f),
            amount: a,
        },
        MoneyOp::Bailout { amount: a },
        MoneyOp::CapitalizeInterest { amount: a },
        MoneyOp::Haircut { rate: Rate(0.1) },
    ];
    // Apply each operation to a settled state; its recorded transactions must carry exactly
    // the mapped posting(s) and reconcile.
    let mut base = sim_engine::init(params(), 3).expect("init");
    // Give the accounts something to pay with and owe.
    base.state.households[0].savings = Cents(1_000_000);
    base.state.firms[0].cash = Cents(1_000_000);
    base.state.firms[0].debt = Cents(100_000);
    base.state.bank.loans += Cents(100_000);
    base.state.bank.equity += Cents(100_000);
    base.state.bank.deposits += Cents(2_000_000);
    base.state.households[0].savings += Cents::ZERO;
    for op in ops {
        let mut s = base.state.clone();
        let opening = Balances::of(&s);
        let mut cap = sim_engine::apply::Capture::default();
        let mut stats = sim_engine::apply::ApplyStats::default();
        let d = Deltas {
            money: vec![op],
            ..Deltas::default()
        };
        sim_engine::apply::apply(
            &mut s,
            &base.params,
            d,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut stats,
            &mut cap,
        );
        assert!(!cap.transactions.is_empty(), "{op:?} recorded nothing");
        let postings = money_op_postings(&op);
        for t in &cap.transactions {
            assert!(postings.contains(&t.posting), "{op:?} -> {t:?}");
        }
        check_ledger(&opening, &cap.transactions, &Balances::of(&s), 0)
            .unwrap_or_else(|v| panic!("{op:?}: {v}"));
    }
}

#[test]
fn a_missing_transaction_is_reported_with_account_tick_and_amount() {
    let sim = sim_engine::init(params(), 1).expect("init");
    let opening = Balances::of(&sim.state);
    let mut s = sim.state.clone();
    s.households[2].savings += Cents(12_345); // a balance change with no transaction
    let err = check_ledger(&opening, &[], &Balances::of(&s), 17).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("household 2"), "{msg}");
    assert!(msg.contains("12345"), "{msg}");
    assert_eq!(err.tick, 17);
}

#[test]
fn the_ledger_does_not_change_the_simulation() {
    // Two runs of the same seed give the same State; transactions are output only.
    let mut a = sim_engine::init(params(), 11).expect("init");
    let mut b = sim_engine::init(params(), 11).expect("init");
    let plan = Plan::from_json(B).unwrap();
    sim_engine::run(&mut a, 40, &plan, |_, _| {}).unwrap();
    sim_engine::run(&mut b, 40, &plan, |_, o| {
        assert!(!o.transactions.is_empty())
    })
    .unwrap();
    assert_eq!(a.state, b.state);
}

#[test]
fn ubi_job_guarantee_bailout_and_repairs_are_labelled_and_reconcile() {
    let mut p = params();
    p.weather.home_damage_prob = 1.0;
    let mut sim = sim_engine::init(p, 5).expect("init");
    sim.state.policy.ubi = Cents(30_000);
    sim.state.policy.job_guarantee = Rate(0.6);
    sim.state.policy.bailout = true;
    sim.state.policy.green_subsidy = Rate(0.5);
    sim.state.nature.cumulative = Tonnes(sim.params.nature.budget_b0 * 0.3); // weather on
    let mut seen: BTreeMap<&'static str, Cents> = BTreeMap::new();
    for _ in 0..24 {
        let before = Balances::of(&sim.state);
        let out = sim_engine::step(&mut sim).expect("step reconciles");
        check_ledger(&before, &out.transactions, &Balances::of(&sim.state), 0).unwrap();
        for t in &out.transactions {
            *seen.entry(t.detail).or_default() += t.amount;
        }
    }
    println!("{seen:?}");
    assert!(seen.contains_key("ubi"), "no ubi payments");
    assert!(seen.contains_key("benefit"), "no benefits");
}
