//! One `HistoryPoint` per completed turn, kept by the `Api` and saved with the game.
use serde::{Deserialize, Serialize};
use sim_params::Params;
use sim_types::State;

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub turn: u32,
    pub gdp_real_annual: f64,
    pub unemployment: f64,
    pub gini: f64,
    /// Net emissions (after the forest sink) in tonnes per month at the end of the turn.
    pub emissions: f64,
    pub budget_share: f64,
    pub legitimacy: f64,
    pub avg_wellbeing: f64,
    pub gpi_annual: f64,
    pub debt_to_gdp: f64,
    /// Mean monthly income per person, dollars.
    pub avg_income_per_person: f64,
    pub median_income_per_person: f64,
    pub top_household_wealth_share: f64,
    /// Contract v1.2: climate stress and the three health shares, 0..=1.
    pub climate_stress: f64,
    pub forest_health: f64,
    pub commons_health: f64,
    pub farm_health: f64,
    /// Contract v1.4: government revenue and spending over the turn (three months, dollars,
    /// from the ledger), the bank's capital ratio (equity over loans, 1 with no loans), the
    /// renewable share of plant capacity and the share of electricity demand served.
    pub gov_revenue: f64,
    pub gov_spending: f64,
    pub bank_capital_ratio: f64,
    pub renewable_share: f64,
    pub grid_served: f64,
    /// Contract v1.6: gross emissions (tonnes per month, before the sink) and the cumulative net
    /// emissions in tonnes, at the end of the turn.
    pub gross_emissions: f64,
    pub cumulative_emissions: f64,
    /// Contract v2.3 (Markets view): the price of each market at the end of the turn. Shelf
    /// prices in dollars per unit, the average monthly wage and the private rent in dollars, the
    /// average annual loan rate as a fraction. `None` when there was nothing to average (no
    /// active firm); a saved game from before v2.3 is refused (`SCHEMA_VERSION` 7), so the
    /// optional fields only keep hand-built histories loading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub price_food: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub price_goods: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub price_services: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wage_avg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rent_private: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub loan_rate_avg: Option<f64>,
}

/// Average shelf price of a sector in dollars; `None` when the sector has no active firm.
pub fn shelf_price(s: &State, sector: sim_types::Sector) -> Option<f64> {
    s.firms
        .iter()
        .any(|f| f.active && f.sector == sector)
        .then(|| sim_metrics::sector_price(s, sector) / 100.0)
}

/// Government debt over annual GDP, 0 when GDP is not positive.
pub fn debt_to_gdp(s: &State) -> f64 {
    sim_metrics::debt_to_gdp(s.gov.debt.dollars(), s.indicators.gdp_annual.dollars())
}

/// Share of the carbon budget left, clamped to 0..=1.
pub fn budget_share(s: &State, p: &Params) -> f64 {
    (s.indicators.remaining_budget.0 / p.nature.budget_b0).clamp(0.0, 1.0)
}

/// The point for the turn that has just been played (`played_turn`).
pub fn point(
    s: &State,
    p: &Params,
    played_turn: u32,
    gov_revenue: f64,
    gov_spending: f64,
) -> HistoryPoint {
    let i = &s.indicators;
    let (fossil, renewable) = (
        s.capacity(sim_types::PlantKind::Fossil),
        s.capacity(sim_types::PlantKind::Renewable),
    );
    HistoryPoint {
        turn: played_turn,
        gdp_real_annual: i.gdp_real_annual,
        unemployment: i.unemployment.0,
        gini: i.gini.0,
        emissions: s.nature.net_emissions.0,
        budget_share: budget_share(s, p),
        legitimacy: s.gov.legitimacy.0,
        avg_wellbeing: i.avg_wellbeing.0,
        gpi_annual: i.gpi_annual,
        debt_to_gdp: debt_to_gdp(s),
        avg_income_per_person: i.avg_income_pp.dollars(),
        median_income_per_person: i.median_income_pp.dollars(),
        top_household_wealth_share: i.top_wealth_share.0,
        climate_stress: i.climate_stress,
        forest_health: i.forest_health,
        commons_health: i.commons_health,
        farm_health: i.farm_health,
        gov_revenue,
        gov_spending,
        bank_capital_ratio: if s.bank.loans.0 > 0 {
            s.bank.equity.as_f64() / s.bank.loans.as_f64()
        } else {
            1.0
        },
        renewable_share: if fossil + renewable > 0.0 {
            renewable / (fossil + renewable) + 0.0
        } else {
            0.0
        },
        grid_served: s.energy.served.0,
        gross_emissions: s.nature.gross_emissions.0,
        cumulative_emissions: s.nature.cumulative.0,
        price_food: shelf_price(s, sim_types::Sector::Food),
        price_goods: shelf_price(s, sim_types::Sector::Goods),
        price_services: shelf_price(s, sim_types::Sector::Services),
        wage_avg: sim_metrics::average_wage(s).map(|c| c / 100.0),
        rent_private: Some(s.housing.rent_private.dollars()),
        loan_rate_avg: sim_metrics::average_loan_rate(s),
    }
}
