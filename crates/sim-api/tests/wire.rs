//! Wire-format tests: serde round trips, metadata, buildings, command validation.
use serde::de::DeserializeOwned;
use serde::Serialize;
use sim_api::events::{EmissionLine, EmissionSourceKind, IncomeKindKey, SimEvent, SimEventKind};
use sim_api::keys::{
    BlockReason, ClassKey, CollapseReason, GameOutcome, InfraKind, LeverKey, PlainOutcome,
    SystemKey,
};
use sim_api::*;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(v: &T) {
    let json = serde_json::to_string(v).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(&back, v, "round trip changed the value: {json}");
}

fn num(lever: LeverKey, v: f64) -> Command {
    Command::SetPolicy {
        lever,
        value: PolicyValue::Number(v),
    }
}

#[test]
fn commands_round_trip_and_match_contract_json() {
    let cmds = vec![
        num(LeverKey::CarbonPrice, 50.0),
        Command::SetPolicy {
            lever: LeverKey::Bailout,
            value: PolicyValue::Bool(true),
        },
        Command::SetPolicy {
            lever: LeverKey::CommonsAccess,
            value: PolicyValue::Commons(CommonsAccessWire::Community { cap: 0.8 }),
        },
        Command::SetPolicy {
            lever: LeverKey::CommonsAccess,
            value: PolicyValue::Commons(CommonsAccessWire::Open),
        },
        Command::SwitchSystem {
            to: SystemKey::Regulated,
        },
        Command::Place {
            kind: InfraKind::FossilPlant,
            count: 2,
            tile: Some(Tile { x: 4, y: 7 }),
        },
        Command::Remove {
            kind: InfraKind::PublicHousing,
            count: 1,
        },
    ];
    round_trip(&cmds);
    let j = serde_json::to_value(&cmds).unwrap();
    assert_eq!(
        j[0],
        serde_json::json!({"type":"set_policy","lever":"carbon_price","value":50.0})
    );
    assert_eq!(
        j[2]["value"],
        serde_json::json!({"mode":"community","cap":0.8})
    );
    assert_eq!(j[5]["tile"], serde_json::json!({"x":4,"y":7}));
    assert_eq!(
        j[6],
        serde_json::json!({"type":"remove","kind":"public_housing","count":1})
    );
    // parse_commands accepts the contract's literal JSON, integers included.
    let parsed = api::parse_commands(
        r#"[{"type":"set_policy","lever":"tax_wage","value":0},{"type":"place","kind":"renewable_plant","count":1,"tile":{"x":1,"y":2}}]"#,
    )
    .unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(
        api::parse_commands("[{\"type\":\"nope\"}]")
            .unwrap_err()
            .code,
        "bad_request"
    );
}

#[test]
fn every_event_and_trace_variant_round_trips() {
    use SimEventKind as K;
    let kinds = [
        K::Commuted {
            household: 1,
            firm: 0,
            people: 12.5,
        },
        K::Hired {
            firm: 0,
            people: 3.0,
        },
        K::Fired {
            firm: 1,
            people: 2.0,
        },
        K::LoanDefaulted { firm: 1 },
        K::FirmFounded { firm: 2 },
        K::FirmClosed { firm: 2 },
        K::ForestShrank { stock: 60000.0 },
        K::CommonsCollapsed,
        K::CommonsEnclosed { owner: 0 },
        K::PolicyBlocked {
            reason: BlockReason::LeverUnavailable,
        },
        K::CommandActivated { id: 7 },
        K::BankFailed,
        K::RentShortfall {
            household: 3,
            amount: 12.0,
        },
        K::SpendingCut { shortfall: 99.5 },
        K::BankBailout { amount: 1e6 },
        K::ClassChanged {
            household: 2,
            to: ClassKey::Worker,
        },
        K::SystemSwitchStarted {
            to: SystemKey::Coop,
        },
        K::SystemSwitchCompleted {
            to: SystemKey::Coop,
        },
        K::BudgetExhausted,
        K::Collapse {
            reason: CollapseReason::MassUnemployment,
        },
    ];
    assert_eq!(kinds.len(), 20);
    for k in kinds {
        let e = SimEvent::new(k);
        assert!((1..=3).contains(&e.importance));
        round_trip(&e);
        let j = serde_json::to_value(e).unwrap();
        assert!(j["type"].is_string() && j["importance"].is_u64(), "{j}");
    }
    let j = serde_json::to_value(SimEvent::new(K::Hired {
        firm: 4,
        people: 1.0,
    }))
    .unwrap();
    assert_eq!(
        j,
        serde_json::json!({"importance":1,"type":"hired","firm":4,"people":1.0})
    );

    let traces = [
        TraceRecord::Emissions {
            source: EmissionSourceKind::Firm,
            id: Some(0),
            tonnes: 3.0,
        },
        TraceRecord::Emissions {
            source: EmissionSourceKind::Sink,
            id: None,
            tonnes: -1.0,
        },
        TraceRecord::Income {
            household: 1,
            kind: IncomeKindKey::Benefits,
            amount: 5.0,
        },
        TraceRecord::PolicyGap {
            lever: LeverKey::CarbonPrice,
            nominal: 50.0,
            effective: 31.0,
            net_lobby: 0.38,
        },
    ];
    for t in traces {
        round_trip(&t);
    }
    let j = serde_json::to_value(traces[3]).unwrap();
    assert_eq!(j["type"], "policy_gap");
    assert!(j.get("netLobby").is_some());
    round_trip(&EmissionLine {
        source: EmissionSourceKind::Households,
        id: None,
        tonnes: 1.0,
    });
}

#[test]
fn outcomes_serialise_as_the_contract_says() {
    let plain = GameOutcome::Plain(PlainOutcome::BudgetExhausted);
    assert_eq!(
        serde_json::to_value(plain).unwrap(),
        serde_json::json!("budget_exhausted")
    );
    let col = GameOutcome::Collapse {
        collapse: CollapseReason::BankFailureCascade,
    };
    assert_eq!(
        serde_json::to_value(col).unwrap(),
        serde_json::json!({"collapse":"bank_failure_cascade"})
    );
    round_trip(&plain);
    round_trip(&col);
    round_trip(&GameOutcome::Plain(PlainOutcome::Survived));
}

#[test]
fn snapshot_levers_actions_and_turn_result_round_trip() {
    let mut api = Api::init(7).unwrap();
    round_trip(&api.snapshot());
    round_trip(&api.levers().to_vec());
    round_trip(&api.actions().to_vec());
    let cmds = [
        num(LeverKey::CarbonPrice, 50.0),
        Command::Place {
            kind: InfraKind::PublicHousing,
            count: 1,
            tile: None,
        },
        num(LeverKey::CampaignCap, 0.5),
    ];
    round_trip(&api.preview(&cmds).unwrap());
    let r = api.submit_turn(&cmds).unwrap();
    round_trip(&r);
    round_trip(&r.snapshot);
    round_trip(&api.cancel_pending(r.accepted[0].id).unwrap());
    round_trip(&ApiError::new("game_over", "x"));
    let j = serde_json::to_value(&r.snapshot).unwrap();
    for k in [
        "schemaVersion",
        "game",
        "turn",
        "tick",
        "gameOver",
        "pc",
        "system",
        "buildings",
        "pending",
        "policy",
        "netLobby",
        "unionDensity",
        "indicators",
        "households",
        "firms",
        "bank",
        "government",
        "nature",
        "housing",
        "energy",
        "history",
    ] {
        assert!(j.get(k).is_some(), "snapshot lacks {k}");
    }
    assert_eq!(j["policy"].as_object().unwrap().len(), 19);
    assert_eq!(
        j["game"],
        serde_json::json!({"startYear":2031,"firstTurn":1,"lastTurn":160,"turnsPerYear":4})
    );
    assert_eq!(j["gameOver"], serde_json::Value::Null);
    assert_eq!(j["history"].as_array().unwrap().len(), 1);
}

#[test]
fn initial_town_building_counts_and_shape() {
    let api = Api::init(1).unwrap();
    let s = api.snapshot();
    assert_eq!(
        (
            s.buildings.public_housing,
            s.buildings.fossil_plant,
            s.buildings.renewable_plant
        ),
        (0, 5, 0)
    );
    assert_eq!(s.households.len(), 6);
    assert_eq!(s.firms.len(), 4);
    assert_eq!(s.firms.iter().filter(|f| f.active).count(), 2);
    assert_eq!(s.turn, 1);
    assert!(s.game_over.is_none());
    assert_eq!(s.pc.current, 60.0);
    assert!((s.pc.regen_next_turn - 8.0 * 0.7).abs() < 1e-12);
    assert_eq!(s.energy.fossil_cap, 100.0);
    // Lobbied levers carry an effective value, the others null.
    for (k, e) in &s.policy {
        let info = api.levers().iter().find(|l| l.key == *k).unwrap();
        assert_eq!(e.effective.is_some(), info.lobbied, "{k:?}");
    }
    assert_eq!(s.firms[0].name, "Factory");
    assert_eq!(s.firms[3].name, "Farm 2");
}

#[test]
fn metadata_is_consistent_with_params_and_start_policy() {
    let api = Api::init(1).unwrap();
    assert_eq!(api.levers().len(), 19);
    assert_eq!(api.actions().len(), 7);
    let params = &api.sim().params;
    let snap = api.snapshot();
    for l in api.levers() {
        let cfg = params.turn.lever(l.key.as_str());
        assert_eq!(l.base_cost_pc, cfg.base_cost);
        assert_eq!(l.lead_time, cfg.lead_time);
        assert_eq!(l.cost_model.unit, cfg.unit);
        assert_eq!(l.cost_model.min_size, cfg.min_size);
        assert!(l.min <= l.max);
        if let sim_api::snapshot::PolicyScalar::Number(v) = snap.policy[&l.key].nominal {
            assert!(
                v >= l.min && v <= l.max,
                "{:?} starts at {v}, outside {}..{}",
                l.key,
                l.min,
                l.max
            );
        }
    }
    let lobbied: Vec<_> = api
        .levers()
        .iter()
        .filter(|l| l.lobbied)
        .map(|l| l.key)
        .collect();
    assert_eq!(
        lobbied,
        vec![
            LeverKey::CarbonPrice,
            LeverKey::TaxProfit,
            LeverKey::WealthTax,
            LeverKey::EmissionsCap
        ]
    );
    let ec = api
        .levers()
        .iter()
        .find(|l| l.key == LeverKey::EmissionsCap)
        .unwrap();
    assert_eq!(ec.zero_means_none, Some(true));
    let ca = api
        .levers()
        .iter()
        .find(|l| l.key == LeverKey::CommonsAccess)
        .unwrap();
    assert_eq!(
        ca.options.as_deref(),
        Some(&["open".to_string(), "community".to_string()][..])
    );
    let camp = api
        .levers()
        .iter()
        .find(|l| l.key == LeverKey::CampaignCap)
        .unwrap();
    assert!(!camp.available_in.contains(&SystemKey::Market));
    let plant = api
        .actions()
        .iter()
        .find(|a| a.key == "place:fossil_plant")
        .unwrap();
    assert_eq!(plant.money_cost, params.government.plant_cost_usd);
    assert_eq!(plant.lead_time, 4);
}

#[test]
fn preview_validates_ranges_and_availability() {
    let api = Api::init(1).unwrap();
    let blocked = |c: Command| api.preview(&[c]).unwrap()[0].blocked;
    assert_eq!(
        blocked(num(LeverKey::CarbonPrice, 1e9)),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(num(LeverKey::CarbonPrice, -1.0)),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(num(LeverKey::CarbonPrice, f64::NAN)),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(num(LeverKey::BankCapitalRatio, 0.0)),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(Command::SetPolicy {
            lever: LeverKey::Bailout,
            value: PolicyValue::Number(1.0)
        }),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(Command::Place {
            kind: InfraKind::FossilPlant,
            count: 0,
            tile: None
        }),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(Command::Place {
            kind: InfraKind::FossilPlant,
            count: 100_000,
            tile: None
        }),
        Some(BlockReason::InvalidCommand)
    );
    assert_eq!(
        blocked(Command::SetPolicy {
            lever: LeverKey::CommonsAccess,
            value: PolicyValue::Commons(CommonsAccessWire::Community { cap: f64::INFINITY }),
        }),
        Some(BlockReason::InvalidCommand)
    );
    // Market rules from validate(): no campaign cap, no community commons.
    assert_eq!(
        blocked(num(LeverKey::CampaignCap, 0.5)),
        Some(BlockReason::LeverUnavailable)
    );
    assert_eq!(
        blocked(Command::SetPolicy {
            lever: LeverKey::CommonsAccess,
            value: PolicyValue::Commons(CommonsAccessWire::Community { cap: 0.5 }),
        }),
        Some(BlockReason::LeverUnavailable)
    );
    assert_eq!(
        blocked(Command::SwitchSystem {
            to: SystemKey::Market
        }),
        Some(BlockReason::SameSystem)
    );
    assert_eq!(
        blocked(Command::Remove {
            kind: InfraKind::PublicHousing,
            count: 1
        }),
        Some(BlockReason::NothingToRemove)
    );
    assert_eq!(blocked(num(LeverKey::CarbonPrice, 100.0)), None);
    // Previews keep the input order and length even with invalid commands in between.
    let p = api
        .preview(&[
            num(LeverKey::CarbonPrice, 100.0),
            num(LeverKey::CarbonPrice, 1e9),
            num(LeverKey::TaxWage, 0.3),
        ])
        .unwrap();
    assert_eq!(p.len(), 3);
    assert_eq!(p[1].blocked, Some(BlockReason::InvalidCommand));
    assert_eq!(p[1].pc, 0.0);
    assert!(p[2].blocked.is_none());
}

#[test]
fn second_switch_system_is_transition_in_progress() {
    let mut api = Api::init(1).unwrap();
    let sw = Command::SwitchSystem {
        to: SystemKey::Regulated,
    };
    // Within one batch: the second sees the first as pending.
    let p = api
        .preview(&[
            sw,
            Command::SwitchSystem {
                to: SystemKey::Coop,
            },
        ])
        .unwrap();
    assert_eq!(p[0].blocked, None);
    assert_eq!(p[1].blocked, Some(BlockReason::TransitionInProgress));
    // Across turns: pending, then in transition.
    let r = api.submit_turn(&[sw]).unwrap();
    assert_eq!(r.accepted.len(), 1);
    let again = api
        .preview(&[Command::SwitchSystem {
            to: SystemKey::Coop,
        }])
        .unwrap();
    assert_eq!(again[0].blocked, Some(BlockReason::TransitionInProgress));
    let r2 = api.submit_turn(&[]).unwrap();
    assert!(
        r2.snapshot.system.transition.is_some()
            || r2.snapshot.system.current == SystemKey::Regulated
    );
    let during = api
        .preview(&[Command::SwitchSystem {
            to: SystemKey::Coop,
        }])
        .unwrap();
    if r2.snapshot.system.transition.is_some() {
        assert_eq!(during[0].blocked, Some(BlockReason::TransitionInProgress));
    }
}

#[test]
fn bad_inputs_never_panic() {
    let mut api = Api::init(3).unwrap();
    let nasty = [
        num(LeverKey::CarbonPrice, f64::NAN),
        num(LeverKey::MinWage, f64::INFINITY),
        num(LeverKey::PublicSpending, 1e300),
        Command::Place {
            kind: InfraKind::RenewablePlant,
            count: u32::MAX,
            tile: None,
        },
        Command::Remove {
            kind: InfraKind::FossilPlant,
            count: u32::MAX,
        },
    ];
    let r = api.submit_turn(&nasty).unwrap();
    assert_eq!(r.accepted.len(), 0);
    assert_eq!(r.rejected.len(), 5);
    assert!(r
        .rejected
        .iter()
        .all(|x| x.reason == BlockReason::InvalidCommand));
    assert_eq!(
        r.months[0]
            .events
            .iter()
            .filter(|e| matches!(e.kind, SimEventKind::PolicyBlocked { .. }))
            .count(),
        5
    );
    assert_eq!(api.cancel_pending(999).unwrap_err().code, "no_such_pending");
    for blob in [
        "",
        "{",
        "[]",
        "{\"schemaVersion\":99}",
        "{\"schemaVersion\":1}",
    ] {
        let before = api.export_state().unwrap();
        assert!(api.import_state(blob).is_err(), "{blob}");
        assert_eq!(
            api.export_state().unwrap(),
            before,
            "failed import changed the game"
        );
    }
    assert_eq!(
        api.import_state("{\"schemaVersion\":99}").unwrap_err().code,
        "schema_mismatch"
    );
}
