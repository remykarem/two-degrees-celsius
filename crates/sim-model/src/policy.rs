//! Step 1 of the tick: reset per-tick flows, advance system transitions.
//! A switch is slow: the new rules take effect when the transition completes, and during it
//! output is temporarily lower and affected classes resist (a legitimacy cost, see S8).

use sim_params::{CommonsRule, DividendRule, Params};
use sim_rng::Rng;
use sim_types::*;

fn owner_for(rule: DividendRule, founder: HouseholdId) -> Owner {
    match rule {
        DividendRule::Owner => Owner::Household(founder),
        DividendRule::Employees => Owner::Workers,
        DividendRule::State => Owner::State,
    }
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    d.changes.push(Delta::ResetFlows);
    let Some(t) = s.transition else {
        if s.output_penalty.0 != 0.0 {
            d.changes.push(Delta::OutputPenalty(Rate::ZERO));
        }
        return d;
    };
    let target = p.systems.get(t.to);
    let left = t.months_left.saturating_sub(1);
    if left > 0 {
        d.changes.push(Delta::Transition(Some(Transition { months_left: left, ..t })));
        d.changes.push(Delta::OutputPenalty(Rate(target.transition_output_penalty)));
        return d;
    }
    // Completion: the new system's ownership and policy defaults take effect.
    d.changes.push(Delta::Transition(None));
    d.changes.push(Delta::OutputPenalty(Rate::ZERO));
    d.changes.push(Delta::System(t.to));
    for f in &s.firms {
        d.firm(f.id, FirmPatch { owner: Some(owner_for(target.dividend_rule, f.founder)), ..FirmPatch::default() });
    }
    if s.policy.green_subsidy.0 < target.green_subsidy_floor {
        d.changes.push(Delta::SetPolicy(PolicyChange::GreenSubsidy(Rate(target.green_subsidy_floor))));
    }
    if t.to == EconomicSystem::Market {
        d.changes.push(Delta::SetPolicy(PolicyChange::CampaignCap(Rate(1.0))));
    } else if target.campaign_cap < s.policy.campaign_cap.0 {
        d.changes.push(Delta::SetPolicy(PolicyChange::CampaignCap(Rate(target.campaign_cap))));
    }
    let floor = Cents::from_dollars(target.carbon_price_floor_usd);
    if s.policy.carbon_price < floor {
        d.changes.push(Delta::SetPolicy(PolicyChange::CarbonPrice(floor)));
    }
    match (target.commons_rule, s.nature.commons_access) {
        (CommonsRule::Community, CommonsAccess::Open | CommonsAccess::Enclosed(_)) => {
            d.changes.push(Delta::SetPolicy(PolicyChange::CommonsAccess(CommonsAccess::Community {
                cap: Rate(1.0),
            })));
        }
        (CommonsRule::Open, CommonsAccess::Community { .. }) => {
            d.changes.push(Delta::SetPolicy(PolicyChange::CommonsAccess(CommonsAccess::Open)));
        }
        _ => {}
    }
    d.events.push(Event::SystemSwitchCompleted { to: t.to });
    d
}
