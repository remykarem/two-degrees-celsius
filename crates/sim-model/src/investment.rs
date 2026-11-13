//! S6 investment and technology: the growth engine. Firms invest when the expected return
//! beats the loan rate plus a hurdle; carbon costs and regulation trigger green investment;
//! (Private housing is not built by the model in Town v2: new homes are public, placed by the player.)

use crate::util::{
    announced_carbon_price, apportion, cash_reserve, cfg, labour_weights, MONTHS_PER_YEAR,
};
use libm::pow;
use sim_params::{InvestParams, Params};
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::QuantityId as Q;
use sim_types::*;

// ---- Decision: a step of green technology ----

pub const FIRM_GREEN: DecisionSpec = DecisionSpec {
    id: D::FirmGreen,
    system: SystemId::Investment,
    inputs: &[
        Q::Footprint,
        Q::CarbonPrice,
        Q::Revenue,
        Q::EmissionsCap,
        Q::GreenSubsidy,
        Q::TreasuryCash,
        Q::Cash,
        Q::CashReserve,
        Q::GreenTech,
    ],
    outputs: &[Q::GreenTech, Q::GreenPlan],
    params: &[
        "invest.green_payback_years",
        "invest.green_cost_share",
        "invest.green_spend_share",
        "invest.cap_binding_share",
        "invest.green_max",
        "invest.green_step",
    ],
    rule: "carbon_payback",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GreenView {
    pub footprint: f64,
    /// The carbon price firms plan with: the effective price pulled toward a pending one.
    pub carbon_price: Cents,
    pub revenue: Cents,
    pub emissions_cap: f64,
    pub green_subsidy: f64,
    pub treasury_cash: Cents,
    pub cash: Cents,
    pub cash_reserve: Cents,
    pub green_tech: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GreenChoice {
    /// Buy a step this month.
    pub buys: bool,
    /// Worthwhile but not affordable: keep cash back for it.
    pub plan: bool,
    pub cost: Cents,
    pub gov_share: Cents,
    pub firm_share: Cents,
    pub green_tech: f64,
}

pub fn green_view(s: &State, p: &Params, f: &Firm, cash: Cents, reserve: Cents) -> GreenView {
    GreenView {
        footprint: f.footprint.0,
        carbon_price: announced_carbon_price(s, p),
        revenue: f.revenue,
        emissions_cap: s.effective.emissions_cap.0,
        green_subsidy: s.policy.green_subsidy.0,
        treasury_cash: s.gov.cash,
        cash,
        cash_reserve: reserve,
        green_tech: f.green_tech.0,
    }
}

/// Spread over months: a tick buys the share `part` of a step, never more than a set fraction of
/// the month's revenue (smooth transition, no lump).
pub fn decide_green(v: &GreenView, ip: &InvestParams) -> GreenChoice {
    let annual_carbon = v.footprint * MONTHS_PER_YEAR * v.carbon_price.as_f64();
    let full_cost = v.revenue.scale(MONTHS_PER_YEAR * ip.green_cost_share);
    let cost = full_cost.min(v.revenue.scale(ip.green_spend_share));
    let part = safe_div(cost.as_f64(), full_cost.as_f64(), 0.0);
    let cap = v.emissions_cap;
    let cap_binding = cap > 0.0 && v.footprint >= ip.cap_binding_share * cap;
    let worthwhile = (annual_carbon * ip.green_payback_years > full_cost.as_f64() || cap_binding)
        && v.green_tech < ip.green_max
        && cost > Cents::ZERO;
    let mut c = GreenChoice {
        buys: false,
        plan: false,
        cost,
        gov_share: Cents::ZERO,
        firm_share: Cents::ZERO,
        green_tech: v.green_tech,
    };
    if worthwhile {
        // The subsidy is limited to the treasury's cash; the firm bears any remainder.
        c.gov_share = cost
            .scale(v.green_subsidy)
            .min(v.treasury_cash.max(Cents::ZERO));
        c.firm_share = cost - c.gov_share;
        if v.cash - v.cash_reserve >= c.firm_share {
            c.buys = true;
            c.green_tech = 1.0 - (1.0 - v.green_tech) * pow(1.0 - ip.green_step, part);
        } else {
            c.plan = true;
        }
    }
    c
}

// ---- Decision: next tick's capital purchase and the loan to fund it ----

pub const FIRM_CAPEX: DecisionSpec = DecisionSpec {
    id: D::FirmCapex,
    system: SystemId::Investment,
    inputs: &[
        Q::Capital,
        Q::ProfitTrend,
        Q::Profit,
        Q::LoanRate,
        Q::InvestHurdle,
        Q::ExpectedSales,
        Q::CapitalPerSale,
        Q::Distressed,
        Q::Cash,
        Q::CashReserve,
        Q::GreenPlan,
        Q::GreenCashShare,
    ],
    outputs: &[Q::InvestmentPlan, Q::LoanRequest, Q::ProfitTrend],
    params: &[
        "invest.capital_price_usd",
        "invest.smoothing",
        "invest.depreciation",
        "invest.accelerator",
        "invest.max_invest_share",
        "invest.return_scale",
        "invest.retain_share",
    ],
    rule: "accelerator_with_return_gate",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapexView {
    /// Capital after depreciation and this tick's purchases.
    pub capital: f64,
    pub profit_trend: Cents,
    pub profit: Cents,
    pub loan_rate: f64,
    /// The economic system's required return above the loan rate.
    pub invest_hurdle: f64,
    pub expected_sales: f64,
    pub capital_per_sale: f64,
    /// In arrears for months, or could not pay wages.
    pub distressed: bool,
    pub cash: Cents,
    pub cash_reserve: Cents,
    pub green_plan: bool,
    /// The share of cash held back for a planned green step (set by the economic system).
    pub green_cash_share: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapexChoice {
    pub profit_ema: Cents,
    pub plan: Cents,
    pub request: Cents,
}

pub fn capex_view(
    s: &State,
    p: &Params,
    f: &Firm,
    capital: f64,
    cash: Cents,
    reserve: Cents,
    green_plan: bool,
) -> CapexView {
    let c = cfg(s, p);
    CapexView {
        capital,
        profit_trend: f.profit_ema,
        profit: f.profit,
        loan_rate: f.loan_rate.0,
        invest_hurdle: c.invest_hurdle,
        expected_sales: f.expected_sales.0,
        capital_per_sale: f.capital_per_sale,
        distressed: f.distress_months > 0 || f.wage_shortfall,
        cash,
        cash_reserve: reserve,
        green_plan,
        green_cash_share: c.green_cash_share,
    }
}

pub fn decide_capex(v: &CapexView, ip: &InvestParams) -> CapexChoice {
    let kp = ip.capital_price_usd * 100.0;
    let k = v.capital;
    let k_value = (k * kp).max(1.0);
    let profit_ema = Cents::from_f64(
        (1.0 - ip.smoothing) * v.profit_trend.as_f64() + ip.smoothing * v.profit.as_f64(),
    );
    let ret = MONTHS_PER_YEAR * profit_ema.as_f64() / k_value;
    let thr = v.loan_rate + v.invest_hurdle;
    let gate = (0.5 + (ret - thr) / (2.0 * ip.return_scale)).clamp(0.0, 1.0);
    let gap = v.expected_sales * v.capital_per_sale - k;
    let wanted = ip.depreciation * k + ip.accelerator * gap * if gap > 0.0 { gate } else { 1.0 };
    let units = wanted.clamp(0.0, (ip.depreciation + ip.max_invest_share) * k);
    let (plan, request) = if !v.distressed && v.capital_per_sale > 0.0 {
        let target = Cents::from_f64(units * kp);
        let mut free = (v.cash - v.cash_reserve).max(Cents::ZERO);
        if v.green_plan {
            free = free.scale(1.0 - v.green_cash_share);
        }
        let from_cash = free.scale(ip.retain_share);
        (target, (target - from_cash).max(Cents::ZERO))
    } else {
        (Cents::ZERO, Cents::ZERO)
    };
    CapexChoice {
        profit_ema,
        plan,
        request,
    }
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let ip = &p.invest;
    let kp = ip.capital_price_usd * 100.0;
    let lw = labour_weights(s);
    let gov = Account::Government;
    let mut clear_usd = 0.0;

    for f in s.firms.iter().filter(|f| f.active) {
        let acct = Account::Firm(f.id);
        let reserve = cash_reserve(s, p, f);
        let cash = f.cash;
        let mut cash = cash;
        let mut k = f.capital.0 * (1.0 - ip.depreciation);

        // Capital goods bought from the goods sector in S3 this tick add to the capital stock.
        // The payment is already the goods sector's revenue, so it is not added to GDP again.
        if f.capex > Cents::ZERO {
            k += f.capex.as_f64() / kp;
            clear_usd += f.capex.dollars() * p.nature.forest_clear_per_build;
        }

        // Green investment: pays off when the carbon bill over the payback period exceeds the cost,
        // or when an emissions cap binds. Firms read an announced (pending) carbon price with a
        // discount.
        let green = decide_green(&green_view(s, p, f, cash, reserve), ip);
        let green_plan = green.plan;
        let mut green_tech = f.green_tech.0;
        if green.buys {
            for (h, part) in apportion(green.firm_share, &lw).into_iter().enumerate() {
                d.transfer_as(
                    acct,
                    Account::Household(HouseholdId(h as u32)),
                    part,
                    FlowKind::Wage,
                    Purpose::InvestmentLabour,
                );
            }
            for (h, part) in apportion(green.gov_share, &lw).into_iter().enumerate() {
                d.transfer_as(
                    gov,
                    Account::Household(HouseholdId(h as u32)),
                    part,
                    FlowKind::Wage,
                    Purpose::GreenSubsidy,
                );
            }
            cash -= green.firm_share;
            green_tech = green.green_tech;
            d.changes.push(Delta::Gdp(GdpPart::Investment, green.cost));
        }

        // Plan next tick's capital goods purchase: replacement plus an accelerator term toward the
        // capital that expected sales call for. Profit against the hurdle gates expansion only.
        let capex = decide_capex(&capex_view(s, p, f, k, cash, reserve, green_plan), ip);
        let (plan, request, profit_ema) = (capex.plan, capex.request, capex.profit_ema);
        d.firm(
            f.id,
            FirmPatch {
                capital: Some(Units(k)),
                green_tech: Some(Rate(green_tech)),
                green_plan: Some(green_plan),
                plan_invest: Some(plan),
                profit_ema: Some(profit_ema),
                loan_request: Some(request),
                ..FirmPatch::default()
            },
        );
    }

    if clear_usd > 0.0 {
        let mut n = s.nature.clone();
        n.pending_clearing += Units(clear_usd);
        d.changes.push(Delta::Nature(n));
    }
    d
}
