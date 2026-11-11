//! S2 production, electricity and emissions. Y = A K^a L^b with L the sum over workers of
//! skill x effective hours. Firms and households draw electricity; plants supply it by capacity
//! share and fossil plants emit. If demand exceeds supply, output is rationed in proportion.

use libm::pow;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Resource health of the farm: average of forest and commons relative to capacity.
pub fn resource_health(s: &State, p: &Params) -> f64 {
    sim_metrics::farm_health(s, p)
}

/// Share of farm output lost to climate damage: 0.3 * (CE/B0)^2.
pub fn climate_factor(s: &State, p: &Params) -> f64 {
    let ratio = (s.nature.cumulative.0 / p.nature.budget_b0).clamp(0.0, 1.5);
    1.0 - p.production.climate_damage * ratio * ratio
}

/// Output of `tfp x K^alpha x L^beta` for capital `k` and effective labour `l`.
pub fn cobb_douglas(p: &Params, tfp: f64, k: f64, l: f64) -> f64 {
    tfp * pow(k, p.production.alpha_k) * pow(l, p.production.beta_n)
}

/// Output the firm would produce with `l` effective labour, before rationing and caps.
pub fn capacity(s: &State, p: &Params, f: &Firm, l: f64) -> f64 {
    if l <= 0.0 || f.capital.0 <= 0.0 {
        return 0.0;
    }
    let mut y = cobb_douglas(p, f.tfp.0, f.capital.0, l);
    if f.sector == Sector::Food {
        y *= pow(
            resource_health(s, p).max(1e-6),
            p.production.farm_health_exp,
        );
        y *= climate_factor(s, p).max(0.0);
        // Drought (S11): the farm loses output while it lasts.
        y *= (1.0 - p.weather.a_f * crate::weather::effective_drought(s, p)).max(0.0);
        if let CommonsAccess::Community { .. } = s.nature.commons_access {
            y *= 1.0 - p.nature.community_coord_cost;
        }
    }
    // Smoke from a fire nearby costs every firm a little output (S11).
    if crate::weather::burning_near(s, f.tile, p.weather.smoke_radius) {
        y *= 1.0 - p.weather.smoke_productivity;
    }
    y * (1.0 - s.output_penalty.0).max(0.0)
}

/// Effective labour of a firm: the sum over its workers of skill x effective hours.
pub fn effective_labour(s: &State, f: &Firm) -> f64 {
    s.persons
        .iter()
        .filter(|x| x.job == Some(f.id))
        .map(|x| x.skill.0 * x.effective_hours.0)
        .sum()
}

/// Tonnes of CO2 per energy unit delivered: the fossil share of capacity times the fossil factor.
pub fn grid_intensity(s: &State, p: &Params) -> f64 {
    let supply: f64 = s.plants.iter().map(|x| x.capacity.0).sum();
    grid_intensity_of(p, s.capacity(PlantKind::Fossil), supply)
}

/// Grid intensity from the fossil capacity and the total capacity.
pub fn grid_intensity_of(p: &Params, fossil_capacity: f64, total_capacity: f64) -> f64 {
    sim_types::safe_div(fossil_capacity, total_capacity, 0.0) * p.energy.factor_fossil
}

/// Electricity each household would like this month (before rationing), from last month's
/// consumption; `heat` is the drought multiplier.
pub fn household_demand(s: &State, p: &Params, heat: f64) -> Vec<f64> {
    s.households
        .iter()
        .map(|h| household_energy_use(p, heat, h.members.len() as f64, h.consumption.dollars()))
        .collect()
}

/// Electricity one household would like per month: `heat * n * (base + k * c / ref)` with `n`
/// people spending `consumption_usd` a month (`c` per person).
pub fn household_energy_use(p: &Params, heat: f64, members: f64, consumption_usd: f64) -> f64 {
    let ep = &p.energy;
    let cons_pc = consumption_usd / members.max(1.0);
    heat * members
        * (ep.hh_energy_base_pc + ep.hh_energy_consumption_pc * cons_pc / ep.hh_ref_consumption_usd)
}

/// Direct CO2 per unit of output of a firm: its base intensity cut by green technology.
pub fn firm_intensity(intensity_base: f64, green_tech: f64) -> f64 {
    intensity_base * (1.0 - green_tech)
}

/// Electricity per unit of output of a firm: its base need cut by green technology, scaled by
/// the heat multiplier.
pub fn firm_energy_per_unit(energy_per_unit: f64, green_tech: f64, heat: f64) -> f64 {
    energy_per_unit * (1.0 - green_tech) * heat
}

/// Direct CO2 of a firm making `output` units.
pub fn firm_direct_emissions(output: f64, intensity_base: f64, green_tech: f64) -> f64 {
    output * firm_intensity(intensity_base, green_tech)
}

/// CO2 a plant emits delivering `delivered` energy units: only fossil plants emit.
pub fn plant_emissions(p: &Params, kind: PlantKind, delivered: f64) -> f64 {
    match kind {
        PlantKind::Fossil => delivered * p.energy.factor_fossil,
        PlantKind::Renewable => 0.0,
    }
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let gi = grid_intensity(s, p);
    // A drought raises electricity demand (cooling, pumping): S11.
    let heat = 1.0 + p.weather.a_e * crate::weather::effective_drought(s, p);

    // Electricity the households would like, from last month's consumption.
    let hh_use = household_demand(s, p, heat);

    // Production before rationing.
    struct Plan {
        id: FirmId,
        y0: f64,
        e_eff: f64,
        iota: f64,
    }
    let mut plans: Vec<Plan> = Vec::new();
    for f in s.firms.iter().filter(|f| f.active) {
        let y0 = capacity(s, p, f, effective_labour(s, f));
        plans.push(Plan {
            id: f.id,
            y0,
            e_eff: firm_energy_per_unit(f.energy_per_unit.0, f.green_tech.0, heat),
            iota: firm_intensity(f.intensity_base.0, f.green_tech.0),
        });
    }
    let firm_demand: f64 = plans.iter().map(|x| x.y0 * x.e_eff).sum();
    let demand = firm_demand + hh_use.iter().sum::<f64>();
    let supply: f64 = s.plants.iter().map(|x| x.capacity.0).sum();
    let served = if demand > supply && demand > 0.0 {
        supply / demand
    } else {
        1.0
    };
    if served < 0.999 {
        d.events.push(Event::ElectricityRationed {
            served: Rate(served),
        });
    }

    let mut firm_use = 0.0;
    for plan in &plans {
        let f = &s.firms[plan.id.idx()];
        let mut y = plan.y0 * served;
        // An emissions cap limits the firm's whole footprint: direct plus its electricity.
        let per_unit = plan.iota + plan.e_eff * gi;
        let cap = s.effective.emissions_cap.0;
        if cap > 0.0 && per_unit > 0.0 {
            y = y.min(cap / per_unit);
        }
        let direct = y * plan.iota;
        let energy = y * plan.e_eff;
        firm_use += energy;
        let growth = match f.sector {
            Sector::Goods => p.production.tfp_growth_goods,
            Sector::Food => p.production.tfp_growth_food,
            Sector::Services => p.production.tfp_growth_services,
        };
        let hold = if f.sector == Sector::Services {
            p.goods.services_holding_cost
        } else {
            p.goods.holding_cost
        };
        d.firm(
            f.id,
            FirmPatch {
                output: Some(Units(y)),
                emissions: Some(Tonnes(direct)),
                footprint: Some(Tonnes(direct + energy * gi)),
                energy_use: Some(Units(energy)),
                inventory: Some(Units(f.inventory.0 * (1.0 - hold) + y)),
                tfp: Some(Rate(f.tfp.0 * (1.0 + growth))),
                ..FirmPatch::default()
            },
        );
    }
    for f in s.firms.iter().filter(|f| !f.active) {
        d.firm(
            f.id,
            FirmPatch {
                output: Some(Units::ZERO),
                emissions: Some(Tonnes::ZERO),
                footprint: Some(Tonnes::ZERO),
                energy_use: Some(Units::ZERO),
                ..FirmPatch::default()
            },
        );
    }

    // Households use electricity too; they are served in the same proportion.
    let mut hh_total = 0.0;
    for (h, u) in s.households.iter().zip(&hh_use) {
        hh_total += u * served;
        d.household(
            h.id,
            HouseholdPatch {
                energy_use: Some(Units(u * served)),
                ..HouseholdPatch::default()
            },
        );
    }

    // Plants deliver by capacity share; only fossil plants emit.
    let delivered = firm_use + hh_total;
    for pl in &s.plants {
        let out = sim_types::safe_div(delivered * pl.capacity.0, supply, 0.0);
        let em = plant_emissions(p, pl.kind, out);
        d.changes.push(Delta::PlantOutput {
            id: pl.id,
            output: Units(out),
            emissions: Tonnes(em),
        });
    }
    d.changes.push(Delta::Energy(Energy {
        demand: Units(demand),
        supply: Units(supply),
        delivered: Units(delivered),
        served: Rate(served),
        grid_intensity: Rate(gi),
    }));
    d
}
