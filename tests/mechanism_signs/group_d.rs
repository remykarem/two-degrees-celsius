//! System probes, group D: the investment and nature systems (capital accumulation, emissions
//! accounting, the carbon budget, the forest and the commons). Each probe nudges one field of a
//! real mid-run `State`, runs the real system(s) with `common::after` and reads one field.
use crate::common::{after, params, states};
use sim_model::causal::Node;
use sim_model::sign_probe::{Probe, Report};
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;

type Nudges = Vec<(Q, fn(&mut State))>;
type Reads = Vec<(Q, fn(&State) -> f64)>;

fn mech(m: M, nudges: Nudges, reads: Reads, bases: Vec<State>, r: &mut Report) {
    r.check(&Probe {
        node: Node::Mechanism(m),
        bases,
        nudges,
        levels: crate::common::levels(),
        reads,
    });
}

fn inv(s: &State) -> State {
    after(SystemId::Investment, s, &params())
}
fn nat(s: &State) -> State {
    after(SystemId::Nature, s, &params())
}
/// Investment then Nature, so a forest clearing queued by building reaches the forest stock.
fn inv_nat(s: &State) -> State {
    nat(&inv(s))
}

/// Mid-run states with the estate jet grounded and a positive cumulative total (so the `max(0)`
/// on it does not bind).
fn bases() -> Vec<State> {
    states()
        .into_iter()
        .take(7)
        .map(|mut s| {
            if let Some(e) = s.estate.as_mut() {
                e.jet.flying_this_month = false;
            }
            s.nature.cumulative = Tonnes(s.nature.cumulative.0.max(50.0));
            s
        })
        .collect()
}

fn each_firm(s: &mut State, f: impl Fn(&mut Firm)) {
    for x in s.firms.iter_mut().filter(|x| x.active) {
        f(x);
    }
}

fn each_food_firm(s: &mut State) {
    for f in s
        .firms
        .iter_mut()
        .filter(|f| f.active && f.sector == Sector::Food)
    {
        f.output = Units(f.output.0 * 1.2 + 5.0);
    }
}

fn capital(s: &State) -> f64 {
    s.firms
        .iter()
        .filter(|f| f.active)
        .map(|f| f.capital.0)
        .sum()
}

fn more_emissions(s: &mut State) {
    each_firm(s, |f| f.emissions = Tonnes(f.emissions.0 * 1.1 + 1.0));
}

pub fn probe_all(r: &mut Report) {
    // Capital accumulation (investment::run). Capital: every active firm's capital stock, read
    // after the system. InvestmentPlan: what a firm bought this tick (`capex`, last month's plan
    // paid in the goods system) is what is added to capital and what clears forest; `plan_invest`
    // itself is only carried forward, so `capex` is the field that carries the effect. ForestStock
    // is read after the nature system, which applies the clearing the investment system queued.
    let mut with_capex = bases();
    for s in &mut with_capex {
        each_firm(s, |f| f.capex = Cents(50_000_000));
    }
    mech(
        M::CapitalAccumulation,
        vec![
            (Q::Capital, |s| {
                each_firm(s, |f| f.capital = Units(f.capital.0 * 1.1 + 1.0))
            }),
            (Q::InvestmentPlan, |s| {
                each_firm(s, |f| f.capex = Cents(f.capex.0 * 2 + 10_000_000))
            }),
        ],
        vec![
            (Q::Capital, |s| capital(&inv(s))),
            (Q::ForestStock, |s| inv_nat(s).nature.forest.0),
        ],
        with_capex,
        r,
    );

    // Emissions accounting (nature::run). DirectEmissions: each active firm's monthly emissions;
    // PlantEmissions: each plant's; HouseholdSize: one more member in each non-estate household
    // (a copy of an existing id stands in for the extra person); Flight: the jet flies this
    // month. Read: nature.gross_emissions.
    mech(
        M::EmissionsAccounting,
        vec![
            (Q::DirectEmissions, more_emissions),
            (Q::PlantEmissions, |s| {
                for p in &mut s.plants {
                    p.emissions = Tonnes(p.emissions.0 * 1.1 + 1.0);
                }
            }),
            (Q::HouseholdSize, |s| {
                let est = s.estate.as_ref().map(|e| e.household);
                for h in &mut s.households {
                    if Some(h.id) != est {
                        if let Some(m) = h.members.first().copied() {
                            h.members.push(m);
                        }
                    }
                }
            }),
            (Q::Flight, |s| {
                if let Some(e) = s.estate.as_mut() {
                    e.jet.flying_this_month = true;
                }
            }),
        ],
        vec![(Q::GrossEmissions, |s| nat(s).nature.gross_emissions.0)],
        bases(),
        r,
    );

    // Carbon budget (nature::run). GrossEmissions: nudged through firm emissions (the sum the
    // system reads); ForestSink: nudged through the forest stock (sink = sigma x the stock at
    // the start of the month); CumulativeEmissions: nature.cumulative. Read: nature.net_emissions
    // and nature.cumulative after the system.
    mech(
        M::CarbonBudget,
        vec![
            (Q::GrossEmissions, more_emissions),
            (Q::ForestSink, |s| {
                s.nature.forest = Units(s.nature.forest.0 * 1.05 + 1.0)
            }),
            (Q::CumulativeEmissions, |s| {
                s.nature.cumulative = Tonnes(s.nature.cumulative.0 * 1.1 + 1.0)
            }),
        ],
        vec![
            (Q::NetEmissions, |s| nat(s).nature.net_emissions.0),
            (Q::CumulativeEmissions, |s| nat(s).nature.cumulative.0),
        ],
        bases(),
        r,
    );

    // Forest stock change (nature::run): food firms' output raises the forest harvest. Nudge
    // each food firm's output; read nature.forest after the system.
    mech(
        M::ForestStockChange,
        vec![(Q::Output, each_food_firm)],
        vec![(Q::ForestStock, |s| nat(s).nature.forest.0)],
        bases(),
        r,
    );

    // Commons stock change (nature::run). Output: food firms' output; CommonsHarvest: the policy
    // lever; ResilienceSpending: the policy spending (cuts the drought damage, so the bases
    // carry a drought); CommonsEnclosed: access rule open -> enclosed (harvest x multiplier;
    // states already enclosed or community are flat and ignored). Read: nature.commons.
    let droughty = bases()
        .into_iter()
        .map(|mut s| {
            if s.weather.drought.is_none() {
                s.weather.drought = Some(Drought {
                    severity: 0.5,
                    months_left: 3,
                });
            }
            s
        })
        .collect();
    mech(
        M::CommonsStockChange,
        vec![
            (Q::Output, each_food_firm),
            (Q::CommonsHarvest, |s| {
                s.policy.commons_harvest = Rate(s.policy.commons_harvest.0 * 1.2 + 0.1)
            }),
            (Q::ResilienceSpending, |s| {
                s.policy.resilience_spending = Cents(s.policy.resilience_spending.0 * 2 + 200_000)
            }),
            (Q::CommonsEnclosed, |s| {
                if matches!(s.nature.commons_access, CommonsAccess::Open) {
                    s.nature.commons_access = CommonsAccess::Enclosed(HouseholdId(0));
                }
            }),
        ],
        vec![(Q::CommonsStock, |s| nat(s).nature.commons.0)],
        droughty,
        r,
    );
}
