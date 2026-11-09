//! S6 investment and technology: the growth engine. Firms invest when the expected return
//! beats the loan rate plus a hurdle; carbon costs and regulation trigger green investment;
//! the landlord builds housing when rents justify it.

use crate::government::labour_weights;
use crate::util::{apportion, cfg, monthly};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let ip = &p.invest;
    let c = cfg(s, p);
    let kp = ip.capital_price_usd * 100.0;
    let lw = labour_weights(s);
    let gov = Account::Government;
    let mut clear_usd = 0.0;

    for f in s.firms.iter().filter(|f| f.active) {
        let acct = Account::Firm(f.id);
        let wage_bill = f.wage.scale(f.workers.0);
        let service = f.debt.scale(monthly(f.loan_rate.0)) + f.amort.min(f.debt);
        let reserve = (wage_bill + service).scale(ip.min_cash_months);
        let mut cash = f.cash;
        let mut k = f.capital.0 * (1.0 - ip.depreciation);

        // Execute last tick's capital plan (a loan may have arrived in S5).
        let mut avail = (cash - reserve).max(Cents::ZERO);
        if f.green_plan {
            avail = avail.scale(1.0 - c.green_cash_share);
        }
        let spend_k = f.plan_invest.min(avail);
        if spend_k > Cents::ZERO {
            for (h, part) in apportion(spend_k, &lw).into_iter().enumerate() {
                d.transfer(acct, Account::Household(HouseholdId(h as u32)), part, FlowKind::Wage);
            }
            k += spend_k.as_f64() / kp;
            cash -= spend_k;
            d.changes.push(Delta::Gdp(GdpPart::Investment, spend_k));
            clear_usd += spend_k.dollars() * p.nature.forest_clear_per_build;
        }

        // Green investment: pays off when the carbon bill over the payback period exceeds the cost,
        // or when an emissions cap binds.
        let annual_carbon = f.carbon_cost.as_f64() * 12.0;
        let cost = f.revenue.scale(12.0 * ip.green_cost_share);
        let cap = s.effective.emissions_cap.0;
        let cap_binding = cap > 0.0 && f.emissions.0 >= 0.999 * cap;
        let worthwhile = (annual_carbon * ip.green_payback_years > cost.as_f64() || cap_binding)
            && f.green_tech.0 < ip.green_max
            && cost > Cents::ZERO;
        let mut green_tech = f.green_tech.0;
        let mut green_plan = false;
        if worthwhile {
            let sub = s.policy.green_subsidy.0;
            let firm_share = cost.scale(1.0 - sub);
            let gov_share = cost - firm_share;
            if cash - reserve >= firm_share {
                for (h, part) in apportion(firm_share, &lw).into_iter().enumerate() {
                    d.transfer(acct, Account::Household(HouseholdId(h as u32)), part, FlowKind::Wage);
                }
                for (h, part) in apportion(gov_share, &lw).into_iter().enumerate() {
                    d.transfer(gov, Account::Household(HouseholdId(h as u32)), part, FlowKind::Wage);
                }
                cash -= firm_share;
                green_tech = 1.0 - (1.0 - green_tech) * (1.0 - ip.green_step);
                d.changes.push(Delta::Gdp(GdpPart::Investment, cost));
            } else {
                green_plan = true;
            }
        }

        // Plan next tick's capital investment.
        let k_value = (k * kp).max(1.0);
        let profit_ema = Cents::from_f64((1.0 - ip.smoothing) * f.profit_ema.as_f64() + ip.smoothing * f.profit.as_f64());
        let ret = 12.0 * profit_ema.as_f64() / k_value;
        let thr = f.loan_rate.0 + c.invest_hurdle;
        let distressed = f.distress_months > 0 || f.wage_shortfall;
        let (plan, request) = if ret > thr && !distressed {
            let intensity = ((ret - thr) / ip.return_scale).clamp(0.0, 1.0);
            let target = Cents::from_f64(k_value * ip.max_invest_share * intensity);
            let mut free = (cash - reserve).max(Cents::ZERO);
            if green_plan {
                free = free.scale(1.0 - c.green_cash_share);
            }
            let from_cash = free.scale(ip.retain_share);
            (target, (target - from_cash).max(Cents::ZERO))
        } else {
            (Cents::ZERO, Cents::ZERO)
        };
        let plan = Cents::from_f64((1.0 - ip.smoothing) * f.plan_invest.as_f64() + ip.smoothing * plan.as_f64());
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

    // ---- Landlord builds housing when occupancy is high and the rent yield beats the hurdle ----
    let owner = &s.households[s.housing.owner.idx()];
    let housing_cost = Cents::from_dollars(ip.housing_cost_usd);
    let rental_cap = s.housing.private_cap.0 + s.housing.public_cap.0;
    let renters: f64 = s.households.iter().map(|h| h.weight.0 * h.renter_share.0).sum();
    let rent_yield = 12.0 * s.housing.rent_private.as_f64() / housing_cost.as_f64();
    let needed = renters / ip.housing_target_occupancy - rental_cap;
    if needed >= 1.0 && rent_yield > s.policy.base_rate.0 + ip.housing_hurdle {
        let budget = owner.savings.scale(0.5);
        let units = needed.min(budget.as_f64() / housing_cost.as_f64()).floor();
        if units >= 1.0 {
            let spend = housing_cost.scale(units);
            for (h, part) in apportion(spend, &lw).into_iter().enumerate() {
                d.transfer(
                    Account::Household(owner.id),
                    Account::Household(HouseholdId(h as u32)),
                    part,
                    FlowKind::Wage,
                );
            }
            let mut housing = s.housing.clone();
            housing.private_cap += Units(units);
            d.changes.push(Delta::Housing(housing));
            d.changes.push(Delta::Gdp(GdpPart::Investment, spend));
            clear_usd += spend.dollars() * p.nature.forest_clear_per_build;
        }
    }
    if clear_usd > 0.0 {
        let mut n = s.nature.clone();
        n.pending_clearing += Units(clear_usd);
        d.changes.push(Delta::Nature(n));
    }
    d
}
