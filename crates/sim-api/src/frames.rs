//! Month frames: the per-tick events and flows the scene plays back.
//!
//! What is real and what is empty (see docs/sim-api-notes.md):
//! - `events`: every sim event of the tick, with importance. Month 0 also carries the events
//!   raised before the first tick (rejections, activations).
//! - `commutes`: one entry per household cohort and active firm with employed people,
//!   from the end-of-tick employment table (every month, not only new hires).
//! - `emissions`: the tick's emission trace records (per firm, households, sink).
//! - `rent`: rent paid by each household this tick.
//! - `wages`: per household and firm, wage x employment x wage premium (the exact S4 formula),
//!   capped at the wages the household actually received this tick.
//! - `purchases`: always empty. The sim records only a household's total spending, with no
//!   sector or counterparty, so any split would be invented.
use crate::events::{EmissionLine, SimEvent};
use crate::turn::{Commute, MonthFlows, RentFlow, WageFlow};
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
    for h in &s.households {
        let mut wage_rows: Vec<(u32, f64)> = Vec::new();
        for fi in s.firms.iter().filter(|x| x.active) {
            let n = h.employment.get(fi.id.idx()).map_or(0.0, |p| p.0);
            if n > 1e-9 {
                f.commutes.push(Commute {
                    household: h.id.0,
                    firm: fi.id.0,
                    people: n,
                });
                wage_rows.push((fi.id.0, fi.wage.scale(n * h.wage_premium.0).as_f64()));
            }
        }
        let due: f64 = wage_rows.iter().map(|r| r.1).sum();
        let paid = h.income.wages.as_f64();
        let factor = if due > paid && due > 0.0 {
            paid / due
        } else {
            1.0
        };
        for (firm, cents) in wage_rows {
            let amount = cents * factor / 100.0;
            if amount > 0.0 {
                f.wages.push(WageFlow {
                    household: h.id.0,
                    firm,
                    amount,
                });
            }
        }
        if h.income.rent_paid > st::Cents::ZERO {
            f.rent.push(RentFlow {
                household: h.id.0,
                amount: h.income.rent_paid.dollars(),
            });
        }
    }
    for t in &out.trace {
        if let st::TraceRecord::Emissions { source, tonnes } = *t {
            f.emissions.push(EmissionLine::from_sim(source, tonnes));
        }
    }
    f
}
