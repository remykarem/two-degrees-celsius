//! Markets view (contract v2.3): every market reconciles, over seeds, turns and live months,
//! and the content and the code agree on the ids.
use sim_api::keys::{LeverKey, SystemKey};
use sim_api::markets::{direction, MarketView, MARKET_IDS};
use sim_api::snapshot::Snapshot;
use sim_api::*;

fn tol(a: f64, b: f64) -> f64 {
    1e-6 * a.abs().max(b.abs()).max(1.0)
}

fn check(s: &Snapshot, what: &str) {
    if s.markets.is_empty() && s.history.is_empty() {
        return; // before the first month there is nothing to report
    }
    assert_eq!(s.markets.len(), MARKET_IDS.len(), "{what}");
    for (m, id) in s.markets.iter().zip(MARKET_IDS) {
        assert_eq!(m.id, id, "{what}: content order");
        one(m, what);
    }
}

fn one(m: &MarketView, what: &str) {
    let w = format!("{what}: {}", m.id);
    assert!(m.reconciles, "{w}: does not reconcile {m:?}");
    assert!(
        (m.demand - (m.traded + m.unmet)).abs() <= tol(m.demand, m.traded + m.unmet),
        "{w}: demand = traded + unmet"
    );
    assert!(
        (m.supply - (m.traded + m.unsold)).abs() <= tol(m.supply, m.traded + m.unsold),
        "{w}: supply = traded + unsold"
    );
    assert!(m.unmet >= 0.0 && m.unsold >= 0.0 && m.traded >= 0.0, "{w}");
    // Text is finished: no placeholder is left over, and a percentage always has its unit.
    let mut texts = vec![m.verdict.clone(), m.price_note.clone()];
    texts.extend(m.gap_words.clone());
    texts.extend(m.short.iter().map(|x| x.label.clone()));
    texts.extend(m.drivers.iter().map(|x| x.value.clone()));
    for t in &texts {
        assert!(
            !t.contains('{') && !t.contains('}'),
            "{w}: placeholder in {t:?}"
        );
        assert!(!t.contains("NaN") && !t.contains("inf"), "{w}: {t:?}");
    }
    for d in &m.drivers {
        if d.value.contains('%') {
            assert!(
                d.value.contains(" of ")
                    || d.value.contains(" on top ")
                    || d.value.contains(" a year"),
                "{w}: bare percentage {:?}",
                d.value
            );
        }
    }
    assert!(!m.gap_words.is_empty() && !m.verdict.is_empty(), "{w}");
    // The direction word matches the series the card draws.
    if m.price.is_some() {
        assert!(!m.price_series.is_empty(), "{w}");
        assert_eq!(
            m.price,
            m.price_series.last().copied(),
            "{w}: series ends now"
        );
    } else {
        assert_eq!(m.price_direction, "none", "{w}");
    }
}

#[test]
fn every_market_reconciles_in_several_seeds_and_turns() {
    for seed in [1u64, 7, 42, 99, 2026] {
        let mut api = Api::init(seed).unwrap();
        check(&api.snapshot(), &format!("seed {seed} start"));
        for turn in 1..=40 {
            if api.snapshot().game_over.is_some() {
                break;
            }
            api.submit_turn(&[]).unwrap();
            check(&api.snapshot(), &format!("seed {seed} turn {turn}"));
        }
    }
}

#[test]
fn live_months_reconcile_too_and_the_open_month_extends_the_series() {
    let mut api = Api::init(5).unwrap();
    api.submit_turn(&[]).unwrap();
    let before = api.snapshot().markets[0].price_series.len();
    api.begin_turn(&[]).unwrap();
    for m in 0..3 {
        let r = api.advance_month().unwrap();
        check(&r.snapshot, &format!("live month {m}"));
        // History holds finished turns; the open month is appended.
        assert_eq!(r.snapshot.markets[0].price_series.len(), before + 1);
    }
    api.finish_turn().unwrap();
    assert_eq!(api.snapshot().markets[0].price_series.len(), before + 1);
}

#[test]
fn a_shock_and_a_coop_still_reconcile() {
    let mut api = Api::init(3).unwrap();
    api.submit_turn(&[Command::SetPolicy {
        lever: LeverKey::CarbonPrice,
        value: command::PolicyValue::Number(200.0),
    }])
    .unwrap();
    for turn in 0..10 {
        api.submit_turn(&[]).unwrap();
        check(&api.snapshot(), &format!("carbon shock turn {turn}"));
    }
    api.submit_turn(&[Command::SwitchSystem {
        to: SystemKey::Coop,
    }])
    .unwrap();
    for turn in 0..10 {
        api.submit_turn(&[]).unwrap();
        check(&api.snapshot(), &format!("coop turn {turn}"));
    }
}

#[test]
fn the_state_is_what_the_cards_say() {
    let mut api = Api::init(42).unwrap();
    for _ in 0..8 {
        api.submit_turn(&[]).unwrap();
    }
    let s = api.snapshot();
    let by = |id: &str| s.markets.iter().find(|m| m.id == id).unwrap();
    // Housing: the same counts as the housing block.
    let h = by("housing");
    assert_eq!(h.supply, f64::from(s.housing.homes));
    assert_eq!(h.unmet, f64::from(s.housing.unhoused_households));
    assert_eq!(h.price, Some(s.housing.rent_private));
    // Labour: people without a firm job match the unemployment the town reports plus the guarantee.
    let l = by("labour");
    let lf = s.people.iter().filter(|p| p.in_labour_force).count() as f64;
    assert!(l.supply <= lf);
    // Credit and energy agree with the bank and the grid.
    let c = by("credit");
    assert!(c.traded <= c.supply + 1e-6 && c.traded <= c.demand + 1e-6);
    let e = by("energy");
    assert_eq!(e.price, None);
    assert!(e.traded <= e.supply + 1e-6);
    // The goods price is the shelf price of the sector's firms.
    let f = by("food");
    let farm: Vec<_> = s
        .firms
        .iter()
        .filter(|x| x.active && x.sector == sim_api::keys::SectorKey::Food)
        .collect();
    assert!(!farm.is_empty());
    let p = f.price.unwrap();
    let (lo, hi) = farm.iter().fold((f64::MAX, 0.0_f64), |(a, b), x| {
        (a.min(x.price), b.max(x.price))
    });
    assert!(p >= lo - 1e-9 && p <= hi + 1e-9);
}

#[test]
fn price_direction_agrees_with_the_series() {
    let mut api = Api::init(11).unwrap();
    let band = api.catalog().unwrap().markets.steady_band;
    for _ in 0..30 {
        if api.snapshot().game_over.is_some() {
            break;
        }
        api.submit_turn(&[]).unwrap();
        for m in &api.snapshot().markets {
            if m.price.is_some() {
                assert_eq!(
                    m.price_direction,
                    direction(&m.price_series, band),
                    "{}",
                    m.id
                );
            }
        }
    }
}

#[test]
fn history_carries_the_price_series() {
    let mut api = Api::init(2).unwrap();
    for _ in 0..4 {
        api.submit_turn(&[]).unwrap();
    }
    let s = api.snapshot();
    assert_eq!(s.history.len(), 4);
    let h = s.history.last().unwrap();
    assert!(h.price_food.is_some_and(|p| p > 0.0));
    assert!(h.wage_avg.is_some_and(|p| p > 0.0));
    assert_eq!(h.rent_private, Some(s.housing.rent_private));
}

#[test]
fn simulation_state_is_not_touched_by_the_view() {
    // Two games, one of which builds snapshots all the way, end in the same state.
    let mut a = Api::init(9).unwrap();
    let mut b = Api::init(9).unwrap();
    for _ in 0..12 {
        a.submit_turn(&[]).unwrap();
        let _ = a.snapshot();
        b.submit_turn(&[]).unwrap();
    }
    assert_eq!(
        serde_json::to_string(&a.sim().state).unwrap(),
        serde_json::to_string(&b.sim().state).unwrap()
    );
}

#[test]
fn content_and_code_agree_on_the_market_ids() {
    use sim_api::markets::{chip_ids, driver_ids};
    let c = Api::init(1).unwrap().catalog().unwrap().markets;
    let ids: Vec<&str> = c.list.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, MARKET_IDS.to_vec());
    for d in &c.list {
        assert_eq!(d.short.len(), chip_ids(&d.id).len(), "{}", d.id);
        assert_eq!(d.drivers.len(), driver_ids(&d.id).len(), "{}", d.id);
    }
    // Content that drops a market, adds an unknown one or points at a missing category is refused.
    let mut missing = c.clone();
    missing.list.pop();
    assert!(missing.validated().is_err(), "missing market");
    let mut extra = c.clone();
    let mut m = extra.list[0].clone();
    m.id = "bread".into();
    extra.list.push(m);
    assert!(extra.validated().is_err(), "market the code cannot total");
    let mut cat = c.clone();
    cat.list[0].category = "nowhere".into();
    assert!(cat.validated().is_err(), "unknown category");
    let mut mech = c.clone();
    mech.list[0].mechanisms = vec!["magic".into()];
    assert!(mech.validated().is_err(), "unknown mechanism");
    assert!(c.validated().is_ok());
}

#[test]
fn coordination_text_is_in_the_catalog() {
    let cat = Api::init(1).unwrap().catalog().unwrap();
    let ids: Vec<&str> = cat
        .markets
        .mechanisms
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    for want in ["market", "rules", "hierarchy", "commons"] {
        assert!(ids.contains(&want), "{want}");
    }
    assert!(cat.systems.iter().all(|s| !s.coordination.is_empty()));
    assert!(cat
        .text
        .get("markets")
        .is_some_and(|g| g.contains_key("intro_body")));
}

#[test]
fn every_how_anchor_has_an_explorer_topic() {
    let api = Api::init(1).unwrap();
    let topics = api.assumptions().unwrap();
    let used: Vec<&str> = topics
        .iter()
        .flat_map(|t| t.used_by.iter().map(String::as_str))
        .collect();
    for m in api.catalog().unwrap().markets.list {
        assert!(used.contains(&m.how_id.as_str()), "{}", m.how_id);
    }
    for a in ["markets:intro", "markets:prices", "panel:markets"] {
        assert!(used.contains(&a), "{a}");
    }
    assert!(topics.iter().any(|t| t.id == "coordination_mechanisms"));
    // The price rule's worked example is the model's own function.
    let t = topics
        .iter()
        .find(|t| t.id == "market_price_setting")
        .unwrap();
    assert_eq!(
        t.examples[0].result.value,
        sim_model::goods::price_target(0.4, 20.0)
    );
}
