//! Probes of the 24 decisions: states are views, nudges raise a view field, reads call the real
//! `decide_*` and map its choice to the registry quantity (each mapping is commented below).
use crate::causal::Node;
use crate::class::*;
use crate::credit::*;
use crate::estate::*;
use crate::goods::*;
use crate::government::*;
use crate::income::*;
use crate::investment::*;
use crate::labour::*;
use crate::lobbying::*;
use crate::nature::*;
use crate::production::CapacityFactors;
use crate::sign_probe::{Probe, Report};
use sim_params::Params;
use sim_types::DecisionId as D;
use sim_types::QuantityId as Q;
use sim_types::*;
use std::sync::OnceLock;

fn p() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(crate::decision_tests::params)
}

type N<V> = (Q, fn(&mut V));
type R<V> = (Q, fn(&V) -> f64);

fn n<V>(q: Q, f: fn(&mut V)) -> N<V> {
    (q, f)
}
fn r<V>(q: Q, f: fn(&V) -> f64) -> R<V> {
    (q, f)
}
fn b(x: bool) -> f64 {
    if x {
        1.0
    } else {
        0.0
    }
}

pub(super) fn probe_all(rep: &mut Report) {
    hire(rep);
    fire(rep);
    wage(rep);
    price(rep);
    green(rep);
    capex(rep);
    payout(rep);
    exit(rep);
    found(rep);
    spend(rep);
    shop(rep);
    lobby(rep);
    expect(rep);
    headroom(rep);
    lend(rep);
    spread(rep);
    dividend(rep);
    rescue(rep);
    fund(rep);
    cut(rep);
    legitimacy(rep);
    pilot(rep);
    flight(rep);
    enclosure(rep);
}

// ---- labour ----

fn hire(rep: &mut Report) {
    let base = |sales, inv, cover, workers, skilled, eff, cap_out| HireView {
        expected_sales: sales,
        inventory: inv,
        inventory_cover: cover,
        workers,
        skilled_workers: skilled,
        effective_labour: eff,
        capacity: CapacityFactors {
            capital_output: cap_out,
            farm_health: 1.0,
            climate: 1.0,
            drought: 1.0,
            community: 1.0,
            smoke: 1.0,
            transition: 1.0,
        },
    };
    let probe = Probe {
        node: Node::Decision(D::FirmHire),
        bases: vec![
            base(100.0, 10.0, 1.0, 2.0, 0.0, 1.0, 2.0),
            base(400.0, 300.0, 0.0, 10.0, 1.0, 1.0, 2.0),
            base(900.0, 0.0, 2.0, 1.0, 0.0, 1.0, 2.0),
            base(200.0, 50.0, 1.0, 5.0, 1.0, 1.5, 3.0),
            base(50.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0),
            base(1000.0, 100.0, 2.0, 20.0, 3.0, 0.8, 2.5),
            base(600.0, 40.0, 1.0, 3.0, 2.0, 1.2, 2.0),
        ],
        nudges: vec![
            n(Q::ExpectedSales, |v: &mut HireView| {
                v.expected_sales = v.expected_sales * 1.5 + 5.0
            }),
            n(Q::Inventory, |v: &mut HireView| {
                v.inventory = v.inventory * 1.5 + 5.0
            }),
            n(Q::InventoryCover, |v: &mut HireView| {
                v.inventory_cover += 0.5
            }),
            n(Q::Workers, |v: &mut HireView| v.workers += 2.0),
            n(Q::SkilledWorkers, |v: &mut HireView| {
                v.skilled_workers += 2.0
            }),
            n(Q::EffectiveLabour, |v: &mut HireView| {
                v.effective_labour *= 1.5
            }),
            n(Q::CapitalOutput, |v: &mut HireView| {
                v.capacity.capital_output *= 1.5
            }),
            // Weather: the climate factor (drought and smoke are the other two writers).
            n(Q::WeatherFactor, |v: &mut HireView| {
                v.capacity.climate *= 1.5
            }),
            n(Q::ResourceFactor, |v: &mut HireView| {
                v.capacity.farm_health *= 1.5
            }),
            n(Q::TransitionFactor, |v: &mut HireView| {
                v.capacity.transition *= 1.5
            }),
        ],
        reads: vec![
            r(Q::WantedWorkers, |v: &HireView| {
                decide_hire(v, &p().labour, &p().production).workers_wanted
            }),
            // Vacancies posted: skilled plus general (what `labour::run` posts).
            r(Q::Vacancies, |v: &HireView| {
                let c = decide_hire(v, &p().labour, &p().production);
                c.skilled_vacancies + c.general_vacancies
            }),
        ],
    };
    rep.check(&probe);
}

fn fire(rep: &mut Report) {
    let lp = &p().labour;
    let k = lp.fire_after_months;
    let base = |w, want, over| FireView {
        workers: w,
        workers_wanted: want,
        over_target_months: over,
    };
    let probe = Probe {
        node: Node::Decision(D::FirmFire),
        bases: vec![
            base(20.0, 5.0, k.saturating_sub(1)),
            base(20.0, 5.0, 0),
            base(20.0, 5.0, k.saturating_sub(2)),
            base(2.0, 2.0, 3),
            base(30.0, 10.0, k.saturating_sub(1)),
            base(8.0, 8.0, 0),
            base(15.0, 4.0, 1),
            base(8.0, 5.0, k.saturating_sub(1)),
            base(7.0, 4.0, k.saturating_sub(1)),
            base(8.0, 5.0, 1),
        ],
        nudges: vec![
            n(Q::Workers, |v: &mut FireView| v.workers += 3.0),
            n(Q::WantedWorkers, |v: &mut FireView| v.workers_wanted += 3.0),
            n(Q::OverTargetMonths, |v: &mut FireView| {
                v.over_target_months += 1
            }),
        ],
        reads: vec![
            // Workers after the decision: headcount less the people fired.
            r(Q::Workers, |v: &FireView| {
                v.workers - f64::from(decide_fire(v, &p().labour).to_fire)
            }),
            r(Q::OverTargetMonths, |v: &FireView| {
                f64::from(decide_fire(v, &p().labour).over_target_months)
            }),
        ],
    };
    rep.check(&probe);
}

fn wage(rep: &mut Report) {
    let lp = &p().labour;
    let base = |wage: i64, min: i64, u, g| WageView {
        wage: Cents(wage),
        min_wage: Cents(min),
        unemployment: u,
        sector_tfp_growth: g,
    };
    let probe = Probe {
        node: Node::Decision(D::FirmWage),
        bases: vec![
            base(300_000, 100_000, 0.05, 0.001),
            base(300_000, 400_000, 0.05, 0.001),
            base(300_000, 100_000, lp.u_ref, 0.0),
            base(300_000, 310_000, 0.10, 0.0),
            base(500_000, 200_000, 0.02, 0.002),
            base(300_000, 100_000, 0.0001, 0.001),
        ],
        nudges: vec![
            n(Q::Wage, |v: &mut WageView| {
                v.wage = Cents(v.wage.0 * 6 / 5 + 1_000)
            }),
            n(Q::MinWage, |v: &mut WageView| {
                v.min_wage = Cents(v.min_wage.0 + 200_000)
            }),
            n(Q::Unemployment, |v: &mut WageView| v.unemployment += 0.01),
            n(Q::SectorTfpGrowth, |v: &mut WageView| {
                v.sector_tfp_growth += 0.0005
            }),
        ],
        reads: vec![r(Q::Wage, |v: &WageView| {
            decide_wage(v, &p().labour).0 as f64
        })],
    };
    rep.check(&probe);
}

// ---- goods ----

fn price(rep: &mut Report) {
    let base = |price: i64, markup, unit_cost| PriceView {
        price: Cents(price),
        markup,
        unit_cost,
    };
    // No base has unit cost 0: that is the "made nothing, keep the price" regime, a different
    // rule (a discontinuity), not the clamp the sign is about.
    let probe = Probe {
        node: Node::Decision(D::FirmPrice),
        bases: vec![
            base(1_000, 0.2, 500.0),
            base(1_000, 0.2, 1_500.0),
            base(1, 0.2, 500.0),
            base(5_000, 0.5, 2_000.0),
            base(800, 0.1, 900.0),
        ],
        nudges: vec![
            n(Q::Price, |v: &mut PriceView| {
                v.price = Cents(v.price.0 * 2 + 500)
            }),
            n(Q::Markup, |v: &mut PriceView| v.markup += 0.3),
            n(Q::UnitCost, |v: &mut PriceView| {
                v.unit_cost = v.unit_cost * 2.0 + 100.0
            }),
        ],
        reads: vec![r(Q::Price, |v: &PriceView| {
            decide_price(v, &p().goods).0 as f64
        })],
    };
    rep.check(&probe);
}

fn spend(rep: &mut Report) {
    let base = |size, fp, sav, disp, prop, nw, wr| SpendView {
        household_size: size,
        food_price: fp,
        savings: Cents(sav),
        last_disposable: Cents(disp),
        spending_propensity: prop,
        net_wealth: nw,
        wealth_spending_rate: wr,
    };
    let probe = Probe {
        node: Node::Decision(D::HouseholdSpend),
        bases: vec![
            base(3, 500.0, 5_000_000, 400_000, 0.8, 0.0, 0.0),
            base(2, 500.0, 5_000_000, 100_000, 0.6, 0.0, 0.0),
            base(4, 800.0, 100_000, 400_000, 0.8, 0.0, 0.0),
            base(1, 300.0, 9_000_000, 900_000, 0.7, 20_000_000.0, 0.01),
            base(3, 500.0, 600_000, 300_000, 0.5, 0.0, 0.0),
            base(2, 400.0, 2_000_000, 0, 0.8, 5_000_000.0, 0.005),
        ],
        nudges: vec![
            n(Q::HouseholdSize, |v: &mut SpendView| v.household_size += 1),
            n(Q::FoodPrice, |v: &mut SpendView| v.food_price *= 1.5),
            n(Q::Savings, |v: &mut SpendView| {
                v.savings = Cents(v.savings.0 * 2 + 1_000)
            }),
            n(Q::LastDisposable, |v: &mut SpendView| {
                v.last_disposable = Cents(v.last_disposable.0 * 2 + 100_000)
            }),
            n(Q::SpendingPropensity, |v: &mut SpendView| {
                v.spending_propensity += 0.1
            }),
            n(Q::NetWealth, |v: &mut SpendView| {
                v.net_wealth = v.net_wealth * 2.0 + 1_000_000.0
            }),
            n(Q::WealthSpendingRate, |v: &mut SpendView| {
                v.wealth_spending_rate += 0.01
            }),
        ],
        reads: vec![
            r(Q::FoodNeed, |v: &SpendView| {
                decide_spend(v, &p().goods).food_need_units
            }),
            // Total spending: food plus goods plus services (what leaves savings).
            r(Q::ConsumptionBudget, |v: &SpendView| {
                let c = decide_spend(v, &p().goods);
                c.food_budget + c.goods_budget + c.services_budget
            }),
        ],
    };
    rep.check(&probe);
}

/// An owned shop view: firm 0 is the one whose stock and price are nudged and whose sales are read.
#[derive(Clone)]
struct ShopState {
    budget: f64,
    stock: Vec<f64>,
    price: Vec<f64>,
}

fn shop_choice(v: &ShopState) -> ShopChoice {
    decide_shop(
        &ShopView {
            budget: v.budget,
            stock: &v.stock,
            price: &v.price,
        },
        &p().goods,
    )
}

fn shop(rep: &mut Report) {
    let base = |budget, stock: &[f64], price: &[f64]| ShopState {
        budget,
        stock: stock.to_vec(),
        price: price.to_vec(),
    };
    let probe = Probe {
        node: Node::Decision(D::HouseholdShop),
        bases: vec![
            base(1_000_000.0, &[100.0, 50.0], &[1_000.0, 1_200.0]),
            base(100_000.0, &[100.0, 50.0], &[1_000.0, 1_200.0]),
            base(10_000_000.0, &[10.0, 500.0], &[500.0, 500.0]),
            base(500_000.0, &[300.0, 300.0, 300.0], &[800.0, 900.0, 700.0]),
            base(300_000.0, &[20.0, 20.0], &[1_000.0, 1_000.0]),
            base(2_000_000.0, &[1_000.0, 5.0], &[600.0, 400.0]),
        ],
        nudges: vec![
            n(Q::ConsumptionBudget, |v: &mut ShopState| v.budget *= 1.5),
            n(Q::Inventory, |v: &mut ShopState| {
                v.stock[0] = v.stock[0] * 1.5 + 5.0
            }),
            n(Q::Price, |v: &mut ShopState| v.price[0] *= 1.5),
        ],
        reads: vec![
            r(Q::UnitsSold, |v: &ShopState| shop_choice(v).sold[0]),
            r(Q::DemandUnits, |v: &ShopState| {
                shop_choice(v).demand_units[0]
            }),
        ],
    };
    rep.check(&probe);
}

fn expect(rep: &mut Report) {
    let base = |d, e| ExpectView {
        demand_units: d,
        expected_sales: e,
    };
    let probe = Probe {
        node: Node::Decision(D::FirmExpect),
        bases: vec![
            base(100.0, 80.0),
            base(0.0, 80.0),
            base(500.0, 10.0),
            base(30.0, 300.0),
        ],
        nudges: vec![
            n(Q::DemandUnits, |v: &mut ExpectView| v.demand_units += 20.0),
            n(Q::ExpectedSales, |v: &mut ExpectView| {
                v.expected_sales += 20.0
            }),
        ],
        reads: vec![r(Q::ExpectedSales, |v: &ExpectView| {
            decide_expect(v, &p().labour).0
        })],
    };
    rep.check(&probe);
}

// ---- investment, income ----

fn green(rep: &mut Report) {
    let base = |footprint, cash, cap, tech| GreenView {
        footprint,
        carbon_price: Cents(10_000),
        revenue: Cents(1_000_000),
        emissions_cap: cap,
        green_subsidy: 0.5,
        treasury_cash: Cents(10_000_000),
        cash: Cents(cash),
        cash_reserve: Cents(100_000),
        green_tech: tech,
    };
    let mut poor = base(1_000.0, 50_000_000, 0.0, 0.1);
    poor.treasury_cash = Cents(0);
    let mut low_carbon = base(5.0, 50_000_000, 0.0, 0.1);
    low_carbon.carbon_price = Cents(100);
    let mut mid_cash = base(1_000.0, 350_000, 0.0, 0.1);
    mid_cash.green_subsidy = 0.0;
    // Not worthwhile at 0.7 tonnes (carbon bill 0.42M against a step costing 1.2M a year of sales).
    let low = |cash| base(0.7, cash, 0.0, 0.1);
    // A step whose carbon bill just beats its cost, so more revenue loses it.
    let edge = |cash| {
        let mut v = base(1.5, cash, 0.0, 0.1);
        v.carbon_price = Cents(20_000);
        v
    };
    // Cheap carbon, large footprint: only a binding cap makes it worthwhile.
    let capped = |cash, cap| {
        let mut v = base(12.0, cash, cap, 0.1);
        v.carbon_price = Cents(10);
        v
    };
    // Subsidy decides whether the firm can pay its share now.
    let mut sub0 = base(1_000.0, 180_000, 0.0, 0.1);
    sub0.green_subsidy = 0.0;
    let mut no_treasury = base(1_000.0, 180_000, 0.0, 0.1);
    no_treasury.treasury_cash = Cents(0);
    let probe = Probe {
        node: Node::Decision(D::FirmGreen),
        bases: vec![
            base(1_000.0, 50_000_000, 0.0, 0.1),
            base(1_000.0, 100_000, 0.0, 0.1),
            base(0.0, 50_000_000, 0.0, 0.1),
            base(5.0, 50_000_000, 6.0, 0.1),
            base(1_000.0, 50_000_000, 0.0, 0.0),
            base(1.5, 50_000_000, 1.0, 0.1),
            base(1.5, 50_000_000, 0.0, 0.1),
            poor,
            low_carbon,
            mid_cash,
            low(100_000),
            low(50_000_000),
            edge(100_000),
            edge(50_000_000),
            capped(50_000_000, 0.0),
            capped(50_000_000, 6.0),
            capped(100_000, 0.0),
            capped(100_000, 6.0),
            sub0,
            no_treasury,
            base(1_000.0, 100_000, 0.0, 0.78),
        ],
        nudges: vec![
            n(Q::Footprint, |v: &mut GreenView| {
                v.footprint = v.footprint * 3.0 + 1.0
            }),
            n(Q::CarbonPrice, |v: &mut GreenView| {
                v.carbon_price = Cents(v.carbon_price.0 * 3)
            }),
            n(Q::Revenue, |v: &mut GreenView| {
                v.revenue = Cents(v.revenue.0 * 3)
            }),
            n(Q::EmissionsCap, |v: &mut GreenView| v.emissions_cap += 10.0),
            n(Q::GreenSubsidy, |v: &mut GreenView| v.green_subsidy += 0.3),
            n(Q::TreasuryCash, |v: &mut GreenView| {
                v.treasury_cash = Cents(v.treasury_cash.0 + 10_000_000)
            }),
            n(Q::Cash, |v: &mut GreenView| {
                v.cash = Cents(v.cash.0 * 2 + 5_000_000)
            }),
            n(Q::CashReserve, |v: &mut GreenView| {
                v.cash_reserve = Cents(v.cash_reserve.0 + 6_000_000)
            }),
            n(Q::GreenTech, |v: &mut GreenView| v.green_tech += 0.05),
        ],
        reads: vec![
            r(Q::GreenTech, |v: &GreenView| {
                decide_green(v, &p().invest).green_tech
            }),
            // GreenPlan: the firm saves up for a step it cannot yet afford.
            r(Q::GreenPlan, |v: &GreenView| {
                b(decide_green(v, &p().invest).plan)
            }),
        ],
    };
    rep.check(&probe);
}

/// Profit (cents a month) at which the capex return equals the hurdle for capital `k`, scaled.
fn mid_profit(k: f64, scale: f64) -> i64 {
    let ip = &p().invest;
    let thr = 0.06 + 0.02;
    (thr * k * ip.capital_price_usd * 100.0 / 12.0 * scale) as i64
}

fn capex(rep: &mut Report) {
    let base = |capital: f64, sales, per_sale, distressed, plan, pscale: f64| {
        let pr = Cents(mid_profit(capital, pscale));
        CapexView {
            capital,
            profit_trend: pr,
            profit: pr,
            loan_rate: 0.06,
            invest_hurdle: 0.02,
            expected_sales: sales,
            capital_per_sale: per_sale,
            distressed,
            cash: Cents(1_200_000),
            cash_reserve: Cents(1_000_000),
            green_plan: plan,
            green_cash_share: 0.5,
        }
    };
    let mut poor_cash = base(100.0, 100.0, 2.0, false, false, 1.0);
    poor_cash.cash = Cents(5_000_000);
    let mut tiny_cash = base(100.0, 100.0, 2.0, false, false, 1.0);
    tiny_cash.cash = Cents(500_000);
    let probe = Probe {
        node: Node::Decision(D::FirmCapex),
        bases: vec![
            base(100.0, 100.0, 2.0, false, false, 1.0),
            base(100.0, 100.0, 2.0, false, false, 0.9),
            base(100.0, 100.0, 2.0, false, false, 1.1),
            base(100.0, 100.0, 2.0, false, false, 0.2),
            base(100.0, 100.0, 2.0, false, false, 3.0),
            base(100.0, 51.0, 2.0, false, false, 1.0),
            base(100.0, 300.0, 2.0, false, true, 1.0),
            base(100.0, 100.0, 2.0, false, true, 1.0),
            base(100.0, 20.0, 2.0, false, false, 1.0),
            poor_cash,
            tiny_cash,
        ],
        nudges: vec![
            n(Q::Capital, |v: &mut CapexView| v.capital *= 1.2),
            n(Q::ProfitTrend, |v: &mut CapexView| {
                v.profit_trend = Cents(v.profit_trend.0 * 2 + 10_000)
            }),
            n(Q::Profit, |v: &mut CapexView| {
                v.profit = Cents(v.profit.0 * 2 + 10_000)
            }),
            n(Q::LoanRate, |v: &mut CapexView| v.loan_rate += 0.02),
            n(Q::InvestHurdle, |v: &mut CapexView| v.invest_hurdle += 0.02),
            n(Q::ExpectedSales, |v: &mut CapexView| {
                v.expected_sales *= 1.3
            }),
            n(Q::CapitalPerSale, |v: &mut CapexView| {
                v.capital_per_sale += 0.5
            }),
            n(Q::Distressed, |v: &mut CapexView| v.distressed = true),
            n(Q::Cash, |v: &mut CapexView| {
                v.cash = Cents(v.cash.0 + 3_000_000)
            }),
            n(Q::CashReserve, |v: &mut CapexView| {
                v.cash_reserve = Cents(v.cash_reserve.0 + 1_000_000)
            }),
            n(Q::GreenPlan, |v: &mut CapexView| v.green_plan = true),
            n(Q::GreenCashShare, |v: &mut CapexView| {
                v.green_cash_share += 0.2
            }),
        ],
        reads: vec![
            r(Q::InvestmentPlan, |v: &CapexView| {
                decide_capex(v, &p().invest).plan.0 as f64
            }),
            r(Q::LoanRequest, |v: &CapexView| {
                decide_capex(v, &p().invest).request.0 as f64
            }),
            r(Q::ProfitTrend, |v: &CapexView| {
                decide_capex(v, &p().invest).profit_ema.0 as f64
            }),
        ],
    };
    rep.check(&probe);
}

fn payout(rep: &mut Report) {
    let base = |profit, cash, payroll, ds, plan, revenue| PayoutView {
        profit: Cents(profit),
        cash: Cents(cash),
        payroll: Cents(payroll),
        debt_service: Cents(ds),
        green_plan: plan,
        revenue: Cents(revenue),
        green_subsidy: 0.5,
        payout_ratio: 0.5,
    };
    let probe = Probe {
        node: Node::Decision(D::FirmPayout),
        bases: vec![
            base(200_000, 5_000_000, 1_000_000, 100_000, false, 3_000_000),
            base(200_000, 5_000_000, 1_000_000, 100_000, true, 3_000_000),
            base(200_000, 1_300_000, 1_000_000, 100_000, false, 3_000_000),
            base(200_000, 1_300_000, 1_000_000, 100_000, true, 3_000_000),
            base(900_000, 20_000_000, 1_000_000, 100_000, false, 3_000_000),
            base(900_000, 20_000_000, 1_000_000, 100_000, true, 3_000_000),
            base(-50_000, 20_000_000, 500_000, 0, false, 1_000_000),
            base(300_000, 3_000_000, 2_000_000, 500_000, true, 5_000_000),
        ],
        nudges: vec![
            n(Q::Profit, |v: &mut PayoutView| {
                v.profit = Cents(v.profit.0 * 2 + 100_000)
            }),
            n(Q::Cash, |v: &mut PayoutView| {
                v.cash = Cents(v.cash.0 * 2 + 500_000)
            }),
            n(Q::Payroll, |v: &mut PayoutView| {
                v.payroll = Cents(v.payroll.0 * 2 + 100_000)
            }),
            n(Q::DebtService, |v: &mut PayoutView| {
                v.debt_service = Cents(v.debt_service.0 * 2 + 100_000)
            }),
            n(Q::GreenPlan, |v: &mut PayoutView| v.green_plan = true),
            n(Q::Revenue, |v: &mut PayoutView| {
                v.revenue = Cents(v.revenue.0 * 2 + 100_000)
            }),
            n(Q::GreenSubsidy, |v: &mut PayoutView| v.green_subsidy += 0.3),
            n(Q::PayoutRatio, |v: &mut PayoutView| v.payout_ratio += 0.3),
        ],
        reads: vec![r(Q::Dividends, |v: &PayoutView| {
            decide_payout(v, &p().income, &p().invest).0 as f64
        })],
    };
    rep.check(&probe);
}

// ---- credit ----

fn exit(rep: &mut Report) {
    let dam = p().credit.default_after_months;
    let base = |s, m, d| ExitView {
        payment_shortfall: s,
        distress_months: m,
        debt: Cents(d),
    };
    let probe = Probe {
        node: Node::Decision(D::FirmExit),
        bases: vec![
            base(false, 0, 0),
            base(true, 0, 0),
            base(true, dam.saturating_sub(1), 500_000),
            base(true, dam.saturating_sub(1), 0),
            base(true, dam.saturating_sub(2), 500_000),
            base(false, dam.saturating_sub(1), 500_000),
            base(true, dam, 100),
            base(false, dam, 5_000),
        ],
        nudges: vec![
            n(Q::PaymentShortfall, |v: &mut ExitView| {
                v.payment_shortfall = true
            }),
            n(Q::DistressMonths, |v: &mut ExitView| v.distress_months += 1),
            n(Q::Debt, |v: &mut ExitView| {
                v.debt = Cents(v.debt.0 + 100_000)
            }),
        ],
        reads: vec![
            r(Q::DistressMonths, |v: &ExitView| {
                f64::from(decide_exit(v, &p().credit).distress_months)
            }),
            // FirmOpen: 1 unless the firm closes.
            r(Q::FirmOpen, |v: &ExitView| {
                1.0 - b(decide_exit(v, &p().credit).closes)
            }),
        ],
    };
    rep.check(&probe);
}

fn headroom_bases() -> Vec<HeadroomView> {
    let base = |eq, loans, ratio| HeadroomView {
        bank_equity: Cents(eq),
        bank_loans: Cents(loans),
        capital_ratio: ratio,
    };
    vec![
        base(1_000_000, 5_000_000, 0.1),
        base(1_000_000, 20_000_000, 0.1),
        base(3_000_000, 10_000_000, 0.08),
        base(2_000_000, 2_000_000, 0.2),
        base(500_000, 0, 0.1),
        base(5_000_000, 30_000_000, 0.12),
    ]
}

fn headroom(rep: &mut Report) {
    let probe = Probe {
        node: Node::Decision(D::BankHeadroom),
        bases: headroom_bases(),
        nudges: vec![
            n(Q::BankEquity, |v: &mut HeadroomView| {
                v.bank_equity = Cents(v.bank_equity.0 * 3 / 2 + 1_000)
            }),
            n(Q::BankLoans, |v: &mut HeadroomView| {
                v.bank_loans = Cents(v.bank_loans.0 * 3 / 2 + 1_000)
            }),
            n(Q::CapitalRatio, |v: &mut HeadroomView| {
                v.capital_ratio += 0.05
            }),
        ],
        reads: vec![r(Q::CreditHeadroom, |v: &HeadroomView| {
            decide_headroom(v).0 as f64
        })],
    };
    rep.check(&probe);
}

fn dividend(rep: &mut Report) {
    let probe = Probe {
        node: Node::Decision(D::BankDividend),
        bases: headroom_bases(),
        nudges: vec![
            n(Q::BankEquity, |v: &mut HeadroomView| {
                v.bank_equity = Cents(v.bank_equity.0 * 3 / 2 + 1_000)
            }),
            n(Q::BankLoans, |v: &mut HeadroomView| {
                v.bank_loans = Cents(v.bank_loans.0 * 3 / 2 + 1_000)
            }),
            n(Q::CapitalRatio, |v: &mut HeadroomView| {
                v.capital_ratio += 0.05
            }),
        ],
        reads: vec![r(Q::BankDividend, |v: &HeadroomView| {
            decide_bank_dividend(v, &p().credit).0 as f64
        })],
    };
    rep.check(&probe);
}

fn lend(rep: &mut Report) {
    let base = |req, head, assets, debt| LendView {
        loan_request: Cents(req),
        credit_headroom: Cents(head),
        firm_assets: assets,
        debt: Cents(debt),
    };
    let probe = Probe {
        node: Node::Decision(D::BankLend),
        bases: vec![
            base(1_000_000, 5_000_000, 10_000_000.0, 1_000_000),
            base(9_000_000, 5_000_000, 10_000_000.0, 1_000_000),
            base(9_000_000, 50_000_000, 10_000_000.0, 1_000_000),
            base(1_000_000, 5_000_000, 10_000_000.0, 0),
            base(9_000_000, 50_000_000, 2_000_000.0, 500_000),
            base(500_000, 5_000_000, 100_000.0, 50_000),
            base(9_000_000, 3_000_000, 10_000_000.0, 1_000_000),
        ],
        nudges: vec![
            n(Q::LoanRequest, |v: &mut LendView| {
                v.loan_request = Cents(v.loan_request.0 * 2 + 1_000)
            }),
            n(Q::CreditHeadroom, |v: &mut LendView| {
                v.credit_headroom = Cents(v.credit_headroom.0 * 2 + 1_000)
            }),
            n(Q::FirmAssets, |v: &mut LendView| {
                v.firm_assets = v.firm_assets * 2.0 + 1_000.0
            }),
            n(Q::Debt, |v: &mut LendView| {
                v.debt = Cents(v.debt.0 * 2 + 100_000)
            }),
        ],
        reads: vec![r(Q::LoanApproved, |v: &LendView| {
            decide_lend(v, &p().credit).0 as f64
        })],
    };
    rep.check(&probe);
}

fn spread(rep: &mut Report) {
    let base = |rate, debt, assets, add| SpreadView {
        base_rate: rate,
        debt: Cents(debt),
        firm_assets: assets,
        credit_spread_add: add,
    };
    let probe = Probe {
        node: Node::Decision(D::BankSpread),
        bases: vec![
            base(0.03, 1_000_000, 10_000_000.0, 0.0),
            base(0.0, 0, 5_000_000.0, 0.01),
            base(0.05, 5_000_000, 6_000_000.0, 0.0),
            base(0.02, 100, 3_000_000.0, 0.0),
        ],
        nudges: vec![
            n(Q::PolicyRate, |v: &mut SpreadView| v.base_rate += 0.01),
            n(Q::Debt, |v: &mut SpreadView| {
                v.debt = Cents(v.debt.0 * 2 + 100_000)
            }),
            n(Q::FirmAssets, |v: &mut SpreadView| {
                v.firm_assets = v.firm_assets * 2.0 + 1_000_000.0
            }),
            n(Q::CreditSpreadAdd, |v: &mut SpreadView| {
                v.credit_spread_add += 0.01
            }),
        ],
        reads: vec![r(Q::LoanRate, |v: &SpreadView| {
            decide_spread(v, &p().credit).0
        })],
    };
    rep.check(&probe);
}

fn rescue(rep: &mut Report) {
    let base = |eq, loans, dep, bail| RescueView {
        bank_equity: Cents(eq),
        bank_loans: Cents(loans),
        bank_deposits: Cents(dep),
        bailout_policy: bail,
    };
    let probe = Probe {
        node: Node::Decision(D::BankRescue),
        bases: vec![
            base(-1_000_000, 10_000_000, 20_000_000, false),
            base(-1_000_000, 10_000_000, 20_000_000, true),
            base(-5_000_000, 40_000_000, 50_000_000, true),
            base(-200_000, 10_000_000, 20_000_000, true),
            base(-200_000, 10_000_000, 20_000_000, false),
            base(1_000_000, 10_000_000, 20_000_000, true),
        ],
        nudges: vec![
            n(Q::BankEquity, |v: &mut RescueView| {
                v.bank_equity = Cents(v.bank_equity.0 + 300_000)
            }),
            n(Q::BankLoans, |v: &mut RescueView| {
                v.bank_loans = Cents(v.bank_loans.0 * 2)
            }),
            n(Q::BankDeposits, |v: &mut RescueView| {
                v.bank_deposits = Cents(v.bank_deposits.0 * 2)
            }),
            n(Q::BailoutPolicy, |v: &mut RescueView| {
                v.bailout_policy = true
            }),
        ],
        reads: vec![
            // BankEquity: the cash the treasury injects (0 unless a bailout).
            r(Q::BankEquity, |v: &RescueView| {
                match decide_rescue(v, &p().credit) {
                    RescueChoice::Bailout { amount } => amount.0 as f64,
                    _ => 0.0,
                }
            }),
            // BankFailed: depositors take a haircut (no bailout, equity negative).
            r(Q::BankFailed, |v: &RescueView| {
                b(matches!(
                    decide_rescue(v, &p().credit),
                    RescueChoice::Haircut { .. }
                ))
            }),
        ],
    };
    rep.check(&probe);
}

// ---- government ----

fn fund(rep: &mut Report) {
    let base = |com, next, cash, tax, head, debt| FundView {
        committed: Cents(com),
        next_month_purchases: Cents(next),
        treasury_cash: Cents(cash),
        treasury_taxes: Cents(tax),
        credit_headroom: Cents(head),
        public_debt: Cents(debt),
    };
    let probe = Probe {
        node: Node::Decision(D::GovernmentFund),
        bases: vec![
            base(
                5_000_000, 1_000_000, 1_000_000, 2_000_000, 10_000_000, 3_000_000,
            ),
            base(
                5_000_000, 1_000_000, 1_000_000, 2_000_000, 1_000_000, 3_000_000,
            ),
            base(
                1_000_000, 500_000, 6_000_000, 4_000_000, 10_000_000, 30_000_000,
            ),
            base(
                1_000_000, 500_000, 6_000_000, 4_000_000, 10_000_000, 1_000_000,
            ),
            base(2_000_000, 0, 1_000_000, 1_000_000, 5_000_000, 9_000_000),
            base(2_000_000, 0, 5_000_000, 1_000_000, 5_000_000, 9_000_000),
            base(
                1_000_000, 500_000, 6_000_000, 4_000_000, 10_000_000, 300_000,
            ),
        ],
        nudges: vec![
            n(Q::CommittedSpending, |v: &mut FundView| {
                v.committed = Cents(v.committed.0 + 1_000_000)
            }),
            n(Q::NextMonthPurchases, |v: &mut FundView| {
                v.next_month_purchases = Cents(v.next_month_purchases.0 + 1_000_000)
            }),
            n(Q::TreasuryCash, |v: &mut FundView| {
                v.treasury_cash = Cents(v.treasury_cash.0 + 1_000_000)
            }),
            n(Q::TreasuryTaxes, |v: &mut FundView| {
                v.treasury_taxes = Cents(v.treasury_taxes.0 + 1_000_000)
            }),
            n(Q::CreditHeadroom, |v: &mut FundView| {
                v.credit_headroom = Cents(v.credit_headroom.0 + 1_000_000)
            }),
            n(Q::PublicDebt, |v: &mut FundView| {
                v.public_debt = Cents(v.public_debt.0 + 1_000_000)
            }),
        ],
        reads: vec![
            r(Q::Borrowing, |v: &FundView| {
                match decide_fund(v, &p().government) {
                    FundChoice::Borrow { amount } => amount.0 as f64,
                    _ => 0.0,
                }
            }),
            r(Q::Repayment, |v: &FundView| {
                match decide_fund(v, &p().government) {
                    FundChoice::Repay { amount } => amount.0 as f64,
                    _ => 0.0,
                }
            }),
        ],
    };
    rep.check(&probe);
}

fn cut(rep: &mut Report) {
    let base = |com, cash, tax, bor| CutView {
        committed: Cents(com),
        treasury_cash: Cents(cash),
        treasury_taxes: Cents(tax),
        borrowing: Cents(bor),
    };
    let probe = Probe {
        node: Node::Decision(D::GovernmentCut),
        bases: vec![
            base(10_000_000, 1_000_000, 2_000_000, 1_000_000),
            base(10_000_000, 1_000_000, 2_000_000, 0),
            base(2_000_000, 1_000_000, 2_000_000, 0),
            base(5_000_000, 3_000_000, 1_000_000, 500_000),
        ],
        nudges: vec![
            n(Q::CommittedSpending, |v: &mut CutView| {
                v.committed = Cents(v.committed.0 + 2_000_000)
            }),
            n(Q::TreasuryCash, |v: &mut CutView| {
                v.treasury_cash = Cents(v.treasury_cash.0 + 1_000_000)
            }),
            n(Q::TreasuryTaxes, |v: &mut CutView| {
                v.treasury_taxes = Cents(v.treasury_taxes.0 + 1_000_000)
            }),
            n(Q::Borrowing, |v: &mut CutView| {
                v.borrowing = Cents(v.borrowing.0 + 1_000_000)
            }),
        ],
        reads: vec![r(Q::SpendingCutRatio, |v: &CutView| decide_cut(v))],
    };
    rep.check(&probe);
}

fn legitimacy(rep: &mut Report) {
    let base = |l, w, u, g, d, bail, tr| LegitView {
        legitimacy: l,
        wellbeing: w,
        unemployment: u,
        inequality: g,
        public_debt_ratio: d,
        bailout: bail,
        transition_resistance: tr,
    };
    let probe = Probe {
        node: Node::Decision(D::GovernmentLegitimacy),
        bases: vec![
            base(0.5, 0.5, 0.15, 0.6, 1.5, false, 0.0),
            base(0.5, 0.3, 0.3, 0.7, 2.0, false, 0.001),
            base(0.5, 0.7, 0.02, 0.2, 0.5, false, 0.0),
            base(0.6, 0.5, 0.15, 0.6, 1.5, true, 0.0),
            base(0.4, 0.4, 0.25, 0.5, 1.2, false, 0.002),
            base(0.0, 0.1, 0.4, 0.8, 3.0, true, 0.01),
        ],
        nudges: vec![
            n(Q::Legitimacy, |v: &mut LegitView| v.legitimacy += 0.05),
            n(Q::Wellbeing, |v: &mut LegitView| v.wellbeing += 0.05),
            n(Q::Unemployment, |v: &mut LegitView| v.unemployment += 0.02),
            n(Q::Inequality, |v: &mut LegitView| v.inequality += 0.05),
            n(Q::PublicDebtRatio, |v: &mut LegitView| {
                v.public_debt_ratio += 0.2
            }),
            n(Q::Bailout, |v: &mut LegitView| v.bailout = true),
            n(Q::TransitionResistance, |v: &mut LegitView| {
                v.transition_resistance += 0.01
            }),
        ],
        // The new legitimacy before the 0..1 clamp is `raw`; `after` is clamped, and a clamped
        // sample reads as flat, so the clamped value is the one read.
        reads: vec![r(Q::Legitimacy, |v: &LegitView| {
            decide_legitimacy(v, &p().government).after
        })],
    };
    rep.check(&probe);
}

// ---- class, lobbying, estate, nature ----

fn found(rep: &mut Report) {
    let mob = p().class.mobility_rate;
    let mut bases = Vec::new();
    for (ret, rate) in [
        (0.20, 0.05),
        (0.08, 0.05),
        (0.06, 0.05),
        (0.12, 0.05),
        (0.04, 0.05),
    ] {
        for f in [0.005, 0.02, 0.08, 0.3] {
            // Draws as multiples of the mobility rate, so some fall under the chance and some over.
            bases.push(FoundView {
                sector_return: ret,
                base_rate: rate,
                random_draw: mob * f,
            });
        }
    }
    let probe = Probe {
        node: Node::Decision(D::HouseholdFoundFirm),
        bases,
        nudges: vec![
            n(Q::SectorReturn, |v: &mut FoundView| v.sector_return += 0.05),
            n(Q::BaseRate, |v: &mut FoundView| v.base_rate += 0.05),
            n(Q::RandomDraw, |v: &mut FoundView| {
                v.random_draw = v.random_draw * 2.0 + 1e-6
            }),
        ],
        reads: vec![r(Q::NewFirm, |v: &FoundView| {
            b(decide_found(v, &p().class))
        })],
    };
    rep.check(&probe);
}

fn lobby(rep: &mut Report) {
    let base = |ci, sav, cap| LobbyView {
        capital_income: Cents(ci),
        savings: Cents(sav),
        campaign_cap: cap,
    };
    let probe = Probe {
        node: Node::Decision(D::HouseholdLobby),
        bases: vec![
            base(1_000_000, 50_000_000, 1.0),
            base(1_000_000, 50_000_000, 0.01),
            base(1_000_000, 50_000, 1.0),
            base(5_000_000, 100_000_000, 0.001),
            base(200_000, 300_000, 0.5),
            base(1_000_000, 5_000, 1.0),
        ],
        nudges: vec![
            n(Q::CapitalIncome, |v: &mut LobbyView| {
                v.capital_income = Cents(v.capital_income.0 * 2 + 1_000)
            }),
            n(Q::Savings, |v: &mut LobbyView| {
                v.savings = Cents(v.savings.0 * 2 + 1_000)
            }),
            n(Q::CampaignCap, |v: &mut LobbyView| v.campaign_cap += 0.05),
        ],
        reads: vec![r(Q::LobbySpend, |v: &LobbyView| {
            decide_lobby(v, &p().lobby).0 as f64
        })],
    };
    rep.check(&probe);
}

fn pilot(rep: &mut Report) {
    let ep = &p().estate;
    let need = Cents::from_dollars(ep.pilot_wage_usd).scale(ep.pilot_rehire_months_of_wages);
    let mx = ep.pilot_unpaid_months_max;
    let base = |emp, unpaid, free, sav: i64| PilotView {
        pilot_employed: emp,
        pilot_unpaid_months: unpaid,
        pilot_free: free,
        estate_savings: Cents(sav),
    };
    let probe = Probe {
        node: Node::Decision(D::EstatePilot),
        bases: vec![
            base(true, 0, false, need.0 * 5),
            base(true, mx.saturating_sub(1), false, need.0 * 5),
            base(true, mx, false, need.0 * 5),
            base(false, 0, true, need.0 * 5),
            base(false, 0, true, 0),
            base(false, 0, false, need.0 * 5),
            base(false, 0, true, need.0 / 2),
        ],
        nudges: vec![
            n(Q::PilotEmployed, |v: &mut PilotView| {
                v.pilot_employed = true
            }),
            n(Q::PilotUnpaidMonths, |v: &mut PilotView| {
                v.pilot_unpaid_months += 1
            }),
            n(Q::PilotFree, |v: &mut PilotView| v.pilot_free = true),
            n(Q::EstateSavings, |v: &mut PilotView| {
                v.estate_savings = Cents(v.estate_savings.0 + 100_000_000)
            }),
        ],
        // PilotEmployed after the decision: Hire -> 1, LetGo -> 0, Keep -> unchanged.
        reads: vec![r(Q::PilotEmployed, |v: &PilotView| {
            match decide_pilot(v, &p().estate) {
                PilotChoice::Hire => 1.0,
                PilotChoice::LetGo => 0.0,
                PilotChoice::Keep => b(v.pilot_employed),
            }
        })],
    };
    rep.check(&probe);
}

fn flight(rep: &mut Report) {
    let pr = p().estate.flight_prob_monthly;
    let base = |d, e| FlightView {
        random_draw: pr * d,
        pilot_employed: e,
    };
    let probe = Probe {
        node: Node::Decision(D::EstateFlight),
        bases: vec![
            base(0.6, true),
            base(0.2, true),
            base(2.0, true),
            base(0.5, false),
            base(0.3, false),
        ],
        nudges: vec![
            n(Q::RandomDraw, |v: &mut FlightView| {
                v.random_draw = v.random_draw * 2.0 + 1e-9
            }),
            n(Q::PilotEmployed, |v: &mut FlightView| {
                v.pilot_employed = true
            }),
        ],
        reads: vec![r(Q::Flight, |v: &FlightView| {
            b(decide_flight(v, &p().estate))
        })],
    };
    rep.check(&probe);
}

fn enclosure(rep: &mut Report) {
    let base = |c, d, o| EnclosureView {
        enclosure_chance: c,
        random_draw: d,
        commons_open: o,
    };
    let probe = Probe {
        node: Node::Decision(D::NatureEnclosure),
        bases: vec![
            base(0.01, 0.006, true),
            base(0.01, 0.015, true),
            base(0.0, 0.005, true),
            base(0.02, 0.005, false),
            base(0.0, 0.5, true),
            base(0.05, 0.03, true),
        ],
        nudges: vec![
            n(Q::EnclosureChance, |v: &mut EnclosureView| {
                v.enclosure_chance += 0.02
            }),
            n(Q::RandomDraw, |v: &mut EnclosureView| {
                v.random_draw = v.random_draw * 2.0 + 1e-6
            }),
            n(Q::CommonsOpen, |v: &mut EnclosureView| {
                v.commons_open = true
            }),
        ],
        reads: vec![r(Q::CommonsEnclosed, |v: &EnclosureView| {
            b(decide_enclosure(v))
        })],
    };
    rep.check(&probe);
}
