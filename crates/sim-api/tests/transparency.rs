//! Town v5.2 (contract v1.6): breakdowns, the emissions tally, policies and industry on the real
//! sim. Parts must sum to the result (tolerance `breakdowns::parts_tolerance`) and each result
//! must equal the headline indicator.
use sim_api::breakdowns::{parts_tolerance, Breakdown};
use sim_api::keys::LeverKey;
use sim_api::log::PolicyEventKind;
use sim_api::snapshot::{PolicyScalar, Snapshot};
use sim_api::*;

fn num(lever: LeverKey, v: f64) -> Command {
    Command::SetPolicy {
        lever,
        value: PolicyValue::Number(v),
    }
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
        "{what}: {a} vs {b}"
    );
}

fn check_breakdown(b: &Breakdown, headline: f64) {
    close(
        b.result.value,
        headline,
        &format!("{} result vs headline", b.id),
    );
    if let Some(parts) = &b.parts {
        let sum: f64 = parts.iter().map(|p| p.value).sum();
        assert!(
            (sum - b.result.value).abs() <= parts_tolerance(b.result.value, parts),
            "{}: parts sum {sum} vs result {}",
            b.id,
            b.result.value
        );
        assert_eq!(b.sums_to_result, Some(true), "{}", b.id);
    }
    assert!(!b.formula.is_empty() && !b.inputs.is_empty(), "{}", b.id);
}

fn check_snapshot(s: &Snapshot) {
    let b = &s.breakdowns;
    check_breakdown(&b.gdp, s.indicators.gdp_real_annual);
    check_breakdown(&b.gpi, s.indicators.gpi_annual);
    check_breakdown(&b.wage_share, s.indicators.wage_share);
    check_breakdown(&b.profit_share, s.indicators.profit_share);
    check_breakdown(&b.debt_to_gdp, s.government.debt_to_gdp);
    check_breakdown(&b.gini.base, s.indicators.gini);
    check_breakdown(&b.legitimacy.base, s.government.legitimacy);
    check_breakdown(&b.capital_ratio, s.finance.bank.capital_ratio);
    assert_eq!(b.legitimacy.terms.len(), 5);
    // Emissions: lines tally by construction and equal the state's own headline numbers.
    let e = &b.emissions;
    assert!(e.reconciles, "emissions do not reconcile: {e:?}");
    if s.tick > 0 {
        close(e.month.net, s.nature.net_emissions_monthly, "month net");
        close(
            e.month.gross_total,
            s.nature.gross_emissions_monthly,
            "month gross",
        );
        close(e.month.sink, -s.nature.sink_monthly, "month sink");
        let lines: f64 = e
            .month_lines
            .iter()
            .filter(|l| !l.informational)
            .map(|l| l.tonnes)
            .sum();
        close(lines, e.month.net, "sum of lines");
        close(e.turn.net, e.turn.headline, "turn net vs headline");
        // Contract v2.0: the sources of a line are a list that adds up to the gross total, each
        // with a label and a category from content/ui.json.
        for l in [&e.month, &e.turn] {
            let listed: f64 = l.sources.iter().map(|x| x.tonnes).sum();
            close(listed, l.gross_total, "sources sum to gross");
            assert!(l.sources.iter().all(|x| !x.label.is_empty()));
        }
        assert!(e.turn_months >= 1 && e.turn_months <= 3);
    }
    close(
        e.cumulative.tonnes,
        s.nature.cumulative_tonnes,
        "cumulative",
    );
    close(
        e.cumulative.remaining,
        e.cumulative.budget - e.cumulative.tonnes,
        "remaining",
    );
    // Gini: Lorenz curve is increasing from (0,0) to (1,1) and its area gives the headline Gini
    // (only when nobody has negative wealth).
    let l = &b.gini.lorenz;
    if let (Some(first), Some(last)) = (l.first(), l.last()) {
        assert_eq!((first.population_share, first.wealth_share), (0.0, 0.0));
        close(last.population_share, 1.0, "lorenz end x");
        close(last.wealth_share, 1.0, "lorenz end y");
        if b.gini.by_household.iter().all(|h| h.net_worth >= 0.0) {
            let area: f64 = l
                .windows(2)
                .map(|w| {
                    (w[1].population_share - w[0].population_share)
                        * (w[1].wealth_share + w[0].wealth_share)
                        / 2.0
                })
                .sum();
            assert!(
                (1.0 - 2.0 * area - s.indicators.gini).abs() < 1e-9,
                "lorenz area"
            );
        }
    }
    assert_eq!(b.taxes.len(), 6);
    let gst = b.taxes.iter().find(|t| t.id == "gst").unwrap();
    assert!(!gst.modelled);
    assert_eq!(b.taxes[0].label, "Income tax");
    // Taxes: revenue lines equal the finance panel's.
    let rev = &s.finance.government.revenue;
    close(b.taxes[0].revenue_month, rev.wage_tax, "income tax revenue");
    close(b.taxes[4].revenue_month, rev.carbon, "carbon revenue");
    // Industry.
    for f in &s.industry.firms {
        let total: f64 = f.ownership.iter().map(|o| o.share).sum();
        close(total, 1.0, "ownership shares");
        assert_eq!(f.state_share, 0.0);
    }
    assert_eq!(s.industry.infrastructure.len(), 5);
    assert_eq!(s.policies.rows.len(), 20);
}

#[test]
fn every_breakdown_ties_to_its_headline_over_a_real_game() {
    for seed in [1u64, 7, 42] {
        let mut api = Api::init(seed).unwrap();
        check_snapshot(&api.snapshot());
        let plans: Vec<Vec<Command>> = vec![
            vec![
                num(LeverKey::CarbonPrice, 60.0),
                num(LeverKey::TaxProfit, 0.3),
            ],
            vec![num(LeverKey::Ubi, 100.0)],
            vec![],
            vec![num(LeverKey::PublicSpending, 5000.0)],
        ];
        for turn in 0..16 {
            let cmds = plans.get(turn).cloned().unwrap_or_default();
            if api.snapshot().game_over.is_some() {
                break;
            }
            if turn % 2 == 0 {
                api.submit_turn(&cmds).unwrap();
                check_snapshot(&api.snapshot());
            } else {
                // Live mode: check after every month.
                api.begin_turn(&cmds).unwrap();
                for _ in 0..3 {
                    let m = api.advance_month().unwrap();
                    check_snapshot(&m.snapshot);
                }
                api.finish_turn().unwrap();
                check_snapshot(&api.snapshot());
            }
        }
    }
}

#[test]
fn history_carries_cumulative_and_gross_emissions() {
    let mut api = Api::init(3).unwrap();
    for _ in 0..4 {
        api.submit_turn(&[]).unwrap();
    }
    let s = api.snapshot();
    let last = s.history.last().unwrap();
    close(
        last.cumulative_emissions,
        s.nature.cumulative_tonnes,
        "history cumulative",
    );
    close(
        last.gross_emissions,
        s.nature.gross_emissions_monthly,
        "history gross",
    );
    assert!(s
        .history
        .windows(2)
        .all(|w| w[1].cumulative_emissions >= w[0].cumulative_emissions));
}

#[test]
fn legitimacy_parts_explain_the_monthly_step() {
    let mut api = Api::init(5).unwrap();
    api.submit_turn(&[]).unwrap();
    let l = api.snapshot().breakdowns.legitimacy;
    let parts = l.base.parts.as_ref().unwrap();
    assert!(parts.len() >= 7, "level, five terms, resistance, clamp");
    // Terms are the same numbers as the matching parts.
    for (t, p) in l.terms.iter().zip(&parts[1..6]) {
        close(t.contribution, p.value, &t.id);
    }
}

#[test]
fn policy_history_records_queue_activation_cancel_and_rejection() {
    let mut api = Api::init(11).unwrap();
    let initial = api.snapshot().policies.rows;
    let carbon0 = initial
        .iter()
        .find(|r| r.lever == LeverKey::CarbonPrice)
        .unwrap();
    assert_eq!(carbon0.initial, carbon0.active);
    assert!(initial
        .iter()
        .any(|r| r.lever == LeverKey::TaxWage && r.label == "Income tax"));
    // Queue carbon price, an out-of-range wage tax (rejected) and a ubi we then cancel.
    api.submit_turn(&[
        num(LeverKey::CarbonPrice, 50.0),
        num(LeverKey::TaxWage, 9.0),
        num(LeverKey::Ubi, 40.0),
    ])
    .unwrap();
    let s = api.snapshot();
    let row = s
        .policies
        .rows
        .iter()
        .find(|r| r.lever == LeverKey::CarbonPrice)
        .unwrap();
    assert!(row.pending.is_some() || row.last_change_turn.is_some());
    let kinds = |k: PolicyEventKind, l: LeverKey| {
        s.policies
            .timeline
            .iter()
            .any(|e| e.kind == k && e.lever == l)
    };
    assert!(kinds(PolicyEventKind::Queued, LeverKey::CarbonPrice));
    assert!(kinds(PolicyEventKind::Rejected, LeverKey::TaxWage));
    if let Some(p) = s.pending.iter().find(|p| {
        matches!(
            p.command,
            Command::SetPolicy {
                lever: LeverKey::Ubi,
                ..
            }
        )
    }) {
        api.cancel_pending(p.id).unwrap();
        let s2 = api.snapshot();
        assert!(s2
            .policies
            .timeline
            .iter()
            .any(|e| e.kind == PolicyEventKind::Cancelled && e.lever == LeverKey::Ubi));
    }
    for _ in 0..6 {
        api.submit_turn(&[]).unwrap();
    }
    let s = api.snapshot();
    let row = s
        .policies
        .rows
        .iter()
        .find(|r| r.lever == LeverKey::CarbonPrice)
        .unwrap();
    assert_eq!(row.active, PolicyScalar::Number(50.0));
    assert!(row.last_change_turn.is_some() && row.pending.is_none());
    assert!(s
        .policies
        .timeline
        .iter()
        .any(|e| e.kind == PolicyEventKind::Activated && e.lever == LeverKey::CarbonPrice));
    // Weakened share exists only for lobbied levers.
    assert!(row.weakened_by.is_some());
    let wage = s
        .policies
        .rows
        .iter()
        .find(|r| r.lever == LeverKey::TaxWage)
        .unwrap();
    assert!(wage.weakened_by.is_none());
}

#[test]
fn save_round_trips_the_log_and_refuses_older_saves() {
    let mut api = Api::init(2).unwrap();
    api.submit_turn(&[num(LeverKey::CarbonPrice, 30.0)])
        .unwrap();
    api.submit_turn(&[]).unwrap();
    let blob = api.export_state().unwrap();
    let before = serde_json::to_value(api.snapshot()).unwrap();
    let mut other = Api::init(99).unwrap();
    other.import_state(&blob).unwrap();
    assert_eq!(serde_json::to_value(other.snapshot()).unwrap(), before);
    // A save from schema 4 is refused cleanly.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["schemaVersion"] = 4.into();
    let err = other.import_state(&v.to_string()).unwrap_err();
    assert_eq!(err.code, sim_api::error::code::SCHEMA_MISMATCH);
    // A log that does not match the played ticks is refused.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["log"]["ticks"].as_array_mut().unwrap().pop();
    assert!(other.import_state(&v.to_string()).is_err());
    // The game continues identically after a load.
    let a = api.submit_turn(&[]).unwrap();
    let b = other.submit_turn(&[]).unwrap();
    assert_eq!(
        serde_json::to_value(a.snapshot).unwrap(),
        serde_json::to_value(b.snapshot).unwrap()
    );
}

#[test]
fn emissions_lines_before_any_month_are_empty_and_reconcile() {
    let api = Api::init(1).unwrap();
    let e = api.snapshot().breakdowns.emissions;
    assert!(e.reconciles && e.month_lines.is_empty());
    assert_eq!(e.cumulative.tonnes, 0.0);
}

#[test]
fn emission_and_income_sources_come_from_content_and_validate() {
    use sim_api::sources::{EmissionSourceDef, Sources, EMISSION_IDS};
    let api = Api::init(1).unwrap();
    let e = api.snapshot().breakdowns.emissions;
    let ids: Vec<&str> = e.month.sources.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(
        ids,
        ["firms", "households", "plants"],
        "zero aviation is left out"
    );
    let hh = &api.snapshot().households[0].income.sources;
    assert_eq!(
        hh.len(),
        5,
        "household income lists five sources (no domestic wages)"
    );
    let flows = &api.snapshot().finance.quarter.household_income;
    assert!(flows
        .iter()
        .any(|x| x.id == "domestic_wages" && x.part_of.as_deref() == Some("wages")));
    // A content file that drops an id the model totals is refused.
    let def = |id: &str| EmissionSourceDef {
        id: id.into(),
        label: id.into(),
        category: sim_api::sources::EmissionCategory::Industry,
        show_when_zero: false,
    };
    let short: Vec<_> = EMISSION_IDS[1..].iter().map(|i| def(i)).collect();
    assert!(Sources::new(
        short,
        Vec::new(),
        sim_api::networth::NetWorthDefs {
            lines: Vec::new(),
            holders: Vec::new()
        }
    )
    .is_err());
}
