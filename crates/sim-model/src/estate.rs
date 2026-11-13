//! S12 the estate (Town v7, docs/town-v7.md): the jet's monthly flight draw, the carbon price on
//! a flight, and the pilot's dismissal and rehire. The pilot's wage itself is paid in S4
//! (`income.rs`); the estate's household emissions and the flight's tonnes are totalled in S7
//! (`nature.rs`).
//!
//! One Bernoulli draw a month while the estate exists, taken even when the pilot is gone, so a
//! dismissal never shifts the random stream. A town without an estate draws nothing.

use sim_params::{EstateParams, Params};
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::Effect as Fx;
use sim_types::QuantityId as Q;
use sim_types::*;

// ---- Decisions (docs/features/decision-points.md, stage 4) ----
//
// The estate decides whether to keep its pilot (let go after months of short pay, hire back once
// savings cover wages) and whether the jet flies. The pilot's pay itself is accounting (S4); the
// carbon charge on a flight is the player's carbon price applied, not a choice. The monthly draw
// stays in `run` and enters as `random_draw`.

pub const ESTATE_PILOT: DecisionSpec = DecisionSpec {
    id: D::EstatePilot,
    system: SystemId::Estate,
    inputs: &[
        Q::PilotEmployed,
        Q::PilotUnpaidMonths,
        Q::PilotFree,
        Q::EstateSavings,
    ],
    outputs: &[Q::PilotEmployed],
    effects: &[
        Fx::up(Q::PilotEmployed, Q::PilotEmployed),
        Fx::down(Q::PilotUnpaidMonths, Q::PilotEmployed),
        Fx::up(Q::PilotFree, Q::PilotEmployed),
        Fx::up(Q::EstateSavings, Q::PilotEmployed),
    ],
    params: &[
        "estate.pilot_unpaid_months_max",
        "estate.pilot_rehire_months_of_wages",
        "estate.pilot_wage_usd",
    ],
    rule: "dismiss_after_short_pay_rehire_when_flush",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PilotView {
    /// The pilot is on the estate's payroll right now.
    pub pilot_employed: bool,
    /// Consecutive months the wage could not be paid in full.
    pub pilot_unpaid_months: u32,
    /// The pilot has no other job and is not on the job guarantee, so can be hired back.
    pub pilot_free: bool,
    pub estate_savings: Cents,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PilotChoice {
    Keep,
    LetGo,
    Hire,
}

/// `None` when the content has no pilot.
pub fn pilot_view(s: &State, e: &Estate) -> Option<PilotView> {
    let pilot = s.person(e.pilot?);
    Some(PilotView {
        pilot_employed: pilot_employed(s, e),
        pilot_unpaid_months: e.unpaid_months,
        pilot_free: pilot.job.is_none() && !pilot.jg,
        estate_savings: s.households[e.household.idx()].savings,
    })
}

pub fn decide_pilot(v: &PilotView, ep: &EstateParams) -> PilotChoice {
    let wage = Cents::from_dollars(ep.pilot_wage_usd);
    if v.pilot_employed && v.pilot_unpaid_months >= ep.pilot_unpaid_months_max {
        PilotChoice::LetGo
    } else if !v.pilot_employed
        && v.pilot_free
        && v.estate_savings >= wage.scale(ep.pilot_rehire_months_of_wages)
    {
        PilotChoice::Hire
    } else {
        PilotChoice::Keep
    }
}

pub const ESTATE_FLIGHT: DecisionSpec = DecisionSpec {
    id: D::EstateFlight,
    system: SystemId::Estate,
    inputs: &[Q::RandomDraw, Q::PilotEmployed],
    outputs: &[Q::Flight],
    effects: &[
        Fx::down(Q::RandomDraw, Q::Flight),
        Fx::up(Q::PilotEmployed, Q::Flight),
    ],
    params: &["estate.flight_prob_monthly"],
    rule: "monthly_chance_with_pilot",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightView {
    /// This month's uniform draw in [0, 1), taken by `run` whether or not the jet can fly.
    pub random_draw: f64,
    /// A pilot is on the payroll, or the content has none to need.
    pub pilot_employed: bool,
}

pub fn flight_view(random_draw: f64, pilot_ready: bool) -> FlightView {
    FlightView {
        random_draw,
        pilot_employed: pilot_ready,
    }
}

pub fn decide_flight(v: &FlightView, ep: &EstateParams) -> bool {
    v.random_draw < ep.flight_prob_monthly && v.pilot_employed
}

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
    let draw = r.next_f64();
    let hh = &s.households[e.household.idx()];

    // Dismissal after too many months of short pay; rehire once savings cover the wages again.
    let mut employed = pilot_employed(s, e);
    if let (Some(pid), Some(view)) = (e.pilot, pilot_view(s, e)) {
        match decide_pilot(&view, ep) {
            PilotChoice::LetGo => {
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
            }
            PilotChoice::Hire => {
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
            PilotChoice::Keep => {}
        }
    }

    // A flight needs the draw and, if the content has a pilot, an employed pilot.
    let flies = decide_flight(&flight_view(draw, e.pilot.is_none() || employed), ep);
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
