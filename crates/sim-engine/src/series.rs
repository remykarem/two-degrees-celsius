//! One row of the headline series per tick, formatted for CSV (no I/O here).

use crate::Sim;
use sim_types::*;

pub const CSV_HEADER: &str = "tick,turn,pc,year,gdp_annual,gdp_real_annual,unemployment,gini,avg_income_pp,median_income_pp,top_wealth_share,emissions_gross_month,emissions_net_month,cumulative_emissions,remaining_budget,temperature,bank_equity,money_in_circulation,legitimacy,wage_share,profit_share,wellbeing,gpi_annual,price_index,gov_debt,em_firms,em_households,em_plants,forest,commons,system,employed,wage_goods,price_goods,inventory_goods,profit_goods,investment,gov_purchases,lent,top_savings,pending,gov_receipts,gov_spending,gov_cash,unhoused,energy_served,roads,climate_stress,burning,drought,farm_health";

/// Emissions by source this tick, read from the trace: firms (direct), households (direct), plants.
pub fn emissions_by_source(trace: &[TraceRecord]) -> (f64, f64, f64) {
    let (mut firms, mut hh, mut plants) = (0.0, 0.0, 0.0);
    for t in trace {
        if let TraceRecord::Emissions { source, tonnes } = t {
            match source {
                EmissionSource::Firm(_) => firms += tonnes.0,
                EmissionSource::Households => hh += tonnes.0,
                EmissionSource::Plant(_) => plants += tonnes.0,
                EmissionSource::Sink | EmissionSource::FireSinkLoss => {}
            }
        }
    }
    (firms, hh, plants)
}

pub fn csv_row(sim: &Sim, trace: &[TraceRecord]) -> String {
    let s = &sim.state;
    let i = &s.indicators;
    let (f, h, p) = emissions_by_source(trace);
    let roads = s
        .map
        .cells
        .iter()
        .filter(|c| c.kind == TileKind::Road)
        .count();
    format!(
        "{},{},{:.1},{:.3},{:.0},{:.0},{:.5},{:.5},{:.0},{:.0},{:.5},{:.3},{:.3},{:.3},{:.3},{:.5},{:.0},{:.0},{:.5},{:.5},{:.5},{:.5},{:.0},{:.5},{:.0},{:.3},{:.3},{:.3},{:.1},{:.1},{:?},{:.1},{:.2},{:.3},{:.1},{:.0},{:.0},{:.0},{:.0},{:.0},{},{:.0},{:.0},{:.0},{},{:.3},{},{:.4},{},{:.3},{:.4}",
        s.tick,
        s.turn,
        s.political_capital,
        s.tick as f64 / 12.0,
        i.gdp_annual.dollars(),
        i.gdp_real_annual,
        i.unemployment.0,
        i.gini.0,
        i.avg_income_pp.dollars(),
        i.median_income_pp.dollars(),
        i.top_wealth_share.0,
        s.nature.gross_emissions.0,
        s.nature.net_emissions.0,
        s.nature.cumulative.0,
        i.remaining_budget.0,
        i.temperature,
        s.bank.equity.dollars(),
        i.money.dollars(),
        s.gov.legitimacy.0,
        i.wage_share.0,
        i.profit_share.0,
        i.avg_wellbeing.0,
        i.gpi_annual,
        i.price_index,
        s.gov.debt.dollars(),
        f,
        h,
        p,
        s.nature.forest.0,
        s.nature.commons.0,
        s.system,
        i.employed.0,
        s.firms[0].wage.dollars(),
        s.firms[0].price.dollars(),
        s.firms[0].inventory.0,
        s.firms[0].profit.dollars(),
        s.gdp_parts.investment.dollars(),
        s.policy.public_spending.dollars(),
        s.bank.lent_this_tick.dollars(),
        s.households[0].savings.dollars(),
        s.pending.len(),
        s.gov.receipts.dollars(),
        s.gov.spending.dollars(),
        s.gov.cash.dollars(),
        s.housing.unhoused_households,
        s.energy.served.0,
        roads,
        i.climate_stress,
        s.weather.fires.len(),
        s.weather.drought.map_or(0.0, |d| d.severity),
        i.farm_health
    )
}
