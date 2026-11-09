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
            Command::SetSpeed(v) => {
                if (1..=4).contains(&v) {
                    s.speed = v;
                } else {
                    events.push(Event::PolicyBlocked {
                        reason: BlockReason::InvalidSpeed,
                    });
                }
            }
            Command::Place { kind, count } => {
                if count > 0 {
                    s.gov.pending_build.push((kind, count));
                }
            }
            Command::Remove { kind, count } => {
                let removed = match kind {
                    InfraKind::PublicHousing => {
                        let r = (count as f64).min(s.housing.public_cap.0);
                        s.housing.public_cap -= Units(r);
                        r
                    }
                    InfraKind::FossilPlant => {
                        let r =
                            (count as f64 * p.government.plant_capacity).min(s.energy.fossil_cap.0);
                        s.energy.fossil_cap -= Units(r);
                        r
                    }
                    InfraKind::RenewablePlant => {
                        let r = (count as f64 * p.government.plant_capacity)
                            .min(s.energy.renewable_cap.0);
                        s.energy.renewable_cap -= Units(r);
                        r
                    }
                };
                if removed <= 0.0 {
                    events.push(Event::PolicyBlocked {
                        reason: BlockReason::NothingToRemove,
                    });
                }
            }
        }
    }
}
