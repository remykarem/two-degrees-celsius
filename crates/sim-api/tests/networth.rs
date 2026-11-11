//! Town v8 (contract v2.2): the net worth statement reconciles to the headline net worth.
use sim_api::networth::{NetWorthCategory, HOLDER_IDS, LINE_IDS};
use sim_api::snapshot::Snapshot;
use sim_api::*;

fn check(s: &Snapshot, what: &str) {
    let g = &s.breakdowns.gini;
    assert_eq!(g.by_household.len(), s.households.len(), "{what}");
    let mut headline: f64 = 0.0;
    for w in &g.by_household {
        let st = &w.statement;
        assert!(
            st.reconciles,
            "{what}: household {} does not reconcile",
            w.household
        );
        let tol = 1e-6 * st.assets.max(1.0);
        assert!(
            (st.net_worth - w.net_worth).abs() <= tol,
            "{what}: net worth vs headline"
        );
        assert!(
            (st.assets - st.liabilities - st.net_worth).abs() <= tol,
            "{what}"
        );
        let a: f64 = st
            .lines
            .iter()
            .filter(|l| l.category == NetWorthCategory::Asset)
            .map(|l| l.value)
            .sum();
        let l: f64 = st
            .lines
            .iter()
            .filter(|l| l.category == NetWorthCategory::Liability)
            .map(|l| l.value)
            .sum();
        assert!(
            (a - st.assets).abs() <= tol && (l - st.liabilities).abs() <= tol,
            "{what}"
        );
        // The household view's net worth is the same figure.
        let hv = s.households.iter().find(|h| h.id == w.household).unwrap();
        assert!(
            (hv.net_worth - st.net_worth).abs() <= tol,
            "{what}: HouseholdView.netWorth"
        );
        for line in &st.lines {
            assert!(LINE_IDS.contains(&line.id.as_str()), "{what}: {}", line.id);
            assert!(
                line.value != 0.0 && line.how_id.is_some(),
                "{what}: {}",
                line.id
            );
            assert!(
                (line.per_person * f64::from(st.people) - line.value).abs() <= tol,
                "{what}"
            );
        }
        assert!(HOLDER_IDS.contains(&w.holder.as_str()));
        headline += w.net_worth;
    }
    let t = &g.town;
    assert!(
        t.reconciles,
        "{what}: town does not reconcile ({} vs {})",
        t.holders_total, t.headline
    );
    assert!(
        (t.headline - headline).abs() <= 1e-6 * headline.abs().max(1.0),
        "{what}"
    );
    assert_eq!(t.holders.len(), HOLDER_IDS.len());
    let hh: u32 = t
        .holders
        .iter()
        .filter(|h| h.in_headline)
        .map(|h| h.households)
        .sum();
    let pp: u32 = t
        .holders
        .iter()
        .filter(|h| h.in_headline)
        .map(|h| h.people)
        .sum();
    assert_eq!(
        hh as usize,
        s.households.len(),
        "{what}: every household is in one group"
    );
    assert_eq!(pp as usize, s.people.len(), "{what}");
    let gov = t.holders.iter().find(|h| h.id == "government").unwrap();
    assert!(!gov.in_headline);
    assert!(
        (gov.net_worth - (s.government.cash - s.government.debt)).abs()
            <= 1e-6 * s.government.cash.abs().max(1.0)
    );
}

#[test]
fn every_household_reconciles_in_several_seeds_and_turns() {
    for seed in [1u64, 7, 42, 99, 2026] {
        let mut api = Api::init(seed).unwrap();
        check(&api.snapshot(), &format!("seed {seed} start"));
        for turn in 1..=40 {
            if api.snapshot().game_over.is_some() {
                break;
            }
            api.submit_turn(&[]).unwrap();
            if turn % 4 == 0 || turn <= 3 {
                check(&api.snapshot(), &format!("seed {seed} turn {turn}"));
            }
        }
    }
}

#[test]
fn live_months_reconcile_too() {
    let mut api = Api::init(5).unwrap();
    api.begin_turn(&[]).unwrap();
    for m in 0..3 {
        let r = api.advance_month().unwrap();
        check(&r.snapshot, &format!("live month {m}"));
    }
}

#[test]
fn the_estate_holds_the_jet_and_labels_come_from_content() {
    let api = Api::init(7).unwrap();
    let s = api.snapshot();
    let g = &s.breakdowns.gini;
    let e = s.estate.as_ref().unwrap();
    let w = g
        .by_household
        .iter()
        .find(|w| w.household == e.household)
        .unwrap();
    assert_eq!(w.holder, "estate");
    let jet = w.statement.lines.iter().find(|l| l.id == "jet").unwrap();
    assert_eq!(jet.value, e.jet.value);
    assert_eq!(jet.category, NetWorthCategory::Asset);
    let defs = &api.catalog().unwrap().net_worth_lines;
    assert_eq!(
        jet.label,
        defs.iter().find(|d| d.id == "jet").unwrap().label
    );
    // Debt is a liability line; the bank's owner holds the bank's equity.
    assert!(g
        .by_household
        .iter()
        .any(|w| w.holder == "bank" && w.statement.lines.iter().any(|l| l.id == "bank_equity")));
}

#[test]
fn content_with_a_missing_or_unknown_line_is_refused() {
    use sim_api::networth::NetWorthDefs;
    let c = api_catalog();
    let mut lines = c.net_worth_lines.clone();
    let holders = c.net_worth_holders.clone();
    assert!(NetWorthDefs::new(lines.clone(), holders.clone()).is_ok());
    let mut extra = lines[0].clone();
    extra.id = "crypto".into();
    lines.push(extra);
    assert!(
        NetWorthDefs::new(lines.clone(), holders.clone()).is_err(),
        "unknown id"
    );
    lines.pop();
    lines.pop();
    assert!(NetWorthDefs::new(lines, holders).is_err(), "missing id");
}

fn api_catalog() -> sim_api::catalog::Catalog {
    Api::init(1).unwrap().catalog().unwrap()
}

#[test]
fn a_worker_cooperative_shares_firm_lines_and_still_reconciles() {
    use sim_api::keys::SystemKey;
    let mut api = Api::init(3).unwrap();
    api.submit_turn(&[Command::SwitchSystem {
        to: SystemKey::Coop,
    }])
    .unwrap();
    for turn in 0..14 {
        api.submit_turn(&[]).unwrap();
        check(&api.snapshot(), &format!("coop turn {turn}"));
    }
}
