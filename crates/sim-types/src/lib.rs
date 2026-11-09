//! Data only: IDs, quantities, enums, state structs, commands, events, deltas.
//! The only logic here is the arithmetic of the quantity newtypes.
#![forbid(unsafe_code)]

pub mod deltas;
pub mod enums;
pub mod ids;
pub mod state;
pub mod units;

pub use deltas::*;
pub use enums::*;
pub use ids::*;
pub use state::*;
pub use units::*;

/// Version of the serialised state layout.
pub const SCHEMA_VERSION: u32 = 1;
/// Hard cap on agents: 6 households, 4 firm slots (2 dormant), bank, government = 12.
pub const MAX_FIRMS: usize = 4;
