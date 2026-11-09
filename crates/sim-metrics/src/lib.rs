//! Derived indicators: wealth, Gini, GDP, GPI, carbon budget, temperature.
//! Pure functions of the state and parameters.
#![forbid(unsafe_code)]

use sim_params::Params;
use sim_types::*;

/// Net worth of each household (total, not per person), in cents as f64.
/// Savings, homes owned, firm and bank equity, minus debt.
pub fn household_wealth(s: &State, p: &Params) -> Vec<f64> {
    let n = s.households.len();
    let mut w = vec![0.0; n];
    for (i, h) in s.households.iter().enumerate() {
        w[i] += (h.savings - h.debt).as_f64();
    }
    for home in &s.homes {
        if let HomeOwner::Household(o) = home.owner {
            w[o.idx()] += home.value.as_f64();
        }
    }
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
                let total = s.persons.iter().filter(|x| x.job == Some(f.id)).count() as f64;
                if total > 0.0 {
                    for x in s.persons.iter().filter(|x| x.job == Some(f.id)) {
                        w[x.household.idx()] += equity / total;
                    }
                }
            }
        }
    }
    w
}

/// Gini of a list of values (wealth of each person). Zero for an empty list or a non-positive
/// total, and clamped to 0..1 (debt can push the raw formula above 1).
pub fn gini(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let total: f64 = values.iter().sum();
    if values.is_empty() || total <= 0.0 {
        return 0.0;
    }
    let mut acc = 0.0;
    for a in values {
        for b in values {
            acc += (a - b).abs();
        }
    }
    (acc / (2.0 * n * total)).clamp(0.0, 1.0)
}

/// Wealth of each person: the household's wealth shared equally among its members.
pub fn person_wealth(s: &State, household_wealth: &[f64]) -> Vec<f64> {
    s.persons
        .iter()
        .map(|x| {
            let h = &s.households[x.household.idx()];
            safe_div(
                household_wealth[x.household.idx()],
                h.members.len() as f64,
                0.0,
            )
        })
        .collect()
}

/// Monthly gross income of each person: the household's gross income shared equally.
pub fn person_incomes(s: &State) -> Vec<Cents> {
    s.persons
        .iter()
        .map(|x| {
            let h = &s.households[x.household.idx()];
            Cents::from_f64(safe_div(
                h.income.gross_income().as_f64(),
                h.members.len() as f64,
                0.0,
            ))
        })
        .collect()
}

pub fn median(v: &[Cents]) -> Cents {
    if v.is_empty() {
        return Cents::ZERO;
    }
    let mut x = v.to_vec();
    x.sort();
    let m = x.len() / 2;
    if x.len() % 2 == 1 {
        x[m]
    } else {
        Cents((x[m - 1].0 + x[m].0) / 2)
    }
}

/// Wealthiest household's wealth over the positive wealth of all households.
pub fn top_share(household_wealth: &[f64]) -> f64 {
    let total: f64 = household_wealth.iter().map(|x| x.max(0.0)).sum();
    let top = household_wealth.iter().cloned().fold(0.0, f64::max);
    safe_div(top, total, 0.0)
}

/// Employed, labour force and the unemployment rate (labour-force persons without a job).
pub fn unemployment(s: &State) -> (People, People, Rate) {
    let (emp, lf) = s.labour_force();
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
    let wealth = household_wealth(s, p);
    let g = gini(&person_wealth(s, &wealth));
    let incomes = person_incomes(s);
    let avg_income =
        Cents::from_f64(Cents::sum(incomes.iter().copied()).as_f64() / incomes.len().max(1) as f64);
    let wages: f64 = s.households.iter().map(|h| h.income.wages.as_f64()).sum();
    let profits: f64 = s
        .firms
        .iter()
        .filter(|f| f.active)
        .map(|f| f.profit.as_f64().max(0.0))
        .sum();
    let gdp_f = gdp_month.as_f64();
    let wb: f64 =
        s.persons.iter().map(|x| x.wellbeing.0).sum::<f64>() / (s.persons.len() as f64).max(1.0);
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
        avg_income_pp: avg_income,
        median_income_pp: median(&incomes),
        top_wealth_share: Rate(top_share(&wealth)),
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
        assert!(gini(&[10.0, 10.0, 10.0]).abs() < 1e-12);
    }

    #[test]
    fn gini_unequal_is_positive() {
        let g = gini(&[10.0, 90.0]);
        assert!((g - 0.4).abs() < 1e-9, "got {g}");
    }

    #[test]
    fn gini_zero_wealth_is_explicit() {
        assert_eq!(gini(&[0.0]), 0.0);
        assert_eq!(gini(&[]), 0.0);
    }

    #[test]
    fn median_of_even_and_odd() {
        assert_eq!(median(&[Cents(5), Cents(1), Cents(3)]), Cents(3));
        assert_eq!(median(&[Cents(4), Cents(2)]), Cents(3));
    }

    #[test]
    fn top_share_uses_positive_wealth() {
        assert!((top_share(&[60.0, 40.0, -10.0]) - 0.6).abs() < 1e-12);
    }
}
