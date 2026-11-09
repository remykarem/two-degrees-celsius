//! The causal rules. One module per system (see docs/causal-model.md).
//! Every system is a pure function `fn(&State, &Params, &mut Rng) -> Deltas`:
//! it reads a snapshot of the state and returns changes; the engine applies them.
#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

pub mod policy;
pub mod util;

use sim_params::Params;
use sim_rng::Rng;
use sim_types::{Deltas, State, SystemId};

pub type SystemFn = fn(&State, &Params, &mut Rng) -> Deltas;

/// The systems in tick order (SPEC section 11), with the ids used in checks.
pub const TICK_ORDER: [(SystemId, SystemFn); 1] = [(SystemId::Policy, policy::run)];
