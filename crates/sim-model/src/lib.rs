//! The causal rules. One module per system (see docs/causal-model.md).
//! Every system is a pure function `fn(&State, &Params, &mut Rng) -> Deltas`:
//! it reads a snapshot of the state and returns changes; the engine applies them.
#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

pub mod class;
pub mod credit;
pub mod goods;
pub mod government;
pub mod income;
pub mod investment;
pub mod labour;
pub mod lobbying;
pub mod nature;
pub mod policy;
pub mod production;
pub mod util;
pub mod wellbeing;

use sim_params::Params;
use sim_rng::Rng;
use sim_types::{Deltas, State, SystemId};

pub type SystemFn = fn(&State, &Params, &mut Rng) -> Deltas;

/// The systems in tick order (SPEC section 11), with the ids used in checks.
pub const TICK_ORDER: [(SystemId, SystemFn); 12] = [
    (SystemId::Policy, policy::run),
    (SystemId::Labour, labour::run),
    (SystemId::Production, production::run),
    (SystemId::Nature, nature::run),
    (SystemId::Goods, goods::run),
    (SystemId::Income, income::run),
    (SystemId::Credit, credit::run),
    (SystemId::Government, government::run),
    (SystemId::Investment, investment::run),
    (SystemId::Class, class::run),
    (SystemId::Lobbying, lobbying::run),
    (SystemId::Wellbeing, wellbeing::run),
];
