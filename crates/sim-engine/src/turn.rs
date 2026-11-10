//! S0: the turn layer. Validates commands against political capital and funds, queues them
//! with lead times, activates them when due, and refunds cancellations.
//! `preview` and `submit_turn` share one evaluation routine, so a preview always matches
//! what a submit would charge.

use crate::commands::apply_commands;
use sim_params::{Params, TurnParams};
use sim_types::*;

/// Active value of the lever a policy change targets, in friendly units.
fn active_value(p: &Policy, c: &PolicyChange) -> Option<f64> {
    Some(match c {
        PolicyChange::CarbonPrice(_) => p.carbon_price.dollars(),
        PolicyChange::TaxWage(_) => p.tax_wage.0,
        PolicyChange::TaxProfit(_) => p.tax_profit.0,
        PolicyChange::TaxCapital(_) => p.tax_capital.0,
        PolicyChange::WealthTax(_) => p.wealth_tax.0,
        PolicyChange::WealthThreshold(_) => p.wealth_threshold.dollars(),
        PolicyChange::BaseRate(_) => p.base_rate.0,
        PolicyChange::MinWage(_) => p.min_wage.dollars(),
        PolicyChange::Benefit(_) => p.benefit.dollars(),
        PolicyChange::Ubi(_) => p.ubi.dollars(),
        PolicyChange::GreenSubsidy(_) => p.green_subsidy.0,
        PolicyChange::CampaignCap(_) => p.campaign_cap.0,
        PolicyChange::EmissionsCap(_) => p.emissions_cap.0,
        PolicyChange::CommonsHarvest(_) => p.commons_harvest.0,
        PolicyChange::BankCapitalRatio(_) => p.bank_capital_ratio.0,
        PolicyChange::PublicSpending(_) => p.public_spending.dollars(),
        PolicyChange::JobGuarantee(_) => p.job_guarantee.0,
        PolicyChange::ResilienceSpending(_) => p.resilience_spending.dollars(),
        PolicyChange::CommonsAccess(_) | PolicyChange::Bailout(_) => return None,
    })
}

fn build_cost(p: &Params, c: &Command) -> Cents {
    match *c {
        Command::Place { kind, .. } => Cents::from_dollars(match kind {
            InfraKind::PublicHousing => p.government.public_housing_cost_usd,
            InfraKind::FossilPlant | InfraKind::RenewablePlant => p.government.plant_cost_usd,
            InfraKind::Road => p.government.road_cost_usd,
            InfraKind::FireStation => p.weather.fire_station_cost_usd,
        }),
        _ => Cents::ZERO,
    }
}

/// Government funds a new build can count on: cash plus the bank's lending headroom,
/// minus builds already queued.
fn available_funds(s: &State, pending: &[PendingCommand]) -> Cents {
    let kappa = s.policy.bank_capital_ratio.0.max(1e-9);
    let max_loans = s.bank.equity.max(Cents::ZERO).scale(1.0 / kappa);
    let headroom = (max_loans - s.bank.loans).max(Cents::ZERO);
    let committed = Cents::sum(pending.iter().map(|c| c.money_cost));
    s.gov.cash + headroom - committed
}

/// Is the command allowed at all right now (before costs)?
pub fn validate(s: &State, pending: &[PendingCommand], c: &Command) -> Option<BlockReason> {
    let switch_pending = pending
        .iter()
        .any(|x| matches!(x.command, Command::SwitchSystem(_)));
    match *c {
        Command::SetPolicy(ch) => match ch {
            PolicyChange::CampaignCap(_)
            | PolicyChange::CommonsAccess(CommonsAccess::Community { .. })
                if s.system == EconomicSystem::Market =>
            {
                Some(BlockReason::LeverUnavailable)
            }
            // Enclosure happens through lobbying, never by player command.
            PolicyChange::CommonsAccess(CommonsAccess::Enclosed(_)) => {
                Some(BlockReason::LeverUnavailable)
            }
            _ => None,
        },
        Command::SwitchSystem(to) => {
            if s.transition.is_some() || switch_pending {
                Some(BlockReason::TransitionInProgress)
            } else if to == s.system {
                Some(BlockReason::SameSystem)
            } else {
                None
            }
        }
        Command::Place { .. } | Command::Remove { .. } => tile_problem(s, pending, c),
    }
}

/// Why a tile command cannot be carried out, if it cannot: the tile is off the map or not
/// buildable (`TileInvalid`), already holds something or has something queued on it
/// (`TileOccupied`), or has nothing to remove (`NothingToRemove`). `None` for other commands.
pub fn tile_problem(s: &State, pending: &[PendingCommand], c: &Command) -> Option<BlockReason> {
    match *c {
        Command::Place { tile, .. } => match s.map.kind_at(tile) {
            None | Some(TileKind::Forest | TileKind::Commons) => Some(BlockReason::TileInvalid),
            Some(TileKind::Empty) => pending
                .iter()
                .any(|x| matches!(x.command, Command::Place { tile: t, .. } if t == tile))
                .then_some(BlockReason::TileOccupied),
            Some(_) => Some(BlockReason::TileOccupied),
        },
        Command::Remove { tile } => match s.map.kind_at(tile) {
            None => Some(BlockReason::TileInvalid),
            Some(TileKind::Empty) => Some(BlockReason::NothingToRemove),
            Some(
                TileKind::PublicHousing
                | TileKind::FossilPlant
                | TileKind::RenewablePlant
                | TileKind::Road
                | TileKind::FireStation,
            ) => None,
            Some(_) => Some(BlockReason::TileInvalid),
        },
        _ => None,
    }
}

/// Tiles where `kind` could be placed now: empty and with nothing queued on them.
pub fn valid_tiles(sim: &crate::Sim, kind: InfraKind) -> Vec<Tile> {
    let s = &sim.state;
    s.map
        .tiles()
        .filter(|t| tile_problem(s, &s.pending, &Command::Place { kind, tile: *t }).is_none())
        .collect()
}

fn size_of(s: &State, tp: &TurnParams, pending: &[PendingCommand], c: &Command) -> f64 {
    let cfg = tp.lever(c.lever_key(&s.map));
    let raw = match *c {
        Command::SetPolicy(ch) => match (ch.value(), active_value(&s.policy, &ch)) {
            (Some(new), Some(active)) => {
                // Compare with the latest pending value for the same lever, if any.
                let reference = pending
                    .iter()
                    .rev()
                    .find_map(|x| match x.command {
                        Command::SetPolicy(o) if o.key() == ch.key() => o.value(),
                        _ => None,
                    })
                    .unwrap_or(active);
                (new - reference).abs() / cfg.unit
            }
            _ => 1.0,
        },
        Command::Place { .. } | Command::Remove { .. } | Command::SwitchSystem(_) => 1.0,
    };
    raw.max(cfg.min_size)
}

fn opposition(s: &State, c: &Command) -> f64 {
    match c {
        Command::SetPolicy(ch) if ch.opposed_by_capital(&s.policy) => s.net_lobby.0,
        _ => 0.0,
    }
}

/// Cost of one command given the commands already queued.
fn cost_one(s: &State, p: &Params, pending: &[PendingCommand], c: &Command) -> CommandCost {
    let cfg = p.turn.lever(c.lever_key(&s.map));
    let opp = opposition(s, c);
    CommandCost {
        command: *c,
        pc: cfg.base_cost * size_of(s, &p.turn, pending, c) * (1.0 + opp),
        money: build_cost(p, c),
        lead_time: cfg.lead_time,
        effective_turn: s.turn + cfg.lead_time,
        opposition: opp,
        blocked: None,
        tile_valid: true,
        tile_reason: None,
    }
}

struct Evaluation {
    costs: Vec<CommandCost>,
    accepted: Vec<PendingCommand>,
    rejected: Vec<Rejected>,
    pending: Vec<PendingCommand>,
    pc_left: f64,
}

fn evaluate(s: &State, p: &Params, cmds: &[Command]) -> Evaluation {
    let mut ev = Evaluation {
        costs: Vec::new(),
        accepted: Vec::new(),
        rejected: Vec::new(),
        pending: s.pending.clone(),
        pc_left: s.political_capital,
    };
    let mut next_id = s.next_pending_id;
    for (index, c) in cmds.iter().enumerate() {
        let mut cost = cost_one(s, p, &ev.pending, c);
        cost.blocked = validate(s, &ev.pending, c);
        let tile_reason = tile_problem(s, &ev.pending, c);
        cost.tile_valid = tile_reason.is_none();
        cost.tile_reason = tile_reason;
        if cost.blocked.is_none() && cost.pc > ev.pc_left + 1e-9 {
            cost.blocked = Some(BlockReason::InsufficientPoliticalCapital);
        }
        if cost.blocked.is_none() && cost.money > available_funds(s, &ev.pending) {
            cost.blocked = Some(BlockReason::InsufficientFunds);
        }
        match cost.blocked {
            Some(reason) => ev.rejected.push(Rejected {
                index,
                command: *c,
                reason,
            }),
            None => {
                ev.pc_left -= cost.pc;
                let pc = PendingCommand {
                    id: next_id,
                    command: *c,
                    submitted_turn: s.turn,
                    effective_turn: cost.effective_turn,
                    pc_cost: cost.pc,
                    money_cost: cost.money,
                };
                next_id += 1;
                ev.pending.push(pc);
                ev.accepted.push(pc);
            }
        }
        ev.costs.push(cost);
    }
    ev
}

/// Costs of a list of commands, without changing any state. Commands are costed in order,
/// as if the earlier ones had been queued.
pub fn preview(sim: &crate::Sim, cmds: &[Command]) -> Vec<CommandCost> {
    evaluate(&sim.state, &sim.params, cmds).costs
}

/// Queue the affordable commands (the rest are reported as rejected) and charge political capital.
pub fn queue(sim: &mut crate::Sim, cmds: &[Command]) -> (Vec<PendingCommand>, Vec<Rejected>) {
    let ev = evaluate(&sim.state, &sim.params, cmds);
    sim.state.pending = ev.pending;
    sim.state.next_pending_id += ev.accepted.len() as u32;
    sim.state.political_capital = ev.pc_left;
    (ev.accepted, ev.rejected)
}

/// Activate every pending command whose lead time has ended; the model then reads the new
/// active values. Returns the activated items.
pub fn activate_due(sim: &mut crate::Sim, events: &mut Vec<Event>) -> Vec<PendingCommand> {
    let turn = sim.state.turn;
    let (due, keep): (Vec<_>, Vec<_>) = sim
        .state
        .pending
        .iter()
        .copied()
        .partition(|x| x.effective_turn <= turn);
    sim.state.pending = keep;
    for x in &due {
        apply_commands(&mut sim.state, &sim.params, &[x.command], events);
        events.push(Event::CommandActivated { id: x.id });
    }
    due
}

/// Cancel a pending command. The player gets back `cancel_refund` of its political capital.
pub fn cancel_pending(sim: &mut crate::Sim, id: u32) -> Result<f64, String> {
    let i = sim
        .state
        .pending
        .iter()
        .position(|x| x.id == id)
        .ok_or_else(|| format!("no pending command {id}"))?;
    let c = sim.state.pending.remove(i);
    let refund = c.pc_cost * sim.params.turn.cancel_refund;
    sim.state.political_capital =
        (sim.state.political_capital + refund).min(sim.params.turn.pc_cap);
    Ok(refund)
}

/// End-of-turn political capital regeneration.
pub fn regenerate(s: &mut State, tp: &TurnParams) {
    s.political_capital = (s.political_capital + tp.pc_base * s.gov.legitimacy.0).min(tp.pc_cap);
}
