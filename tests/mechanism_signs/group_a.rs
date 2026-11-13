//! Group A: the production system (`production::run` and the factors of `capacity_factors`).
//! Every probe nudges one concrete field of a real `State` and reads one concrete field of the
//! state after the real system ran (`common::after`), or of the real factor function on a state.
//! Town aggregates are sums over active firms, as `live.rs` binds them.
use crate::common::{after, params, states};
use sim_model::causal::Node;
use sim_model::production::{capacity_factors, effective_labour};
use sim_model::sign_probe::{Probe, Report};
use sim_params::Params;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;
use std::sync::OnceLock;

fn p() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(params)
}

type N = (Q, fn(&mut State));
type R = (Q, fn(&State) -> f64);

/// The mid-run states, the stressed ones and a thinned sample of the weather seeds.
fn base() -> Vec<State> {
    states()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i < 8 || i % 40 == 0)
        .map(|(_, s)| s)
        .collect()
}

/// States where electricity is short: plant capacity cut to 30 per cent, so the served share binds.
fn tight() -> Vec<State> {
    base()
        .into_iter()
        .map(|mut s| {
            for pl in &mut s.plants {
                pl.capacity.0 *= 0.3;
            }
            s
        })
        .collect()
}

/// States whose first plant is renewable (the town starts all fossil, so grid intensity is 1 and
/// could only fall; with a renewable plant a fossil one can raise it).
fn green_grid() -> Vec<State> {
    base()
        .into_iter()
        .take(4)
        .map(|mut s| {
            if let Some(pl) = s.plants.first_mut() {
                pl.kind = PlantKind::Renewable;
            }
            s
        })
        .collect()
}

/// States with an emissions cap (tonnes per firm per month) tight enough to bind, on the fossil
/// and on the half-renewable grid.
fn capped() -> Vec<State> {
    let mut out = Vec::new();
    for cap in [0.2, 2.0, 20.0] {
        for mut s in base().into_iter().take(4).chain(green_grid()) {
            s.effective.emissions_cap.0 = cap;
            out.push(s);
        }
    }
    out
}

/// Sums rounded to 1e-6 so that float summation noise (1e-14, from a cap that binds on the same
/// firms either way) is not read as a movement.
fn settle(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

fn firm_out(s: &State) -> f64 {
    settle(
        after(SystemId::Production, s, p())
            .firms
            .iter()
            .filter(|f| f.active)
            .map(|f| f.output.0)
            .sum(),
    )
}

fn firms(s: &mut State, f: impl Fn(&mut Firm)) {
    for x in s.firms.iter_mut().filter(|x| x.active) {
        f(x);
    }
}

fn food_firm(s: &State) -> Option<&Firm> {
    s.firms
        .iter()
        .find(|f| f.active && f.sector == Sector::Food)
}

/// The weather part of a firm's capacity: climate damage x drought loss x smoke.
fn weather_of(s: &State, f: &Firm) -> f64 {
    let c = capacity_factors(s, p(), f, 1.0);
    c.climate * c.drought * c.smoke
}

fn weather_sum(s: &State) -> f64 {
    s.firms
        .iter()
        .filter(|f| f.active)
        .map(|f| weather_of(s, f))
        .sum()
}

fn n(q: Q, f: fn(&mut State)) -> N {
    (q, f)
}
fn r(q: Q, f: fn(&State) -> f64) -> R {
    (q, f)
}

fn go(rep: &mut Report, m: M, bases: Vec<State>, nudges: Vec<N>, reads: Vec<R>) {
    rep.check(&Probe {
        node: Node::Mechanism(m),
        bases,
        nudges,
        levels: crate::common::levels(),
        reads,
    });
}

fn bump_drought(s: &mut State) {
    let d = s.weather.drought;
    s.weather.drought = Some(Drought {
        severity: (d.map_or(0.0, |d| d.severity) * 1.5 + 0.1).min(1.0),
        months_left: d.map_or(3, |d| d.months_left),
    });
}

fn add_fires(s: &mut State) {
    let tiles: Vec<Tile> = s
        .firms
        .iter()
        .filter(|f| f.active)
        .map(|f| f.tile)
        .collect();
    for tile in tiles {
        if !s.weather.fires.iter().any(|f| f.tile == tile) {
            s.weather.fires.push(Fire {
                tile,
                months_left: 2,
            });
        }
    }
}

fn more_forest(s: &mut State) {
    let f = s.nature.forest.0;
    // Raise only: some states start above the maximum.
    s.nature.forest.0 = (f * 1.2 + 1.0).min(p().nature.forest_max).max(f);
}

fn more_capital(s: &mut State) {
    firms(s, |f| f.capital.0 *= 1.2);
}

fn more_resilience(s: &mut State) {
    s.policy.resilience_spending = Cents(s.policy.resilience_spending.0 * 2 + 100_000);
}

fn more_plant_capacity(s: &mut State) {
    for pl in &mut s.plants {
        pl.capacity.0 *= 1.3;
    }
}

fn renewable_to_fossil(s: &mut State) {
    if let Some(pl) = s.plants.iter_mut().find(|x| x.kind == PlantKind::Renewable) {
        pl.kind = PlantKind::Fossil;
    }
}

pub fn probe_all(rep: &mut Report) {
    // technology_growth: every firm's tfp up 10 per cent; read the tfp after the production system.
    go(
        rep,
        M::TechnologyGrowth,
        base(),
        vec![n(Q::Technology, |s| firms(s, |f| f.tfp.0 *= 1.1))],
        vec![r(Q::Technology, |s| {
            after(SystemId::Production, s, p())
                .firms
                .iter()
                .filter(|f| f.active)
                .map(|f| f.tfp.0)
                .sum()
        })],
    );

    // effective_labour_supply: read the sum of `effective_labour` over active firms. Workers: a
    // jobless person takes a job at the first active firm; SkilledWorkers: every employed
    // person's skill rises 20 per cent (skill is what makes a worker count for more hours).
    go(
        rep,
        M::EffectiveLabourSupply,
        base(),
        vec![
            n(Q::Workers, |s| {
                let id = s.firms.iter().find(|f| f.active).map(|f| f.id);
                if let Some(x) = s.persons.iter_mut().find(|x| x.job.is_none()) {
                    x.job = id;
                }
            }),
            n(Q::SkilledWorkers, |s| {
                for x in s.persons.iter_mut().filter(|x| x.job.is_some()) {
                    x.skill.0 *= 1.2;
                }
            }),
        ],
        vec![r(Q::EffectiveLabour, |s| {
            s.firms
                .iter()
                .filter(|f| f.active)
                .map(|f| effective_labour(s, f))
                .sum()
        })],
    );

    // farm_health_factor: the farm's health factor (`capacity_factors(..).farm_health`) of the first
    // food firm. Forest and commons stocks rise (capped at their maxima). CommonsCommunity: access
    // moves to (or loosens) a community cap, which must not move the health factor.
    go(
        rep,
        M::FarmHealthFactor,
        base(),
        vec![
            n(Q::ForestStock, more_forest),
            n(Q::CommonsStock, |s| {
                let c = s.nature.commons.0;
                s.nature.commons.0 = (c * 1.2 + 1.0).min(p().nature.commons_max).max(c)
            }),
            n(Q::CommonsCommunity, |s| {
                s.nature.commons_access = match s.nature.commons_access {
                    CommonsAccess::Community { cap } => CommonsAccess::Community {
                        cap: Rate(cap.0 * 1.5),
                    },
                    _ => CommonsAccess::Community { cap: Rate(0.02) },
                }
            }),
        ],
        vec![r(Q::ResourceFactor, |s| {
            food_firm(s).map_or(0.0, |f| capacity_factors(s, p(), f, 1.0).farm_health)
        })],
    );

    // drought_loss: weather part of capacity (climate x drought x smoke) summed over active firms
    // (only farms lose to drought). Drought: severity up (or a new drought); ResilienceSpending: up.
    go(
        rep,
        M::DroughtLoss,
        base(),
        vec![
            n(Q::Drought, bump_drought),
            n(Q::ResilienceSpending, more_resilience),
        ],
        vec![r(Q::WeatherFactor, weather_sum)],
    );

    // fire_smoke: a fire starts on every active firm's tile; read the same weather factor.
    go(
        rep,
        M::FireSmoke,
        base(),
        vec![n(Q::Fire, add_fires)],
        vec![r(Q::WeatherFactor, weather_sum)],
    );

    // transition_loss (runs in the policy system): a transition to the co-op system starts; read
    // 1 - output_penalty after `policy::run`, which is the transition factor firms multiply by.
    go(
        rep,
        M::TransitionLoss,
        base(),
        vec![n(Q::SystemChange, |s| {
            if s.transition.is_none() {
                s.transition = Some(Transition {
                    from: s.system,
                    to: EconomicSystem::Coop,
                    months_left: 5,
                    total: 6,
                });
            }
        })],
        vec![r(Q::TransitionFactor, |s| {
            1.0 - after(SystemId::Policy, s, p()).output_penalty.0
        })],
    );

    // production_output: total output of active firms after `production::run`. Nudges: capital up
    // (the Cobb-Douglas part), forest up (farm health), resilience spending up (weather, needs a
    // drought), output penalty halved (transition), plant capacity up (served share, tight states),
    // emissions cap loosened or first set (capped states), a renewable plant turned fossil (grid
    // intensity, matters only under a cap).
    let mut bases = base();
    bases.extend(tight());
    bases.extend(capped());
    go(
        rep,
        M::ProductionOutput,
        bases,
        vec![
            n(Q::CapitalOutput, more_capital),
            n(Q::ResourceFactor, more_forest),
            n(Q::WeatherFactor, more_resilience),
            n(Q::TransitionFactor, |s| s.output_penalty.0 *= 0.5),
            n(Q::ElectricityServed, more_plant_capacity),
            n(Q::EmissionsCap, |s| {
                s.effective.emissions_cap.0 = s.effective.emissions_cap.0 * 1.2 + 0.5
            }),
            n(Q::GridIntensity, renewable_to_fossil),
        ],
        vec![r(Q::Output, firm_out)],
    );

    // inventory_stock: total firm inventory after the production system. Inventory: every firm's
    // stock up; Output: capital up (more capacity, so more output; output itself is computed).
    go(
        rep,
        M::InventoryStock,
        base(),
        vec![
            n(Q::Inventory, |s| {
                firms(s, |f| f.inventory.0 = f.inventory.0 * 1.2 + 1.0)
            }),
            n(Q::Output, more_capital),
        ],
        vec![r(Q::Inventory, |s| {
            after(SystemId::Production, s, p())
                .firms
                .iter()
                .filter(|f| f.active)
                .map(|f| f.inventory.0)
                .sum()
        })],
    );

    // electricity_demand: `energy.demand` after the system (firms' planned output x energy per
    // unit, plus households). Factors are raised through their leaves: capital up, forest up,
    // cumulative emissions down (the climate factor up), output penalty halved.
    go(
        rep,
        M::ElectricityDemand,
        base(),
        vec![
            n(Q::CapitalOutput, more_capital),
            n(Q::ResourceFactor, more_forest),
            n(Q::WeatherFactor, |s| s.nature.cumulative.0 *= 0.8),
            n(Q::TransitionFactor, |s| s.output_penalty.0 *= 0.5),
        ],
        vec![r(Q::EnergyDemand, |s| {
            after(SystemId::Production, s, p()).energy.demand.0
        })],
    );

    // electricity_rationing: `energy.served` (a share, supply over demand, at most 1) after the
    // system. Demand rises through every firm's capital; capacity through every plant. Served is a
    // share, so more demand lowers it.
    go(
        rep,
        M::ElectricityRationing,
        tight(),
        vec![
            n(Q::EnergyDemand, |s| firms(s, |f| f.capital.0 *= 1.3)),
            n(Q::PlantCapacity, more_plant_capacity),
        ],
        vec![r(Q::ElectricityServed, |s| {
            after(SystemId::Production, s, p()).energy.served.0
        })],
    );

    // plant_emissions_output: tonnes the plants emit after the system. EnergyDemand: firm capital
    // up; RenewableShare: one fossil plant becomes renewable (same capacity).
    go(
        rep,
        M::PlantEmissionsOutput,
        base(),
        vec![
            n(Q::EnergyDemand, |s| firms(s, |f| f.capital.0 *= 1.3)),
            n(Q::RenewableShare, |s| {
                if let Some(pl) = s.plants.iter_mut().find(|x| x.kind == PlantKind::Fossil) {
                    pl.kind = PlantKind::Renewable;
                }
            }),
        ],
        vec![r(Q::PlantEmissions, |s| {
            after(SystemId::Production, s, p())
                .plants
                .iter()
                .map(|x| x.emissions.0)
                .sum()
        })],
    );

    // firm_emissions: direct emissions and footprint of the non-food firms after the system
    // (non-food, so a drought's effect on farm output does not confound the heat effect). Output:
    // capital up; GreenTech: green tech up; Drought: severity up; GridIntensity: a renewable plant
    // turned fossil.
    let mut fe_bases = base();
    fe_bases.extend(green_grid());
    go(
        rep,
        M::FirmEmissions,
        fe_bases,
        vec![
            n(Q::Output, more_capital),
            n(Q::GreenTech, |s| {
                firms(s, |f| f.green_tech.0 = (f.green_tech.0 + 0.1).min(0.95))
            }),
            n(Q::Drought, bump_drought),
            n(Q::GridIntensity, renewable_to_fossil),
        ],
        vec![
            r(Q::DirectEmissions, |s| {
                after(SystemId::Production, s, p())
                    .firms
                    .iter()
                    .filter(|f| f.active && f.sector != Sector::Food)
                    .map(|f| f.emissions.0)
                    .sum()
            }),
            r(Q::Footprint, |s| {
                after(SystemId::Production, s, p())
                    .firms
                    .iter()
                    .filter(|f| f.active && f.sector != Sector::Food)
                    .map(|f| f.footprint.0)
                    .sum()
            }),
        ],
    );
}
