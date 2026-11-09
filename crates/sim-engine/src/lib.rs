//! The tick loop: applies commands, runs the systems in a fixed order,
//! applies their deltas, and runs `sim-checks` after every system.
#![forbid(unsafe_code)]

pub mod apply;
pub mod commands;
pub mod init;
pub mod script;
pub mod series;

use apply::ApplyStats;
use serde::Serialize;
use sim_checks::{check_money, check_state, MoneyLedger, Violation};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub use script::Script;

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
    pub tick: u32,
    pub state: &'a State,
}

pub fn snapshot(sim: &Sim) -> Snapshot<'_> {
    Snapshot {
        schema_version: SCHEMA_VERSION,
        tick: sim.state.tick,
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
    };
    sim.state.indicators = sim_metrics::compute(&sim.state, &sim.params, &sim.base_prices);
    check_state(&sim.state, 0, SystemId::Commands, sim.initial_population)
        .map_err(|v| v.to_string())?;
    // Settle: run the economy for a while with no player input, then start the clock.
    for _ in 0..sim.params.run.warmup_ticks {
        step(&mut sim, &[]).map_err(|v| v.to_string())?;
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
    verify(sim, id, before, ledger)
}

fn verify(sim: &Sim, id: SystemId, before: Cents, ledger: MoneyLedger) -> Result<(), Violation> {
    let tick = sim.state.tick;
    check_state(&sim.state, tick, id, sim.initial_population)?;
    check_money(before, sim.state.total_deposits(), ledger, tick, id)
}

/// Advance one monthly tick.
pub fn step(sim: &mut Sim, cmds: &[Command]) -> Result<StepOutput, Violation> {
    let mut out = StepOutput::default();
    // Commands are applied directly by the engine, then checked.
    let before = sim.state.total_deposits();
    commands::apply_commands(&mut sim.state, &sim.params, cmds, &mut out.events);
    verify(sim, SystemId::Commands, before, MoneyLedger::default())?;

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
    } else if s.indicators.unemployment.0 > sim.params.government.collapse_unemployment {
        s.outcome = Some((tick, Outcome::Collapse(CollapseReason::MassUnemployment)));
        events.push(Event::Collapse {
            reason: CollapseReason::MassUnemployment,
        });
    } else if s.bank.failed && s.indicators.unemployment.0 > 0.2 {
        s.outcome = Some((tick, Outcome::Collapse(CollapseReason::BankFailureCascade)));
        events.push(Event::Collapse {
            reason: CollapseReason::BankFailureCascade,
        });
    }
}

/// Run `ticks` ticks, feeding scripted commands, calling `on_tick` after each.
pub fn run(
    sim: &mut Sim,
    ticks: u32,
    script: &Script,
    mut on_tick: impl FnMut(&Sim, &StepOutput),
) -> Result<(), Violation> {
    for _ in 0..ticks {
        let cmds = script.commands_at(sim.state.tick);
        let out = step(sim, &cmds)?;
        on_tick(sim, &out);
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
