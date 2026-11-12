//! Behaviour tests through the API: costs, halting, determinism, long runs, money timing.
use sim_api::content;
use sim_api::keys::{BlockReason, GameOutcome, InfraKind, LeverKey, PlainOutcome, SystemKey};
use sim_api::*;

fn num(lever: LeverKey, v: f64) -> Command {
    Command::SetPolicy {
        lever,
        value: PolicyValue::Number(v),
    }
}

/// A place command on the `n`-th currently valid tile for `kind`.
fn place_at(api: &Api, kind: InfraKind, n: usize) -> Command {
    let tiles = api.valid_tiles(kind);
    Command::Place {
        kind,
        tile: tiles[n % tiles.len()],
    }
}

fn place(kind: InfraKind, tile: Tile) -> Command {
    Command::Place { kind, tile }
}

fn batch(api: &Api, i: usize) -> Vec<Command> {
    match i {
        0 => vec![
            num(LeverKey::CarbonPrice, 80.0),
            num(LeverKey::CarbonPrice, 120.0),
            place_at(api, InfraKind::PublicHousing, 0),
            place_at(api, InfraKind::RenewablePlant, 5),
            place_at(api, InfraKind::Road, 9),
            // The same tile twice: the second is occupied by the first.
            place_at(api, InfraKind::Road, 9),
            // Off the map: refused by the wire layer.
            place(InfraKind::Road, Tile { x: 40, y: 2 }),
            num(LeverKey::CampaignCap, 0.5),
            num(LeverKey::TaxWage, 9.0),
            Command::SwitchSystem {
                to: SystemKey::Regulated,
            },
            Command::SwitchSystem {
                to: SystemKey::Coop,
            },
        ],
        1 => {
            let plant = api.snapshot().plants[0].tile;
            vec![
                num(LeverKey::Benefit, 1000.0),
                Command::SetPolicy {
                    lever: LeverKey::Bailout,
                    value: PolicyValue::Bool(true),
                },
                Command::Remove { tile: plant },
                // Nothing stands here.
                Command::Remove {
                    tile: api.valid_tiles(InfraKind::Road)[3],
                },
                place_at(api, InfraKind::Road, 20),
            ]
        }
        // More than the political capital can pay for: the tail is refused.
        _ => (0..40)
            .map(|n| place_at(api, InfraKind::FossilPlant, n))
            .collect(),
    }
}

#[test]
fn preview_costs_equal_what_submit_charges() {
    let mut api = Api::init(11).unwrap();
    let mut tile_commands = 0;
    let mut refused_tiles = 0;
    for i in 0..3 {
        let batch = batch(&api, i);
        let pc_before = api.snapshot().pc.current;
        let preview = api.preview(&batch).unwrap();
        let r = api.submit_turn(&batch).unwrap();
        let mut charged = 0.0;
        for (i, p) in preview.iter().enumerate() {
            let rejected = r.rejected.iter().find(|x| x.index == i);
            if matches!(batch[i], Command::Place { .. } | Command::Remove { .. }) {
                tile_commands += 1;
                if !p.tile_valid {
                    refused_tiles += 1;
                    assert!(p.blocked.is_some() && p.tile_reason.is_some());
                }
            }
            match (p.blocked, rejected) {
                (Some(b), Some(x)) => assert_eq!(b, x.reason),
                (None, None) => {
                    let item = r
                        .accepted
                        .iter()
                        .find(|a| a.command == batch[i])
                        .expect("accepted item");
                    assert_eq!(item.pc_cost, p.pc);
                    assert_eq!(item.money_cost, p.money);
                    assert_eq!(item.effective_turn, p.effective_turn);
                    charged += item.pc_cost;
                }
                other => panic!("preview and submit disagree on command {i}: {other:?}"),
            }
        }
        assert_eq!(r.pc_before, pc_before);
        let regen = r.snapshot.pc.regen_next_turn;
        // pcAfter = PC after charging plus regeneration, capped.
        let expect = (pc_before - charged + regen).min(r.snapshot.pc.cap);
        assert!(
            (r.pc_after - expect).abs() < 1e-9,
            "{} vs {expect}",
            r.pc_after
        );
        assert_eq!(r.snapshot.pc.current, r.pc_after);
        if i == 2 {
            assert!(r
                .rejected
                .iter()
                .any(|x| x.reason == BlockReason::InsufficientPoliticalCapital));
        }
    }
    assert!(tile_commands >= 10 && refused_tiles >= 3);
}

#[test]
fn cancel_refunds_pc_only() {
    let mut api = Api::init(5).unwrap();
    let first = place_at(&api, InfraKind::PublicHousing, 0);
    let r = api
        .submit_turn(&[first, num(LeverKey::TaxCapital, 0.1)])
        .unwrap();
    let id = r.accepted[0].id;
    let pc = api.snapshot().pc.current;
    let c = api.cancel_pending(id).unwrap();
    assert!((c.refund_pc - 0.5 * r.accepted[0].pc_cost).abs() < 1e-12);
    assert!((c.snapshot.pc.current - (pc + c.refund_pc).min(100.0)).abs() < 1e-12);
    assert!(c.snapshot.pending.iter().all(|p| p.id != id));
    assert_eq!(api.cancel_pending(id).unwrap_err().code, "no_such_pending");
}

fn tiny_budget_content() -> String {
    let mut v: serde_json::Value = serde_json::from_str(content::PARAMS).unwrap();
    v["nature"]["budget_b0"] = serde_json::json!(5.0);
    v.to_string()
}

#[test]
fn halts_at_the_first_outcome() {
    let params = tiny_budget_content();
    let c = sim_params::ContentJson {
        params: &params,
        initial_town: content::INITIAL_TOWN,
        market: content::MARKET,
        regulated: content::REGULATED,
        coop: content::COOP,
    };
    let mut api = Api::from_content(&c, content::LEVERS, 2).unwrap();
    assert!(api.snapshot().game_over.is_none());
    let r = api.submit_turn(&[]).unwrap();
    let go = r
        .snapshot
        .game_over
        .expect("budget should be gone in turn 1");
    // With a tiny budget the economy may collapse before the budget check fires; either
    // way the first outcome is the one reported.
    assert_ne!(go.outcome, GameOutcome::Plain(PlainOutcome::Survived));
    assert_eq!(go.turn, 1);
    assert!(r.months.iter().flat_map(|m| &m.events).any(|e| matches!(
        e.kind,
        SimEventKind::BudgetExhausted | SimEventKind::Collapse { .. }
    )));
    let tick_after = api.snapshot().tick;
    for _ in 0..2 {
        let e = api.submit_turn(&[]).unwrap_err();
        assert_eq!(e.code, "game_over");
    }
    assert_eq!(api.cancel_pending(1).unwrap_err().code, "game_over");
    assert_eq!(api.snapshot().tick, tick_after, "no ticks after game over");
    assert_eq!(api.snapshot().history.len(), 1);
    // A halted game still saves and loads, and stays halted.
    let blob = api.export_state().unwrap();
    let mut other = Api::from_content(&c, content::LEVERS, 99).unwrap();
    let snap = other.import_state(&blob).unwrap();
    assert_eq!(snap.game_over, api.snapshot().game_over);
    assert_eq!(other.submit_turn(&[]).unwrap_err().code, "game_over");
}

#[test]
fn export_import_round_trip_is_deterministic() {
    let mut a = Api::init(21).unwrap();
    let plan = |api: &Api, t: u32| -> Vec<Command> {
        match t {
            1 => vec![
                num(LeverKey::CarbonPrice, 60.0),
                place_at(api, InfraKind::RenewablePlant, 3),
                place_at(api, InfraKind::Road, 30),
            ],
            3 => vec![num(LeverKey::Ubi, 100.0)],
            _ => vec![],
        }
    };
    for t in 1..=6 {
        let cmds = plan(&a, t);
        a.submit_turn(&cmds).unwrap();
    }
    let blob = a.export_state().unwrap();
    let mut b = Api::init(777).unwrap();
    let snap = b.import_state(&blob).unwrap();
    assert_eq!(snap, a.snapshot());
    assert_eq!(b.export_state().unwrap(), blob, "export is stable");
    for t in 7..=12 {
        let (ca, cb) = (plan(&a, t), plan(&b, t));
        assert_eq!(ca, cb);
        let ra = a.submit_turn(&ca).unwrap();
        let rb = b.submit_turn(&cb).unwrap();
        assert_eq!(ra, rb, "turn {t} differs after import");
    }
    assert_eq!(a.export_state().unwrap(), b.export_state().unwrap());
    assert_eq!(a.snapshot().history.len(), 12);
    // A save from other content is refused.
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["paramsHash"] = serde_json::json!("0000000000000000");
    assert_eq!(
        b.import_state(&v.to_string()).unwrap_err().code,
        "params_mismatch"
    );
}

#[test]
fn builds_are_paid_at_activation_not_at_queueing() {
    let mut api = Api::init(1).unwrap();
    let mut control = Api::init(1).unwrap();
    let cost = api.sim().params.government.plant_cost_usd;
    let lead = api
        .sim()
        .params
        .turn
        .lever("place_renewable_plant")
        .lead_time;
    let build = place_at(&api, InfraKind::RenewablePlant, 12);
    let tile = match build {
        Command::Place { tile, .. } => tile,
        _ => unreachable!(),
    };
    let p = api.preview(&[build]).unwrap();
    assert_eq!(
        (p[0].money, p[0].lead_time, p[0].effective_turn),
        (cost, lead, 1 + lead)
    );
    let r1 = api.submit_turn(&[build]).unwrap();
    control.submit_turn(&[]).unwrap();
    assert_eq!(r1.accepted[0].money_cost, cost);
    assert_eq!(r1.accepted[0].command, build, "the tile is kept");
    // Same seed, same economy: until activation the queued build moves no money at all and
    // the tile stays empty (but is no longer offered).
    for t in 2..=lead {
        let r = api.submit_turn(&[]).unwrap();
        let c = control.submit_turn(&[]).unwrap();
        assert_eq!(r.snapshot.buildings.renewable_plant, 0, "turn {t}");
        assert_eq!(r.snapshot.pending.len(), 1);
        assert_eq!(r.snapshot.government, c.snapshot.government, "turn {t}");
        assert_eq!(r.snapshot.bank, c.snapshot.bank, "turn {t}");
    }
    let r5 = api.submit_turn(&[]).unwrap();
    let c5 = control.submit_turn(&[]).unwrap();
    assert_eq!(r5.activated.len(), 1);
    assert_eq!(r5.activated[0].money_cost, cost);
    assert_eq!(r5.snapshot.buildings.renewable_plant, 1);
    assert!(r5.snapshot.pending.is_empty());
    let at = &r5.snapshot.map.tiles[(tile.y * 12 + tile.x) as usize];
    assert_eq!(at.kind, sim_api::keys::TileKindKey::RenewablePlant);
    assert!(r5
        .months
        .iter()
        .flat_map(|m| &m.events)
        .any(|e| matches!(e.kind, SimEventKind::Built { tile: t, .. } if t == tile)));
    let net = |s: &Snapshot| s.government.cash - s.government.debt;
    let paid = net(&c5.snapshot) - net(&r5.snapshot);
    assert!(
        (0.5 * cost..1.5 * cost).contains(&paid),
        "the bill is paid in the activation turn, got {paid}"
    );
}

/// A plan that mixes every kind of command over a full game.
fn scripted(api: &Api, turn: u32) -> Vec<Command> {
    let remove_fossil = || {
        api.snapshot()
            .plants
            .iter()
            .find(|p| p.kind == sim_api::snapshot::PlantKindKey::Fossil)
            .map(|p| vec![Command::Remove { tile: p.tile }])
            .unwrap_or_default()
    };
    match turn {
        1 => vec![
            num(LeverKey::CarbonPrice, 60.0),
            num(LeverKey::GreenSubsidy, 0.3),
            place_at(api, InfraKind::PublicHousing, 0),
            place_at(api, InfraKind::Road, 7),
        ],
        2 => vec![Command::SwitchSystem {
            to: SystemKey::Regulated,
        }],
        6 => vec![
            num(LeverKey::CarbonPrice, 140.0),
            place_at(api, InfraKind::RenewablePlant, 1),
        ],
        10 => vec![
            num(LeverKey::CampaignCap, 0.3),
            Command::SetPolicy {
                lever: LeverKey::CommonsAccess,
                value: PolicyValue::Commons(CommonsAccessWire::Community { cap: 0.8 }),
            },
        ],
        t if t % 8 == 0 => vec![
            place_at(api, InfraKind::RenewablePlant, t as usize),
            place_at(api, InfraKind::Road, t as usize * 3),
            num(LeverKey::WealthTax, 0.01),
        ],
        t if t % 13 == 0 => remove_fossil(),
        t if t % 17 == 0 => vec![num(LeverKey::Benefit, 900.0 + f64::from(t))],
        _ => vec![],
    }
}

/// No switch, gentle policy: a plan that should last longer than `scripted`.
fn mild(api: &Api, turn: u32) -> Vec<Command> {
    match turn {
        1 => vec![
            num(LeverKey::CarbonPrice, 40.0),
            num(LeverKey::GreenSubsidy, 0.2),
        ],
        8 => vec![
            num(LeverKey::CarbonPrice, 80.0),
            place_at(api, InfraKind::RenewablePlant, 2),
        ],
        20 => vec![num(LeverKey::CarbonPrice, 120.0)],
        t if t % 16 == 0 => vec![place_at(api, InfraKind::RenewablePlant, t as usize)],
        _ => vec![],
    }
}

type Plan = fn(&Api, u32) -> Vec<Command>;

fn run_to_the_end(seed: u64, plan: Plan) -> (u32, Snapshot) {
    let mut api = Api::init(seed).unwrap();
    let mut played = 0u32;
    loop {
        let t = api.snapshot().turn;
        let cmds = plan(&api, t);
        match api.submit_turn(&cmds) {
            Ok(r) => {
                played += 1;
                assert_eq!(r.turn, t);
                assert_eq!(r.months.len(), 3);
                for (m, f) in r.months.iter().enumerate() {
                    assert_eq!(usize::from(f.month), m);
                    assert!(f.flows.emissions.len() >= 2);
                }
                assert_eq!(r.snapshot.history.len(), played as usize);
                assert_eq!(r.snapshot.history.last().unwrap().turn, t);
                assert_eq!(r.snapshot.people.len(), 22);
                assert_eq!(r.snapshot.map.tiles.len(), 144);
            }
            Err(e) => {
                assert_eq!(e.code, "game_over");
                break;
            }
        }
        assert!(played <= 40);
    }
    (played, api.snapshot())
}

#[test]
fn full_runs_through_the_api_never_panic_and_record_history() {
    let plans: [(&str, Plan); 3] = [
        ("scripted", scripted),
        ("mild", mild),
        ("no action", |_, _| vec![]),
    ];
    for (name, plan) in plans {
        let (played, s) = run_to_the_end(42, plan);
        eprintln!("{name}: played {played} turns, outcome {:?}", s.game_over);
        assert_eq!(s.history.len(), played as usize, "{name}");
        let go = s.game_over.expect("the loop only ends at game over");
        if played == 40 {
            assert_eq!(go.turn, 41, "{name}");
            assert_eq!(
                go.outcome,
                GameOutcome::Plain(PlainOutcome::Survived),
                "{name}"
            );
        } else {
            assert!(played < 40);
            assert_eq!(go.turn, played, "{name}: history stops at the halting turn");
        }
        for h in &s.history {
            let j = serde_json::to_value(h).unwrap();
            assert!(
                j.as_object().unwrap().values().all(|v| v.is_number()),
                "{name}"
            );
        }
    }
}

#[test]
fn frames_carry_real_flows() {
    let mut api = Api::init(1).unwrap();
    let r = api
        .submit_turn(&[num(LeverKey::CarbonPrice, 50.0)])
        .unwrap();
    let snap = &r.snapshot;
    let employed = snap.people.iter().filter(|p| p.job.is_some()).count();
    assert!(employed > 0);
    for (mi, f) in r.months.iter().enumerate() {
        // One commute and one wage per person with a firm job, with real tiles. The snapshot is
        // the end of the turn, so jobs are compared on the last month's frame (people can be
        // hired or fired between months).
        let last_month = mi == r.months.len() - 1;
        if last_month {
            assert_eq!(f.flows.commutes.len(), employed);
        }
        for c in f.flows.commutes.iter().filter(|_| last_month) {
            let p = &snap.people[c.person as usize];
            assert_eq!(p.job.unwrap().firm, c.firm);
            assert_eq!(c.household, p.household);
            assert_eq!(c.to, snap.firms[c.firm as usize].tile);
            assert_eq!(c.tiles, p.commute_tiles);
            assert_eq!(c.minutes, p.commute_minutes);
            let home = snap.households[c.household as usize].home;
            assert_eq!(
                c.from,
                home.map(|h| snap.homes.iter().find(|x| x.id == h).unwrap().tile)
            );
        }
        assert!(!f.flows.wages.is_empty());
        if last_month {
            assert!(f.flows.wages.len() <= employed);
        }
        assert!(!f.flows.rent.is_empty());
        let sources: Vec<_> = f.flows.emissions.iter().map(|e| e.source).collect();
        assert!(sources.contains(&sim_api::events::EmissionSourceKind::Sink));
        assert!(sources.contains(&sim_api::events::EmissionSourceKind::Households));
        assert!(sources.contains(&sim_api::events::EmissionSourceKind::Plant));
        for e in &f.flows.emissions {
            // Plant lines carry the plant's tile; no other source does.
            assert_eq!(
                e.tile.is_some(),
                e.source == sim_api::events::EmissionSourceKind::Plant
            );
        }
    }
    // Purchases are real: household to firm, matching the purchase events of the month.
    let purchases: usize = r.months.iter().map(|m| m.flows.purchases.len()).sum();
    assert!(purchases > 0, "no purchase flows in a whole turn");
    for m in &r.months {
        let events = m
            .events
            .iter()
            .filter(|e| matches!(e.kind, SimEventKind::Purchase { .. }))
            .count();
        assert_eq!(events, m.flows.purchases.len());
        for p in &m.flows.purchases {
            assert!(p.amount > 0.0);
            assert_eq!(snap.firms[p.firm as usize].sector, p.sector);
        }
    }
    // Wage flows add up to what the people were paid in the last tick.
    let last = &r.months[2].flows;
    let wage_sum: f64 = last.wages.iter().map(|w| w.amount).sum();
    let paid: f64 = snap
        .people
        .iter()
        .filter(|p| p.job.is_some())
        .map(|p| p.wage)
        .sum();
    assert!((wage_sum - paid).abs() < 1e-6);
    // Events and trace of the turn are not lost: every tick's events appear in some frame.
    let n_events: usize = r.months.iter().map(|m| m.events.len()).sum();
    assert!(n_events > 0);
    assert!(r
        .trace
        .iter()
        .any(|t| matches!(t, TraceRecord::PolicyGap { .. })));
    assert!(r.trace.iter().any(|t| matches!(
        t,
        TraceRecord::Emissions {
            source: sim_api::events::EmissionSourceKind::Plant,
            tile: Some(_),
            ..
        }
    )));
    // Per-person events exist.
    assert!(r
        .months
        .iter()
        .flat_map(|m| &m.events)
        .any(|e| matches!(e.kind, SimEventKind::Commuted { .. })));
}

#[test]
fn the_api_adds_nothing_to_the_simulation() {
    // Same seed, same commands: the wrapped sim ends in exactly the state the bare engine reaches.
    let params = sim_params::Params::from_content(&content::default_content()).unwrap();
    let mut bare = sim_engine::init(params, 42).unwrap();
    let mut api = Api::init(42).unwrap();
    for t in 1..=12u32 {
        let wire = mild(&api, t);
        let sim_cmds: Vec<sim_types::Command> = wire
            .iter()
            .map(|c| match *c {
                Command::SetPolicy {
                    lever: LeverKey::CarbonPrice,
                    value: PolicyValue::Number(v),
                } => sim_types::Command::SetPolicy(sim_types::PolicyChange::CarbonPrice(
                    sim_types::Cents::from_dollars(v),
                )),
                Command::SetPolicy {
                    lever: LeverKey::GreenSubsidy,
                    value: PolicyValue::Number(v),
                } => sim_types::Command::SetPolicy(sim_types::PolicyChange::GreenSubsidy(
                    sim_types::Rate(v),
                )),
                Command::Place { kind, tile } => sim_types::Command::Place {
                    kind: kind.to_sim(),
                    tile: tile.to_sim().unwrap(),
                },
                _ => unreachable!("mild() only uses these"),
            })
            .collect();
        sim_engine::submit_turn(&mut bare, &sim_cmds).unwrap();
        api.submit_turn(&wire).unwrap();
    }
    assert_eq!(bare.state, api.sim().state);
    assert_eq!(bare.rng, api.sim().rng);
}

#[test]
fn import_refuses_crafted_huge_money_instead_of_panicking() {
    let mut a = Api::init(21).unwrap();
    a.submit_turn(&[]).unwrap();
    let blob = a.export_state().unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    v["state"]["households"][0]["savings"] = serde_json::json!(9_223_372_036_854_775_000_i64);
    let mut b = Api::init(1).unwrap();
    let err = b.import_state(&v.to_string()).unwrap_err();
    assert_eq!(err.code, "bad_state");
    // The untouched save still loads.
    assert!(b.import_state(&blob).is_ok());
}
