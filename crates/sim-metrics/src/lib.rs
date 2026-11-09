//! Derived indicators: wealth, Gini, GDP, GPI, carbon budget, temperature.
//! Pure functions of the state and parameters.
#![forbid(unsafe_code)]

use sim_params::Params;
use sim_types::*;

/// Net worth of each household cohort (total, not per capita), in cents as f64.
/// Savings, home value and rental housing, minus debt, plus firm equity and bank equity.
pub fn cohort_wealth(s: &State, p: &Params) -> Vec<f64> {
    let n = s.households.len();
    let mut w = vec![0.0; n];
    for (i, h) in s.households.iter().enumerate() {
        w[i] += (h.savings + h.home_value - h.debt).as_f64();
    }
    let housing_value = s.housing.private_cap.0 * p.invest.housing_cost_usd * 100.0;
    w[s.housing.owner.idx()] += housing_value;
    w[s.bank.owner.idx()] += s.bank.equity.as_f64();
    let kp = p.invest.capital_price_usd * 100.0;
    for f in s.firms.iter().filter(|f| f.active) {
        let equity =
            f.capital.0 * kp + f.cash.as_f64() + f.inventory.0 * f.price.as_f64() - f.debt.as_f64();
        match f.owner {
            Owner::Household(h) => w[h.idx()] += equity,
            Owner::Bank => w[s.bank.owner.idx()] += equity,
            Owner::State => {}
            Owner::Workers => {
                let total: f64 = s
                    .households
                    .iter()
                    .map(|h| h.employment[f.id.idx()].0)
                    .sum();
                if total > 0.0 {
                    for (i, h) in s.households.iter().enumerate() {
                        w[i] += equity * h.employment[f.id.idx()].0 / total;
                    }
                }
            }
        }
    }
    w
}

/// Gini over person-weighted wealth. Cohorts are groups of identical people.
pub fn gini(weights: &[f64], totals: &[f64]) -> f64 {
    let total_pop: f64 = weights.iter().sum();
    let total_wealth: f64 = totals.iter().sum();
    if total_pop <= 0.0 || total_wealth <= 0.0 {
        return 0.0;
    }
    let pc: Vec<f64> = weights
        .iter()
        .zip(totals)
        .map(|(w, t)| safe_div(*t, *w, 0.0))
        .collect();
    let mean = total_wealth / total_pop;
    let mut acc = 0.0;
    for i in 0..pc.len() {
        for j in 0..pc.len() {
            acc += weights[i] * weights[j] * (pc[i] - pc[j]).abs();
        }
    }
    (acc / (2.0 * total_pop * total_pop * mean)).clamp(0.0, 1.0)
}

pub fn unemployment(s: &State) -> (People, People, Rate) {
    let lf: f64 = s.households.iter().map(|h| h.labour_force().0).sum();
    let emp: f64 = s.households.iter().map(|h| h.employed().0).sum();
    (
        People(emp),
        People(lf),
        Rate(((lf - emp) / lf.max(1e-9)).clamp(0.0, 1.0)),
    )
}

/// Warming estimate: hits the limit exactly when the budget is gone.
pub fn temperature(cumulative: Tonnes, p: &Params) -> f64 {
    p.nature.t0 + (p.nature.t_limit - p.nature.t0) * cumulative.0 / p.nature.budget_b0
}

/// Compute all derived indicators from the current state.
pub fn compute(s: &State, p: &Params, base_prices: &[f64]) -> Indicators {
    let sales_value: Cents = Cents::sum(s.firms.iter().filter(|f| f.active).map(|f| f.revenue));
    let gdp_month = sales_value + s.gdp_parts.investment + s.gdp_parts.gov_services;
    let gdp_annual = gdp_month.scale(12.0);
    let real_sales: f64 = s
        .firms
        .iter()
        .filter(|f| f.active)
        .map(|f| f.sales_units.0 * base_prices[f.id.idx()] * 100.0)
        .sum();
    let price_index = safe_div(sales_value.as_f64(), real_sales, 1.0).max(1e-6);
    let (employed, labour_force, unemp) = unemployment(s);
    let weights: Vec<f64> = s.households.iter().map(|h| h.weight.0).collect();
    let wealth = cohort_wealth(s, p);
    let g = gini(&weights, &wealth);
    let wages: f64 = s.households.iter().map(|h| h.income.wages.as_f64()).sum();
    let profits: f64 = s
        .firms
        .iter()
        .filter(|f| f.active)
        .map(|f| f.profit.as_f64().max(0.0))
        .sum();
    let gdp_f = gdp_month.as_f64();
    let pop: f64 = weights.iter().sum();
    let wb: f64 = s
        .households
        .iter()
        .map(|h| h.wellbeing.0 * h.weight.0)
        .sum::<f64>()
        / pop.max(1e-9);
    let annual_usd = gdp_annual.dollars();
    let gpi = annual_usd * (1.0 - g.min(1.0))
        - p.metrics.scc_usd * 12.0 * s.nature.gross_emissions.0
        - p.metrics.depletion_usd_per_unit * 12.0 * s.nature.depletion.0;
    Indicators {
        gdp_month,
        gdp_annual,
        gdp_real_annual: annual_usd / price_index,
        unemployment: unemp,
        employed,
        labour_force,
        gini: Rate(g),
        wage_share: Rate(safe_div(wages, gdp_f, 0.0)),
        profit_share: Rate(safe_div(profits, gdp_f, 0.0)),
        avg_wellbeing: Rate(wb),
        remaining_budget: Tonnes(p.nature.budget_b0 - s.nature.cumulative.0),
        temperature: temperature(s.nature.cumulative, p),
        gpi_annual: gpi,
        money: s.total_deposits(),
        price_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gini_equal_is_zero() {
        assert!(gini(&[10.0, 10.0], &[100.0, 100.0]).abs() < 1e-12);
    }

    #[test]
    fn gini_unequal_is_positive() {
        let g = gini(&[50.0, 50.0], &[10.0, 90.0]);
        assert!((g - 0.4).abs() < 1e-9, "got {g}");
    }

    #[test]
    fn gini_zero_wealth_is_explicit() {
        assert_eq!(gini(&[1.0], &[0.0]), 0.0);
    }
}
