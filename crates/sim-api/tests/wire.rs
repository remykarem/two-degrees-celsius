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
            tile: Tile { x: 4, y: 7 },
        },
        Command::Remove {
            tile: Tile { x: 3, y: 3 },
        },
        Command::Place {
            kind: InfraKind::Road,
            tile: Tile { x: 0, y: 11 },
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
        serde_json::json!({"type":"remove","tile":{"x":3,"y":3}})
    );
    assert_eq!(j[7]["kind"], "road");
    // parse_commands accepts the contract's literal JSON, integers included.
    let parsed = api::parse_commands(
        r#"[{"type":"set_policy","lever":"tax_wage","value":0},{"type":"place","kind":"renewable_plant","tile":{"x":1,"y":2}}]"#,
    )
    .unwrap();
    assert_eq!(parsed.len(), 2);
    // A place command without a tile is a bad request: the tile is required.
    assert_eq!(
        api::parse_commands(r#"[{"type":"place","kind":"road"}]"#)
            .unwrap_err()
            .code,
        "bad_request"
    );
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
            person: 4,
            firm: 0,
            tiles: 7,
            minutes: 30,
        },
        K::PersonHired { person: 4, firm: 0 },
        K::PersonFired { person: 5, firm: 1 },
        K::Purchase {
            household: 2,
            firm: 2,
            amount: 31.5,
        },
        K::HouseholdMoved {
            household: 7,
            to: 3,
        },
        K::HouseholdEvicted { household: 6 },
        K::Built {
            kind: InfraKind::Road,
            tile: Tile { x: 4, y: 6 },
        },
        K::Removed {
            tile: Tile { x: 4, y: 6 },
        },
        K::ElectricityRationed { served: 0.8 },
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
        K::FireStarted {
            tile: Tile { x: 1, y: 0 },
        },
        K::FireSpread {
            from: Tile { x: 1, y: 0 },
            to: Tile { x: 2, y: 0 },
        },
        K::FireOut {
            tile: Tile { x: 1, y: 0 },
        },
        K::ForestBurned {
            tile: Tile { x: 1, y: 0 },
        },
        K::DroughtStarted {
            severity: 0.6,
            months: 4,
        },
        K::DroughtEnded,
        K::HomeDamaged { home: 2 },
        K::PrivateJetFlight {
            household: 8,
            tonnes: 2.0,
        },
        K::PilotLetGo { person: 18 },
        K::PilotHired { person: 18 },
    ];
    assert_eq!(kinds.len(), 36);
    for k in kinds {
        let e = SimEvent::new(k);
        assert!((1..=3).contains(&e.importance));
        round_trip(&e);
        let j = serde_json::to_value(e).unwrap();
        assert!(j["type"].is_string() && j["importance"].is_u64(), "{j}");
    }
    let j = serde_json::to_value(SimEvent::new(K::PersonHired { person: 9, firm: 4 })).unwrap();
    assert_eq!(
        j,
        serde_json::json!({"importance":1,"type":"person_hired","person":9,"firm":4})
    );
    let j = serde_json::to_value(SimEvent::new(K::HouseholdMoved {
        household: 7,
        to: 3,
    }))
    .unwrap();
    assert_eq!(j["type"], "household_moved");
    assert_eq!(j["importance"], 2);

    let traces = [
        TraceRecord::Emissions {
            source: EmissionSourceKind::Firm,
            id: Some(0),
            tile: None,
            tonnes: 3.0,
        },
        TraceRecord::Emissions {
            source: EmissionSourceKind::Sink,
            id: None,
            tile: None,
            tonnes: -1.0,
        },
        TraceRecord::Emissions {
            source: EmissionSourceKind::Plant,
            id: Some(1),
            tile: Some(Tile { x: 10, y: 5 }),
            tonnes: 12.0,
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
    let j = serde_json::to_value(traces[2]).unwrap();
    assert_eq!(j["source"], "plant");
    assert_eq!(j["tile"], serde_json::json!({"x":10,"y":5}));
    let j = serde_json::to_value(traces[4]).unwrap();
    assert_eq!(j["type"], "policy_gap");
    assert!(j.get("netLobby").is_some());
    round_trip(&EmissionLine {
        source: EmissionSourceKind::Households,
        id: None,
        tile: None,
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
            kind: InfraKind::Road,
            tile: Tile { x: 4, y: 6 },
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
        "map",
        "buildings",
        "pending",
        "policy",
        "netLobby",
        "unionDensity",
        "indicators",
        "people",
        "households",
        "homes",
        "plants",
        "firms",
        "bank",
        "government",
        "nature",
        "weather",
        "housing",
        "energy",
        "history",
    ] {
        assert!(j.get(k).is_some(), "snapshot lacks {k}");
    }
    assert_eq!(j["policy"].as_object().unwrap().len(), 20);
    assert_eq!(j["schemaVersion"], sim_api::SCHEMA_VERSION);
    assert_eq!(j["map"]["width"], 12);
    assert_eq!(j["map"]["tiles"].as_array().unwrap().len(), 144);
    for k in [
        "avgIncomePerPerson",
        "medianIncomePerPerson",
        "topHouseholdWealthShare",
    ] {
        assert!(j["indicators"][k].is_number(), "indicators lack {k}");
        assert!(j["history"][0][k].is_number(), "history lacks {k}");
    }
    assert_eq!(
        j["game"],
        serde_json::json!({"startYear":2026,"firstTurn":1,"lastTurn":40,"turnsPerYear":4,"ticksPerTurn":3,"temperatureLimit":1.5})
    );
    assert_eq!(j["gameOver"], serde_json::Value::Null);
    assert_eq!(j["history"].as_array().unwrap().len(), 1);
}

#[test]
fn initial_town_map_people_homes_and_shape() {
    let api = Api::init(1).unwrap();
    let s = api.snapshot();
    assert_eq!(s.people.len(), 22);
    assert_eq!(s.households.len(), 9);
    assert_eq!(s.homes.len(), 8);
    assert_eq!(s.households.iter().filter(|h| h.home.is_none()).count(), 1);
    assert_eq!(s.housing.unhoused_households, 1);
    assert_eq!(s.firms.len(), 4);
    assert_eq!(s.firms.iter().filter(|f| f.active).count(), 3);
    assert_eq!(s.turn, 1);
    assert!(s.game_over.is_none());
    assert_eq!(s.pc.current, 60.0);
    assert!((s.pc.regen_next_turn - 8.0 * 0.7).abs() < 1e-12);
    // Every person is in exactly one household, and the household lists them.
    for p in &s.people {
        let h = &s.households[p.household as usize];
        assert_eq!(h.members.iter().filter(|m| **m == p.id).count(), 1);
    }
    assert_eq!(
        s.households.iter().map(|h| h.members.len()).sum::<usize>(),
        22
    );
    // Buildings are a summary of the map.
    let count =
        |k: sim_api::keys::TileKindKey| s.map.tiles.iter().filter(|t| t.kind == k).count() as u32;
    use sim_api::keys::TileKindKey as T;
    assert_eq!(s.buildings.fossil_plant, count(T::FossilPlant));
    assert_eq!(s.buildings.renewable_plant, count(T::RenewablePlant));
    assert_eq!(s.buildings.public_housing, count(T::PublicHousing));
    assert_eq!(s.buildings.road, count(T::Road));
    assert_eq!(s.buildings.fossil_plant as usize, s.plants.len());
    assert!(s.buildings.fossil_plant >= 1);
    assert_eq!(count(T::Home), 8);
    assert_eq!(count(T::EstateGrounds), 3);
    assert_eq!(count(T::Runway), 3);
    assert_eq!(count(T::Bank), 1);
    assert_eq!(count(T::TownHall), 1);
    // Tiles are row-major, homes and plants agree with the map, plant tiles report emissions.
    for (i, t) in s.map.tiles.iter().enumerate() {
        assert_eq!((t.y * 12 + t.x) as usize, i);
        assert_eq!(t.buildable, t.kind == T::Empty);
        assert_eq!(
            t.emissions_monthly.is_some(),
            matches!(t.kind, T::FossilPlant | T::RenewablePlant)
        );
    }
    for h in &s.homes {
        let t = &s.map.tiles[(h.tile.y * 12 + h.tile.x) as usize];
        assert_eq!(t.reference, Some(h.id));
    }
    assert!(s.plants.iter().any(|p| p.emissions_monthly > 0.0));
    // Lobbied levers carry an effective value, the others null.
    for (k, e) in &s.policy {
        let info = api.levers().iter().find(|l| l.key == *k).unwrap();
        assert_eq!(e.effective.is_some(), info.lobbied, "{k:?}");
    }
    let names: Vec<_> = s.firms.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Factory", "Farm", "Shop", "Shop 2"]);
    assert_eq!(s.firms[2].sector, sim_api::keys::SectorKey::Services);
    // Inequality is harsh from the start; a household can have negative net worth.
    assert!(s.indicators.gini > 0.5, "{}", s.indicators.gini);
    assert!(s.indicators.top_household_wealth_share > 0.4);
    assert!(s.indicators.avg_income_per_person > 0.0);
    assert!(s.households.iter().any(|h| h.net_worth < 0.0));
}

#[test]
fn metadata_is_consistent_with_params_and_start_policy() {
    let api = Api::init(1).unwrap();
    assert_eq!(api.levers().len(), 20);
    assert_eq!(api.actions().len(), 11);
    for a in api.actions() {
        assert_eq!(a.needs_tile, a.key != "switch_system", "{}", a.key);
        assert_eq!(a.tile_rule.is_some(), a.needs_tile, "{}", a.key);
    }
    let road = api
        .actions()
        .iter()
        .find(|a| a.key == "place:road")
        .unwrap();
    assert_eq!(road.money_cost, api.sim().params.government.road_cost_usd);
    assert_eq!(
        road.lead_time,
        api.sim().params.turn.lever("place_road").lead_time
    );
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
    assert_eq!(
        plant.lead_time,
        params.turn.lever("place_fossil_plant").lead_time
    );
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
            tile: Tile { x: 6, y: 8 }
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
            tile: Tile {
                x: i32::MAX,
                y: i32::MIN,
            },
        },
        Command::Remove {
            tile: Tile { x: -1, y: 300 },
        },
    ];
    let r = api.submit_turn(&nasty).unwrap();
    assert_eq!(r.accepted.len(), 0);
    assert_eq!(r.rejected.len(), 5);
    assert_eq!(
        r.rejected.iter().map(|x| x.reason).collect::<Vec<_>>(),
        vec![
            BlockReason::InvalidCommand,
            BlockReason::InvalidCommand,
            BlockReason::InvalidCommand,
            BlockReason::TileInvalid,
            BlockReason::TileInvalid
        ]
    );
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

#[test]
fn old_saves_are_refused_with_a_clear_error() {
    let mut api = Api::init(3).unwrap();
    let blob = api.export_state().unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    assert_eq!(v["schemaVersion"], sim_api::SCHEMA_VERSION);
    v["schemaVersion"] = serde_json::json!(1);
    let e = api.import_state(&v.to_string()).unwrap_err();
    assert_eq!(e.code, "schema_mismatch");
    assert!(e.message.contains("schemaVersion"), "{}", e.message);
    // A current blob whose ids or tiles point at nothing is refused, not trusted.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["state"]["persons"][0]["household"] = serde_json::json!(99);
    assert_eq!(
        api.import_state(&v.to_string()).unwrap_err().code,
        "bad_state"
    );
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["state"]["homes"][0]["tile"] = serde_json::json!({"x": 200, "y": 200});
    assert_eq!(
        api.import_state(&v.to_string()).unwrap_err().code,
        "bad_state"
    );
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["state"]["map"]["cells"] = serde_json::json!([]);
    assert_eq!(
        api.import_state(&v.to_string()).unwrap_err().code,
        "bad_state"
    );
}

#[test]
fn tile_commands_report_tile_validity() {
    let api = Api::init(1).unwrap();
    let s = api.snapshot();
    let kind_at = |x: i32, y: i32| s.map.tiles[(y * 12 + x) as usize].kind;
    use sim_api::keys::TileKindKey as T;
    let place = |kind: InfraKind, x: i32, y: i32| Command::Place {
        kind,
        tile: Tile { x, y },
    };
    // Factory tile (occupied), forest (invalid), off the map (invalid), empty (valid).
    assert_eq!(kind_at(9, 3), T::Firm);
    assert_eq!(kind_at(0, 0), T::Forest);
    let p = api
        .preview(&[
            place(InfraKind::Road, 9, 3),
            place(InfraKind::Road, 0, 0),
            place(InfraKind::Road, 12, 0),
            place(InfraKind::Road, -1, 4),
            place(InfraKind::Road, 6, 7),
            Command::Remove {
                tile: Tile { x: 6, y: 7 },
            },
            Command::Remove {
                tile: Tile { x: 9, y: 3 },
            },
        ])
        .unwrap();
    let got: Vec<_> = p.iter().map(|x| (x.tile_valid, x.tile_reason)).collect();
    assert_eq!(
        got,
        vec![
            (false, Some(BlockReason::TileOccupied)),
            (false, Some(BlockReason::TileInvalid)),
            (false, Some(BlockReason::TileInvalid)),
            (false, Some(BlockReason::TileInvalid)),
            (true, None),
            (false, Some(BlockReason::NothingToRemove)),
            (false, Some(BlockReason::TileInvalid)),
        ]
    );
    assert_eq!(p[0].blocked, Some(BlockReason::TileOccupied));
    assert_eq!(p[4].blocked, None);
    // Non-tile commands are always tile-valid.
    let q = api.preview(&[num(LeverKey::TaxWage, 0.2)]).unwrap();
    assert!(q[0].tile_valid && q[0].tile_reason.is_none());
    // A second command on a tile that already has a queued build is occupied.
    let q = api
        .preview(&[
            place(InfraKind::Road, 4, 6),
            place(InfraKind::FossilPlant, 4, 6),
        ])
        .unwrap();
    assert_eq!(q[1].tile_reason, Some(BlockReason::TileOccupied));
}

#[test]
fn valid_tiles_match_the_map_and_queued_builds() {
    let mut api = Api::init(1).unwrap();
    let s = api.snapshot();
    let empty = s.map.tiles.iter().filter(|t| t.buildable).count();
    for k in InfraKind::ALL {
        let v = api.valid_tiles(k);
        assert_eq!(v.len(), empty, "{k:?}");
        // Row-major, and each one really is empty.
        for t in &v {
            assert!(s.map.tiles[(t.y * 12 + t.x) as usize].buildable);
        }
        assert!(v.windows(2).all(|w| (w[0].y, w[0].x) < (w[1].y, w[1].x)));
        // Every valid tile previews as valid for that kind.
        let cmds: Vec<_> = v
            .iter()
            .take(5)
            .map(|t| Command::Place { kind: k, tile: *t })
            .collect();
        assert!(api.preview(&cmds).unwrap().iter().all(|p| p.tile_valid));
    }
    // A queued road takes its tile out of every list.
    let t = api.valid_tiles(InfraKind::Road)[10];
    api.submit_turn(&[Command::Place {
        kind: InfraKind::Road,
        tile: t,
    }])
    .unwrap();
    for k in InfraKind::ALL {
        assert!(!api.valid_tiles(k).contains(&t), "{k:?}");
    }
}

// ---------- contract v1.2: weather ----------

#[test]
fn weather_block_health_fields_and_tile_state_match_the_contract() {
    let api = Api::init(1).unwrap();
    let j = serde_json::to_value(api.snapshot()).unwrap();
    let w = &j["weather"];
    assert_eq!(w["stress"], 0.0);
    // Contract v1.3: the stress curve is on the wire (content/params.json `weather`).
    assert_eq!(
        (w["onset"].as_f64(), w["full"].as_f64(), w["curve"].as_f64()),
        (Some(0.75), Some(0.1), Some(1.0))
    );
    assert!(w["drought"].is_null() && w["fires"].as_array().unwrap().is_empty());
    assert_eq!(
        w["forecast"],
        serde_json::json!({"fireRisk":"low","droughtRisk":"low"})
    );
    for k in ["forestHealth", "commonsHealth", "farmHealth"] {
        let v = j["nature"][k].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&v), "{k} {v}");
    }
    // Forest tiles carry a state; no other tile does.
    for t in j["map"]["tiles"].as_array().unwrap() {
        let forest = t["kind"] == "forest";
        assert_eq!(t.get("state").is_some(), forest, "{t}");
        if forest {
            assert_eq!(t["state"], "standing");
        }
    }
    assert_eq!(j["policy"]["resilience_spending"]["nominal"], 0.0);
    let m = &j["history"];
    assert!(m.as_array().unwrap().is_empty());
}

#[test]
fn history_points_carry_the_weather_metrics() {
    let mut api = Api::init(1).unwrap();
    let t = api.submit_turn(&[]).unwrap();
    let p = serde_json::to_value(t.snapshot.history.last().unwrap()).unwrap();
    for k in [
        "climateStress",
        "forestHealth",
        "commonsHealth",
        "farmHealth",
    ] {
        assert!(p[k].is_number(), "history lacks {k}");
    }
}

#[test]
fn weather_events_and_fire_sink_loss_have_the_documented_wire_shapes() {
    let ev = |k: SimEventKind| serde_json::to_value(SimEvent::new(k)).unwrap();
    use SimEventKind as K;
    let t = |x, y| Tile { x, y };
    assert_eq!(
        ev(K::FireSpread {
            from: t(1, 0),
            to: t(2, 0)
        }),
        serde_json::json!({"importance":1,"type":"fire_spread","from":{"x":1,"y":0},"to":{"x":2,"y":0}})
    );
    assert_eq!(
        ev(K::DroughtStarted {
            severity: 0.5,
            months: 3
        }),
        serde_json::json!({"importance":3,"type":"drought_started","severity":0.5,"months":3})
    );
    assert_eq!(ev(K::DroughtEnded)["type"], "drought_ended");
    assert_eq!(ev(K::ForestBurned { tile: t(1, 0) })["importance"], 3);
    let line = EmissionLine {
        source: EmissionSourceKind::FireSinkLoss,
        id: None,
        tile: None,
        tonnes: 0.2,
    };
    assert_eq!(
        serde_json::to_value(line).unwrap(),
        serde_json::json!({"source":"fire_sink_loss","tonnes":0.2})
    );
}

#[test]
fn a_weather_turn_reports_events_and_the_fire_sink_loss_trace() {
    let t = sim_api::fixtures::weather_sample().unwrap();
    let j = serde_json::to_value(&t).unwrap();
    let has_loss = j["trace"].as_array().unwrap().iter().any(|r| {
        r["type"] == "emissions"
            && r["source"] == "fire_sink_loss"
            && r["tonnes"].as_f64().unwrap() > 0.0
    });
    assert!(has_loss, "a turn with fires traces the sink lost");
    for m in j["months"].as_array().unwrap() {
        for e in m["events"].as_array().unwrap() {
            assert!(e["importance"].is_u64());
        }
    }
}

#[test]
fn fire_station_and_resilience_spending_work_through_the_wire() {
    let mut api = Api::init(1).unwrap();
    assert!(api
        .actions()
        .iter()
        .any(|a| a.key == "place:fire_station" && a.money_cost > 0.0));
    assert!(api.actions().iter().any(|a| a.key == "remove:fire_station"));
    let res = api
        .levers()
        .iter()
        .find(|l| l.key == LeverKey::ResilienceSpending)
        .unwrap();
    assert_eq!(res.lead_time, 1);
    let tile = api.valid_tiles(InfraKind::FireStation)[0];
    let cmds = [
        Command::Place {
            kind: InfraKind::FireStation,
            tile,
        },
        num(LeverKey::ResilienceSpending, 2000.0),
        num(LeverKey::ResilienceSpending, 1e9),
    ];
    let prev = api.preview(&cmds).unwrap();
    assert!(prev[0].tile_valid && prev[0].money > 0.0);
    // Out of range is refused before the engine sees it.
    let bad = api.preview(&cmds[2..]).unwrap();
    assert_eq!(bad[0].blocked, Some(BlockReason::InvalidCommand));
    let r = api.submit_turn(&cmds[..2]).unwrap();
    assert_eq!(r.accepted.len(), 2);
    for _ in 0..3 {
        api.submit_turn(&[]).unwrap();
    }
    let s = api.snapshot();
    assert_eq!(
        s.policy[&LeverKey::ResilienceSpending].nominal,
        sim_api::snapshot::PolicyScalar::Number(2000.0)
    );
    assert!(s
        .map
        .tiles
        .iter()
        .any(|t| t.kind == sim_api::keys::TileKindKey::FireStation));
}

#[test]
fn saves_round_trip_with_weather_and_refuse_old_or_inconsistent_ones() {
    let mut api = sim_api::Api::init(3).unwrap();
    for _ in 0..15 {
        api.submit_turn(&[]).unwrap();
    }
    let blob = api.export_state().unwrap();
    let snap = api.snapshot();
    let mut other = sim_api::Api::init(9).unwrap();
    assert_eq!(other.import_state(&blob).unwrap(), snap);
    assert_eq!(other.export_state().unwrap(), blob);
    // A version-3 save (before the ledger) is refused and says why.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["schemaVersion"] = serde_json::json!(3);
    let e = other.import_state(&v.to_string()).unwrap_err();
    assert_eq!(e.code, "schema_mismatch");
    assert!(
        e.message.contains("ledger") && e.message.contains("new game"),
        "{}",
        e.message
    );
    // A fire on a tile that is not forest is refused.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["state"]["weather"]["fires"] = serde_json::json!([{"tile":{"x":11,"y":11},"months_left":2}]);
    assert_eq!(
        other.import_state(&v.to_string()).unwrap_err().code,
        "bad_state"
    );
}
