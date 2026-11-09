//! Turn-level wire types: pending items, previews, month frames and the turn result.
use crate::command::Command;
use crate::events::{EmissionLine, SimEvent, TraceRecord};
use crate::keys::{BlockReason, LeverKey, SectorKey};
use crate::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use sim_types as st;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingItem {
    pub id: u32,
    pub command: Command,
    pub submitted_turn: u32,
    pub effective_turn: u32,
    pub pc_cost: f64,
    /// Dollars charged when the command activates (see docs/sim-api-notes.md).
    pub money_cost: f64,
}

impl PendingItem {
    pub fn from_sim(p: &st::PendingCommand) -> PendingItem {
        PendingItem {
            id: p.id,
            command: Command::from_sim(&p.command),
            submitted_turn: p.submitted_turn,
            effective_turn: p.effective_turn,
            pc_cost: p.pc_cost,
            money_cost: p.money_cost.dollars(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPreview {
    pub command: Command,
    pub pc: f64,
    pub money: f64,
    pub lead_time: u32,
    pub effective_turn: u32,
    pub opposition: f64,
    pub blocked: Option<BlockReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RejectedCommand {
    pub index: usize,
    pub command: Command,
    pub reason: BlockReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyGap {
    pub lever: LeverKey,
    pub nominal: f64,
    pub effective: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commute {
    pub household: u32,
    pub firm: u32,
    pub people: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Purchase {
    pub household: u32,
    pub sector: SectorKey,
    pub amount: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RentFlow {
    pub household: u32,
    pub amount: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WageFlow {
    pub household: u32,
    pub firm: u32,
    pub amount: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MonthFlows {
    pub commutes: Vec<Commute>,
    pub purchases: Vec<Purchase>,
    pub rent: Vec<RentFlow>,
    pub wages: Vec<WageFlow>,
    pub emissions: Vec<EmissionLine>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonthFrame {
    pub month: u8,
    pub events: Vec<SimEvent>,
    pub flows: MonthFlows,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnResult {
    pub turn: u32,
    pub accepted: Vec<PendingItem>,
    pub rejected: Vec<RejectedCommand>,
    pub activated: Vec<PendingItem>,
    pub pc_before: f64,
    pub pc_after: f64,
    pub months: Vec<MonthFrame>,
    pub policy_gaps: Vec<PolicyGap>,
    pub trace: Vec<TraceRecord>,
    pub snapshot: Snapshot,
}

/// Result of `cancel_pending`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResult {
    pub refund_pc: f64,
    pub snapshot: Snapshot,
}
