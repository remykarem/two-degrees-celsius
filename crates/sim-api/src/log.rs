//! The wrapper-level log (Town v5.2): what the wire layer records about the game beyond the
//! model `State`. Saved with the game (`schemaVersion` 5), never read by the model.
//!
//! - one `TickRecord` per monthly tick: emissions by source as the model reported them in that
//!   tick (the trace), the state's own gross and net figures, and the legitimacy update;
//! - the policy history: initial values, queued, activated, cancelled, rejected and weakened
//!   events.
use crate::command::{Command, PolicyValue};
use crate::keys::LeverKey;
use crate::snapshot::{PolicyEntry, PolicyScalar};
use crate::turn::{PendingItem, PolicyGap, RejectedCommand};
use serde::{Deserialize, Serialize};
use sim_engine::{Sim, StepOutput};
use sim_types::{EmissionSource, LegitStep, TraceRecord};
use std::collections::BTreeMap;

/// One monthly tick, as the trace reported it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TickRecord {
    pub turn: u32,
    /// 0..=2 within the turn.
    pub month: u8,
    /// `(firm id, direct tonnes)` of every active firm.
    pub firms: Vec<(u32, f64)>,
    /// `(plant id, tonnes)` of every plant.
    pub plants: Vec<(u32, f64)>,
    pub households: f64,
    /// Contract v1.9: private jet flight tonnes this tick (zero without a flight).
    #[serde(default)]
    pub aviation: f64,
    /// Forest sink, negative (carbon absorbed).
    pub sink: f64,
    /// Informational: sink capacity lost to fire. Not a term of net emissions.
    pub fire_sink_loss: f64,
    /// The state's own `nature.gross_emissions` and `nature.net_emissions` after the tick.
    pub gross_state: f64,
    pub net_state: f64,
    pub legitimacy: Option<LegitStep>,
}

impl TickRecord {
    pub fn new(sim: &Sim, out: &StepOutput) -> TickRecord {
        let mut r = TickRecord {
            turn: sim.state.turn,
            month: sim.turn_phase.months_done.saturating_sub(1) as u8,
            firms: Vec::new(),
            plants: Vec::new(),
            households: 0.0,
            aviation: 0.0,
            sink: 0.0,
            fire_sink_loss: 0.0,
            gross_state: sim.state.nature.gross_emissions.0,
            net_state: sim.state.nature.net_emissions.0,
            legitimacy: None,
        };
        for t in &out.trace {
            match *t {
                TraceRecord::Emissions { source, tonnes } => match source {
                    EmissionSource::Firm(f) => r.firms.push((f.0, tonnes.0)),
                    EmissionSource::Plant(p) => r.plants.push((p.0, tonnes.0)),
                    EmissionSource::Households => r.households += tonnes.0,
                    EmissionSource::Aviation => r.aviation += tonnes.0,
                    EmissionSource::Sink => r.sink += tonnes.0,
                    EmissionSource::FireSinkLoss => r.fire_sink_loss += tonnes.0,
                },
                TraceRecord::Legitimacy(l) => r.legitimacy = Some(l),
                TraceRecord::Income { .. } | TraceRecord::PolicyGap { .. } => {}
            }
        }
        r
    }

    pub fn firms_total(&self) -> f64 {
        self.firms.iter().map(|x| x.1).sum()
    }
    pub fn plants_total(&self) -> f64 {
        self.plants.iter().map(|x| x.1).sum()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEventKind {
    Queued,
    Activated,
    Cancelled,
    Rejected,
    Weakened,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEvent {
    pub turn: u32,
    pub lever: LeverKey,
    pub kind: PolicyEventKind,
    pub from: PolicyScalar,
    pub to: PolicyScalar,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyLog {
    /// Lever values at turn 0.
    pub initial: BTreeMap<LeverKey, PolicyScalar>,
    pub events: Vec<PolicyEvent>,
    /// The weakened share last reported per lobbied lever (an event is logged when it moves).
    pub weakened: BTreeMap<LeverKey, f64>,
}

/// Share of a lobbied policy lost to lobbying: 0 when none. For a cap (a limit that lobbying
/// raises) it is `1 - nominal / effective`, for the others `1 - effective / nominal`.
pub fn weakened_share(lever: LeverKey, nominal: f64, effective: f64) -> f64 {
    let (num, den) = if lever == LeverKey::EmissionsCap {
        (nominal, effective)
    } else {
        (effective, nominal)
    };
    if den > 0.0 {
        (1.0 - num / den).max(0.0)
    } else {
        0.0
    }
}

pub fn scalar_of(v: &PolicyValue) -> PolicyScalar {
    match *v {
        PolicyValue::Bool(b) => PolicyScalar::Bool(b),
        PolicyValue::Number(n) => PolicyScalar::Number(n),
        PolicyValue::Commons(crate::command::CommonsAccessWire::Open) => {
            PolicyScalar::Mode("open".to_string())
        }
        PolicyValue::Commons(crate::command::CommonsAccessWire::Community { .. }) => {
            PolicyScalar::Mode("community".to_string())
        }
    }
}

fn active(map: &BTreeMap<LeverKey, PolicyEntry>, lever: LeverKey) -> PolicyScalar {
    map.get(&lever)
        .map_or(PolicyScalar::Number(0.0), |e| e.nominal.clone())
}

impl PolicyLog {
    pub fn new(initial: &BTreeMap<LeverKey, PolicyEntry>) -> PolicyLog {
        PolicyLog {
            initial: initial
                .iter()
                .map(|(k, v)| (*k, v.nominal.clone()))
                .collect(),
            events: Vec::new(),
            weakened: BTreeMap::new(),
        }
    }

    /// Queued and rejected commands of a turn, and what activated at its start. `before` is the
    /// policy as it was before the turn's commands.
    pub fn log_turn_start(
        &mut self,
        turn: u32,
        before: &BTreeMap<LeverKey, PolicyEntry>,
        accepted: &[PendingItem],
        rejected: &[RejectedCommand],
        activated: &[PendingItem],
    ) {
        for r in rejected {
            if let Command::SetPolicy { lever, value } = r.command {
                self.events.push(PolicyEvent {
                    turn,
                    lever,
                    kind: PolicyEventKind::Rejected,
                    from: active(before, lever),
                    to: scalar_of(&value),
                    reason: serde_json::to_value(r.reason)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string)),
                });
            }
        }
        for a in accepted {
            if let Command::SetPolicy { lever, value } = a.command {
                self.events.push(PolicyEvent {
                    turn,
                    lever,
                    kind: PolicyEventKind::Queued,
                    from: active(before, lever),
                    to: scalar_of(&value),
                    reason: Some(format!("takes effect in turn {}", a.effective_turn)),
                });
            }
        }
        for a in activated {
            if let Command::SetPolicy { lever, value } = a.command {
                self.events.push(PolicyEvent {
                    turn,
                    lever,
                    kind: PolicyEventKind::Activated,
                    from: active(before, lever),
                    to: scalar_of(&value),
                    reason: None,
                });
            }
        }
    }

    pub fn log_cancel(
        &mut self,
        turn: u32,
        item: &PendingItem,
        refund_pc: f64,
        active_now: PolicyScalar,
    ) {
        if let Command::SetPolicy { lever, value } = item.command {
            self.events.push(PolicyEvent {
                turn,
                lever,
                kind: PolicyEventKind::Cancelled,
                from: active_now,
                to: scalar_of(&value),
                reason: Some(format!("{refund_pc:.1} political capital refunded")),
            });
        }
    }

    /// Lobbying gaps at the end of a turn: an event whenever a lever's weakened share moves.
    pub fn log_gaps(&mut self, turn: u32, gaps: &[PolicyGap]) {
        for g in gaps {
            let share = weakened_share(g.lever, g.nominal, g.effective);
            let last = self.weakened.get(&g.lever).copied().unwrap_or(0.0);
            if (share - last).abs() >= 0.005 {
                self.events.push(PolicyEvent {
                    turn,
                    lever: g.lever,
                    kind: PolicyEventKind::Weakened,
                    from: PolicyScalar::Number(g.nominal),
                    to: PolicyScalar::Number(g.effective),
                    reason: Some(format!("lobbying cut it by {:.0}%", share * 100.0)),
                });
                self.weakened.insert(g.lever, share);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WrapperLog {
    pub ticks: Vec<TickRecord>,
    pub policy: PolicyLog,
}

impl WrapperLog {
    pub fn new(initial: &BTreeMap<LeverKey, PolicyEntry>) -> WrapperLog {
        WrapperLog {
            ticks: Vec::new(),
            policy: PolicyLog::new(initial),
        }
    }

    /// Consistency checks for a loaded save.
    pub fn verify(&self, tick: u32, turn: u32) -> Result<(), String> {
        if self.ticks.len() != tick as usize {
            return Err(format!(
                "{} tick records for {} ticks played",
                self.ticks.len(),
                tick
            ));
        }
        if self.ticks.iter().any(|t| t.turn > turn || t.month > 2) {
            return Err("a tick record has a turn or month out of range".to_string());
        }
        if self.policy.events.iter().any(|e| e.turn > turn) {
            return Err("a policy event is from a turn not yet played".to_string());
        }
        if self.policy.events.windows(2).any(|w| w[0].turn > w[1].turn) {
            return Err("policy events are out of order".to_string());
        }
        if self.policy.initial.len() != LeverKey::ALL.len() {
            return Err("the initial policy is missing levers".to_string());
        }
        Ok(())
    }
}
