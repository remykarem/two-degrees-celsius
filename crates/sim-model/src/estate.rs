//! S12 the estate (Town v7, docs/town-v7.md): the jet's monthly flight draw, the carbon price on
//! a flight, and the pilot's dismissal and rehire. The pilot's wage itself is paid in S4
//! (`income.rs`); the estate's household emissions and the flight's tonnes are totalled in S7
//! (`nature.rs`).
//!
//! One Bernoulli draw a month while the estate exists, taken even when the pilot is gone, so a
//! dismissal never shifts the random stream. A town without an estate draws nothing.

use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Is this person the jet's pilot, on the estate's payroll right now?
pub fn pilot_employed(s: &State, e: &Estate) -> bool {
    e.pilot
        .is_some_and(|id| s.person(id).staff_of == Some(e.household))
}

pub fn run(s: &State, p: &Params, r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let Some(e) = &s.estate else {
        return d;
    };
    let ep = &p.estate;
    let mut next = e.clone();
    // The draw is always taken.
    let draw = r.chance(ep.flight_prob_monthly);
    let hh = &s.households[e.household.idx()];
    let wage = Cents::from_dollars(ep.pilot_wage_usd);

    // Dismissal after too many months of short pay; rehire once savings cover the wages again.
    let mut employed = pilot_employed(s, e);
    if let Some(pid) = e.pilot {
        let pilot = s.person(pid);
        if employed && e.unpaid_months >= ep.pilot_unpaid_months_max {
            d.changes.push(Delta::Person(
                pid,
                PersonPatch {
                    staff_of: Some(None),
                    commute_tiles: Some(0),
                    commute_minutes: Some(0),
                    effective_hours: Some(Rate(1.0)),
                    ..PersonPatch::default()
                },
            ));
            d.events.push(Event::PilotLetGo { person: pid });
            next.unpaid_months = 0;
            employed = false;
        } else if !employed
            && pilot.job.is_none()
            && !pilot.jg
            && hh.savings >= wage.scale(ep.pilot_rehire_months_of_wages)
        {
            d.changes.push(Delta::Person(
                pid,
                PersonPatch {
                    staff_of: Some(Some(e.household)),
                    ..PersonPatch::default()
                },
            ));
            d.events.push(Event::PilotHired { person: pid });
            next.unpaid_months = 0;
            employed = true;
        }
    }

    // A flight needs the draw and, if the content has a pilot, an employed pilot.
    let flies = draw && (e.pilot.is_none() || employed);
    next.jet.flying_this_month = flies;
    next.wage_paid = Cents::ZERO;
    if flies {
        let tonnes = Tonnes(ep.flight_tonnes);
        next.flights_total += 1;
        next.tonnes_total = Tonnes(next.tonnes_total.0 + tonnes.0);
        next.last_flight = Some(LastFlight {
            tick: s.tick,
            tonnes,
        });
        // The carbon price falls on the flight, from the estate's savings, after lobbying.
        let due = crate::util::carbon_cost(s.effective.carbon_price, tonnes.0);
        let paid = due.min(hh.savings.max(Cents::ZERO));
        d.transfer_as(
            Account::Household(e.household),
            Account::Government,
            paid,
            FlowKind::Tax,
            Purpose::CarbonTax,
        );
        next.carbon_paid_total += paid;
        d.events.push(Event::PrivateJetFlight {
            household: e.household,
            tonnes,
        });
    }
    d.changes.push(Delta::Estate(next));
    d
}
