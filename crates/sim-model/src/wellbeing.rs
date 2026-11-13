//! S10 wellbeing and household bookkeeping: person wellbeing, household wealth, disposable income.

use sim_metrics::household_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::Effect as Fx;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;

// ---- Mechanisms (causality graph stage 1): declarations of the plain code below; they change no behaviour. ----

/// Each person's wellbeing moves toward a weighted blend of consumption, a job, housing, a clean environment, savings, a short commute and clean local air (`wellbeing::run`).
pub const WELLBEING_SCORE: MechanismSpec = MechanismSpec {
    id: M::WellbeingScore,
    system: SystemId::Wellbeing,
    inputs: &[
        Q::Consumption,
        Q::Unemployment,
        Q::Occupancy,
        Q::ForestStock,
        Q::GrossEmissions,
        Q::Savings,
        Q::Fire,
    ],
    outputs: &[Q::Wellbeing],
    effects: &[
        Fx::up(Q::Consumption, Q::Wellbeing).system(),
        Fx::down(Q::Unemployment, Q::Wellbeing).system(),
        Fx::up(Q::Occupancy, Q::Wellbeing).system(),
        Fx::up(Q::ForestStock, Q::Wellbeing).system(),
        Fx::down(Q::GrossEmissions, Q::Wellbeing).system(),
        Fx::up(Q::Savings, Q::Wellbeing).system(),
        Fx::down(Q::Fire, Q::Wellbeing).system(),
    ],
    params: &[
        "wellbeing.needs_usd_pc",
        "wellbeing.w_consumption",
        "wellbeing.w_employment",
        "wellbeing.w_housing",
        "wellbeing.w_environment",
        "wellbeing.w_security",
        "wellbeing.security_months",
        "wellbeing.smoothing",
        "wellbeing.pollution_scale",
        "nature.forest_max",
        "wellbeing.w_commute",
        "wellbeing.commute_scale_minutes",
        "space.free_minutes",
        "wellbeing.w_local_pollution",
        "wellbeing.pollution_radius",
        "weather.smoke_wellbeing",
        "weather.smoke_radius",
    ],
    rule: "weighted_needs_blend",
};

/// Inequality is the Gini of wealth across people (`sim_metrics::gini`).
pub const INEQUALITY_GINI: MechanismSpec = MechanismSpec {
    id: M::InequalityGini,
    system: SystemId::Metrics,
    inputs: &[Q::NetWealth],
    outputs: &[Q::Inequality],
    effects: &[
        Fx::varies(Q::NetWealth, Q::Inequality, "the Gini rises if wealth is added to the already rich and falls if it is added to the poor, so total wealth alone does not set its direction"),
    ],
    params: &[],
    rule: "gini_of_wealth",
};

/// The household's disposable income is recorded each month for next month's spending and moving decisions (`wellbeing::run`).
pub const LAST_DISPOSABLE_RECORD: MechanismSpec = MechanismSpec {
    id: M::LastDisposableRecord,
    system: SystemId::Wellbeing,
    inputs: &[
        Q::Wage,
        Q::CapitalIncome,
        Q::Rent,
        Q::WageTaxRate,
        Q::CapitalTaxRate,
    ],
    outputs: &[Q::LastDisposable],
    effects: &[
        Fx::up(Q::Wage, Q::LastDisposable).system(),
        Fx::up(Q::CapitalIncome, Q::LastDisposable).system(),
        Fx::down(Q::Rent, Q::LastDisposable).system(),
        Fx::down(Q::WageTaxRate, Q::LastDisposable).unverified("wellbeing::run only copies the income record; the tax rates act through tax_paid, which another system writes, so no run of the wellbeing system responds to a rate"),
        Fx::down(Q::CapitalTaxRate, Q::LastDisposable).unverified("wellbeing::run only copies the income record; the tax rates act through tax_paid, which another system writes, so no run of the wellbeing system responds to a rate"),
    ],
    params: &[],
    rule: "income_less_taxes_and_rent",
};

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let wp = &p.wellbeing;
    let forest = (s.nature.forest.0 / p.nature.forest_max).clamp(0.0, 1.0);
    let pollution = (s.nature.gross_emissions.0 / wp.pollution_scale).clamp(0.0, 1.0);
    let environment = forest * (1.0 - pollution);
    let wealth = household_wealth(s, p);
    for (i, h) in s.households.iter().enumerate() {
        let size = (h.members.len() as f64).max(1.0);
        let consumption = (h.consumption.dollars() / size / wp.needs_usd_pc).min(1.0);
        let housed = if h.home.is_some() { 1.0 } else { 0.0 };
        // Without a home the sense of security is gone as well, whatever the savings.
        let security = if h.home.is_some() {
            (h.savings.dollars() / size / (wp.security_months * wp.needs_usd_pc)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // Local pollution: a home within a few tiles of a fossil plant or a goods factory.
        let local = match h.home.and_then(|id| s.home(id)) {
            Some(home) if wp.w_local_pollution > 0.0 => {
                let near = |t: Tile| home.tile.chebyshev(t) <= wp.pollution_radius;
                let plants = s
                    .plants
                    .iter()
                    .any(|pl| pl.kind == PlantKind::Fossil && near(pl.tile));
                let factory = s
                    .firms
                    .iter()
                    .any(|f| f.active && f.sector == Sector::Goods && near(f.tile));
                if plants || factory {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        // Smoke (S11): a home within `smoke_radius` tiles of a burning tile.
        let smoke = match h.home.and_then(|id| s.home(id)) {
            Some(home) if crate::weather::burning_near(s, home.tile, p.weather.smoke_radius) => 1.0,
            _ => 0.0,
        };
        let mut sum = 0.0;
        for m in &h.members {
            let x = s.person(*m);
            let employment = if x.in_labour_force {
                if x.employed() {
                    1.0
                } else {
                    0.0
                }
            } else {
                1.0
            };
            // Commute: time above the free minutes costs wellbeing, up to w_commute.
            let commute = (x.commute_minutes as f64 - p.space.free_minutes).max(0.0)
                / wp.commute_scale_minutes;
            let target = wp.w_consumption * consumption
                + wp.w_employment * employment
                + wp.w_housing * housed
                + wp.w_environment * environment
                + wp.w_security * security
                - wp.w_commute * commute.min(1.0)
                - wp.w_local_pollution * local
                - p.weather.smoke_wellbeing * smoke;
            let wb = ((1.0 - wp.smoothing) * x.wellbeing.0 + wp.smoothing * target).clamp(0.0, 1.0);
            sum += wb;
            d.changes.push(Delta::Person(
                x.id,
                PersonPatch {
                    wellbeing: Some(Rate(wb)),
                    ..PersonPatch::default()
                },
            ));
        }
        d.household(
            h.id,
            HouseholdPatch {
                wellbeing: Some(Rate(sum / size)),
                last_disposable: Some(h.income.disposable()),
                wealth: Some(Cents::from_f64(wealth[i])),
                ..HouseholdPatch::default()
            },
        );
    }
    d
}
