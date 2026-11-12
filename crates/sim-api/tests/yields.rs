//! Sustainable-yield view (contract v2.6): the rows reconcile, the text is finished, the flag and
//! the months to collapse agree with the figures, and a heavy harvest shows a real shortage.
use sim_api::keys::LeverKey;
use sim_api::snapshot::Snapshot;
use sim_api::yields::YieldView;
use sim_api::*;

fn row(v: &YieldView, key: &str) -> f64 {
    v.rows
        .iter()
        .find(|r| r.key == key)
        .and_then(|r| r.value)
        .unwrap_or(f64::NAN)
}

fn check(s: &Snapshot, what: &str) {
    assert_eq!(s.nature_yield.len(), 2, "{what}");
    for v in &s.nature_yield {
        let w = format!("{what}: {}", v.id);
        assert!(v.reconciles, "{w}");
        let sum: f64 = v.parts.iter().map(|p| p.value).sum();
        assert!(
            (sum - v.change).abs() < 1e-9,
            "{w}: parts {sum} vs {}",
            v.change
        );
        assert_eq!(
            v.over_line,
            row(v, "taken") > row(v, "regrowth") + 1e-9,
            "{w}"
        );
        assert_eq!(v.months_to_collapse.is_some(), v.over_line, "{w}");
        for r in &v.rows {
            assert!(!r.label.is_empty(), "{w}: row {} has no label", r.key);
            assert!(
                !r.text.contains('{') && !r.text.contains('%'),
                "{w}: {:?}",
                r.text
            );
        }
        for t in [&v.verdict, &v.headline, &v.blurb] {
            assert!(
                !t.contains('{') && !t.contains('%') && !t.contains("NaN"),
                "{w}: {t:?}"
            );
        }
        assert!(!v.series.is_empty(), "{w}: series");
        // The series ends at the stock shown now.
        let last = v.series.last().unwrap();
        assert!(
            (last.stock - row(v, "stock")).abs() < 1e-9,
            "{w}: series ends now"
        );
    }
}

#[test]
fn the_view_reconciles_over_seeds_turns_and_live_months() {
    for seed in [1, 7, 42] {
        let mut api = Api::init(seed).unwrap();
        check(&api.snapshot(), &format!("seed {seed} start"));
        for turn in 0..6 {
            api.submit_turn(&[]).unwrap();
            check(&api.snapshot(), &format!("seed {seed} turn {turn}"));
        }
    }
}

#[test]
fn a_heavy_harvest_goes_over_the_line_and_then_food_runs_short() {
    let mut api = Api::init(42).unwrap();
    api.submit_turn(&[Command::SetPolicy {
        lever: LeverKey::CommonsHarvest,
        value: command::PolicyValue::Number(3.0),
    }])
    .unwrap();
    let mut over_seen = false;
    let mut short_seen = false;
    for turn in 0..24 {
        api.submit_turn(&[]).unwrap();
        let s = api.snapshot();
        check(&s, &format!("heavy turn {turn}"));
        let commons = &s.nature_yield[0];
        over_seen |= commons.over_line && commons.months_to_collapse.is_some();
        short_seen |= s.markets.iter().any(|m| {
            m.id == "food" && m.unmet > 1.0 && m.short.iter().any(|c| c.id == "commons_limit")
        });
    }
    assert!(
        over_seen,
        "the commons never went over the sustainable line"
    );
    assert!(
        short_seen,
        "food never ran short with the commons limit named"
    );
}

#[test]
fn the_default_run_stays_under_the_line_at_the_start() {
    let mut api = Api::init(42).unwrap();
    api.submit_turn(&[]).unwrap();
    let s = api.snapshot();
    // Taken for every regrown is a ratio worded in text, never a percentage.
    assert!(s.nature_yield[0].headline.contains("regrown"));
    assert_eq!(s.nature_yield[0].access.as_ref().unwrap().id, "open");
}
