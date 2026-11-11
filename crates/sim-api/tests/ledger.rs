//! Town v5 ledger through the API: reconciliation, roll-up, memory bound, save/load, person
//! ledgers, finance totals traced to ledger entries.
use sim_api::content;
use sim_api::finance::FinanceFlows;
use sim_api::keys::LeverKey;
use sim_api::ledger::{LedgerKindKey, LedgerPage};
use sim_api::*;
use sim_params::ContentJson;

fn with_detail(turns: u32) -> String {
    assert!(content::LEVERS.contains("\"detail_turns\": 8"));
    content::LEVERS.replace("\"detail_turns\": 8", &format!("\"detail_turns\": {turns}"))
}

fn api_with(seed: u64, detail: u32) -> Api {
    let c: ContentJson = content::default_content();
    Api::from_content(&c, &with_detail(detail), seed).expect("init")
}

fn plan(i: u32) -> Vec<Command> {
    // A varied game: taxes, spending, builds, benefits.
    let n = |lever, v| Command::SetPolicy {
        lever,
        value: PolicyValue::Number(v),
    };
    match i {
        0 => vec![n(LeverKey::CarbonPrice, 60.0), n(LeverKey::TaxWage, 0.15)],
        2 => vec![
            n(LeverKey::Benefit, 800.0),
            n(LeverKey::PublicSpending, 30_000.0),
        ],
        4 => vec![n(LeverKey::JobGuarantee, 0.4)],
        6 => vec![n(LeverKey::GreenSubsidy, 0.4)],
        _ => vec![],
    }
}

fn accounts(api: &Api) -> Vec<AccountRef> {
    let s = api.snapshot();
    let mut v: Vec<AccountRef> = (0..s.households.len() as u32)
        .map(|id| AccountRef::Household { id })
        .collect();
    v.extend((0..s.firms.len() as u32).map(|id| AccountRef::Firm { id }));
    v.push(AccountRef::Government);
    v.push(AccountRef::Bank);
    v
}

fn page(api: &Api, a: &AccountRef) -> LedgerPage {
    api.ledger(a, &LedgerQuery::default()).unwrap()
}

/// Play `turns` turns of the scripted game (stops at game over).
fn play(api: &mut Api, turns: u32) {
    for i in 0..turns {
        if api.submit_turn(&plan(i)).is_err() {
            break;
        }
    }
}

#[test]
fn every_account_reconciles_after_every_turn() {
    for seed in [1u64, 2, 3] {
        let mut api = api_with(seed, 8);
        for i in 0..40 {
            if api.submit_turn(&plan(i)).is_err() {
                break;
            }
            for a in accounts(&api) {
                let p = page(&api, &a);
                assert!(
                    p.reconciles,
                    "seed {seed} turn {i}: {a:?} does not reconcile"
                );
            }
        }
    }
}

type Totals = std::collections::BTreeMap<(String, LedgerKindKey), (i64, i64, i64, i64)>;

/// Exact totals per (counterparty, kind) over entries and roll-ups, in cents.
fn totals(p: &LedgerPage) -> Totals {
    let c = |d: f64| (d * 100.0).round() as i64;
    let mut t = Totals::new();
    let me = serde_json::to_string(&p.account).unwrap();
    for e in &p.entries {
        let (from, to) = (
            serde_json::to_string(&e.from).unwrap(),
            serde_json::to_string(&e.to).unwrap(),
        );
        // Counterparty rule (also used by the roll-up): the other end of the entry; for the bank
        // as a bystander of a loan, the borrower.
        let cp = if from == me {
            &e.to
        } else if to == me {
            &e.from
        } else if from.contains("external") {
            &e.to
        } else {
            &e.from
        };
        let key = (serde_json::to_string(cp).unwrap(), e.kind);
        let x = t.entry(key).or_default();
        if to == me {
            x.0 += c(e.amount);
        } else if from == me {
            x.1 += c(e.amount);
        }
        x.2 += 1;
    }
    for r in &p.summary {
        let key = (serde_json::to_string(&r.counterparty).unwrap(), r.kind);
        let x = t.entry(key).or_default();
        x.0 += c(r.inflow);
        x.1 += c(r.outflow);
        x.2 += i64::from(r.count);
    }
    t
}

#[test]
fn rolling_up_preserves_totals_exactly() {
    let mut full = api_with(5, 200);
    let mut rolled = api_with(5, 2);
    play(&mut full, 30);
    play(&mut rolled, 30);
    assert_eq!(full.snapshot().turn, rolled.snapshot().turn);
    for a in accounts(&full) {
        let (pf, pr) = (page(&full, &a), page(&rolled, &a));
        assert!(pf.reconciles && pr.reconciles, "{a:?}");
        assert_eq!(pf.opening_balance, pr.opening_balance);
        assert_eq!(pf.closing_balance, pr.closing_balance);
        assert_eq!(pf.closing_debt, pr.closing_debt);
        // The dormant firm slot has no transactions at all.
        if !pf.entries.is_empty() {
            assert!(
                pr.entries.len() < pf.entries.len(),
                "{a:?} was not rolled up"
            );
            assert!(!pr.summary.is_empty());
        }
        // Same totals per counterparty and kind, to the cent.
        assert_eq!(totals(&pf), totals(&pr), "{a:?}");
        // Roll-up rows are monthly and only for turns before the detail window.
        assert!(pr.summary.iter().all(|r| r.turn < pr.detail_from_turn));
        assert!(pr.entries.iter().all(|e| e.turn >= pr.detail_from_turn));
    }
}

#[test]
fn ledger_memory_is_bounded_over_a_40_turn_game() {
    let mut api = api_with(7, 8);
    let mut sizes = Vec::new();
    for i in 0..40 {
        if api.submit_turn(&plan(i)).is_err() {
            break;
        }
        sizes.push((i, api.ledger_size()));
    }
    let max_entries = sizes.iter().map(|s| s.1.entries).max().unwrap();
    let last = sizes.last().unwrap().1;
    println!(
        "ledger after turn {}: {last:?}; max detail entries {max_entries}",
        sizes.len()
    );
    // Full detail never exceeds 8 turns' worth (about 8 x 3 x 150 entries); it stops growing once
    // the window is full.
    let at_10 = sizes[9].1.entries;
    assert!(max_entries < at_10 * 14 / 10, "{max_entries} vs {at_10}");
    // Roll-ups are monthly rows: far fewer than the entries they replace, and they grow only
    // with the number of turns, not with the number of transactions.
    assert!(last.rollups < 14 * 3 * 40 * 12);
    assert!(last.earnings <= 20 * 3 * 8 + 20);
    let blob = api.export_state().unwrap();
    println!("save blob {} bytes at turn {}", blob.len(), sizes.len());
}

#[test]
fn save_and_load_round_trips_the_ledger() {
    let mut a = api_with(11, 4);
    play(&mut a, 15);
    let blob = a.export_state().unwrap();
    let mut b = api_with(99, 4);
    b.import_state(&blob).unwrap();
    for acct in accounts(&a) {
        assert_eq!(page(&a, &acct), page(&b, &acct), "{acct:?}");
    }
    assert_eq!(a.person_ledger(3).unwrap(), b.person_ledger(3).unwrap());
    assert_eq!(a.ledger_size(), b.ledger_size());
    assert_eq!(
        b.export_state().unwrap(),
        blob,
        "export after import is identical"
    );
    // They stay identical as the game goes on.
    for i in 15..20 {
        a.submit_turn(&plan(i)).unwrap();
        b.submit_turn(&plan(i)).unwrap();
    }
    assert_eq!(a.export_state().unwrap(), b.export_state().unwrap());
    for acct in accounts(&a) {
        assert!(page(&b, &acct).reconciles);
    }
}

#[test]
fn old_saves_and_corrupt_ledgers_are_refused() {
    let mut a = api_with(11, 8);
    play(&mut a, 3);
    let blob = a.export_state().unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    assert_eq!(v["schemaVersion"], 5);
    // A schema 3 save (no ledger).
    let mut old = v.clone();
    old["schemaVersion"] = 3.into();
    old.as_object_mut().unwrap().remove("ledger");
    let e = a.import_state(&old.to_string()).unwrap_err();
    assert_eq!(e.code, "schema_mismatch");
    assert!(e.message.contains("ledger"), "{}", e.message);
    // A tampered ledger (an entry amount changed) no longer matches the saved state.
    let entry = &mut v["ledger"]["entries"][0];
    entry["amount"] = (entry["amount"].as_i64().unwrap() + 1).into();
    let e = a.import_state(&v.to_string()).unwrap_err();
    assert_eq!(e.code, "bad_state");
    assert!(e.message.contains("ledger"), "{}", e.message);
    // The game is untouched by the failures.
    assert!(page(&a, &AccountRef::Government).reconciles);
}

#[test]
fn live_turns_and_submit_turn_give_the_same_ledger() {
    let mut a = api_with(13, 8);
    let mut b = api_with(13, 8);
    for i in 0..6 {
        a.submit_turn(&plan(i)).unwrap();
        b.begin_turn(&plan(i)).unwrap();
        for _ in 0..3 {
            b.advance_month().unwrap();
            // Entries of the open turn are visible and reconcile mid-turn.
            assert!(page(&b, &AccountRef::Bank).reconciles);
        }
        b.finish_turn().unwrap();
    }
    assert_eq!(a.export_state().unwrap(), b.export_state().unwrap());
}

#[test]
fn query_filters_by_turn_and_kind_without_breaking_the_running_balance() {
    let mut api = api_with(21, 8);
    play(&mut api, 6);
    let h = AccountRef::Household { id: 0 };
    let all = page(&api, &h);
    let q = LedgerQuery {
        from_turn: Some(3),
        to_turn: Some(4),
        kind: Some(LedgerKindKey::Purchase),
    };
    let some = api.ledger(&h, &q).unwrap();
    assert!(!some.entries.is_empty());
    assert!(some
        .entries
        .iter()
        .all(|e| (3..=4).contains(&e.turn) && e.kind == LedgerKindKey::Purchase));
    // balanceAfter is the same number the unfiltered page shows for that entry.
    for e in &some.entries {
        let twin = all
            .entries
            .iter()
            .find(|x| x.tick == e.tick && x.memo == e.memo && x.amount == e.amount)
            .unwrap();
        assert_eq!(twin.balance_after, e.balance_after);
    }
    // Entries follow the running balance: each balanceAfter differs from the last by the effect.
    let mut bal = all.opening_balance;
    for e in &all.entries {
        let delta = if e.to == h { e.amount } else { -e.amount };
        let cash = !matches!(e.kind, LedgerKindKey::LoanWriteoff);
        if cash {
            bal += delta;
        }
        assert!(
            (bal - e.balance_after).abs() < 0.011,
            "{bal} vs {}",
            e.balance_after
        );
    }
    assert!(api
        .ledger(&AccountRef::Household { id: 99 }, &LedgerQuery::default())
        .is_err());
    assert!(api.person_ledger(999).is_err());
}

#[test]
fn person_ledger_lists_wages_credited_to_the_household_and_employment_events() {
    let mut api = api_with(4, 8);
    play(&mut api, 5);
    let snap = api.snapshot();
    let worker = snap
        .people
        .iter()
        .find(|p| p.job.is_some())
        .expect("someone has a job");
    let pl = api.person_ledger(worker.id).unwrap();
    assert_eq!(pl.household, worker.household);
    assert!(!pl.earnings.is_empty());
    for e in &pl.earnings {
        assert_eq!(e.kind, LedgerKindKey::Wage);
        assert_eq!(
            e.to,
            AccountRef::Household {
                id: worker.household
            }
        );
        assert!(matches!(
            e.from,
            AccountRef::Firm { .. } | AccountRef::Government
        ));
    }
    // Everything the household's members earned adds up to the household's wage entries from
    // firms (wages detail), to the cent.
    let members: Vec<u32> = snap
        .people
        .iter()
        .filter(|p| p.household == worker.household)
        .map(|p| p.id)
        .collect();
    let earned: i64 = members
        .iter()
        .flat_map(|m| api.person_ledger(*m).unwrap().earnings)
        .map(|e| (e.amount * 100.0).round() as i64)
        .sum();
    let hp = page(
        &api,
        &AccountRef::Household {
            id: worker.household,
        },
    );
    let credited: i64 = hp
        .entries
        .iter()
        .filter(|e| e.kind == LedgerKindKey::Wage && e.detail.as_deref() == Some("wages"))
        .map(|e| (e.amount * 100.0).round() as i64)
        .sum();
    assert_eq!(earned, credited);
    assert_eq!(pl.events.len(), pl.event_turns.len());
}

fn cents(d: f64) -> i64 {
    (d * 100.0).round() as i64
}

#[test]
fn finance_totals_equal_the_sums_of_ledger_entries() {
    let mut api = api_with(8, 8);
    play(&mut api, 10);
    let snap = api.snapshot();
    let f = &snap.finance;
    let tick = snap.tick - 1;
    let q = LedgerQuery::default();
    let gov = api.ledger(&AccountRef::Government, &q).unwrap();
    let sum = |from_tick: u32,
               to_tick: u32,
               pred: &dyn Fn(&sim_api::ledger::LedgerEntry) -> bool|
     -> i64 {
        gov.entries
            .iter()
            .filter(|e| e.tick >= from_tick && e.tick <= to_tick && pred(e))
            .map(|e| cents(e.amount))
            .sum()
    };
    let into_gov = |e: &sim_api::ledger::LedgerEntry| e.to == AccountRef::Government;
    let from_gov = |e: &sim_api::ledger::LedgerEntry| e.from == AccountRef::Government;
    for (from_tick, flows) in [
        (tick, (&f.government.revenue, &f.government.spending)),
        (tick - 2, (&f.quarter.gov_revenue, &f.quarter.gov_spending)),
    ] {
        let (rev, sp) = flows;
        let detail = |d: &'static str| {
            move |e: &sim_api::ledger::LedgerEntry| into_gov(e) && e.detail.as_deref() == Some(d)
        };
        assert_eq!(
            cents(rev.wage_tax),
            sum(from_tick, tick, &detail("wage tax"))
        );
        assert_eq!(
            cents(rev.profit_tax),
            sum(from_tick, tick, &detail("profit tax"))
        );
        assert_eq!(
            cents(rev.capital_tax),
            sum(from_tick, tick, &detail("capital tax"))
        );
        assert_eq!(
            cents(rev.carbon),
            sum(from_tick, tick, &detail("carbon price"))
        );
        let rev_total: i64 = cents(rev.wage_tax)
            + cents(rev.profit_tax)
            + cents(rev.capital_tax)
            + cents(rev.wealth_tax)
            + cents(rev.carbon)
            + cents(rev.other);
        // Revenue = every transfer into the treasury (loans are not revenue).
        let inflows = sum(from_tick, tick, &|e| {
            into_gov(e) && e.kind != LedgerKindKey::LoanNew
        });
        assert_eq!(rev_total, inflows);
        let sp_detail = |d: &'static str| {
            move |e: &sim_api::ledger::LedgerEntry| from_gov(e) && e.detail.as_deref() == Some(d)
        };
        assert_eq!(
            cents(sp.benefits),
            sum(from_tick, tick, &sp_detail("benefit"))
        );
        assert_eq!(
            cents(sp.public_spending),
            sum(from_tick, tick, &sp_detail("public spending"))
        );
        assert_eq!(
            cents(sp.upkeep),
            sum(from_tick, tick, &|e| from_gov(e)
                && e.kind == LedgerKindKey::Upkeep)
        );
        assert_eq!(
            cents(sp.builds),
            sum(from_tick, tick, &|e| from_gov(e)
                && e.kind == LedgerKindKey::Build)
        );
        let sp_total = cents(sp.benefits)
            + cents(sp.ubi)
            + cents(sp.public_spending)
            + cents(sp.job_guarantee)
            + cents(sp.upkeep)
            + cents(sp.subsidy)
            + cents(sp.interest)
            + cents(sp.builds)
            + cents(sp.other);
        // Spending = every outflow of the treasury except principal repaid.
        let outflows = sum(from_tick, tick, &|e| {
            from_gov(e) && e.kind != LedgerKindKey::LoanRepay
        });
        assert_eq!(sp_total, outflows);
    }
    // Revenue minus spending is the change in the government's net position over the month
    // (cash minus debt): shown by the ledger's own balance and debt columns.
    let last: Vec<_> = gov.entries.iter().filter(|e| e.tick == tick).collect();
    let prev_tick_entry = gov.entries.iter().rev().find(|e| e.tick < tick).unwrap();
    let net_before = prev_tick_entry.balance_after - prev_tick_entry.debt_after;
    let after = last.last().unwrap();
    let net_after = after.balance_after - after.debt_after;
    assert!(
        (net_after - net_before - f.government.balance).abs() < 0.011,
        "{} vs {}",
        net_after - net_before,
        f.government.balance
    );
}

#[test]
fn finance_households_and_firms_agree_with_the_state() {
    let mut api = api_with(9, 8);
    play(&mut api, 8);
    let snap = api.snapshot();
    let f = &snap.finance;
    let s = api.sim();
    let wages: i64 = s.state.households.iter().map(|h| h.income.wages.0).sum();
    let divs: i64 = s
        .state
        .households
        .iter()
        .map(|h| h.income.dividends.0)
        .sum();
    let rent: i64 = s
        .state
        .households
        .iter()
        .map(|h| h.income.rent_received.0)
        .sum();
    let int: i64 = s
        .state
        .households
        .iter()
        .map(|h| h.income.interest_received.0)
        .sum();
    let ben: i64 = s.state.households.iter().map(|h| h.income.benefits.0).sum();
    assert_eq!(cents(f.households.income.wages), wages);
    assert_eq!(cents(f.households.income.dividends), divs);
    assert_eq!(cents(f.households.income.rent), rent);
    assert_eq!(cents(f.households.income.interest), int);
    assert_eq!(cents(f.households.income.benefits), ben);
    let spending: i64 = s.state.households.iter().map(|h| h.income.spending.0).sum();
    let tax: i64 = s.state.households.iter().map(|h| h.income.tax_paid.0).sum();
    let rent_paid: i64 = s
        .state
        .households
        .iter()
        .map(|h| h.income.rent_paid.0)
        .sum();
    let loan: i64 = s
        .state
        .households
        .iter()
        .map(|h| h.income.loan_payments.0)
        .sum();
    assert_eq!(cents(f.households.spending), spending);
    assert_eq!(cents(f.households.taxes), tax);
    assert_eq!(cents(f.households.rent_paid), rent_paid);
    assert_eq!(cents(f.households.loan_payments), loan);
    let revenue: i64 = s.state.firms.iter().map(|x| x.revenue.0).sum();
    // Revenue here is money received: a firm buying capital goods from itself (the Factory is
    // the only goods firm) records sales in the model but moves no money, so the ledger's
    // figure is at most the model's `firm.revenue` and the gap is those self-purchases.
    assert!(cents(f.firms.revenue) <= revenue && cents(f.firms.revenue) * 10 >= revenue * 8);
    assert!(f.bank.capital_ratio > 0.0 && f.bank.capital_ratio_limit > 0.0);
    assert!(f.money.supply > 0.0 && f.money.created_this_turn > 0.0);
    assert!(
        (0.0..=1.0).contains(&f.wage_share) && (f.wage_share + f.profit_share - 1.0).abs() < 1e-9
    );
}

#[test]
fn quarter_is_the_sum_of_three_months() {
    let mut api = api_with(6, 8);
    api.submit_turn(&plan(0)).unwrap();
    api.begin_turn(&plan(1)).unwrap();
    let mut sum = FinanceFlows::default();
    for _ in 0..3 {
        api.advance_month().unwrap();
        let m = api.snapshot().finance;
        sum.gov_revenue.wage_tax += m.government.revenue.wage_tax;
        sum.household_spending += m.households.spending;
        sum.firm_revenue += m.firms.revenue;
        sum.bank_interest_income += m.bank.interest_income;
    }
    let q = api.snapshot().finance.quarter;
    assert!((q.gov_revenue.wage_tax - sum.gov_revenue.wage_tax).abs() < 0.011);
    assert!((q.household_spending - sum.household_spending).abs() < 0.011);
    assert!((q.firm_revenue - sum.firm_revenue).abs() < 0.011);
    assert!((q.bank_interest_income - sum.bank_interest_income).abs() < 0.011);
}

#[test]
fn history_points_carry_the_new_fields_and_energy_is_labelled() {
    let mut api = api_with(2, 8);
    play(&mut api, 4);
    let s = api.snapshot();
    let h = s.history.last().unwrap();
    assert!(h.gov_revenue > 0.0 && h.gov_spending > 0.0);
    assert!(h.bank_capital_ratio > 0.0);
    assert!((0.0..=1.0).contains(&h.renewable_share) && (0.0..=1.0).contains(&h.grid_served));
    assert_eq!(s.energy.unit, "MWh");
    assert_eq!(s.energy.supply.len(), s.plants.len());
    let d = &s.energy.demand;
    assert!((d.firms + d.households - d.total).abs() < 1e-9);
    assert_eq!(s.schema_version, 5);
}
