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
    // Town v7: the estate's jet is an asset of its owner.
    if let Some(e) = &s.estate {
        w[e.jet.owner.idx()] += e.jet.value.as_f64();
    }
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

/// What a household's net worth is made of, in cents as f64 (Town v7: the wire's wealth
/// breakdown). `net_cash + homes + bank + jet + firms` equals `household_wealth` up to float
/// rounding (a test checks it). Town v8 splits these further into [`WealthLines`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WealthParts {
    /// Savings minus debt.
    pub net_cash: f64,
    /// Homes owned (the occupied one included).
    pub homes: f64,
    /// Bank equity (the bank's owner).
    pub bank: f64,
    /// The estate's jet.
    pub jet: f64,
    /// Equity of the firms the household owns or shares in.
    pub firms: f64,
}

impl WealthParts {
    pub fn total(&self) -> f64 {
        self.net_cash + self.homes + self.bank + self.jet + self.firms
    }
}

/// A household's net worth as a statement (Town v8): every asset and every liability on its own
/// line, in cents as f64. `assets() - liabilities()` equals [`household_wealth`] up to float
/// rounding (a test checks it for every household). Liabilities are positive magnitudes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WealthLines {
    /// Deposits (the household's savings).
    pub savings: f64,
    /// Homes owned, at value.
    pub homes: f64,
    /// Capital of the firms it owns or shares in, at book value (units x capital price).
    pub firm_capital: f64,
    /// Cash held by those firms.
    pub firm_cash: f64,
    /// Unsold stock of those firms at the firm's price.
    pub firm_inventory: f64,
    /// Bank equity (the bank's owner holds it).
    pub bank_equity: f64,
    /// The estate's jet.
    pub jet: f64,
    /// Loans the household owes.
    pub loans: f64,
    /// Loans of those firms, in proportion to the share held.
    pub firm_loans: f64,
    /// How many homes are owned.
    pub homes_count: u32,
    /// How many firms are owned or shared in.
    pub firms_count: u32,
}

impl WealthLines {
    pub fn assets(&self) -> f64 {
        self.savings
            + self.homes
            + self.firm_capital
            + self.firm_cash
            + self.firm_inventory
            + self.bank_equity
            + self.jet
    }
    pub fn liabilities(&self) -> f64 {
        self.loans + self.firm_loans
    }
    pub fn net_worth(&self) -> f64 {
        self.assets() - self.liabilities()
    }
    fn add_firm(&mut self, f: &FirmBooks, share: f64) {
        self.firm_capital += f.capital * share;
        self.firm_cash += f.cash * share;
        self.firm_inventory += f.inventory * share;
        self.firm_loans += f.debt * share;
        self.firms_count += 1;
    }
}

/// An active firm's books in cents as f64: capital at book value, cash, stock at price, debt.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirmBooks {
    pub capital: f64,
    pub cash: f64,
    pub inventory: f64,
    pub debt: f64,
}

impl FirmBooks {
    pub fn equity(&self) -> f64 {
        self.capital + self.cash + self.inventory - self.debt
    }
}

/// The books of a firm (same figures [`household_wealth`] values it by).
pub fn firm_books(f: &sim_types::Firm, p: &Params) -> FirmBooks {
    FirmBooks {
        capital: f.capital.0 * (p.invest.capital_price_usd * 100.0),
        cash: f.cash.as_f64(),
        inventory: f.inventory.0 * f.price.as_f64(),
        debt: f.debt.as_f64(),
    }
}

/// The statement of every household's net worth (same rules as [`household_wealth`]).
pub fn household_wealth_lines(s: &State, p: &Params) -> Vec<WealthLines> {
    let mut w = vec![WealthLines::default(); s.households.len()];
    for (i, h) in s.households.iter().enumerate() {
        w[i].savings = h.savings.as_f64();
        w[i].loans = h.debt.as_f64();
    }
    for home in &s.homes {
        if let HomeOwner::Household(o) = home.owner {
            w[o.idx()].homes += home.value.as_f64();
            w[o.idx()].homes_count += 1;
        }
    }
    w[s.bank.owner.idx()].bank_equity += s.bank.equity.as_f64();
    if let Some(e) = &s.estate {
        w[e.jet.owner.idx()].jet += e.jet.value.as_f64();
    }
    for f in s.firms.iter().filter(|f| f.active) {
        let books = firm_books(f, p);
        match f.owner {
            Owner::Household(h) => w[h.idx()].add_firm(&books, 1.0),
            Owner::Bank => w[s.bank.owner.idx()].add_firm(&books, 1.0),
            Owner::State => {}
            Owner::Workers => {
                let total = s.persons.iter().filter(|x| x.job == Some(f.id)).count() as f64;
                if total > 0.0 {
                    let mut seen: Vec<usize> = Vec::new();
                    for x in s.persons.iter().filter(|x| x.job == Some(f.id)) {
                        let i = x.household.idx();
                        w[i].add_firm(&books, 1.0 / total);
                        // One firm counts once per household, however many of its members work there.
                        if seen.contains(&i) {
                            w[i].firms_count -= 1;
                        } else {
                            seen.push(i);
                        }
                    }
                }
            }
        }
    }
    w
}

/// The parts of every household's net worth (same rules as [`household_wealth`]).
pub fn household_wealth_parts(s: &State, p: &Params) -> Vec<WealthParts> {
    household_wealth_lines(s, p)
        .iter()
        .map(|l| WealthParts {
            net_cash: l.savings - l.loans,
            homes: l.homes,
            bank: l.bank_equity,
            jet: l.jet,
            firms: l.firm_capital + l.firm_cash + l.firm_inventory - l.firm_loans,
        })
        .collect()
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

/// Carbon budget left: the budget minus the cumulative net emissions.
pub fn remaining_budget(cumulative: Tonnes, p: &Params) -> Tonnes {
    Tonnes(p.nature.budget_b0 - cumulative.0)
}

/// Government debt over annual GDP, 0 when GDP is not positive.
pub fn debt_to_gdp(debt_usd: f64, gdp_annual_usd: f64) -> f64 {
    if gdp_annual_usd > 0.0 {
        debt_usd / gdp_annual_usd
    } else {
        0.0
    }
}

/// Warming estimate: hits the limit exactly when the budget is gone.
pub fn temperature(cumulative: Tonnes, p: &Params) -> f64 {
    p.nature.t0 + (p.nature.t_limit - p.nature.t0) * cumulative.0 / p.nature.budget_b0
}

/// Climate stress 0..1 (docs/causal-model.md section 11): 0 until the carbon budget remaining
/// falls to `onset` of `budget_b0`, 1 at `full`, rising along `curve` in between.
pub fn climate_stress(cumulative: Tonnes, p: &Params) -> f64 {
    let w = &p.weather;
    if let Some(f) = w.force_stress {
        return f.clamp(0.0, 1.0);
    }
    let remaining = 1.0 - cumulative.0 / p.nature.budget_b0;
    let x = ((w.onset - remaining) / (w.onset - w.full)).clamp(0.0, 1.0);
    libm::pow(x, w.curve)
}

/// Forest stock over its maximum, 0..1.
pub fn forest_health(s: &State, p: &Params) -> f64 {
    (s.nature.forest.0 / p.nature.forest_max).clamp(0.0, 1.0)
}

/// Commons stock over its maximum, 0..1.
pub fn commons_health(s: &State, p: &Params) -> f64 {
    (s.nature.commons.0 / p.nature.commons_max).clamp(0.0, 1.0)
}

/// Resource health of the farm: the mean of forest and commons health. Production raises
/// this to `farm_health_exp` to scale the farm's output.
pub fn farm_health(s: &State, p: &Params) -> f64 {
    (0.5 * (forest_health(s, p) + commons_health(s, p))).clamp(0.0, 1.0)
}

/// Sales of all active firms this month.
pub fn firm_sales(s: &State) -> Cents {
    Cents::sum(s.firms.iter().filter(|f| f.active).map(|f| f.revenue))
}

/// GDP per month: firm sales plus investment plus government services.
pub fn gdp_month(s: &State) -> Cents {
    firm_sales(s) + s.gdp_parts.investment + s.gdp_parts.gov_services
}

/// Genuine progress indicator per year: GDP times one minus the Gini, less the social cost of
/// carbon on gross emissions and the cost of depleted natural capital (both annualised).
pub fn gpi_annual(
    p: &Params,
    gdp_annual_usd: f64,
    gini: f64,
    gross_emissions_monthly: f64,
    depletion_monthly: f64,
) -> f64 {
    gdp_annual_usd * (1.0 - gini.min(1.0))
        - p.metrics.scc_usd * 12.0 * gross_emissions_monthly
        - p.metrics.depletion_usd_per_unit * 12.0 * depletion_monthly
}

/// Compute all derived indicators from the current state.
pub fn compute(s: &State, p: &Params, base_prices: &[f64]) -> Indicators {
    let sales_value = firm_sales(s);
    let gdp_month = gdp_month(s);
    let gdp_annual = gdp_month.scale(12.0);
    let real_sales: f64 = s
        .firms
        .iter()
        .filter(|f| f.active)
        .map(|f| f.sales_units.0 * base_prices[f.id.idx()] * 100.0)
        .sum();
    let price_index = safe_div(sales_value.as_f64(), real_sales, 1.0).max(1e-6);
    let (employed, labour_force, unemp) = unemployment(s);
    let prev = s.indicators.unemployment_smooth.0;
    let a = p.metrics.unemployment_smoothing;
    let smooth = if s.tick == 0 && prev == 0.0 {
        unemp.0
    } else {
        (1.0 - a) * prev + a * unemp.0
    };
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
    let gpi = gpi_annual(
        p,
        annual_usd,
        g,
        s.nature.gross_emissions.0,
        s.nature.depletion.0,
    );
    Indicators {
        gdp_month,
        gdp_annual,
        gdp_real_annual: annual_usd / price_index,
        unemployment: unemp,
        unemployment_smooth: Rate(smooth),
        employed,
        labour_force,
        gini: Rate(g),
        avg_income_pp: avg_income,
        median_income_pp: median(&incomes),
        top_wealth_share: Rate(top_share(&wealth)),
        wage_share: Rate(safe_div(wages, gdp_f, 0.0)),
        profit_share: Rate(safe_div(profits, gdp_f, 0.0)),
        avg_wellbeing: Rate(wb),
        remaining_budget: remaining_budget(s.nature.cumulative, p),
        temperature: temperature(s.nature.cumulative, p),
        gpi_annual: gpi,
        money: s.total_deposits(),
        price_index,
        forest_health: forest_health(s, p),
        commons_health: commons_health(s, p),
        farm_health: farm_health(s, p),
        climate_stress: climate_stress(s.nature.cumulative, p),
    }
}

// ---- Markets view (contract v2.3): pure functions of the state ----

/// Average shelf price of a sector in cents: weighted by the stock on the shelf, or the plain
/// mean when nothing is in stock; 0 when the sector has no active firm. The goods step uses
/// the same function for the food price that sets household food budgets.
pub fn sector_price(s: &State, sector: Sector) -> f64 {
    let fs: Vec<&Firm> = s
        .firms
        .iter()
        .filter(|f| f.active && f.sector == sector)
        .collect();
    let q: f64 = fs.iter().map(|f| f.inventory.0).sum();
    if q > 0.0 {
        fs.iter()
            .map(|f| f.inventory.0 * f.price.as_f64())
            .sum::<f64>()
            / q
    } else if !fs.is_empty() {
        fs.iter().map(|f| f.price.as_f64()).sum::<f64>() / fs.len() as f64
    } else {
        0.0
    }
}

/// Mean monthly wage of the active firms in cents, weighted by their workers (plain mean when
/// nobody is employed); `None` with no active firm.
pub fn average_wage(s: &State) -> Option<f64> {
    let fs: Vec<&Firm> = s.firms.iter().filter(|f| f.active).collect();
    if fs.is_empty() {
        return None;
    }
    let w: f64 = fs.iter().map(|f| f.workers.0).sum();
    Some(if w > 0.0 {
        fs.iter()
            .map(|f| f.workers.0 * f.wage.as_f64())
            .sum::<f64>()
            / w
    } else {
        fs.iter().map(|f| f.wage.as_f64()).sum::<f64>() / fs.len() as f64
    })
}

/// Mean annual loan rate over the active firms (as a fraction); `None` with no active firm.
pub fn average_loan_rate(s: &State) -> Option<f64> {
    let fs: Vec<&Firm> = s.firms.iter().filter(|f| f.active).collect();
    (!fs.is_empty()).then(|| fs.iter().map(|f| f.loan_rate.0).sum::<f64>() / fs.len() as f64)
}

/// The labour market in people. `labour_force` = `firm_jobs` + `jobless`; `guarantee` of the
/// jobless are in the government job guarantee. Domestic staff are outside the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabourFigures {
    pub labour_force: u32,
    pub firm_jobs: u32,
    pub jobless: u32,
    pub guarantee: u32,
}

pub fn labour_figures(s: &State) -> LabourFigures {
    let pool = s
        .persons
        .iter()
        .filter(|x| x.in_labour_force && x.staff_of.is_none());
    let (mut labour_force, mut firm_jobs, mut guarantee) = (0, 0, 0);
    for x in pool {
        labour_force += 1;
        if x.job.is_some() {
            firm_jobs += 1;
        } else if x.jg {
            guarantee += 1;
        }
    }
    LabourFigures {
        labour_force,
        firm_jobs,
        jobless: labour_force - firm_jobs,
        guarantee,
    }
}

/// Homes and households. `homes` = `occupied` + `vacant`; `households` = `housed` + `unhoused`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HousingFigures {
    pub homes: u32,
    pub occupied: u32,
    pub vacant: u32,
    pub households: u32,
    pub housed: u32,
    pub unhoused: u32,
}

pub fn housing_figures(s: &State) -> HousingFigures {
    let homes = s.homes.len() as u32;
    let occupied = s.homes.iter().filter(|h| h.household.is_some()).count() as u32;
    let households = s.households.len() as u32;
    let housed = s.households.iter().filter(|h| h.home.is_some()).count() as u32;
    HousingFigures {
        homes,
        occupied,
        vacant: homes - occupied,
        households,
        housed,
        unhoused: households - housed,
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
