//! One row of the headline series per tick, formatted for CSV (no I/O here).

use crate::Sim;
use sim_types::*;

pub const CSV_HEADER: &str = "tick,year,gdp_annual,gdp_real_annual,unemployment,gini,emissions_gross_month,emissions_net_month,cumulative_emissions,remaining_budget,temperature,bank_equity,money_in_circulation,legitimacy,wage_share,profit_share,wellbeing,gpi_annual,price_index,gov_debt,em_goods,em_food,em_households,forest,commons,system,employed,wage_goods,price_goods,inventory_goods,profit_goods,investment,gov_purchases,lent,top_savings";

/// Emissions by source this tick, read from the trace.
pub fn emissions_by_source(sim: &Sim, trace: &[TraceRecord]) -> (f64, f64, f64) {
    let (mut goods, mut food, mut hh) = (0.0, 0.0, 0.0);
    for t in trace {
        if let TraceRecord::Emissions { source, tonnes } = t {
            match source {
                EmissionSource::Firm(f) => match sim.state.firms[f.idx()].sector {
                    Sector::Goods => goods += tonnes.0,
                    Sector::Food => food += tonnes.0,
                },
                EmissionSource::Households => hh += tonnes.0,
                EmissionSource::Sink => {}
            }
        }
    }
    (goods, food, hh)
}

pub fn csv_row(sim: &Sim, trace: &[TraceRecord]) -> String {
    let s = &sim.state;
    let i = &s.indicators;
    let (g, f, h) = emissions_by_source(sim, trace);
    format!(
        "{},{:.3},{:.0},{:.0},{:.5},{:.5},{:.3},{:.3},{:.3},{:.3},{:.5},{:.0},{:.0},{:.5},{:.5},{:.5},{:.5},{:.0},{:.5},{:.0},{:.3},{:.3},{:.3},{:.1},{:.1},{:?},{:.1},{:.2},{:.3},{:.1},{:.0},{:.0},{:.0},{:.0},{:.0}",
        s.tick,
        s.tick as f64 / 12.0,
        i.gdp_annual.dollars(),
        i.gdp_real_annual,
        i.unemployment.0,
        i.gini.0,
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
        g,
        f,
        h,
        s.nature.forest.0,
        s.nature.commons.0,
        s.system,
        i.employed.0,
        s.firms[0].wage.dollars(),
        s.firms[0].price.dollars(),
        s.firms[0].inventory.0,
        s.firms[0].profit.dollars(),
        s.gdp_parts.investment.dollars(),
        s.gov.purchases_budget.dollars(),
        s.bank.lent_this_tick.dollars(),
        s.households[0].savings.dollars()
    )
}
