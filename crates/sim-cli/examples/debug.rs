//! Debug printer: `cargo run -p sim-cli --example debug -- TURNS EVERY [plan.json]`
//! Prints households, firms and headline indicators every `EVERY` ticks. `HH=1` adds detail.
use sim_engine::Plan;
use sim_params::{ContentJson, Params};
use std::fs;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let turns: u32 = a.get(1).and_then(|x| x.parse().ok()).unwrap_or(8);
    let every: u32 = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(1);
    let d = "content/";
    let r = |n: &str| fs::read_to_string(format!("{d}{n}")).unwrap();
    let p = Params::from_content(&ContentJson {
        params: &r("params.json"),
        initial_town: &r("initial_town.json"),
        market: &r("systems/market.json"),
        regulated: &r("systems/regulated.json"),
        coop: &r("systems/coop.json"),
    })
    .unwrap();
    let script = match a.get(3) {
        Some(f) => Plan::from_json(&fs::read_to_string(f).unwrap()).unwrap(),
        None => Plan::default(),
    };
    let seed: u64 = std::env::var("SEED")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(42);
    let mut p = p;
    if let Ok(w) = std::env::var("WARM") {
        p.run.warmup_ticks = w.parse().unwrap();
    }
    let mut sim = sim_engine::init(p, seed).unwrap();
    let res = sim_engine::run(&mut sim, turns, &script, |s, out| {
        let st = &s.state;
        for e in &out.events {
            use sim_types::Event::*;
            if matches!(
                e,
                LoanDefaulted { .. }
                    | FirmClosed { .. }
                    | FirmFounded { .. }
                    | CommonsEnclosed { .. }
                    | CommonsCollapsed
                    | BankFailed
                    | Collapse { .. }
                    | BudgetExhausted
                    | ElectricityRationed { .. }
                    | HouseholdEvicted { .. }
                    | HouseholdMoved { .. }
                    | ClassChanged { .. }
            ) {
                println!("  EVENT t{} {:?}", st.tick, e);
            }
        }
        if st.tick % every != 0 {
            return;
        }
        let i = &st.indicators;
        let fs: Vec<String> = st
            .firms
            .iter()
            .filter(|x| x.active)
            .map(|x| {
                format!(
                    "[{:?} N{:.0} w{:.0} p{:.1} Y{:.0} inv{:.0} rev{:.1}k prof{:.1}k cash{:.1}k debt{:.1}k K{:.0}]",
                    x.sector, x.workers.0, x.wage.dollars(), x.price.dollars(), x.output.0, x.inventory.0,
                    x.revenue.dollars() / 1e3, x.profit.dollars() / 1e3, x.cash.dollars() / 1e3,
                    x.debt.dollars() / 1e3, x.capital.0
                )
            })
            .collect();
        if std::env::var("HH").is_ok() {
            for h in &st.households {
                let f = &h.income;
                println!(
                    "   {:<20} {:?} wages{:.0} div{:.0} rent+{:.0} int{:.0} ben{:.0} tax{:.0} rent-{:.0} loan{:.0} spend{:.0} disp{:.0} sav{:.1}k debt{:.1}k wealth{:.1}k wb{:.2} home{:?} arrears{}",
                    h.label, h.class, f.wages.dollars(), f.dividends.dollars(), f.rent_received.dollars(),
                    f.interest_received.dollars(), f.benefits.dollars(), f.tax_paid.dollars(), f.rent_paid.dollars(),
                    f.loan_payments.dollars(), f.spending.dollars(), h.last_disposable.dollars(),
                    h.savings.dollars() / 1e3, h.debt.dollars() / 1e3, h.wealth.dollars() / 1e3,
                    h.wellbeing.0, h.home.map(|x| x.0), h.arrears_months
                );
            }
            println!(
                "   gov cash {:.1}k debt {:.1}k rev {:.1}k spend {:.1}k energy served {:.2} demand {:.1} supply {:.1}",
                st.gov.cash.dollars() / 1e3, st.gov.debt.dollars() / 1e3, st.gov.tax_revenue.dollars() / 1e3,
                st.gov.spending.dollars() / 1e3, st.energy.served.0, st.energy.demand.0, st.energy.supply.0
            );
        }
        let hh_spend: f64 = st
            .households
            .iter()
            .map(|h| h.income.spending.dollars())
            .sum();
        let capex: f64 = st.firms.iter().map(|f| f.capex.dollars()).sum();
        let wages: f64 = st.households.iter().map(|h| h.income.wages.dollars()).sum();
        let div: f64 = st
            .households
            .iter()
            .map(|h| h.income.dividends.dollars())
            .sum();
        let sav: f64 = st.households.iter().map(|h| h.savings.dollars()).sum();
        println!(
            "   hh spend {:.1}k capex {:.1}k wages {:.1}k div {:.1}k hh savings {:.1}k gov cash {:.1}k firm cash {:.1}k",
            hh_spend / 1e3, capex / 1e3, wages / 1e3, div / 1e3, sav / 1e3, st.gov.cash.dollars() / 1e3,
            st.firms.iter().map(|f| f.cash.dollars()).sum::<f64>() / 1e3
        );
        println!(
            "t{:3} u{:4.1}% gini{:.3} top{:.2} gdp{:.0}k Em{:.1} legit{:.2} wb{:.2} rent{:.0} {}",
            st.tick,
            i.unemployment.0 * 100.0,
            i.gini.0,
            i.top_wealth_share.0,
            i.gdp_annual.dollars() / 1e3,
            st.nature.gross_emissions.0,
            st.gov.legitimacy.0,
            i.avg_wellbeing.0,
            st.housing.rent_private.dollars(),
            fs.join(" ")
        );
    });
    if let Err(e) = res {
        println!("ERR {e}");
    }
}
