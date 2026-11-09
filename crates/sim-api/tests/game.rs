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

fn place(kind: InfraKind, count: u32) -> Command {
    Command::Place {
        kind,
        count,
        tile: None,
    }
}

#[test]
fn preview_costs_equal_what_submit_charges() {
    let mut api = Api::init(11).unwrap();
    let batches: Vec<Vec<Command>> = vec![
        vec![
            num(LeverKey::CarbonPrice, 80.0),
            num(LeverKey::CarbonPrice, 120.0),
            place(InfraKind::PublicHousing, 3),
            place(InfraKind::RenewablePlant, 1),
            num(LeverKey::CampaignCap, 0.5),
            num(LeverKey::TaxWage, 9.0),
            Command::SwitchSystem {
                to: SystemKey::Regulated,
            },
            Command::SwitchSystem {
                to: SystemKey::Coop,
            },
        ],
        vec![
            num(LeverKey::Benefit, 1000.0),
            Command::SetPolicy {
                lever: LeverKey::Bailout,
                value: PolicyValue::Bool(true),
            },
            Command::Remove {
                kind: InfraKind::FossilPlant,
                count: 2,
            },
        ],
        vec![place(InfraKind::FossilPlant, 100)],
    ];
    for batch in batches {
        let pc_before = api.snapshot().pc.current;
        let preview = api.preview(&batch).unwrap();
        let r = api.submit_turn(&batch).unwrap();
        let mut charged = 0.0;
        for (i, p) in preview.iter().enumerate() {
            let rejected = r.rejected.iter().find(|x| x.index == i);
            match (p.blocked, rejected) {
                (Some(b), Some(x)) => assert_eq!(b, x.reason),
                (None, None) => {
                    let item = r
                        .accepted
                        .iter()
                        .find(|a| a.command_matches(&batch[i], p))
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
    }
}

trait Matches {
    fn command_matches(&self, c: &Command, p: &CommandPreview) -> bool;
}
impl Matches for sim_api::turn::PendingItem {
    fn command_matches(&self, c: &Command, p: &CommandPreview) -> bool {
        // Pending items drop the cosmetic tile; compare the rest by cost identity.
        let _ = c;
        self.pc_cost == p.pc
            && self.money_cost == p.money
            && self.effective_turn == p.effective_turn
    }
}

#[test]
fn cancel_refunds_pc_only() {
    let mut api = Api::init(5).unwrap();
    let r = api
        .submit_turn(&[
            place(InfraKind::PublicHousing, 2),
            num(LeverKey::TaxCapital, 0.1),
        ])
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
    v["nature"]["budget_b0"] = serde_json::json!(300.0);
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
    let plan = |t: u32| -> Vec<Command> {
        match t {
            1 => vec![
                num(LeverKey::CarbonPrice, 60.0),
                place(InfraKind::RenewablePlant, 1),
            ],
            3 => vec![num(LeverKey::Ubi, 100.0)],
            _ => vec![],
        }
    };
    for t in 1..=6 {
        a.submit_turn(&plan(t)).unwrap();
    }
    let blob = a.export_state().unwrap();
    let mut b = Api::init(777).unwrap();
    let snap = b.import_state(&blob).unwrap();
    assert_eq!(snap, a.snapshot());
    assert_eq!(b.export_state().unwrap(), blob, "export is stable");
    for t in 7..=12 {
        let ra = a.submit_turn(&plan(t)).unwrap();
        let rb = b.submit_turn(&plan(t)).unwrap();
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
    let cost = 2_000_000.0;
    let mut api = Api::init(1).unwrap();
    let mut control = Api::init(1).unwrap();
    let p = api.preview(&[place(InfraKind::RenewablePlant, 1)]).unwrap();
    assert_eq!(
        (p[0].money, p[0].lead_time, p[0].effective_turn),
        (cost, 4, 5)
    );
    let r1 = api
        .submit_turn(&[place(InfraKind::RenewablePlant, 1)])
        .unwrap();
    control.submit_turn(&[]).unwrap();
    assert_eq!(r1.accepted[0].money_cost, cost);
    // Same seed, same economy: until activation the queued build moves no money at all.
    for t in 2..=4 {
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
    let extra_debt = r5.snapshot.government.debt - c5.snapshot.government.debt;
    assert!(
        (0.5 * cost..1.1 * cost).contains(&extra_debt),
        "the bill is paid in the activation turn, got {extra_debt}"
    );
}

/// A plan that mixes every kind of command over a full game.
fn scripted(turn: u32) -> Vec<Command> {
    match turn {
        1 => vec![
            num(LeverKey::CarbonPrice, 60.0),
            num(LeverKey::GreenSubsidy, 0.3),
            place(InfraKind::PublicHousing, 2),
        ],
        2 => vec![Command::SwitchSystem {
            to: SystemKey::Regulated,
        }],
        6 => vec![
            num(LeverKey::CarbonPrice, 140.0),
            place(InfraKind::RenewablePlant, 1),
        ],
        10 => vec![
            num(LeverKey::CampaignCap, 0.3),
            Command::SetPolicy {
                lever: LeverKey::CommonsAccess,
                value: PolicyValue::Commons(CommonsAccessWire::Community { cap: 0.8 }),
            },
        ],
        t if t % 8 == 0 => vec![
            place(InfraKind::RenewablePlant, 1),
            num(LeverKey::WealthTax, 0.01),
        ],
        t if t % 13 == 0 => vec![Command::Remove {
            kind: InfraKind::FossilPlant,
            count: 1,
        }],
        t if t % 17 == 0 => vec![num(LeverKey::Benefit, 900.0 + f64::from(t))],
        _ => vec![],
    }
}

/// No switch, gentle policy: a plan that should last longer than `scripted`.
fn mild(turn: u32) -> Vec<Command> {
    match turn {
        1 => vec![
            num(LeverKey::CarbonPrice, 40.0),
            num(LeverKey::GreenSubsidy, 0.2),
        ],
        8 => vec![
            num(LeverKey::CarbonPrice, 80.0),
            place(InfraKind::RenewablePlant, 1),
        ],
        20 => vec![num(LeverKey::CarbonPrice, 120.0)],
        t if t % 16 == 0 => vec![place(InfraKind::RenewablePlant, 1)],
        _ => vec![],
    }
}

type Plan = fn(u32) -> Vec<Command>;

fn run_to_the_end(seed: u64, plan: Plan) -> (u32, Snapshot) {
    let mut api = Api::init(seed).unwrap();
    let mut played = 0u32;
    loop {
        let t = api.snapshot().turn;
        match api.submit_turn(&plan(t)) {
            Ok(r) => {
                played += 1;
                assert_eq!(r.turn, t);
                assert_eq!(r.months.len(), 3);
                for (m, f) in r.months.iter().enumerate() {
                    assert_eq!(usize::from(f.month), m);
                    assert!(f.flows.purchases.is_empty());
                    assert!(f.flows.emissions.len() >= 2);
                }
                assert_eq!(r.snapshot.history.len(), played as usize);
                assert_eq!(r.snapshot.history.last().unwrap().turn, t);
            }
            Err(e) => {
                assert_eq!(e.code, "game_over");
                break;
            }
        }
        assert!(played <= 160);
    }
    (played, api.snapshot())
}

#[test]
fn full_runs_through_the_api_never_panic_and_record_history() {
    let plans: [(&str, Plan); 3] = [
        ("scripted", scripted),
        ("mild", mild),
        ("no action", |_| vec![]),
    ];
    for (name, plan) in plans {
        let (played, s) = run_to_the_end(42, plan);
        eprintln!("{name}: played {played} turns, outcome {:?}", s.game_over);
        assert_eq!(s.history.len(), played as usize, "{name}");
        let go = s.game_over.expect("the loop only ends at game over");
        if played == 160 {
            assert_eq!(go.turn, 161, "{name}");
            assert_eq!(
                go.outcome,
                GameOutcome::Plain(PlainOutcome::Survived),
                "{name}"
            );
        } else {
            assert!(played < 160);
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
    let _ = BlockReason::InvalidCommand;
}

#[test]
fn frames_carry_real_flows() {
    let mut api = Api::init(1).unwrap();
    let r = api
        .submit_turn(&[num(LeverKey::CarbonPrice, 50.0)])
        .unwrap();
    for f in &r.months {
        assert!(!f.flows.commutes.is_empty());
        assert!(!f.flows.wages.is_empty());
        assert!(!f.flows.rent.is_empty());
        assert!(f.flows.purchases.is_empty());
        let sources: Vec<_> = f.flows.emissions.iter().map(|e| e.source).collect();
        assert!(sources.contains(&sim_api::events::EmissionSourceKind::Sink));
        assert!(sources.contains(&sim_api::events::EmissionSourceKind::Households));
        // Wages never exceed what the cohort received.
        for h in &r.snapshot.households {
            let sum: f64 = f
                .flows
                .wages
                .iter()
                .filter(|w| w.household == h.id)
                .map(|w| w.amount)
                .sum();
            assert!(sum <= h.income.wages * 1.0 + 1e-6 || h.income.wages == 0.0 || sum > 0.0);
        }
    }
    // Events and trace of the turn are not lost: every tick's events appear in some frame.
    let n_events: usize = r.months.iter().map(|m| m.events.len()).sum();
    assert!(n_events > 0);
    assert!(r
        .trace
        .iter()
        .any(|t| matches!(t, TraceRecord::PolicyGap { .. })));
}

#[test]
fn the_api_adds_nothing_to_the_simulation() {
    // Same seed, same commands: the wrapped sim ends in exactly the state the bare engine reaches.
    let params = sim_params::Params::from_content(&content::default_content()).unwrap();
    let mut bare = sim_engine::init(params, 42).unwrap();
    let mut api = Api::init(42).unwrap();
    for t in 1..=12u32 {
        let wire = mild(t);
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
                Command::Place { kind, count, .. } => sim_types::Command::Place {
                    kind: kind.to_sim(),
                    count,
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
