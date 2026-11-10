//! The tick loop: applies commands, runs the systems in a fixed order,
//! applies their deltas, and runs `sim-checks` after every system.
#![forbid(unsafe_code)]

pub mod apply;
pub mod commands;
pub mod init;
pub mod plan;
pub mod series;
pub mod turn;

use apply::ApplyStats;
use serde::Serialize;
use sim_checks::{
    check_money, check_sector_balance, check_state, MoneyLedger, SectorBalances, Violation,
};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub use plan::Plan;
pub use turn::{cancel_pending, preview};

/// A running simulation: state, parameters and the random number generator.
#[derive(Clone, Debug)]
pub struct Sim {
    pub state: State,
    pub params: Params,
    pub rng: Rng,
    /// Constant base prices per firm slot, for real GDP.
    pub base_prices: Vec<f64>,
    pub initial_population: f64,
    /// Money that payers could not cover in this run so far (diagnostic).
    pub clamped_total: Cents,
    /// Progress inside the current turn (not part of `State` or saves).
    pub turn_phase: TurnPhase,
}

impl Sim {
    /// Start the run clock after settling: tick 0, no emissions or outcome recorded yet.
    fn reset_clock(&mut self) {
        let s = &mut self.state;
        s.tick = 0;
        s.outcome = None;
        s.nature.cumulative = Tonnes::ZERO;
        s.nature.atmosphere = Tonnes(self.params.nature.atmosphere_start);
        s.gov.legitimacy = Rate(self.params.initial.legitimacy);
        s.bank.failed = false;
        for f in &mut s.firms {
            f.closed_tick = None;
        }
        self.clamped_total = Cents::ZERO;
        s.indicators = sim_metrics::compute(s, &self.params, &self.base_prices);
    }
}

#[derive(Clone, Debug, Default)]
pub struct StepOutput {
    pub events: Vec<Event>,
    pub trace: Vec<TraceRecord>,
}

/// JSON-ready view of the simulation for the front end.
#[derive(Serialize)]
pub struct Snapshot<'a> {
    pub schema_version: u32,
    pub turn: u32,
    pub tick: u32,
    pub political_capital: f64,
    /// Scheduled policies and builds, with their effective turns.
    pub pending: &'a [PendingCommand],
    pub state: &'a State,
}

pub fn snapshot(sim: &Sim) -> Snapshot<'_> {
    Snapshot {
        schema_version: SCHEMA_VERSION,
        turn: sim.state.turn,
        tick: sim.state.tick,
        political_capital: sim.state.political_capital,
        pending: &sim.state.pending,
        state: &sim.state,
    }
}

/// Create a simulation from validated parameters and a seed.
pub fn init(params: Params, seed: u64) -> Result<Sim, String> {
    let state = init::initial_state(&params)?;
    let base_prices = params.initial.firms.iter().map(|f| f.price_usd).collect();
    let initial_population = state.total_people().0;
    let mut sim = Sim {
        state,
        params,
        rng: Rng::new(seed, 54),
        base_prices,
        initial_population,
        clamped_total: Cents::ZERO,
        turn_phase: TurnPhase::default(),
    };
    sim.state.indicators = sim_metrics::compute(&sim.state, &sim.params, &sim.base_prices);
    check_state(&sim.state, 0, SystemId::Commands, sim.initial_population)
        .map_err(|v| v.to_string())?;
    // Settle: run the economy for a while with no player input, then start the clock.
    for _ in 0..sim.params.run.warmup_ticks {
        step(&mut sim).map_err(|v| v.to_string())?;
    }
    sim.reset_clock();
    Ok(sim)
}

fn run_system(
    sim: &mut Sim,
    id: SystemId,
    f: sim_model::SystemFn,
    out: &mut StepOutput,
) -> Result<(), Violation> {
    let before = sim.state.total_deposits();
    let balances_before = SectorBalances::of(&sim.state);
    let deltas = f(&sim.state, &sim.params, &mut sim.rng);
    let mut stats = ApplyStats::default();
    let ledger = apply::apply(
        &mut sim.state,
        &sim.params,
        deltas,
        &mut out.events,
        &mut out.trace,
        &mut stats,
    );
    sim.clamped_total += stats.clamped;
    let tick = sim.state.tick;
    if sim.params.run.strict_flows && stats.clamped.dollars() > sim.params.run.flow_tolerance_usd {
        let (what, short) = stats.worst.unwrap_or(("payment", stats.clamped));
        return Err(Violation {
            tick,
            system: id,
            what: format!(
                "unfunded payments: ${:.2} could not be paid (largest: {what}, ${:.2} short)",
                stats.clamped.dollars(),
                short.dollars()
            ),
        });
    }
    check_sector_balance(&balances_before, &SectorBalances::of(&sim.state), tick, id)?;
    verify(sim, id, before, ledger)
}

fn verify(sim: &Sim, id: SystemId, before: Cents, ledger: MoneyLedger) -> Result<(), Violation> {
    let tick = sim.state.tick;
    check_state(&sim.state, tick, id, sim.initial_population)?;
    check_money(before, sim.state.total_deposits(), ledger, tick, id)
}

/// Advance one monthly tick. Commands are handled by the turn layer, never here.
pub fn step(sim: &mut Sim) -> Result<StepOutput, Violation> {
    let mut out = StepOutput::default();
    for (id, f) in sim_model::TICK_ORDER {
        run_system(sim, id, f, &mut out)?;
    }

    // Metrics and win/lose checks.
    sim.state.indicators = sim_metrics::compute(&sim.state, &sim.params, &sim.base_prices);
    check_outcome(sim, &mut out.events);
    verify(
        sim,
        SystemId::Metrics,
        sim.state.total_deposits(),
        MoneyLedger::default(),
    )?;

    // Household income by type into the trace.
    for h in &sim.state.households {
        let rows = [
            (IncomeKind::Wages, h.income.wages),
            (IncomeKind::Dividends, h.income.dividends),
            (IncomeKind::Rent, h.income.rent_received),
            (IncomeKind::Interest, h.income.interest_received),
            (IncomeKind::Benefits, h.income.benefits),
        ];
        for (kind, amount) in rows {
            if amount != Cents::ZERO {
                out.trace.push(TraceRecord::Income {
                    household: h.id,
                    kind,
                    amount,
                });
            }
        }
    }
    sim.state.tick += 1;
    Ok(out)
}

fn check_outcome(sim: &mut Sim, events: &mut Vec<Event>) {
    let s = &mut sim.state;
    if s.outcome.is_some() {
        return;
    }
    let tick = s.tick;
    if s.nature.cumulative.0 >= sim.params.nature.budget_b0 {
        s.outcome = Some((tick, Outcome::BudgetExhausted));
        events.push(Event::BudgetExhausted);
    } else if s.gov.legitimacy.0 <= 0.0 {
        s.outcome = Some((tick, Outcome::Collapse(CollapseReason::LegitimacyZero)));
        events.push(Event::Collapse {
            reason: CollapseReason::LegitimacyZero,
        });
    } else if s.indicators.unemployment_smooth.0 > sim.params.government.collapse_unemployment {
        s.outcome = Some((tick, Outcome::Collapse(CollapseReason::MassUnemployment)));
        events.push(Event::Collapse {
            reason: CollapseReason::MassUnemployment,
        });
    } else if s.bank.failed && s.indicators.unemployment_smooth.0 > 0.2 {
        s.outcome = Some((tick, Outcome::Collapse(CollapseReason::BankFailureCascade)));
        events.push(Event::Collapse {
            reason: CollapseReason::BankFailureCascade,
        });
    }
}

/// Where the sim is inside a turn. Lives in `Sim`, not in the model `State`: it is not part
/// of a save (saves are only made at turn boundaries) and no system reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TurnPhase {
    /// `begin_turn` has run and `finish_turn` has not.
    pub open: bool,
    /// Monthly ticks run since `begin_turn` (0 to `ticks_per_turn`).
    pub months_done: u32,
}

/// What `finish_turn` adds to the turn's output.
#[derive(Clone, Debug)]
pub struct TurnFinish {
    pub policy_gaps: Vec<PolicyGapSummary>,
    pub pc_after: f64,
}

fn order_error(what: &str) -> Violation {
    Violation {
        tick: 0,
        system: SystemId::Commands,
        what: format!("turn calls out of order: {what}"),
    }
}

/// Start a turn: charge and queue the commands, activate what is due. The returned output
/// has the turn number, PC before, accepted, rejected, activated and the events so far.
/// On error the state may be partly changed; the caller restores its backup.
pub fn begin_turn(sim: &mut Sim, cmds: &[Command]) -> Result<TurnOutput, Violation> {
    if sim.turn_phase.open {
        return Err(order_error("begin_turn while a turn is open"));
    }
    let mut out = TurnOutput {
        turn: sim.state.turn,
        pc_before: sim.state.political_capital,
        ..TurnOutput::default()
    };
    let (accepted, rejected) = turn::queue(sim, cmds);
    out.accepted = accepted;
    out.rejected = rejected;
    for r in &out.rejected {
        out.events.push(Event::PolicyBlocked { reason: r.reason });
    }
    let before = sim.state.total_deposits();
    out.activated = turn::activate_due(sim, &mut out.events);
    verify(sim, SystemId::Commands, before, MoneyLedger::default())?;
    sim.turn_phase = TurnPhase {
        open: true,
        months_done: 0,
    };
    Ok(out)
}

/// Run one monthly tick of the open turn.
pub fn advance_month(sim: &mut Sim) -> Result<StepOutput, Violation> {
    let ph = sim.turn_phase;
    if !ph.open || ph.months_done >= sim.params.run.ticks_per_turn {
        return Err(order_error(
            "advance_month needs an open turn with months left",
        ));
    }
    let t = step(sim)?;
    sim.turn_phase.months_done += 1;
    Ok(t)
}

/// Close the turn once all its months are done: policy gaps, PC regeneration, turn += 1.
pub fn finish_turn(sim: &mut Sim) -> Result<TurnFinish, Violation> {
    let ph = sim.turn_phase;
    if !ph.open || ph.months_done != sim.params.run.ticks_per_turn {
        return Err(order_error(
            "finish_turn needs an open turn with all months done",
        ));
    }
    // Nominal vs effective for the lobbied levers, as of the end of the turn.
    let (pol, eff) = (&sim.state.policy, &sim.state.effective);
    let policy_gaps = vec![
        PolicyGapSummary {
            lever: Lever::CarbonPrice,
            nominal: pol.carbon_price.dollars(),
            effective: eff.carbon_price.dollars(),
        },
        PolicyGapSummary {
            lever: Lever::WealthTax,
            nominal: pol.wealth_tax.0,
            effective: eff.wealth_tax.0,
        },
        PolicyGapSummary {
            lever: Lever::ProfitTax,
            nominal: pol.tax_profit.0,
            effective: eff.tax_profit.0,
        },
        PolicyGapSummary {
            lever: Lever::EmissionsCap,
            nominal: pol.emissions_cap.0,
            effective: eff.emissions_cap.0,
        },
    ];
    turn::regenerate(&mut sim.state, &sim.params.turn);
    sim.state.turn += 1;
    sim.turn_phase = TurnPhase::default();
    Ok(TurnFinish {
        policy_gaps,
        pc_after: sim.state.political_capital,
    })
}

/// Play one turn: `begin_turn`, one `advance_month` per tick, `finish_turn`.
/// `on_tick` sees every tick's output.
pub fn submit_turn_with(
    sim: &mut Sim,
    cmds: &[Command],
    mut on_tick: impl FnMut(&Sim, &StepOutput),
) -> Result<TurnOutput, Violation> {
    let mut out = begin_turn(sim, cmds)?;
    for _ in 0..sim.params.run.ticks_per_turn {
        let t = advance_month(sim)?;
        on_tick(sim, &t);
        out.events.extend(t.events);
        out.trace.extend(t.trace);
    }
    let fin = finish_turn(sim)?;
    out.policy_gaps = fin.policy_gaps;
    out.pc_after = fin.pc_after;
    Ok(out)
}

/// Play one turn with the given commands (see `submit_turn_with`).
pub fn submit_turn(sim: &mut Sim, cmds: &[Command]) -> Result<TurnOutput, Violation> {
    submit_turn_with(sim, cmds, |_, _| {})
}

/// Play `turns` turns from a plan, calling `on_tick` after each monthly tick.
pub fn run(
    sim: &mut Sim,
    turns: u32,
    plan: &Plan,
    mut on_tick: impl FnMut(&Sim, &StepOutput),
) -> Result<(), Violation> {
    for _ in 0..turns {
        let cmds = plan.commands_at(sim.state.turn);
        submit_turn_with(sim, &cmds, &mut on_tick)?;
        if sim.params.run.stop_on_loss && sim.state.outcome.is_some() {
            break;
        }
    }
    Ok(())
}

/// Final result of a run: the recorded loss, or `Survived`.
pub fn final_outcome(sim: &Sim) -> Outcome {
    sim.state
        .outcome
        .map(|(_, o)| o)
        .unwrap_or(Outcome::Survived)
}
