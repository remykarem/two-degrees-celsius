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
    let mut sim = sim_engine::init(p, 42).unwrap();
    let res = sim_engine::run(&mut sim, turns, &script, |s, _| {
        let st = &s.state;
        if st.tick % every != 0 {
            return;
        }
        let f = &st.firms;
        let fs: Vec<String> = f.iter().filter(|x| x.active).map(|x| format!("[{:?} N{:.0} w{:.0} p{:.2} Y{:.0} inv{:.0} rev{:.0}k prof{:.0}k cash{:.0}k debt{:.0}k K{:.0}]", x.sector, x.workers.0, x.wage.dollars(), x.price.dollars(), x.output.0, x.inventory.0, x.revenue.dollars()/1e3, x.profit.dollars()/1e3, x.cash.dollars()/1e3, x.debt.dollars()/1e3, x.capital.0)).collect();
        let i = &st.indicators;
        let hs: Vec<String> = st
            .households
            .iter()
            .map(|h| format!("{:.0}k", h.savings.dollars() / 1e3))
            .collect();
        if std::env::var("HH").is_ok() {
            for h in &st.households {
                let i = &h.income;
                println!("   hh {} w{:.0} wages{:.0}k div{:.0}k rent{:.0}k int{:.0}k ben{:.0}k tax{:.0}k rentpaid{:.0}k loan{:.0}k spend{:.0}k lobby{:.0}k disp{:.0}k sav{:.0}k emp{:.0}/{:.0} wb{:.2} {:?}", h.label, h.weight.0, i.wages.dollars()/1e3, i.dividends.dollars()/1e3, i.rent_received.dollars()/1e3, i.interest_received.dollars()/1e3, i.benefits.dollars()/1e3, i.tax_paid.dollars()/1e3, i.rent_paid.dollars()/1e3, i.loan_payments.dollars()/1e3, i.spending.dollars()/1e3, i.lobby_spent.dollars()/1e3, h.last_disposable.dollars()/1e3, h.savings.dollars()/1e3, h.employed().0, h.labour_force().0, h.wellbeing.0, h.class);
            }
            println!("   gov cash {:.0}k purchases_budget {:.0}k rev {:.0}k spend {:.0}k  inv {:.0}k services {:.0}k", st.gov.cash.dollars()/1e3, st.gov.purchases_budget.dollars()/1e3, st.gov.tax_revenue.dollars()/1e3, st.gov.spending.dollars()/1e3, st.gdp_parts.investment.dollars()/1e3, st.gdp_parts.gov_services.dollars()/1e3);
        }
        println!("t{:3} u{:4.1}% gini{:.3} gdp{:.1}M real{:.1}M Em{:.0} legit{:.2} wb{:.2} eq{:.2}M gov{:.0}k/{:.1}M sav[{}] {}", st.tick, i.unemployment.0*100.0, i.gini.0, i.gdp_annual.dollars()/1e6, i.gdp_real_annual/1e6, st.nature.gross_emissions.0, st.gov.legitimacy.0, i.avg_wellbeing.0, st.bank.equity.dollars()/1e6, st.gov.cash.dollars()/1e3, st.gov.debt.dollars()/1e6, hs.join(" "), fs.join(" "));
    });
    if let Err(e) = res {
        println!("ERR {e}");
    }
}
