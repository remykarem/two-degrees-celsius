//! Data only: IDs, quantities, enums, state structs, commands, events, deltas.
//! The only logic here is the arithmetic of the quantity newtypes.
#![forbid(unsafe_code)]

pub mod deltas;
pub mod enums;
pub mod ids;
pub mod ledger;
pub mod state;
pub mod turn;
pub mod units;

pub use deltas::*;
pub use enums::*;
pub use ids::*;
pub use ledger::*;
pub use state::*;
pub use turn::*;
pub use units::*;

/// Version of the serialised state layout.
pub const SCHEMA_VERSION: u32 = 4;
/// Firm slots: Factory, Farm, Shop and one dormant slot.
pub const MAX_FIRMS: usize = 4;
