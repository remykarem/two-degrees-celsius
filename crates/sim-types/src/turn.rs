//! Data returned by the turn API: costs, rejections and the turn summary.
use crate::enums::*;
use crate::state::*;
use crate::units::*;
use serde::{Deserialize, Serialize};

/// What a command would cost, as shown to the player before confirming.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandCost {
    pub command: Command,
    pub pc: f64,
    pub money: Cents,
    pub lead_time: u32,
    pub effective_turn: u32,
    /// Net lobbying opposition included in the PC cost.
    pub opposition: f64,
    /// `None` if the command can be queued, else why not.
    pub blocked: Option<BlockReason>,
    /// For place and remove commands: can the tile take the command? True for other commands.
    pub tile_valid: bool,
    /// Why not (`TileOccupied`, `TileInvalid` or `NothingToRemove`), when `tile_valid` is false.
    pub tile_reason: Option<BlockReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rejected {
    /// Index in the submitted command list.
    pub index: usize,
    pub command: Command,
    pub reason: BlockReason,
}

/// Nominal vs effective value of a lobbied lever at the end of the turn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyGapSummary {
    pub lever: Lever,
    pub nominal: f64,
    pub effective: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TurnOutput {
    /// The turn that was played.
    pub turn: u32,
    pub accepted: Vec<PendingCommand>,
    pub rejected: Vec<Rejected>,
    /// Commands that became active at the start of this turn.
    pub activated: Vec<PendingCommand>,
    pub pc_before: f64,
    pub pc_after: f64,
    pub events: Vec<Event>,
    pub trace: Vec<TraceRecord>,
    pub policy_gaps: Vec<PolicyGapSummary>,
}
