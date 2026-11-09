//! Player commands: the only way a player changes the simulation.

use crate::apply::set_policy;
use sim_params::Params;
use sim_types::*;

pub fn apply_commands(s: &mut State, p: &Params, cmds: &[Command], events: &mut Vec<Event>) {
    for c in cmds {
        match *c {
            Command::SetPolicy(ch) => {
                let blocked = match ch {
                    // The campaign-finance cap only exists outside plain market capitalism.
                    PolicyChange::CampaignCap(_) => s.system == EconomicSystem::Market,
                    // Community management needs a system that supports it; enclosure
                    // happens through lobbying, never by player command.
                    PolicyChange::CommonsAccess(CommonsAccess::Community { .. }) => {
                        s.system == EconomicSystem::Market
                    }
                    PolicyChange::CommonsAccess(CommonsAccess::Enclosed(_)) => true,
                    _ => false,
                };
                if blocked {
                    events.push(Event::PolicyBlocked {
                        reason: BlockReason::LeverUnavailable,
                    });
                } else {
                    set_policy(s, ch);
                }
            }
            Command::SwitchSystem(to) => {
                if s.transition.is_some() {
                    events.push(Event::PolicyBlocked {
                        reason: BlockReason::TransitionInProgress,
                    });
                } else if to == s.system {
                    events.push(Event::PolicyBlocked {
                        reason: BlockReason::SameSystem,
                    });
                } else {
                    let months = p.systems.get(to).transition_months;
                    s.transition = Some(Transition {
                        from: s.system,
                        to,
                        months_left: months.max(1),
                        total: months.max(1),
                    });
                    events.push(Event::SystemSwitchStarted { to });
                }
            }
            Command::Place { kind, tile } => match crate::turn::tile_problem(s, &[], c) {
                // The tile was valid when queued; it may have been taken since.
                Some(reason) => events.push(Event::PolicyBlocked { reason }),
                None => s.gov.pending_build.push((kind, tile)),
            },
            Command::Remove { tile } => match crate::turn::tile_problem(s, &[], c) {
                Some(reason) => events.push(Event::PolicyBlocked { reason }),
                None => remove_tile(s, tile, events),
            },
        }
    }
}

/// Remove the building on `tile` (public housing, plant or road). Callers have validated it.
fn remove_tile(s: &mut State, tile: Tile, events: &mut Vec<Event>) {
    match s.map.kind_at(tile) {
        Some(TileKind::PublicHousing) => {
            if let Some(i) = s.homes.iter().position(|h| h.tile == tile) {
                let home = s.homes.remove(i);
                if let Some(hh) = home.household {
                    s.households[hh.idx()].home = None;
                    events.push(Event::HouseholdEvicted { household: hh });
                }
            }
        }
        Some(TileKind::FossilPlant | TileKind::RenewablePlant) => {
            s.plants.retain(|p| p.tile != tile);
        }
        _ => {}
    }
    s.map.set(tile, MapTile::EMPTY);
    events.push(Event::Removed { tile });
}
