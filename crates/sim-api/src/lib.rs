//! The wire layer between the simulation and the front end.
//!
//! `Api` wraps a `Sim` and exposes the `SimPort` operations of `docs/snapshot-contract.md`
//! (v0.3) using the contract's JSON shapes: camelCase fields, snake_case tagged unions,
//! money in dollars, rates as fractions. It has no model logic and no I/O; a thin `sim-wasm`
//! crate can wrap it directly. Nothing in the model crates depends on this crate.
#![forbid(unsafe_code)]

pub mod api;
pub mod assumptions;
pub mod breakdowns;
pub mod command;
pub mod content;
pub mod emissions;
pub mod error;
pub mod events;
pub mod finance;
pub mod fixtures;
pub mod frames;
pub mod history;
pub mod industry;
pub mod keys;
pub mod ledger;
pub mod log;
pub mod meta;
pub mod persist;
pub mod policies;
pub mod snapshot;
pub mod turn;

pub use api::Api;
pub use assumptions::AssumptionTopic;
pub use command::{Command, CommonsAccessWire, PolicyValue, Tile};
pub use error::{ApiError, ApiResult};
pub use events::{SimEvent, SimEventKind, TraceRecord};
pub use history::HistoryPoint;
pub use keys::{BlockReason, InfraKind, LeverKey, SystemKey};
pub use ledger::{AccountRef, LedgerKindKey, LedgerPage, LedgerQuery, PersonLedger};
pub use meta::{ActionInfo, LeverInfo};
pub use snapshot::Snapshot;
pub use turn::{CommandPreview, MonthFrame, MonthResult, TurnResult, TurnStart};

/// Version of the wire format and of saved games. Bump when either changes shape.
/// Town v5 (contract v1.4) added the ledger to the saved wrapper blob: 4. Town v5.2 (contract
/// v1.6) added the wrapper log (per-tick emissions and legitimacy records, policy history): 5.
/// The model `State` has its own `sim_types::SCHEMA_VERSION` (3), unchanged.
pub const SCHEMA_VERSION: u32 = 5;
