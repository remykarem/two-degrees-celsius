use sim_engine::Script;
use sim_params::{ContentJson, Params};
use std::fs;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ticks: u32 = a.get(1).and_then(|x| x.parse().ok()).unwrap_or(24);
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
        Some(f) => Script::from_json(&fs::read_to_string(f).unwrap()).unwrap(),
        None => Script::default(),
    };
    let mut sim = sim_engine::init(p, 42).unwrap();
    let res = sim_engine::run(&mut sim, ticks, &script, |s, _| {
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
        println!("t{:3} u{:4.1}% gini{:.3} gdp{:.1}M real{:.1}M Em{:.0} legit{:.2} wb{:.2} eq{:.2}M gov{:.0}k/{:.1}M sav[{}] {}", st.tick, i.unemployment.0*100.0, i.gini.0, i.gdp_annual.dollars()/1e6, i.gdp_real_annual/1e6, st.nature.gross_emissions.0, st.gov.legitimacy.0, i.avg_wellbeing.0, st.bank.equity.dollars()/1e6, st.gov.cash.dollars()/1e3, st.gov.debt.dollars()/1e6, hs.join(" "), fs.join(" "));
    });
    if let Err(e) = res {
        println!("ERR {e}");
    }
}
