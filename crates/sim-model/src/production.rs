//! S2 production, electricity and emissions. Y = A K^a L^b with L the sum over workers of
//! skill x effective hours. Firms and households draw electricity; plants supply it by capacity
//! share and fossil plants emit. If demand exceeds supply, output is rationed in proportion.

use libm::pow;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::Effect as Fx;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;

/// Resource health of the farm: average of forest and commons relative to capacity.
pub fn resource_health(s: &State, p: &Params) -> f64 {
    sim_metrics::farm_health(s, p)
}

/// Share of farm output lost to climate damage: `climate_damage` * (CE/B0)^2, where the ratio
/// CE/B0 is capped at `climate_ratio_cap`.
pub fn climate_factor(s: &State, p: &Params) -> f64 {
    climate_factor_of(p, s.nature.cumulative.0)
}

/// `climate_factor` for a cumulative emissions total in tonnes (the same arithmetic, so the sign test can call it without a `State`).
pub fn climate_factor_of(p: &Params, cumulative: f64) -> f64 {
    let ratio = (cumulative / p.nature.budget_b0).clamp(0.0, p.production.climate_ratio_cap);
    1.0 - p.production.climate_damage * ratio * ratio
}

/// Electricity is rationed when the served share is below `energy.rationed_below`. The event and
/// the `rationed` flag on the wire both use this, so the web never repeats the threshold.
pub fn is_rationed(p: &Params, served: f64) -> bool {
    served < p.energy.rationed_below
}

/// Output of `tfp x K^alpha x L^beta` for capital `k` and effective labour `l`.
pub fn cobb_douglas(p: &Params, tfp: f64, k: f64, l: f64) -> f64 {
    tfp * pow(k, p.production.alpha_k) * pow(l, p.production.beta_n)
}

/// The factors `capacity` multiplies, kept apart so that a decision can read weather and
/// resources as separate quantities (`docs/features/decision-points.md` section 11). A factor that
/// does not apply to the firm is exactly 1.0, so the product is bit-identical to the old chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapacityFactors {
    /// Technology x capital^alpha x labour^beta (0 when there is no capital or labour).
    pub capital_output: f64,
    /// Farm health: forest and commons relative to capacity (food only).
    pub farm_health: f64,
    /// Climate damage (food only).
    pub climate: f64,
    /// Drought loss (food only).
    pub drought: f64,
    /// Coordination cost of a community-managed commons (food only).
    pub community: f64,
    /// Smoke from a fire nearby.
    pub smoke: f64,
    /// Output lost while the economic system changes.
    pub transition: f64,
}

impl CapacityFactors {
    /// The product, multiplied in the order the model always used.
    pub fn product(&self) -> f64 {
        let mut y = self.capital_output;
        y *= self.farm_health;
        y *= self.climate;
        y *= self.drought;
        y *= self.community;
        y *= self.smoke;
        y * self.transition
    }
}

/// The factors of a firm's output with `l` effective labour.
pub fn capacity_factors(s: &State, p: &Params, f: &Firm, l: f64) -> CapacityFactors {
    let mut c = CapacityFactors {
        capital_output: if l <= 0.0 || f.capital.0 <= 0.0 {
            0.0
        } else {
            cobb_douglas(p, f.tfp.0, f.capital.0, l)
        },
        farm_health: 1.0,
        climate: 1.0,
        drought: 1.0,
        community: 1.0,
        smoke: 1.0,
        transition: (1.0 - s.output_penalty.0).max(0.0),
    };
    if f.sector == Sector::Food {
        c.farm_health = pow(
            resource_health(s, p).max(crate::util::NUMERIC_FLOOR),
            p.production.farm_health_exp,
        );
        c.climate = climate_factor(s, p).max(0.0);
        // Drought (S11): the farm loses output while it lasts.
        c.drought = (1.0 - p.weather.a_f * crate::weather::effective_drought(s, p)).max(0.0);
        if let CommonsAccess::Community { .. } = s.nature.commons_access {
            c.community = 1.0 - p.nature.community_coord_cost;
        }
    }
    // Smoke from a fire nearby costs every firm a little output (S11).
    if crate::weather::burning_near(s, f.tile, p.weather.smoke_radius) {
        c.smoke = 1.0 - p.weather.smoke_productivity;
    }
    c
}

/// Output the firm would produce with `l` effective labour, before rationing and caps.
pub fn capacity(s: &State, p: &Params, f: &Firm, l: f64) -> f64 {
    if l <= 0.0 || f.capital.0 <= 0.0 {
        return 0.0;
    }
    capacity_factors(s, p, f, l).product()
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

// ---- Mechanisms (causality graph stage 1): declarations of the plain code below; they change no behaviour. ----

/// The share of electricity delivered is supply over demand, at most 1 (`production::run`).
pub const ELECTRICITY_RATIONING: MechanismSpec = MechanismSpec {
    id: M::ElectricityRationing,
    system: SystemId::Production,
    inputs: &[Q::EnergyDemand, Q::PlantCapacity],
    outputs: &[Q::ElectricityServed],
    effects: &[
        Fx::down(Q::EnergyDemand, Q::ElectricityServed).system(),
        Fx::up(Q::PlantCapacity, Q::ElectricityServed).system(),
    ],
    params: &[],
    rule: "served_share",
};

/// Tonnes of CO2 per energy unit: the fossil share of capacity times the fossil factor (`grid_intensity`).
pub const GRID_INTENSITY_MIX: MechanismSpec = MechanismSpec {
    id: M::GridIntensityMix,
    system: SystemId::Production,
    inputs: &[Q::RenewableShare],
    outputs: &[Q::GridIntensity],
    effects: &[Fx::down(Q::RenewableShare, Q::GridIntensity)],
    params: &["energy.factor_fossil"],
    rule: "fossil_share_times_factor",
};

/// Only fossil plants emit, per energy unit delivered (`plant_emissions`).
pub const PLANT_EMISSIONS_OUTPUT: MechanismSpec = MechanismSpec {
    id: M::PlantEmissionsOutput,
    system: SystemId::Production,
    inputs: &[Q::EnergyDemand, Q::ElectricityServed, Q::RenewableShare],
    outputs: &[Q::PlantEmissions],
    effects: &[
        Fx::up(Q::EnergyDemand, Q::PlantEmissions).system(),
        Fx::up(Q::ElectricityServed, Q::PlantEmissions),
        Fx::down(Q::RenewableShare, Q::PlantEmissions).system(),
    ],
    params: &["energy.factor_fossil"],
    rule: "fossil_plants_emit",
};

/// Output before the factors below: technology x capital^alpha x labour^beta (`cobb_douglas`).
pub const PRODUCTION_TECHNOLOGY: MechanismSpec = MechanismSpec {
    id: M::ProductionTechnology,
    system: SystemId::Production,
    inputs: &[Q::Capital, Q::EffectiveLabour, Q::Technology],
    outputs: &[Q::CapitalOutput],
    effects: &[
        Fx::up(Q::Capital, Q::CapitalOutput),
        Fx::up(Q::EffectiveLabour, Q::CapitalOutput),
        Fx::up(Q::Technology, Q::CapitalOutput),
    ],
    params: &["production.alpha_k", "production.beta_n"],
    rule: "cobb_douglas",
};

/// Technology grows a little each month at a rate that depends on the firm's sector (`run`).
pub const TECHNOLOGY_GROWTH: MechanismSpec = MechanismSpec {
    id: M::TechnologyGrowth,
    system: SystemId::Production,
    inputs: &[Q::Technology],
    outputs: &[Q::Technology],
    effects: &[Fx::up(Q::Technology, Q::Technology).system()],
    params: &[
        "production.tfp_growth_goods",
        "production.tfp_growth_food",
        "production.tfp_growth_services",
    ],
    rule: "technology_drift",
};

/// A firm's labour is the sum over its workers of skill x effective hours (`effective_labour`).
pub const EFFECTIVE_LABOUR_SUPPLY: MechanismSpec = MechanismSpec {
    id: M::EffectiveLabourSupply,
    system: SystemId::Production,
    inputs: &[Q::Workers, Q::SkilledWorkers],
    outputs: &[Q::EffectiveLabour],
    effects: &[
        Fx::up(Q::Workers, Q::EffectiveLabour).system(),
        Fx::up(Q::SkilledWorkers, Q::EffectiveLabour).system(),
    ],
    params: &[],
    rule: "skill_times_hours",
};

/// The farm's output is scaled by the health of the forest and commons, and by a coordination cost under community management (`capacity_factors`).
pub const FARM_HEALTH_FACTOR: MechanismSpec = MechanismSpec {
    id: M::FarmHealthFactor,
    system: SystemId::Production,
    inputs: &[Q::ForestStock, Q::CommonsStock, Q::CommonsCommunity],
    outputs: &[Q::ResourceFactor],
    effects: &[
        Fx::up(Q::ForestStock, Q::ResourceFactor).system(),
        Fx::up(Q::CommonsStock, Q::ResourceFactor).system(),
        Fx::none(Q::CommonsCommunity, Q::ResourceFactor, "the farm's health reads the forest and commons stocks; who controls access changes how they are used, not the health factor").system(),
    ],
    params: &[
        "production.farm_health_exp",
        "nature.community_coord_cost",
        "nature.forest_max",
        "nature.commons_max",
    ],
    rule: "farm_health_power",
};

/// Share of the farm's output lost to climate damage, from emissions so far over the carbon budget (`climate_factor`).
pub const CLIMATE_DAMAGE: MechanismSpec = MechanismSpec {
    id: M::ClimateDamage,
    system: SystemId::Production,
    inputs: &[Q::CumulativeEmissions],
    outputs: &[Q::WeatherFactor],
    effects: &[Fx::down(Q::CumulativeEmissions, Q::WeatherFactor)],
    params: &[
        "production.climate_damage",
        "production.climate_ratio_cap",
        "nature.budget_b0",
    ],
    rule: "climate_damage_share",
};

/// The farm loses output while a drought lasts, less where resilience spending cuts its effect (`effective_drought`).
pub const DROUGHT_LOSS: MechanismSpec = MechanismSpec {
    id: M::DroughtLoss,
    system: SystemId::Production,
    inputs: &[Q::Drought, Q::ResilienceSpending],
    outputs: &[Q::WeatherFactor],
    effects: &[
        Fx::down(Q::Drought, Q::WeatherFactor).system(),
        Fx::up(Q::ResilienceSpending, Q::WeatherFactor).system(),
    ],
    params: &[
        "weather.a_f",
        "weather.resilience_drought_cut_max",
        "weather.resilience_scale_usd",
    ],
    rule: "drought_output_loss",
};

/// Every firm near a fire loses a little output to smoke (`capacity_factors`).
pub const FIRE_SMOKE: MechanismSpec = MechanismSpec {
    id: M::FireSmoke,
    system: SystemId::Production,
    inputs: &[Q::Fire],
    outputs: &[Q::WeatherFactor],
    effects: &[Fx::down(Q::Fire, Q::WeatherFactor).system()],
    params: &["weather.smoke_productivity", "weather.smoke_radius"],
    rule: "smoke_output_loss",
};

/// Output is lower while the economic system changes; its size is in the system file (`capacity_factors`, `policy::run`).
pub const TRANSITION_LOSS: MechanismSpec = MechanismSpec {
    id: M::TransitionLoss,
    system: SystemId::Policy,
    inputs: &[Q::SystemChange],
    outputs: &[Q::TransitionFactor],
    effects: &[Fx::down(Q::SystemChange, Q::TransitionFactor).system()],
    params: &[],
    rule: "transition_output_penalty",
};

/// Output is capacity (the factors multiplied), cut by electricity rationing and by the emissions cap (`production::run`).
pub const PRODUCTION_OUTPUT: MechanismSpec = MechanismSpec {
    id: M::ProductionOutput,
    system: SystemId::Production,
    inputs: &[
        Q::CapitalOutput,
        Q::ResourceFactor,
        Q::WeatherFactor,
        Q::TransitionFactor,
        Q::ElectricityServed,
        Q::EmissionsCap,
        Q::GridIntensity,
    ],
    outputs: &[Q::Output],
    effects: &[
        Fx::up(Q::CapitalOutput, Q::Output).system(),
        Fx::up(Q::ResourceFactor, Q::Output).system(),
        Fx::up(Q::WeatherFactor, Q::Output).system(),
        Fx::up(Q::TransitionFactor, Q::Output).system(),
        Fx::up(Q::ElectricityServed, Q::Output).system(),
        Fx::varies(Q::EmissionsCap, Q::Output, "a cap of 0 means no cap, so the first tonne of cap tightens; above that a higher cap lets firms make more").system(),
        Fx::down(Q::GridIntensity, Q::Output).system(),
    ],
    params: &[],
    rule: "capacity_rationed_capped",
};

/// The food the farms can grow is capped by what the forest and the commons can supply (`sim_metrics::food_cap`).
pub const FOOD_CAP: MechanismSpec = MechanismSpec {
    id: M::FoodCap,
    system: SystemId::Production,
    inputs: &[
        Q::ForestStock,
        Q::CommonsStock,
        Q::CommonsOpen,
        Q::CommonsEnclosed,
        Q::CommonsCommunity,
        Q::CommonsHarvest,
    ],
    outputs: &[Q::Output],
    effects: &[
        Fx::up(Q::ForestStock, Q::Output),
        Fx::up(Q::CommonsStock, Q::Output),
        Fx::none(
            Q::CommonsOpen,
            Q::Output,
            "open access is the baseline the other two rules are measured against",
        )
        .unverified("open access is a category, not a quantity: there is nothing to nudge up"),
        Fx::up(Q::CommonsEnclosed, Q::Output),
        Fx::down(Q::CommonsCommunity, Q::Output),
        Fx::down(Q::CommonsHarvest, Q::Output),
    ],
    params: &[
        "nature.farm_cap_share_commons",
        "nature.farm_cap_share_forest",
        "nature.farm_harvest_commons",
        "nature.farm_harvest_forest",
        "nature.enclosure_harvest_multiplier",
        "nature.commons_growth",
        "nature.commons_max",
    ],
    rule: "food_limited_by_stocks",
};

/// Stock carried over loses a share each month and this month's output is added (`production::run`).
pub const INVENTORY_STOCK: MechanismSpec = MechanismSpec {
    id: M::InventoryStock,
    system: SystemId::Production,
    inputs: &[Q::Inventory, Q::Output],
    outputs: &[Q::Inventory],
    effects: &[
        Fx::up(Q::Inventory, Q::Inventory).system(),
        Fx::up(Q::Output, Q::Inventory).system(),
    ],
    params: &["goods.holding_cost", "goods.services_holding_cost"],
    rule: "stock_decay_plus_output",
};

/// Firms need electricity per unit made, households per person and per dollar spent; a drought raises both (`production::run`).
pub const ELECTRICITY_DEMAND: MechanismSpec = MechanismSpec {
    id: M::ElectricityDemand,
    system: SystemId::Production,
    inputs: &[
        Q::CapitalOutput,
        Q::ResourceFactor,
        Q::WeatherFactor,
        Q::TransitionFactor,
        Q::GreenTech,
        Q::HouseholdSize,
        Q::Consumption,
        Q::Drought,
    ],
    outputs: &[Q::EnergyDemand],
    effects: &[
        Fx::up(Q::CapitalOutput, Q::EnergyDemand).system(),
        Fx::up(Q::ResourceFactor, Q::EnergyDemand).system(),
        Fx::up(Q::WeatherFactor, Q::EnergyDemand).system(),
        Fx::up(Q::TransitionFactor, Q::EnergyDemand).system(),
        Fx::down(Q::GreenTech, Q::EnergyDemand),
        Fx::up(Q::HouseholdSize, Q::EnergyDemand),
        Fx::up(Q::Consumption, Q::EnergyDemand),
        Fx::up(Q::Drought, Q::EnergyDemand),
    ],
    params: &[
        "energy.hh_energy_base_pc",
        "energy.hh_energy_consumption_pc",
        "energy.hh_ref_consumption_usd",
        "weather.a_e",
    ],
    rule: "electricity_per_unit",
};

/// A firm's direct emissions are its output times its intensity, cut by green technology; its footprint adds the electricity it uses (`firm_direct_emissions`).
pub const FIRM_EMISSIONS: MechanismSpec = MechanismSpec {
    id: M::FirmEmissions,
    system: SystemId::Production,
    inputs: &[Q::Output, Q::GreenTech, Q::Drought, Q::GridIntensity],
    outputs: &[Q::DirectEmissions, Q::Footprint],
    effects: &[
        Fx::up(Q::Output, Q::DirectEmissions),
        Fx::up(Q::Output, Q::Footprint).system(),
        Fx::down(Q::GreenTech, Q::DirectEmissions),
        Fx::down(Q::GreenTech, Q::Footprint).system(),
        Fx::none(Q::Drought, Q::DirectEmissions, "a drought raises the electricity a firm needs, not its direct emissions").system(),
        Fx::up(Q::Drought, Q::Footprint).system(),
        Fx::none(Q::GridIntensity, Q::DirectEmissions, "the grid's emissions are the plants' and are counted in the footprint, not in a firm's direct emissions").system(),
        Fx::up(Q::GridIntensity, Q::Footprint).system(),
    ],
    params: &[],
    rule: "intensity_times_output",
};

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
    // Farm-from-nature: the stocks left last month limit the food that can be grown. The cap is
    // shared by the active farms in proportion to what each would make.
    let food_total: f64 = s
        .firms
        .iter()
        .filter(|f| f.active && f.sector == Sector::Food)
        .map(|f| capacity(s, p, f, effective_labour(s, f)))
        .sum();
    let food_scale = match sim_metrics::food_cap(s, p) {
        Some(cap) if food_total > cap => cap.max(0.0) / food_total,
        _ => 1.0,
    };
    for f in s.firms.iter().filter(|f| f.active) {
        let mut y0 = capacity(s, p, f, effective_labour(s, f));
        if f.sector == Sector::Food {
            y0 *= food_scale;
        }
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
    if is_rationed(p, served) {
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
