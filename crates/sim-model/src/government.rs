//! S8 government: collect taxes, pay benefits, build and maintain infrastructure,
//! borrow from the bank for deficits, update legitimacy.

use crate::util::{apportion, cfg};
use sim_metrics::cohort_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Cohorts that supply labour, as weights for paying construction and maintenance work.
pub fn labour_weights(s: &State) -> Vec<f64> {
    s.households
        .iter()
        .map(|h| h.weight.0 * h.labour_share.0)
        .collect()
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let gp = &p.government;
    let gov = Account::Government;
    let n_h = s.households.len();

    // ---- Taxes ----
    let wealth = cohort_wealth(s, p);
    let mut taxes = Cents::ZERO;
    for (i, h) in s.households.iter().enumerate() {
        let me = Account::Household(h.id);
        let wage_tax = h.income.wages.scale(s.policy.tax_wage.0);
        let cap_tax = h.income.capital_income().scale(s.policy.tax_capital.0);
        let pc = wealth[i] / 100.0 / h.weight.0.max(1e-9);
        let over = (pc - s.policy.wealth_threshold.dollars()).max(0.0);
        let wealth_tax = Cents::from_dollars(over * h.weight.0 * s.effective.wealth_tax.0 / 12.0);
        for amt in [wage_tax, cap_tax, wealth_tax] {
            let paid = amt.min(h.savings.max(Cents::ZERO));
            taxes += paid;
            d.transfer(me, gov, amt, FlowKind::Tax);
        }
    }
    // Firm-side revenue already collected in S4 this tick.
    let t_prof = s.effective.tax_profit.0;
    let firm_side: Cents = Cents::sum(s.firms.iter().filter(|f| f.active).map(|f| {
        f.carbon_cost
            + f.profit
                .max(Cents::ZERO)
                .scale(safe_div(t_prof, 1.0 - t_prof, 0.0))
    }));
    let revenue = taxes + firm_side;

    // ---- Planned spending ----
    let mut spend = Deltas::new();
    let mut benefits = Cents::ZERO;
    let mut ubi = Cents::ZERO;
    for h in &s.households {
        let unemployed = (h.labour_force().0 - h.employed().0).max(0.0);
        let nonlabour = if h.nonlabour_benefit {
            h.weight.0 - h.labour_force().0
        } else {
            0.0
        };
        let b = s.policy.benefit.scale(unemployed + nonlabour);
        let u = s.policy.ubi.scale(h.weight.0);
        spend.transfer(gov, Account::Household(h.id), b, FlowKind::Benefit);
        spend.transfer(gov, Account::Household(h.id), u, FlowKind::Benefit);
        benefits += b;
        ubi += u;
    }
    let lw = labour_weights(s);
    let plants = (s.energy.fossil_cap.0 + s.energy.renewable_cap.0) / gp.plant_capacity;
    let upkeep = Cents::from_dollars(
        s.housing.public_cap.0 * gp.housing_upkeep_usd + plants * gp.plant_upkeep_usd,
    );
    let mut housing = s.housing.clone();
    let mut energy = s.energy.clone();
    let mut nature = s.nature.clone();
    let mut build_cost = Cents::ZERO;
    for (kind, count) in &s.gov.pending_build {
        let k = *count as f64;
        let cost = match kind {
            InfraKind::PublicHousing => {
                housing.public_cap += Units(k);
                Cents::from_dollars(k * gp.public_housing_cost_usd)
            }
            InfraKind::FossilPlant => {
                energy.fossil_cap += Units(k * gp.plant_capacity);
                Cents::from_dollars(k * gp.plant_cost_usd)
            }
            InfraKind::RenewablePlant => {
                energy.renewable_cap += Units(k * gp.plant_capacity);
                Cents::from_dollars(k * gp.plant_cost_usd)
            }
        };
        build_cost += cost;
        nature.pending_clearing += Units(cost.dollars() * p.nature.forest_clear_per_build);
    }
    let services = upkeep + build_cost;
    // Goods purchases next month: the public_spending share of GDP, scaled up when unemployment
    // is above target and down when below (automatic stabiliser), and cut back as debt rises
    // past the brake threshold. Deficits are financed by borrowing from the bank.
    let gap = s.indicators.unemployment.0 - gp.unemployment_target;
    let cycle = (1.0 + gp.stabilizer * gap).clamp(0.5, 1.6);
    let gdp_annual = s.indicators.gdp_annual.dollars().max(1.0);
    let debt_ratio = s.gov.debt.dollars() / gdp_annual;
    let brake = (1.0 - gp.debt_brake_slope * (debt_ratio - gp.debt_brake_start)).clamp(0.2, 1.0);
    let purchases_next = s
        .indicators
        .gdp_month
        .scale(s.policy.public_spending.0 * cycle * brake);
    // ---- Borrow to cover the plan, or repay a share of any surplus ----
    let outflow = benefits + ubi + services + purchases_next;
    let target_cash = outflow.scale(gp.cash_target_months);
    let available = s.gov.cash + taxes;
    let need = outflow + target_cash - available;
    if need > Cents::ZERO {
        let kappa = s.policy.bank_capital_ratio.0;
        let max_loans = s.bank.equity.max(Cents::ZERO).scale(1.0 / kappa);
        let headroom = (max_loans - s.bank.loans).max(Cents::ZERO);
        let borrow = need.min(headroom);
        d.money.push(MoneyOp::Lend {
            to: gov,
            amount: borrow,
        });
    } else {
        let surplus = -need;
        let repay = surplus.scale(gp.repay_share).min(s.gov.debt);
        d.money.push(MoneyOp::Repay {
            from: gov,
            amount: repay,
        });
    }

    // ---- Pay for services and construction as wages to labour ----
    for (h, part) in apportion(services, &lw).into_iter().enumerate() {
        spend.transfer(
            gov,
            Account::Household(HouseholdId(h as u32)),
            part,
            FlowKind::Wage,
        );
    }
    d.money.extend(spend.money);
    d.changes.push(Delta::Gdp(GdpPart::GovServices, services));
    if !s.gov.pending_build.is_empty() {
        d.changes.push(Delta::Housing(housing));
        d.changes.push(Delta::Energy(energy));
        d.changes.push(Delta::Nature(nature));
    }

    // ---- Legitimacy ----
    let i = &s.indicators;
    let gdp_annual = i.gdp_annual.dollars().max(1.0);
    let mut l = s.gov.legitimacy.0 + gp.legit_wellbeing * (i.avg_wellbeing.0 - 0.5)
        - gp.legit_unemployment * (i.unemployment.0 - gp.legit_unemployment_threshold).max(0.0)
        - gp.legit_gini * (i.gini.0 - gp.legit_gini_threshold).max(0.0)
        - gp.legit_debt * (s.gov.debt.dollars() / gdp_annual - 1.0).max(0.0);
    if s.bank.bailout_this_tick {
        l -= gp.legit_bailout;
    }
    if let Some(t) = &s.transition {
        l -= cfg(s, p).resistance.max(p.systems.get(t.to).resistance);
    }
    let _ = n_h;
    d.changes.push(Delta::Gov(GovPatch {
        legitimacy: Some(Rate(l.clamp(0.0, 1.0))),
        tax_revenue: Some(revenue),
        spending: Some(outflow),
        purchases_budget: Some(purchases_next),
        pending_build: Some(Vec::new()),
    }));
    d
}
