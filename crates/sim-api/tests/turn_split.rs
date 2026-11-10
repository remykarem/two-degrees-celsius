//! Contract v1.1: `beginTurn` + 3 x `advanceMonth` + `finishTurn` must equal `submitTurn`
//! (state, events, trace, history, RNG), and the state machine must refuse out-of-order calls.
use sim_api::content;
use sim_api::keys::{InfraKind, LeverKey, SystemKey};
use sim_api::turn::{MonthResult, TurnStart};
use sim_api::*;

fn num(lever: LeverKey, v: f64) -> Command {
    Command::SetPolicy {
        lever,
        value: PolicyValue::Number(v),
    }
}

fn place(kind: InfraKind, x: i32, y: i32) -> Command {
    Command::Place {
        kind,
        tile: Tile { x, y },
    }
}

/// Scripted plans: policies, a road, a build, a switch of system, a rejected command, repeats.
fn plan(turn: usize, api: &Api) -> Vec<Command> {
    let tiles = |k| api.valid_tiles(k);
    match turn {
        0 => vec![
            num(LeverKey::CarbonPrice, 60.0),
            Command::Place {
                kind: InfraKind::Road,
                tile: tiles(InfraKind::Road)[7],
            },
            Command::Place {
                kind: InfraKind::RenewablePlant,
                tile: tiles(InfraKind::RenewablePlant)[3],
            },
            num(LeverKey::CampaignCap, 0.5), // refused by the Market
            place(InfraKind::Road, 99, 2),   // off the map: refused by the wire layer
        ],
        1 => vec![num(LeverKey::TaxWage, 9.0), num(LeverKey::TaxWage, 11.0)],
        2 => vec![Command::SwitchSystem {
            to: SystemKey::Regulated,
        }],
        3 => vec![
            Command::SwitchSystem {
                to: SystemKey::Coop,
            },
            num(LeverKey::Benefit, 900.0),
        ],
        6 => vec![Command::Place {
            kind: InfraKind::PublicHousing,
            tile: tiles(InfraKind::PublicHousing)[1],
        }],
        _ => vec![],
    }
}

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

#[test]
fn live_turn_equals_submit_turn() {
    for seed in [1u64, 7, 42] {
        let mut a = Api::init(seed).unwrap();
        let mut b = Api::init(seed).unwrap();
        for t in 0..10 {
            let cmds = plan(t, &a);
            if t == 4 {
                // Cancel the first pending item on both, when there is one.
                if let Some(p) = a.snapshot().pending.first() {
                    let id = p.id;
                    let (ca, cb) = (a.cancel_pending(id).unwrap(), b.cancel_pending(id).unwrap());
                    assert_eq!(json(&ca), json(&cb));
                }
            }
            let whole = a.submit_turn(&cmds).unwrap();
            let start = b.begin_turn(&cmds).unwrap();
            assert!(start.snapshot.turn_phase.open && start.snapshot.turn_phase.months_done == 0);
            let months: Vec<MonthResult> = (0..3).map(|_| b.advance_month().unwrap()).collect();
            for (i, m) in months.iter().enumerate() {
                assert_eq!(usize::from(m.month), i);
                assert_eq!(m.frame, whole.months[i], "seed {seed} turn {t} month {i}");
                assert_eq!(m.snapshot.turn_phase.months_done as usize, i + 1);
            }
            let live = b.finish_turn().unwrap();
            assert_eq!(json(&whole), json(&live), "seed {seed} turn {t}");
            assert!(!live.snapshot.turn_phase.open);
            assert_eq!(start.turn, whole.turn);
            assert_eq!(start.accepted, whole.accepted);
            assert_eq!(start.rejected, whole.rejected);
            assert_eq!(start.activated, whole.activated);
            assert_eq!(start.pc_before, whole.pc_before);
            // State, RNG position, history: the save blob holds all three.
            assert_eq!(a.export_state().unwrap(), b.export_state().unwrap());
            assert_eq!(a.sim().rng.parts(), b.sim().rng.parts());
            assert_eq!(a.sim().state, b.sim().state);
            assert_eq!(json(&a.snapshot()), json(&b.snapshot()));
        }
    }
}

#[test]
fn out_of_order_calls_return_turn_state_and_change_nothing() {
    let mut api = Api::init(3).unwrap();
    let check = |api: &Api, before: &str| assert_eq!(json(&api.snapshot()), before);
    let s0 = json(&api.snapshot());
    assert_eq!(api.advance_month().unwrap_err().code, "turn_state");
    assert_eq!(api.finish_turn().unwrap_err().code, "turn_state");
    check(&api, &s0);

    api.begin_turn(&[num(LeverKey::CarbonPrice, 30.0)]).unwrap();
    let s1 = json(&api.snapshot());
    assert_eq!(api.begin_turn(&[]).unwrap_err().code, "turn_state");
    assert_eq!(api.submit_turn(&[]).unwrap_err().code, "turn_state");
    assert_eq!(api.finish_turn().unwrap_err().code, "turn_state");
    assert_eq!(api.export_state().unwrap_err().code, "turn_state");
    check(&api, &s1);
    // Previews are legal mid-turn and change nothing.
    assert_eq!(
        api.preview(&[num(LeverKey::TaxWage, 8.0)]).unwrap().len(),
        1
    );
    check(&api, &s1);

    for _ in 0..3 {
        api.advance_month().unwrap();
    }
    let s3 = json(&api.snapshot());
    assert_eq!(api.advance_month().unwrap_err().code, "turn_state");
    assert_eq!(api.begin_turn(&[]).unwrap_err().code, "turn_state");
    assert_eq!(api.export_state().unwrap_err().code, "turn_state");
    check(&api, &s3);
    api.finish_turn().unwrap();
    assert_eq!(api.finish_turn().unwrap_err().code, "turn_state");
    assert!(api.export_state().is_ok());
    api.submit_turn(&[]).unwrap();
    assert_eq!(api.snapshot().turn, 3);
}

#[test]
fn import_resets_the_open_turn_and_reset_too() {
    let mut a = Api::init(5).unwrap();
    a.submit_turn(&[]).unwrap();
    let blob = a.export_state().unwrap();
    a.begin_turn(&[]).unwrap();
    a.advance_month().unwrap();
    let snap = a.import_state(&blob).unwrap();
    assert!(!snap.turn_phase.open && snap.turn_phase.months_done == 0);
    assert_eq!(a.advance_month().unwrap_err().code, "turn_state");
    assert_eq!(a.finish_turn().unwrap_err().code, "turn_state");
    a.begin_turn(&[]).unwrap();
    assert!(!a.reset(5).unwrap().turn_phase.open);
    assert_eq!(a.advance_month().unwrap_err().code, "turn_state");
}

#[test]
fn live_turn_halts_and_completes_after_the_outcome() {
    let mut v: serde_json::Value = serde_json::from_str(content::PARAMS).unwrap();
    v["nature"]["budget_b0"] = serde_json::json!(5.0);
    let params = v.to_string();
    let c = sim_params::ContentJson {
        params: &params,
        initial_town: content::INITIAL_TOWN,
        market: content::MARKET,
        regulated: content::REGULATED,
        coop: content::COOP,
    };
    let mut whole = Api::from_content(&c, content::LEVERS, 2).unwrap();
    let mut live = Api::from_content(&c, content::LEVERS, 2).unwrap();
    let w = whole.submit_turn(&[]).unwrap();
    live.begin_turn(&[]).unwrap();
    let mut seen_over = false;
    for _ in 0..3 {
        let m = live.advance_month().unwrap();
        seen_over |= m.snapshot.game_over.is_some();
    }
    assert!(seen_over, "gameOver is set on the month it happens");
    let r = live.finish_turn().unwrap();
    assert_eq!(json(&w), json(&r));
    // Now over: a new turn is refused, and nothing is open.
    assert_eq!(live.begin_turn(&[]).unwrap_err().code, "game_over");
    assert_eq!(live.advance_month().unwrap_err().code, "turn_state");
    assert!(live.export_state().is_ok());
}

#[test]
fn serde_round_trips() {
    let mut api = Api::init(1).unwrap();
    let start = api.begin_turn(&[num(LeverKey::CarbonPrice, 50.0)]).unwrap();
    let month = api.advance_month().unwrap();
    let s: TurnStart = serde_json::from_str(&json(&start)).unwrap();
    let m: MonthResult = serde_json::from_str(&json(&month)).unwrap();
    assert_eq!(s, start);
    assert_eq!(m, month);
    let v: serde_json::Value = serde_json::from_str(&json(&month)).unwrap();
    assert_eq!(v["snapshot"]["turnPhase"]["open"], true);
    assert_eq!(v["snapshot"]["turnPhase"]["monthsDone"], 1);
}
