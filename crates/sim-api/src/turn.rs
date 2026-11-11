//! Turn-level wire types: pending items, previews, month frames and the turn result.
use crate::command::{Command, Tile};
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
    /// Contract v1.8: for a lobbied lever (carbon price, profit tax, wealth tax, emissions cap),
    /// the value the model would have in force at today's net lobbying (`sim_model::lobbying::
    /// expected_effective`); absent for other commands. Lobbying can still move before it applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_effective: Option<f64>,
    pub blocked: Option<BlockReason>,
    /// For place and remove: can the tile take the command? `true` for other commands.
    pub tile_valid: bool,
    /// `tile_occupied`, `tile_invalid` or `nothing_to_remove` when `tileValid` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_reason: Option<BlockReason>,
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

/// One person's commute this month. The sim exposes distance and time, not the path, so the
/// tiles are the two ends (`from` is `null` for an unhoused household).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commute {
    pub person: u32,
    pub household: u32,
    pub firm: u32,
    pub from: Option<Tile>,
    pub to: Tile,
    pub tiles: u32,
    pub minutes: u32,
}

/// Contract v1.9: a person on a household's payroll (the estate's pilot) commuting to the
/// employer's workplace (the hangar tile). `wage` is what the employer paid this tick.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaffCommute {
    pub person: u32,
    pub household: u32,
    /// The employer household.
    pub employer: u32,
    pub from: Option<Tile>,
    pub to: Tile,
    pub tiles: u32,
    pub minutes: u32,
    pub wage: f64,
}

/// A household buying from a firm this month.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Purchase {
    pub household: u32,
    pub firm: u32,
    pub sector: SectorKey,
    pub amount: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RentFlow {
    pub household: u32,
    /// The household's home, if it has one.
    pub home: Option<u32>,
    pub amount: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WageFlow {
    pub person: u32,
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
    /// Contract v1.9; omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub staff_commutes: Vec<StaffCommute>,
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

/// Result of `beginTurn` (contract v1.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStart {
    pub turn: u32,
    pub accepted: Vec<PendingItem>,
    pub rejected: Vec<RejectedCommand>,
    pub activated: Vec<PendingItem>,
    pub pc_before: f64,
    pub snapshot: Snapshot,
}

/// Result of `advanceMonth` (contract v1.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthResult {
    pub month: u8,
    pub frame: MonthFrame,
    pub snapshot: Snapshot,
}

/// Result of `cancel_pending`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResult {
    pub refund_pc: f64,
    pub snapshot: Snapshot,
}
