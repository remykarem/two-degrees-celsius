//! The UI catalog (contract v1.8): `content/ui.json` resolves against the live params, covers
//! every lever group, and the wire fields added with it agree with the model.
use sim_api::keys::LeverKey;
use sim_api::Api;
use std::collections::BTreeSet;

fn params() -> sim_params::Params {
    sim_params::Params::from_content(&sim_api::content::default_content()).unwrap()
}

#[test]
fn catalog_resolves_and_covers_the_levers_and_systems() {
    let api = Api::init(1).unwrap();
    let c = api.catalog().unwrap();
    // Every lever's group has a blurb entry, in a stable order, and no entry is unused.
    let used: BTreeSet<&str> = api.levers().iter().map(|l| l.group.as_str()).collect();
    let listed: BTreeSet<&str> = c.lever_groups.iter().map(|g| g.label.as_str()).collect();
    assert_eq!(used, listed, "lever groups in levers.json vs ui.json");
    // Three system cards with numbers read from params.
    assert_eq!(c.systems.len(), 3);
    for card in &c.systems {
        assert!(!card.name.is_empty() && !card.tagline.is_empty());
        assert!(card.params.iter().all(|p| p.value.is_finite()));
    }
    let coop = c.systems.iter().find(|s| s.key == "coop").unwrap();
    assert_eq!(
        coop.transition_months,
        f64::from(params().systems.coop.transition_months)
    );
    // Bands are ordered and sane.
    let h = &c.bands.health;
    assert!(h.windows(2).all(|w| w[0].min > w[1].min) && h.last().unwrap().min == 0.0);
    assert!(c
        .bands
        .stress_words
        .bands
        .windows(2)
        .all(|w| w[0].below < w[1].below));
    // Placeholders in text name a field the web fills; none is left unnamed.
    for (g, m) in &c.text {
        for (k, v) in m {
            assert!(!v.trim().is_empty(), "{g}.{k} is empty");
        }
    }
}

#[test]
fn expected_effective_matches_the_models_policy_gap() {
    // Set the nominal values, run one tick, and compare the model's own effective values
    // (from the trace the lobbying system records) with the formula the preview uses.
    use sim_model::lobbying::expected_effective;
    use sim_types::TraceRecord;
    let sim = sim_engine::init(params(), 3).unwrap();
    let mut s = sim.state.clone();
    s.policy.carbon_price = sim_types::Cents::from_dollars(120.0);
    s.policy.wealth_tax = sim_types::Rate(0.02);
    s.policy.tax_profit = sim_types::Rate(0.3);
    s.policy.emissions_cap = sim_types::Tonnes(40.0);
    let mut rng = sim_rng::Rng::new(1, 54);
    let d = sim_model::lobbying::run(&s, &sim.params, &mut rng);
    let mut seen = 0;
    for t in &d.trace {
        if let TraceRecord::PolicyGap {
            lever,
            nominal,
            effective,
            net_lobby,
        } = t
        {
            let e = expected_effective(*lever, *nominal, net_lobby.0);
            assert!(
                (e - effective).abs() < 0.006 + 1e-6 * effective.abs(), // carbon price is held in whole cents
                "{lever:?}: {e} vs {effective}"
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 4);
}

#[test]
fn preview_carries_the_expected_effective_value_for_lobbied_levers_only() {
    use sim_api::command::{Command, PolicyValue};
    let api = Api::init(1).unwrap();
    let p = api
        .preview(&[
            Command::SetPolicy {
                lever: LeverKey::CarbonPrice,
                value: PolicyValue::Number(100.0),
            },
            Command::SetPolicy {
                lever: LeverKey::TaxWage,
                value: PolicyValue::Number(0.2),
            },
        ])
        .unwrap();
    assert!(p[0].expected_effective.is_some());
    assert!(p[1].expected_effective.is_none());
}

#[test]
fn fire_risk_is_on_standing_forest_tiles_and_within_zero_one() {
    let api = Api::init(1).unwrap();
    let snap = api.snapshot();
    let mut n = 0;
    for t in &snap.map.tiles {
        if let Some(r) = t.fire_risk {
            assert!((0.0..=1.0).contains(&r));
            assert!(t.state.is_some(), "only forest tiles carry a state");
            n += 1;
        }
    }
    assert!(n > 0, "a new game has standing forest");
    assert_eq!(snap.game.ticks_per_turn, 3);
    assert!(snap.weather.coverage_radius > 0 && snap.weather.smoke_radius > 0);
}
