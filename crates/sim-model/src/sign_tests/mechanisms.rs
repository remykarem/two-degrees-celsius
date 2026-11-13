//! Probes of the mechanisms. Only mechanisms that call a pure function (or a small set of them) are
//! probed here, nudging the input through the real function; effects whose arithmetic lives inline in
//! a system's `run` over a whole `State` are marked `declared_unverified` in their spec (sim-model
//! cannot build a `State`). A probe that assembles several real helpers (taxes, energy demand)
//! says so. Params come from `content/params.json`.
use super::{Probe, Report};
use crate::causal::Node;
use crate::decision_tests::params;
use crate::production::{
    climate_factor_of, cobb_douglas, firm_direct_emissions, firm_energy_per_unit,
    grid_intensity_of, household_energy_use, plant_emissions,
};
use crate::util::{carbon_cost, profit_tax_due, tax_on, wealth_tax_monthly};
use crate::weather::drought_p;
use sim_metrics::{
    climate_stress, commons_drought_factor, commons_regrowth, food_cap_commons, food_cap_forest,
    forest_regrowth, gini,
};
use sim_params::Params;
use sim_types::{
    Cents, CommonsAccess, HouseholdId, MechanismId as M, PlantKind, QuantityId as Q, Rate, Tonnes,
};
use std::sync::OnceLock;

fn p() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(params)
}

fn m(id: M) -> Node {
    Node::Mechanism(id)
}

pub(super) fn probe_all(r: &mut Report) {
    production_technology(r);
    climate(r);
    energy(r);
    nature(r);
    food_cap(r);
    taxes(r);
    inequality(r);
}

fn production_technology(r: &mut Report) {
    // v = [technology (tfp), capital, effective labour]
    r.check(&Probe::<[f64; 3]> {
        node: m(M::ProductionTechnology),
        bases: vec![
            [1.0, 100.0, 50.0],
            [2.0, 10.0, 5.0],
            [0.5, 1000.0, 400.0],
            [1.2, 30.0, 80.0],
        ],
        nudges: vec![
            (Q::Technology, |v| v[0] *= 1.1),
            (Q::Capital, |v| v[1] *= 1.1),
            (Q::EffectiveLabour, |v| v[2] *= 1.1),
        ],
        reads: vec![(Q::CapitalOutput, |v| cobb_douglas(p(), v[0], v[1], v[2]))],
    });
}

fn climate(r: &mut Report) {
    let b0 = p().nature.budget_b0;
    let cums: Vec<[f64; 1]> = [0.0, 0.2, 0.4, 0.6, 0.8, 0.95, 1.5]
        .iter()
        .map(|f| [f * b0])
        .collect();
    // v = [cumulative net emissions, tonnes]; stress is flat (clamped) at 0 and 1.
    r.check(&Probe::<[f64; 1]> {
        node: m(M::ClimateStressCurve),
        bases: cums.clone(),
        nudges: vec![(Q::CumulativeEmissions, |v| v[0] = v[0] * 1.05 + 1.0)],
        reads: vec![(Q::ClimateStress, |v| climate_stress(Tonnes(v[0]), p()))],
    });
    r.check(&Probe::<[f64; 1]> {
        node: m(M::ClimateDamage),
        bases: cums,
        nudges: vec![(Q::CumulativeEmissions, |v| v[0] = v[0] * 1.05 + 1.0)],
        reads: vec![(Q::WeatherFactor, |v| climate_factor_of(p(), v[0]))],
    });
    // The monthly chance of a drought (the outcome is random and not probed).
    r.check(&Probe::<[f64; 1]> {
        node: m(M::DroughtOnset),
        bases: vec![[0.0], [0.1], [0.4], [0.8]],
        nudges: vec![(Q::ClimateStress, |v| v[0] = (v[0] + 0.1).min(1.0))],
        reads: vec![(Q::Drought, |v| drought_p(p(), v[0]))],
    });
}

fn energy(r: &mut Report) {
    // v = [renewable share, total capacity]; the fossil capacity is what is not renewable.
    r.check(&Probe::<[f64; 2]> {
        node: m(M::GridIntensityMix),
        bases: vec![[0.0, 100.0], [0.3, 100.0], [0.6, 250.0], [0.9, 40.0]],
        nudges: vec![(Q::RenewableShare, |v| v[0] = (v[0] + 0.05).min(1.0))],
        reads: vec![(Q::GridIntensity, |v| {
            grid_intensity_of(p(), (1.0 - v[0]) * v[1], v[1])
        })],
    });
    // v = [energy delivered by a fossil plant]
    r.check(&Probe::<[f64; 1]> {
        node: m(M::PlantEmissionsOutput),
        bases: vec![[0.0], [10.0], [500.0]],
        nudges: vec![(Q::ElectricityServed, |v| v[0] = v[0] * 1.1 + 1.0)],
        reads: vec![(Q::PlantEmissions, |v| {
            plant_emissions(p(), PlantKind::Fossil, v[0])
        })],
    });
    // v = [output, intensity base, green tech]
    r.check(&Probe::<[f64; 3]> {
        node: m(M::FirmEmissions),
        bases: vec![[100.0, 0.5, 0.0], [10.0, 2.0, 0.4], [1000.0, 0.1, 0.8]],
        nudges: vec![
            (Q::Output, |v| v[0] *= 1.1),
            (Q::GreenTech, |v| v[2] = (v[2] + 0.05).min(1.0)),
        ],
        reads: vec![(Q::DirectEmissions, |v| {
            firm_direct_emissions(v[0], v[1], v[2])
        })],
    });
    // Electricity demand is the households' use plus the firms' use. v = [heat (drought
    // multiplier), members, household consumption in dollars, green tech, units of firm output].
    r.check(&Probe::<[f64; 5]> {
        node: m(M::ElectricityDemand),
        bases: vec![
            [1.0, 3.0, 2000.0, 0.0, 100.0],
            [1.2, 4.0, 500.0, 0.3, 40.0],
            [1.1, 1.0, 100.0, 0.6, 10.0],
        ],
        nudges: vec![
            (Q::Drought, |v| v[0] *= 1.1),
            (Q::HouseholdSize, |v| v[1] += 1.0),
            (Q::Consumption, |v| v[2] = v[2] * 1.2 + 10.0),
            (Q::GreenTech, |v| v[3] = (v[3] + 0.05).min(1.0)),
        ],
        reads: vec![(Q::EnergyDemand, |v| {
            household_energy_use(p(), v[0], v[1], v[2])
                + v[4] * firm_energy_per_unit(0.8, v[3], v[0])
        })],
    });
}

fn nature(r: &mut Report) {
    let n = &p().nature;
    r.check(&Probe::<[f64; 1]> {
        node: m(M::ForestSinkUptake),
        bases: vec![[0.0], [50.0], [n.forest_max]],
        nudges: vec![(Q::ForestStock, |v| v[0] = v[0] * 1.1 + 1.0)],
        reads: vec![(Q::ForestSink, |v| crate::nature::forest_sink(p(), v[0]))],
    });
    // The output is next month's stock level (stock plus regrowth), not the net change.
    r.check(&Probe::<[f64; 1]> {
        node: m(M::ForestStockChange),
        bases: vec![
            [0.1 * n.forest_max],
            [0.5 * n.forest_max],
            [0.9 * n.forest_max],
        ],
        nudges: vec![(Q::ForestStock, |v| v[0] *= 1.05)],
        reads: vec![(Q::ForestStock, |v| v[0] + forest_regrowth(p(), v[0]))],
    });
    // v = [commons stock, drought severity, 1 when a community manages the commons]
    r.check(&Probe::<[f64; 3]> {
        node: m(M::CommonsStockChange),
        bases: vec![
            [0.2 * n.commons_max, 0.5, 0.0],
            [0.5 * n.commons_max, 0.0, 0.0],
            [0.8 * n.commons_max, 0.9, 0.0],
            [0.5 * n.commons_max, 0.5, 1.0],
        ],
        nudges: vec![
            (Q::CommonsStock, |v| v[0] *= 1.05),
            (Q::Drought, |v| v[1] = (v[1] + 0.1).min(1.0)),
            (Q::CommonsCommunity, |v| v[2] = 1.0),
        ],
        reads: vec![(Q::CommonsStock, |v| {
            let access = if v[2] > 0.5 {
                CommonsAccess::Community { cap: Rate(1.0) }
            } else {
                CommonsAccess::Open
            };
            v[0] + commons_regrowth(p(), v[0], commons_drought_factor(p(), access, v[1]))
        })],
    });
}

fn food_cap(r: &mut Report) {
    let n = &p().nature;
    // v = [forest stock, commons stock, access (0 open, 1 enclosed, 2 community), harvest lever].
    // The cap is the tighter of the two caps, as `sim_metrics::food_cap` combines them.
    r.check(&Probe::<[f64; 4]> {
        node: m(M::FoodCap),
        bases: vec![
            [0.9 * n.forest_max, 0.2 * n.commons_max, 0.0, 1.0],
            [0.1 * n.forest_max, 0.9 * n.commons_max, 0.0, 1.0],
            [0.9 * n.forest_max, 0.5 * n.commons_max, 1.0, 1.0],
            [0.5 * n.forest_max, 0.5 * n.commons_max, 2.0, 0.5],
            [0.9 * n.forest_max, 0.3 * n.commons_max, 0.0, 1.0],
        ],
        nudges: vec![
            (Q::ForestStock, |v| v[0] *= 1.1),
            (Q::CommonsStock, |v| v[1] *= 1.1),
            (Q::CommonsEnclosed, |v| v[2] = 1.0),
            (Q::CommonsCommunity, |v| v[2] = 2.0),
            (Q::CommonsHarvest, |v| v[3] *= 1.2),
        ],
        reads: vec![(Q::Output, |v| {
            let access = match v[2] as u8 {
                0 => CommonsAccess::Open,
                1 => CommonsAccess::Enclosed(HouseholdId(0)),
                _ => CommonsAccess::Community { cap: Rate(0.5) },
            };
            let c = food_cap_commons(p(), v[1], access, v[3]);
            let f = food_cap_forest(p(), v[0]);
            match (c, f) {
                (Some(a), Some(b)) => a.min(b),
                (a, b) => a.or(b).unwrap_or(f64::MAX / 4.0),
            }
        })],
    });
}

fn taxes(r: &mut Report) {
    // The treasury's take is the sum of the real helpers `government::run` and `income::run` call.
    // v = [wages (cents; a higher wage or more workers raise it), capital income (cents), net
    // wealth (cents), wage rate, capital rate, wealth rate, wealth threshold (dollars), carbon price
    // (cents), footprint (t), profit (cents), profit rate]
    r.check(&Probe::<[f64; 11]> {
        node: m(M::TaxCollection),
        bases: vec![
            [
                400_000.0,
                100_000.0,
                90_000_000.0,
                0.2,
                0.15,
                0.02,
                100_000.0,
                5_000.0,
                40.0,
                300_000.0,
                0.25,
            ],
            [
                1_000_000.0,
                50_000.0,
                500_000_000.0,
                0.1,
                0.3,
                0.05,
                1_000_000.0,
                0.0,
                5.0,
                0.0,
                0.3,
            ],
            [
                200_000.0,
                900_000.0,
                20_000_000_000.0,
                0.3,
                0.05,
                0.01,
                50_000.0,
                20_000.0,
                100.0,
                900_000.0,
                0.1,
            ],
        ],
        nudges: vec![
            (Q::Wage, |v| v[0] *= 1.2),
            (Q::Workers, |v| v[0] *= 1.2),
            (Q::CapitalIncome, |v| v[1] *= 1.2),
            (Q::NetWealth, |v| v[2] *= 1.2),
            (Q::WageTaxRate, |v| v[3] += 0.05),
            (Q::CapitalTaxRate, |v| v[4] += 0.05),
            (Q::WealthTaxRate, |v| v[5] += 0.01),
            (Q::WealthThreshold, |v| v[6] *= 1.2),
            (Q::CarbonPrice, |v| v[7] = v[7] * 1.2 + 100.0),
            (Q::Footprint, |v| v[8] *= 1.2),
            (Q::Profit, |v| v[9] *= 1.2),
            (Q::ProfitTaxRate, |v| v[10] += 0.05),
        ],
        reads: vec![(Q::TreasuryTaxes, |v| {
            (tax_on(Cents::from_f64(v[0]), v[3])
                + tax_on(Cents::from_f64(v[1]), v[4])
                + wealth_tax_monthly(v[2], Cents::from_dollars(v[6]), v[5])
                + carbon_cost(Cents::from_f64(v[7]), v[8])
                + profit_tax_due(Cents::from_f64(v[9]), v[10]))
            .as_f64()
        })],
    });
}

fn inequality(r: &mut Report) {
    // v = household wealths; the nudge adds to household 0. The Gini rises when it is the richest
    // and falls when it is the poorest, so both signs must show.
    r.check(&Probe::<Vec<f64>> {
        node: m(M::InequalityGini),
        bases: vec![
            vec![1.0, 100.0, 200.0, 300.0],
            vec![300.0, 100.0, 50.0, 10.0],
            vec![10.0, 12.0, 11.0, 9.0],
            vec![1000.0, 10.0, 10.0, 10.0],
        ],
        nudges: vec![(Q::NetWealth, |v| v[0] = v[0] * 1.5 + 1.0)],
        reads: vec![(Q::Inequality, |v| gini(v))],
    });
}
