use sim_engine::Plan;
use sim_params::{ContentJson, Params};
use std::fs;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ticks: u32 = a.get(1).and_then(|x| x.parse().ok()).unwrap_or(3);
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
    let mut p = p;
    for o in a.iter().skip(2) {
        let (k, v) = o.split_once('=').unwrap();
        let v: f64 = v.parse().unwrap();
        match k {
            "wcr" => p.goods.wealth_consumption_rate = v,
            "pc" => p.goods.propensity.capitalist = v,
            "pr" => p.goods.propensity.rentier = v,
            _ => panic!("key"),
        }
    }
    let mut sim = sim_engine::init(p, 42).unwrap();
    let _ = Plan::default();
    let snap = |sim: &sim_engine::Sim| {
        let s = &sim.state;
        (
            s.households
                .iter()
                .map(|h| h.savings.dollars() - h.debt.dollars())
                .collect::<Vec<_>>(),
            s.firms
                .iter()
                .map(|f| f.cash.dollars() - f.debt.dollars())
                .sum::<f64>(),
            s.bank.equity.dollars(),
            s.gov.cash.dollars() - s.gov.debt.dollars(),
        )
    };
    let mut before = None;
    for k in 0..ticks {
        if k == ticks.saturating_sub(12) {
            before = Some(snap(&sim));
        }
        sim_engine::step(&mut sim).unwrap();
        let s = &sim.state;
        println!("--- tick {}", s.tick);
        for h in &s.households {
            let i = &h.income;
            println!(
                "{:<14} wages {:>8.0} div {:>8.0} rent+ {:>7.0} int {:>6.0} ben {:>7.0} tax {:>8.0} rent- {:>7.0} loan {:>7.0} spend {:>8.0} disp {:>8.0} lastdisp {:>8.0}",
                h.label, i.wages.dollars() / 1e0, i.dividends.dollars(), i.rent_received.dollars(), i.interest_received.dollars(), i.benefits.dollars(), i.tax_paid.dollars(), i.rent_paid.dollars(), i.loan_payments.dollars(), i.spending.dollars(), i.disposable().dollars(), h.last_disposable.dollars()
            );
        }
        println!(
            "gov receipts {:.0} spending {:.0} cash {:.0} debt {:.0}",
            s.gov.receipts.dollars(),
            s.gov.spending.dollars(),
            s.gov.cash.dollars(),
            s.gov.debt.dollars()
        );
        for f in s.firms.iter().filter(|f| f.active) {
            println!("firm {:?} rev {:.0} capex {:.0} profit {:.0} taxpaid {:.0} cash {:.0} plan {:.0} expected {:.0}", f.sector, f.revenue.dollars(), f.capex.dollars(), f.profit.dollars(), f.tax_paid.dollars(), f.cash.dollars(), f.plan_invest.dollars(), f.expected_sales.0);
        }
    }

    if let Some(b) = before {
        let a = snap(&sim);
        println!("per month net worth change (financial): households {:?}, firms {:.0}, bank {:.0}, gov {:.0}", a.0.iter().zip(&b.0).map(|(x,y)| ((x-y)/12.0).round()).collect::<Vec<_>>(), (a.1-b.1)/12.0, (a.2-b.2)/12.0, (a.3-b.3)/12.0);
    }
}
