//! Month frames: the per-tick events and flows the scene plays back.
//!
//! What is real and what is empty (see docs/sim-api-notes.md, "Town v2"):
//! - `events`: every sim event of the tick, with importance. Month 0 also carries the events
//!   raised before the first tick (rejections, activations).
//! - `commutes` (real): one entry per person with a firm job, from the end-of-tick state:
//!   home tile (`from`), workplace tile (`to`), tiles and minutes. The sim does not expose the
//!   path itself, so there is no `path` field.
//! - `staff_commutes` (contract v1.9, real): the same for people on a household's payroll (the
//!   estate's pilot): home tile, hangar tile, tiles, minutes and the wage paid this tick.
//! - `purchases` (real): the tick's `purchase` events, household to firm, in dollars.
//! - `wages` (real): each employed person's wage paid this tick, to their firm.
//! - `rent` (real): rent paid by each household this tick, with its home.
//! - `emissions` (real): the tick's emission trace records (per firm, households, plant with
//!   its tile, sink; the sink is negative).
use crate::command::Tile;
use crate::events::{EmissionLine, SimEvent};
use crate::keys::SectorKey;
use crate::turn::{Commute, MonthFlows, Purchase, RentFlow, StaffCommute, WageFlow};
use sim_engine::{Sim, StepOutput};
use sim_types as st;

/// What one tick contributes to a month frame.
#[derive(Clone, Debug, Default)]
pub struct TickCapture {
    /// Number of sim events in this tick (to split the turn's event list afterwards).
    pub sim_event_count: usize,
    pub events: Vec<SimEvent>,
    pub flows: MonthFlows,
}

/// Capture one tick. `sim` is the state after the tick has been applied.
pub fn capture(sim: &Sim, out: &StepOutput) -> TickCapture {
    TickCapture {
        sim_event_count: out.events.len(),
        events: out.events.iter().map(SimEvent::from_sim).collect(),
        flows: flows(sim, out),
    }
}

fn flows(sim: &Sim, out: &StepOutput) -> MonthFlows {
    let s = &sim.state;
    let mut f = MonthFlows::default();
    for p in &s.persons {
        let Some(firm_id) = p.job else { continue };
        let Some(firm) = s.firms.get(firm_id.idx()) else {
            continue;
        };
        let from = s
            .households
            .get(p.household.idx())
            .and_then(|h| h.home)
            .and_then(|id| s.home(id))
            .map(|h| Tile::from_sim(h.tile));
        f.commutes.push(Commute {
            person: p.id.0,
            household: p.household.0,
            firm: firm_id.0,
            from,
            to: Tile::from_sim(firm.tile),
            tiles: p.commute_tiles,
            minutes: p.commute_minutes,
        });
        if p.wage > st::Cents::ZERO {
            f.wages.push(WageFlow {
                person: p.id.0,
                household: p.household.0,
                firm: firm_id.0,
                amount: p.wage.dollars(),
            });
        }
    }
    // Town v7: domestic staff (the estate's pilot) commute to the hangar tile and are paid by
    // their employer household.
    if let Some(est) = &s.estate {
        for p in s
            .persons
            .iter()
            .filter(|p| p.staff_of == Some(est.household))
        {
            f.staff_commutes.push(StaffCommute {
                person: p.id.0,
                household: p.household.0,
                employer: est.household.0,
                from: s
                    .households
                    .get(p.household.idx())
                    .and_then(|h| h.home)
                    .and_then(|id| s.home(id))
                    .map(|h| Tile::from_sim(h.tile)),
                to: Tile::from_sim(est.hangar),
                tiles: p.commute_tiles,
                minutes: p.commute_minutes,
                wage: p.wage.dollars(),
            });
        }
    }
    for e in &out.events {
        if let st::Event::Purchase {
            household,
            firm,
            amount,
        } = *e
        {
            if let Some(fi) = s.firms.get(firm.idx()) {
                f.purchases.push(Purchase {
                    household: household.0,
                    firm: firm.0,
                    sector: SectorKey::from_sim(fi.sector),
                    amount: amount.dollars(),
                });
            }
        }
    }
    for h in &s.households {
        if h.income.rent_paid > st::Cents::ZERO {
            f.rent.push(RentFlow {
                household: h.id.0,
                home: h.home.map(|x| x.0),
                amount: h.income.rent_paid.dollars(),
            });
        }
    }
    for t in &out.trace {
        if let st::TraceRecord::Emissions { source, tonnes } = *t {
            f.emissions
                .push(EmissionLine::from_sim(source, tonnes, &s.plants));
        }
    }
    f
}
