//! S10 wellbeing and household bookkeeping: person wellbeing, household wealth, disposable income.

use sim_metrics::household_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

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
            let target = wp.w_consumption * consumption
                + wp.w_employment * employment
                + wp.w_housing * housed
                + wp.w_environment * environment
                + wp.w_security * security;
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
