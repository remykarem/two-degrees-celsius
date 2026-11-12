//! Market trades (contract v2.4): the list behind each market adds up to the card, across seeds
//! and turns, survives the roll-up and a save, and is the same on every run.
use sim_api::ledger::{AccountRef, LedgerQuery};
use sim_api::markets::{MarketEntity, MarketEntityKind};
use sim_api::trades::{MarketTrades, TradesQuery};
use sim_api::*;

const LISTED: [&str; 6] = ["food", "goods", "services", "labour", "housing", "credit"];

fn trades(api: &Api, id: &str, q: &TradesQuery) -> MarketTrades {
    api.market_trades(id, q).unwrap()
}

fn check(api: &Api, what: &str) {
    for id in LISTED {
        let t = trades(api, id, &TradesQuery::default());
        let w = format!("{what}: {id}");
        assert!(
            t.aggregate.is_some(),
            "{w}: the latest month has the card's figure"
        );
        assert_eq!(
            t.reconciles,
            Some(true),
            "{w}: counted rows add up to the card {t:?}"
        );
        for r in &t.rows {
            assert_eq!(r.cells.len(), t.columns.len(), "{w}: one cell per column");
        }
        assert_eq!(t.totals.cells.len(), t.columns.len(), "{w}");
        for s in [&t.intro, &t.empty, &t.uncounted, &t.totals.text] {
            assert!(
                !s.contains('{') && !s.contains('}') && !s.contains('%'),
                "{w}: {s:?}"
            );
        }
        for r in &t.rows {
            for c in &r.cells {
                assert!(
                    !c.text.contains('{') && !c.text.contains("NaN") && !c.text.contains('%'),
                    "{w}: {c:?}"
                );
            }
        }
    }
    let e = trades(api, "energy", &TradesQuery::default());
    assert!(e.rows.is_empty() && e.columns.is_empty() && e.aggregate.is_none());
}

#[test]
fn every_list_reconciles_in_several_seeds_and_turns() {
    for seed in [1u64, 7, 42, 99, 2026] {
        let mut api = Api::init(seed).unwrap();
        for turn in 1..=40 {
            if api.snapshot().game_over.is_some() {
                break;
            }
            api.submit_turn(&[]).unwrap();
            check(&api, &format!("seed {seed} turn {turn}"));
        }
    }
}

#[test]
fn live_months_reconcile_too() {
    let mut api = Api::init(5).unwrap();
    api.submit_turn(&[]).unwrap();
    api.begin_turn(&[]).unwrap();
    for m in 0..3 {
        api.advance_month().unwrap();
        check(&api, &format!("live month {m}"));
    }
    api.finish_turn().unwrap();
}

/// The money in the three product lists is exactly what the ledger moved to the shops that month
/// (households' purchases, firms' capital goods and the government's spending).
#[test]
fn money_is_what_the_ledger_moved() {
    let mut api = Api::init(42).unwrap();
    for _ in 0..4 {
        api.submit_turn(&[]).unwrap();
    }
    let snap = api.snapshot();
    let ours = [
        "food",
        "goods",
        "services",
        "capital goods",
        "public spending",
    ];
    for month in 0..3u8 {
        let mut listed = 0.0;
        for id in ["food", "goods", "services"] {
            let q = TradesQuery {
                turn: Some(4),
                month: Some(month),
                entity: None,
            };
            listed += trades(&api, id, &q).totals.money;
        }
        let mut moved = 0.0;
        // A firm buying its own output moves no cash; the list says so in its note column.
        for id in ["food", "goods", "services"] {
            let q = TradesQuery {
                turn: Some(4),
                month: Some(month),
                entity: None,
            };
            let t = trades(&api, id, &q);
            let note = t.columns.iter().position(|c| c.id == "note").unwrap();
            let total = t.columns.iter().position(|c| c.id == "total").unwrap();
            moved += t
                .rows
                .iter()
                .filter(|r| !r.cells[note].text.is_empty())
                .map(|r| r.cells[total].sort.unwrap())
                .sum::<f64>();
        }
        for f in &snap.firms {
            let p = api
                .ledger(
                    &AccountRef::Firm { id: f.id },
                    &LedgerQuery {
                        from_turn: Some(4),
                        to_turn: Some(4),
                        ..Default::default()
                    },
                )
                .unwrap();
            moved += p
                .entries
                .iter()
                .filter(|e| {
                    e.month == month && matches!(e.to, AccountRef::Firm { id: x } if x == f.id)
                })
                .filter(|e| e.detail.as_deref().is_some_and(|d| ours.contains(&d)))
                .map(|e| e.amount)
                .sum::<f64>();
        }
        assert!(
            (moved - listed).abs() < 0.005,
            "month {month}: ledger plus own-output {moved}, lists {listed}"
        );
    }
}

#[test]
fn a_whole_turn_is_its_three_months() {
    let mut api = Api::init(7).unwrap();
    for _ in 0..3 {
        api.submit_turn(&[]).unwrap();
    }
    for id in LISTED {
        let whole = trades(
            &api,
            id,
            &TradesQuery {
                turn: Some(3),
                month: None,
                entity: None,
            },
        );
        let (mut u, mut m) = (0.0, 0.0);
        for month in 0..3u8 {
            let t = trades(
                &api,
                id,
                &TradesQuery {
                    turn: Some(3),
                    month: Some(month),
                    entity: None,
                },
            );
            u += t.totals.units;
            m += t.totals.money;
        }
        assert!((whole.totals.units - u).abs() < 1e-6, "{id}");
        assert!((whole.totals.money - m).abs() < 0.005, "{id}");
    }
}

#[test]
fn the_roll_up_keeps_exact_totals_and_memory_bounded() {
    let mut api = Api::init(42).unwrap();
    type Totals = Vec<(f64, f64, u32)>;
    let mut seen: Vec<(u32, Totals)> = Vec::new();
    let mut peak_rows = 0;
    for turn in 1..=24u32 {
        api.submit_turn(&[]).unwrap();
        let v = LISTED
            .iter()
            .map(|id| {
                let t = trades(
                    &api,
                    id,
                    &TradesQuery {
                        turn: Some(turn),
                        month: None,
                        entity: None,
                    },
                );
                (t.totals.units, t.totals.money, t.totals.count)
            })
            .collect();
        seen.push((turn, v));
        peak_rows = peak_rows.max(api.trade_size().rows);
    }
    let size = api.trade_size();
    assert!(size.rolls > 0, "old turns are rolled up");
    let detail_turns: u32 = 8; // content/levers.json ledger.detail_turns
    assert!(
        size.rows <= peak_rows && size.rows < (3 * detail_turns * 120) as usize,
        "{size:?}"
    );
    for (turn, v) in seen {
        for (id, want) in LISTED.iter().zip(v) {
            let t = trades(
                &api,
                id,
                &TradesQuery {
                    turn: Some(turn),
                    month: None,
                    entity: None,
                },
            );
            assert_eq!(t.period.rolled, turn <= 24 - detail_turns, "turn {turn}");
            assert!(
                (t.totals.units - want.0).abs() < 1e-6,
                "{id} turn {turn} units"
            );
            assert_eq!(t.totals.money, want.1, "{id} turn {turn} money");
            assert_eq!(t.totals.count, want.2, "{id} turn {turn} count");
        }
    }
}

#[test]
fn saved_with_the_game_and_the_same_every_run() {
    let play = || {
        let mut a = Api::init(11).unwrap();
        for _ in 0..12 {
            a.submit_turn(&[]).unwrap();
        }
        a
    };
    let (a, b) = (play(), play());
    for id in LISTED {
        let q = TradesQuery::default();
        assert_eq!(trades(&a, id, &q), trades(&b, id, &q), "{id}: determinism");
    }
    let blob = a.export_state().unwrap();
    let mut c = Api::init(1).unwrap();
    c.import_state(&blob).unwrap();
    for id in LISTED {
        for q in [
            TradesQuery::default(),
            TradesQuery {
                turn: Some(2),
                month: None,
                entity: None,
            },
        ] {
            assert_eq!(trades(&a, id, &q), trades(&c, id, &q), "{id}: round trip");
        }
    }
    // A save from before the trades existed still loads; its list starts empty.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["ledger"].as_object_mut().unwrap().remove("trades");
    let mut d = Api::init(1).unwrap();
    d.import_state(&v.to_string()).unwrap();
    let empty = d.market_trades("food", &TradesQuery::default()).unwrap();
    assert!(empty.rows.is_empty() && empty.periods.is_empty());
    d.submit_turn(&[]).unwrap();
    check(&d, "old save, one turn later");
}

#[test]
fn an_entity_sees_only_its_own_trades() {
    let mut api = Api::init(3).unwrap();
    for _ in 0..3 {
        api.submit_turn(&[]).unwrap();
    }
    let all = trades(&api, "food", &TradesQuery::default());
    let me = MarketEntity {
        kind: MarketEntityKind::Household,
        id: 0,
    };
    let mine = trades(
        &api,
        "food",
        &TradesQuery {
            entity: Some(me),
            ..Default::default()
        },
    );
    assert!(!mine.rows.is_empty() && mine.rows.len() < all.rows.len());
    assert!(mine.rows.iter().all(|r| r.cells[0].entity == Some(me)));
    assert!(
        mine.aggregate.is_none(),
        "a filtered list is not the card's figure"
    );
}

#[test]
fn rows_have_who_what_and_how_much() {
    let mut api = Api::init(42).unwrap();
    for _ in 0..3 {
        api.submit_turn(&[]).unwrap();
    }
    let f = trades(&api, "food", &TradesQuery::default());
    let col = |t: &MarketTrades, id: &str| t.columns.iter().position(|c| c.id == id).unwrap();
    let r = &f.rows[0];
    assert!(
        r.cells[col(&f, "buyer")].entity.is_some() && r.cells[col(&f, "seller")].entity.is_some()
    );
    assert!(r.cells[col(&f, "units")].sort.unwrap() > 0.0);
    assert!(r.cells[col(&f, "unit_price")].text.starts_with('$'));
    let l = trades(&api, "labour", &TradesQuery::default());
    assert!(l
        .rows
        .iter()
        .any(|r| r.counted && r.cells[0].entity.is_some()));
    let h = trades(&api, "housing", &TradesQuery::default());
    let snap = api.snapshot();
    let housed = snap.homes.iter().filter(|x| x.household.is_some()).count();
    assert_eq!(h.rows.iter().filter(|r| r.counted).count(), housed);
}
