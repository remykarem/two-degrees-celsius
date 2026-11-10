//! Wire `Command` (contract section 4) and its conversion to and from the sim command.
use crate::keys::{InfraKind, LeverKey, SystemKey};
use serde::{Deserialize, Serialize};
use sim_types as st;

/// A tile of the simulation's 12 x 12 map (`x` column, `y` row, from 0). Integers are accepted
/// on input so that out-of-range values become `tile_invalid` instead of a parse error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
}

impl Tile {
    pub fn from_sim(t: st::Tile) -> Tile {
        Tile {
            x: i32::from(t.x),
            y: i32::from(t.y),
        }
    }
    /// `None` if the coordinates do not fit a map tile at all.
    pub fn to_sim(self) -> Option<st::Tile> {
        let x = u8::try_from(self.x).ok()?;
        let y = u8::try_from(self.y).ok()?;
        Some(st::Tile { x, y })
    }
}

/// Commons rule as the player may send it. `enclosed` only arises from lobbying.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CommonsAccessWire {
    Open,
    Community { cap: f64 },
}

/// A lever value: number, boolean or a commons rule.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyValue {
    Bool(bool),
    Number(f64),
    Commons(CommonsAccessWire),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    SetPolicy { lever: LeverKey, value: PolicyValue },
    SwitchSystem { to: SystemKey },
    Place { kind: InfraKind, tile: Tile },
    Remove { tile: Tile },
}

/// Community commons cap range: multiples of maximum sustainable yield.
pub const COMMUNITY_CAP_MAX: f64 = 1.0;

/// Build the sim policy change for a numeric lever. Returns `None` for non-numeric levers.
pub(crate) fn numeric_change(lever: LeverKey, v: f64) -> Option<st::PolicyChange> {
    use st::PolicyChange as P;
    let usd = st::Cents::from_dollars;
    let r = st::Rate;
    Some(match lever {
        LeverKey::CarbonPrice => P::CarbonPrice(usd(v)),
        LeverKey::TaxWage => P::TaxWage(r(v)),
        LeverKey::TaxProfit => P::TaxProfit(r(v)),
        LeverKey::TaxCapital => P::TaxCapital(r(v)),
        LeverKey::WealthTax => P::WealthTax(r(v)),
        LeverKey::WealthThreshold => P::WealthThreshold(usd(v)),
        LeverKey::BaseRate => P::BaseRate(r(v)),
        LeverKey::MinWage => P::MinWage(usd(v)),
        LeverKey::Benefit => P::Benefit(usd(v)),
        LeverKey::Ubi => P::Ubi(usd(v)),
        LeverKey::GreenSubsidy => P::GreenSubsidy(r(v)),
        LeverKey::CampaignCap => P::CampaignCap(r(v)),
        LeverKey::EmissionsCap => P::EmissionsCap(st::Tonnes(v)),
        LeverKey::CommonsHarvest => P::CommonsHarvest(r(v)),
        LeverKey::BankCapitalRatio => P::BankCapitalRatio(r(v)),
        LeverKey::PublicSpending => P::PublicSpending(usd(v)),
        LeverKey::JobGuarantee => P::JobGuarantee(r(v)),
        LeverKey::ResilienceSpending => P::ResilienceSpending(usd(v)),
        LeverKey::CommonsAccess | LeverKey::Bailout => return None,
    })
}

impl Command {
    /// Wire form of a sim command (used for pending items and echoed commands).
    pub fn from_sim(c: &st::Command) -> Command {
        match *c {
            st::Command::Place { kind, tile } => Command::Place {
                kind: InfraKind::from_sim(kind),
                tile: Tile::from_sim(tile),
            },
            st::Command::Remove { tile } => Command::Remove {
                tile: Tile::from_sim(tile),
            },
            st::Command::SwitchSystem(to) => Command::SwitchSystem {
                to: SystemKey::from_sim(to),
            },
            st::Command::SetPolicy(ch) => {
                let lever = LeverKey::from_policy(&ch);
                let value = match ch {
                    st::PolicyChange::Bailout(b) => PolicyValue::Bool(b),
                    st::PolicyChange::CommonsAccess(a) => PolicyValue::Commons(match a {
                        st::CommonsAccess::Community { cap } => {
                            CommonsAccessWire::Community { cap: cap.0 }
                        }
                        // Enclosure cannot be queued by a player (validate() blocks it), so
                        // this arm is unreachable in practice; `open` is the safe fallback.
                        st::CommonsAccess::Open | st::CommonsAccess::Enclosed(_) => {
                            CommonsAccessWire::Open
                        }
                    }),
                    other => PolicyValue::Number(other.value().unwrap_or(0.0)),
                };
                Command::SetPolicy { lever, value }
            }
        }
    }
}
