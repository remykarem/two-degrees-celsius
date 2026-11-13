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
pub mod catalog;
pub mod command;
pub mod content;
#[cfg(feature = "ts")]
pub mod contract_ts;
pub mod decisions;
pub mod emissions;
pub mod entities;
pub mod error;
pub mod events;
pub mod finance;
pub mod fixtures;
pub mod frames;
pub mod history;
pub mod industry;
pub mod keys;
pub mod ledger;
pub mod live;
pub mod log;
pub mod markets;
pub mod meta;
pub mod networth;
pub mod persist;
pub mod policies;
pub mod recording;
pub mod snapshot;
pub mod sources;
pub mod trades;
pub mod turn;
pub mod yields;

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
pub use trades::{MarketTrades, TradesQuery};
pub use turn::{CommandPreview, MonthFrame, MonthResult, TurnResult, TurnStart};

/// Version of the wire format and of saved games. Bump when either changes shape.
/// Town v5 (contract v1.4) added the ledger to the saved wrapper blob: 4. Town v5.2 (contract
/// v1.6) added the wrapper log (per-tick emissions and legitimacy records, policy history): 5.
/// The model `State` has its own `sim_types::SCHEMA_VERSION`: 3 until Town v7, 4 since the estate
/// joined the state. This wire version stayed 5 through contract v1.9 (additive). Platform cleanup
/// (contract v2.0, `docs/snapshot-contract.md` section 20) reshaped emissions and income into
/// source lists: 6. The saved blob itself did not change shape; the bump keeps the contract
/// version and the snapshot's `schemaVersion` honest, and refuses saves made by the old web.
/// Markets view (contract v2.3, `docs/snapshot-contract.md` section 23) added price series to
/// `HistoryPoint` and market records to the saved wrapper log: 7.
/// Weather got its own random stream, stored in the state (state schema 5): 8.
pub const SCHEMA_VERSION: u32 = 8;

/// Version of the contract text in `docs/snapshot-contract.md` (the newest section). Emitted into
/// the generated TS so the web compares against it, not a hard-coded copy.
pub const CONTRACT_VERSION: &str = "2.10.0";
