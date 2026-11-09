//! S10 wellbeing and household bookkeeping: wellbeing, per-capita wealth, disposable income.

use sim_metrics::cohort_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let wp = &p.wellbeing;
    let forest = (s.nature.forest.0 / p.nature.forest_max).clamp(0.0, 1.0);
    let pollution = (s.nature.gross_emissions.0 / wp.pollution_scale).clamp(0.0, 1.0);
    let environment = forest * (1.0 - pollution);
    let wealth = cohort_wealth(s, p);
    for (i, h) in s.households.iter().enumerate() {
        let w = h.weight.0.max(1e-9);
        let consumption = (h.consumption.dollars() / w / wp.needs_usd_pc).min(1.0);
        let employment = if h.labour_share.0 > 0.0 {
            (h.employed().0 / h.labour_force().0.max(1e-9)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let security =
            (h.savings.dollars() / w / (wp.security_months * wp.needs_usd_pc)).clamp(0.0, 1.0);
        let target = wp.w_consumption * consumption
            + wp.w_employment * employment
            + wp.w_housing * h.housed_share.0
            + wp.w_environment * environment
            + wp.w_security * security;
        let wb = (1.0 - wp.smoothing) * h.wellbeing.0 + wp.smoothing * target;
        d.household(
            h.id,
            HouseholdPatch {
                wellbeing: Some(Rate(wb.clamp(0.0, 1.0))),
                last_disposable: Some(h.income.disposable()),
                wealth_pc: Some(Cents::from_f64(wealth[i] / w)),
                ..HouseholdPatch::default()
            },
        );
    }
    d
}
