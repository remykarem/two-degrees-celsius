//! S4 income distribution: wages, carbon cost, profit tax, dividends by owner type, rents.
//! This is where the economic system plugs in: `match firm.owner` decides who gets the dividends.

use crate::util::{apportion, monthly};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let n_h = s.households.len();
    let payout = crate::util::cfg(s, p).payout_ratio;

    for f in s.firms.iter().filter(|f| f.active) {
        let fi = f.id.idx();
        let firm_acct = Account::Firm(f.id);
        // Wages: each cohort is paid its employment x wage x premium, scaled down if cash is short.
        let due: Vec<Cents> = (0..n_h)
            .map(|h| {
                let hh = &s.households[h];
                f.wage.scale(hh.employment[fi].0 * hh.wage_premium.0)
            })
            .collect();
        let total_due = Cents::sum(due.iter().copied());
        let scale = if total_due > f.cash {
            f.cash.as_f64() / total_due.as_f64()
        } else {
            1.0
        };
        let mut wages_paid = Cents::ZERO;
        for (h, amt) in due.iter().enumerate() {
            let pay = Cents((amt.as_f64() * scale).floor() as i64);
            wages_paid += pay;
            d.transfer(
                firm_acct,
                Account::Household(HouseholdId(h as u32)),
                pay,
                FlowKind::Wage,
            );
        }
        let mut cash = f.cash - wages_paid;

        // Carbon cost, after lobbying.
        let carbon_due = Cents::from_f64(s.effective.carbon_price.as_f64() * f.emissions.0);
        let carbon = carbon_due.min(cash);
        d.transfer(firm_acct, Account::Government, carbon, FlowKind::Tax);
        cash -= carbon;

        // Profit before and after tax.
        let interest_due = f.debt.scale(monthly(f.loan_rate.0));
        let amort_due = f.amort.min(f.debt);
        let pre_tax = f.revenue - wages_paid - interest_due - carbon;
        let tax_due = pre_tax.max(Cents::ZERO).scale(s.effective.tax_profit.0);
        let tax = tax_due.min(cash);
        d.transfer(firm_acct, Account::Government, tax, FlowKind::Tax);
        cash -= tax;
        let profit = pre_tax - tax;

        // Dividends: payout ratio of profit, never below the cash reserve for payroll and debt service.
        let mut reserve =
            (wages_paid + interest_due + amort_due).scale(p.income.cash_reserve_months);
        if f.green_plan {
            // Retained cash for the planned green investment.
            reserve += f
                .revenue
                .scale(12.0 * p.invest.green_cost_share * (1.0 - s.policy.green_subsidy.0));
        }
        let dividends = profit
            .max(Cents::ZERO)
            .scale(payout)
            .min((cash - reserve).max(Cents::ZERO));
        match f.owner {
            Owner::Household(h) => d.transfer(
                firm_acct,
                Account::Household(h),
                dividends,
                FlowKind::Dividend,
            ),
            Owner::Bank => d.transfer(
                firm_acct,
                Account::Household(s.bank.owner),
                dividends,
                FlowKind::Dividend,
            ),
            Owner::State => d.transfer(
                firm_acct,
                Account::Government,
                dividends,
                FlowKind::Dividend,
            ),
            Owner::Workers => {
                let w: Vec<f64> = (0..n_h).map(|h| s.households[h].employment[fi].0).collect();
                let parts = apportion(dividends, &w);
                for (h, part) in parts.into_iter().enumerate() {
                    d.transfer(
                        firm_acct,
                        Account::Household(HouseholdId(h as u32)),
                        part,
                        FlowKind::Dividend,
                    );
                }
            }
        }
        d.firm(
            f.id,
            FirmPatch {
                profit: Some(profit),
                carbon_cost: Some(carbon),
                tax_paid: Some(tax),
                wage_shortfall: Some(scale < 1.0),
                ..FirmPatch::default()
            },
        );
    }

    // Rents: housed renters pay the private landlord or the government.
    let rental_cap = s.housing.private_cap.0 + s.housing.public_cap.0;
    let occ = s.housing.occupancy.0;
    let unhoused = if occ > 1.0 { 1.0 - 1.0 / occ } else { 0.0 };
    let share_pub = safe_div(s.housing.public_cap.0, rental_cap, 0.0);
    for h in &s.households {
        let housed = h.weight.0 * h.renter_share.0 * (1.0 - unhoused);
        let me = Account::Household(h.id);
        let private = s.housing.rent_private.scale(housed * (1.0 - share_pub));
        let public = s.housing.rent_public.scale(housed * share_pub);
        // Rent is paid out of savings; what cannot be paid is reported as a shortfall.
        let pay_private = private.min(h.savings.max(Cents::ZERO));
        let pay_public = public.min((h.savings - pay_private).max(Cents::ZERO));
        d.transfer(
            me,
            Account::Household(s.housing.owner),
            pay_private,
            FlowKind::Rent,
        );
        d.transfer(me, Account::Government, pay_public, FlowKind::Rent);
        let short = private + public - pay_private - pay_public;
        if short > Cents::ZERO {
            d.events.push(Event::RentShortfall {
                household: h.id,
                amount: short,
            });
        }
    }
    d
}
