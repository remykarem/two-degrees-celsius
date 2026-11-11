//! S4 income distribution: wages, carbon cost, profit tax, dividends by owner type, rents.
//! This is where the economic system plugs in: `match firm.owner` decides who gets the dividends.

use crate::util::{apportion, monthly, person_pay, workers_of};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let n_h = s.households.len();
    let payout = crate::util::cfg(s, p).payout_ratio;
    // What each household receives in this system before it pays rent.
    let mut incoming = vec![Cents::ZERO; n_h];

    for f in s.firms.iter().filter(|f| f.active) {
        let firm_acct = Account::Firm(f.id);
        // Wages: each worker is paid base wage x skill multiple, scaled down if cash is short.
        let workers: Vec<&Person> = workers_of(s, f.id).collect();
        let due: Vec<Cents> = workers
            .iter()
            .map(|x| person_pay(s, p, x, f.wage))
            .collect();
        let total_due = Cents::sum(due.iter().copied());
        let scale = if total_due > f.cash {
            f.cash.as_f64() / total_due.as_f64()
        } else {
            1.0
        };
        let mut wages_paid = Cents::ZERO;
        let mut per_household = vec![Cents::ZERO; n_h];
        for (x, amt) in workers.iter().zip(&due) {
            let pay = Cents((amt.as_f64() * scale).floor() as i64);
            wages_paid += pay;
            per_household[x.household.idx()] += pay;
            d.changes.push(Delta::Person(
                x.id,
                PersonPatch {
                    wage: Some(pay),
                    ..PersonPatch::default()
                },
            ));
        }
        for (h, pay) in per_household.iter().enumerate() {
            d.transfer_as(
                firm_acct,
                Account::Household(HouseholdId(h as u32)),
                *pay,
                FlowKind::Wage,
                Purpose::Wages,
            );
            incoming[h] += *pay;
        }
        let mut cash = f.cash - wages_paid;

        // Carbon cost, after lobbying: on the firm's footprint (direct emissions plus the energy
        // emissions of the electricity it uses).
        let carbon_due = crate::util::carbon_cost(s.effective.carbon_price, f.footprint.0);
        let carbon = carbon_due.min(cash);
        d.transfer_as(
            firm_acct,
            Account::Government,
            carbon,
            FlowKind::Tax,
            Purpose::CarbonTax,
        );
        cash -= carbon;

        // Profit before and after tax.
        let interest_due = f.debt.scale(monthly(f.loan_rate.0));
        let amort_due = f.amort.min(f.debt);
        let pre_tax = f.revenue - wages_paid - interest_due - carbon;
        let tax_due = crate::util::profit_tax_due(pre_tax, s.effective.tax_profit.0);
        let tax = tax_due.min(cash);
        d.transfer_as(
            firm_acct,
            Account::Government,
            tax,
            FlowKind::Tax,
            Purpose::ProfitTax,
        );
        cash -= tax;
        let profit = pre_tax - tax;

        // Dividends: payout ratio of profit, never below the cash reserve for payroll and debt service.
        let mut reserve =
            (wages_paid + interest_due + amort_due).scale(p.income.cash_reserve_months);
        if f.green_plan {
            // Retained cash for the planned green investment.
            reserve += f
                .revenue
                .scale(p.invest.green_spend_share * (1.0 - s.policy.green_subsidy.0));
        }
        let regular = profit
            .max(Cents::ZERO)
            .scale(payout)
            .min((cash - reserve).max(Cents::ZERO));
        // Idle cash well above what payroll and debt service need is distributed too.
        let idle_line =
            reserve + (wages_paid + interest_due + amort_due).scale(p.income.excess_cash_months);
        let excess = (cash - regular - idle_line)
            .max(Cents::ZERO)
            .scale(p.income.excess_payout_rate);
        let dividends = regular + excess;
        match f.owner {
            Owner::Household(h) => {
                d.transfer_as(
                    firm_acct,
                    Account::Household(h),
                    dividends,
                    FlowKind::Dividend,
                    Purpose::FirmDividend,
                );
                incoming[h.idx()] += dividends;
            }
            Owner::Bank => {
                d.transfer_as(
                    firm_acct,
                    Account::Household(s.bank.owner),
                    dividends,
                    FlowKind::Dividend,
                    Purpose::FirmDividend,
                );
                incoming[s.bank.owner.idx()] += dividends;
            }
            Owner::State => d.transfer_as(
                firm_acct,
                Account::Government,
                dividends,
                FlowKind::Dividend,
                Purpose::StateFirmDividend,
            ),
            Owner::Workers => {
                let mut w = vec![0.0; n_h];
                for x in &workers {
                    w[x.household.idx()] += 1.0;
                }
                let parts = apportion(dividends, &w);
                for (h, part) in parts.into_iter().enumerate() {
                    d.transfer_as(
                        firm_acct,
                        Account::Household(HouseholdId(h as u32)),
                        part,
                        FlowKind::Dividend,
                        Purpose::FirmDividend,
                    );
                    incoming[h] += part;
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

    // Town v7: the estate pays its pilot (household to household), out of its savings plus what
    // it received this tick. A short payment counts towards the pilot's dismissal.
    if let Some(e) = s
        .estate
        .as_ref()
        .filter(|e| crate::estate::pilot_employed(s, e))
    {
        let pid = e.pilot.expect("employed pilot exists");
        let pilot = s.person(pid);
        let due = Cents::from_dollars(p.estate.pilot_wage_usd);
        let available =
            s.households[e.household.idx()].savings.max(Cents::ZERO) + incoming[e.household.idx()];
        let pay = due.min(available);
        d.transfer_as(
            Account::Household(e.household),
            Account::Household(pilot.household),
            pay,
            FlowKind::Wage,
            Purpose::DomesticWages,
        );
        incoming[pilot.household.idx()] += pay;
        incoming[e.household.idx()] -= pay;
        d.changes.push(Delta::Person(
            pid,
            PersonPatch {
                wage: Some(pay),
                ..PersonPatch::default()
            },
        ));
        let mut next = e.clone();
        next.wage_paid = pay;
        next.unpaid_months = if pay < due { e.unpaid_months + 1 } else { 0 };
        d.changes.push(Delta::Estate(next));
    }

    // Rents: a household that lives in a home it does not own pays the home's rent to the owner
    // (a landlord household, or the government for public housing), out of its savings plus what
    // it earned this tick. Unpaid rent is arrears; the landlord bears the shortfall. A household
    // without a home pays nothing.
    for h in &s.households {
        let Some(home) = h.home.and_then(|id| s.home(id)) else {
            continue;
        };
        if home.owner == HomeOwner::Household(h.id) {
            continue;
        }
        let me = Account::Household(h.id);
        let available = h.savings.max(Cents::ZERO) + incoming[h.id.idx()];
        let pay = home.rent.min(available);
        match home.owner {
            HomeOwner::Household(o) => d.transfer(me, Account::Household(o), pay, FlowKind::Rent),
            HomeOwner::Public => d.transfer_as(
                me,
                Account::Government,
                pay,
                FlowKind::Rent,
                Purpose::PublicRent,
            ),
        }
        let short = home.rent - pay;
        if short > Cents::ZERO {
            d.events.push(Event::RentShortfall {
                household: h.id,
                amount: short,
            });
            d.household(
                h.id,
                HouseholdPatch {
                    arrears_months: Some(h.arrears_months + 1),
                    ..HouseholdPatch::default()
                },
            );
        } else if h.arrears_months > 0 {
            d.household(
                h.id,
                HouseholdPatch {
                    arrears_months: Some(0),
                    ..HouseholdPatch::default()
                },
            );
        }
    }
    d
}
