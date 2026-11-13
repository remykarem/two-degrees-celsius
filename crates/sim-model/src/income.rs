//! S4 income distribution: wages, carbon cost, profit tax, dividends by owner type, rents.
//! This is where the economic system plugs in: `match firm.owner` decides who gets the dividends.

use crate::util::{apportion, monthly, person_pay, workers_of};
use sim_params::{IncomeParams, InvestParams, Params};
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::Effect as Fx;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;

// ---- Decision: the dividend ----

pub const FIRM_PAYOUT: DecisionSpec = DecisionSpec {
    id: D::FirmPayout,
    system: SystemId::Income,
    inputs: &[
        Q::Profit,
        Q::Cash,
        Q::Payroll,
        Q::DebtService,
        Q::GreenPlan,
        Q::Revenue,
        Q::GreenSubsidy,
        Q::PayoutRatio,
    ],
    outputs: &[Q::Dividends],
    effects: &[
        Fx::up(Q::Profit, Q::Dividends),
        Fx::up(Q::Cash, Q::Dividends),
        Fx::down(Q::Payroll, Q::Dividends),
        Fx::down(Q::DebtService, Q::Dividends),
        Fx::down(Q::GreenPlan, Q::Dividends),
        Fx::down(Q::Revenue, Q::Dividends),
        Fx::up(Q::GreenSubsidy, Q::Dividends),
        Fx::up(Q::PayoutRatio, Q::Dividends),
    ],
    params: &[
        "income.cash_reserve_months",
        "income.excess_cash_months",
        "income.excess_payout_rate",
        "invest.green_spend_share",
    ],
    rule: "payout_ratio_above_reserve",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PayoutView {
    /// Profit after tax this month.
    pub profit: Cents,
    /// Cash left after wages, carbon cost and tax.
    pub cash: Cents,
    pub payroll: Cents,
    /// Interest and scheduled principal due.
    pub debt_service: Cents,
    pub green_plan: bool,
    pub revenue: Cents,
    pub green_subsidy: f64,
    /// The share of profit the economic system pays out.
    pub payout_ratio: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn payout_view(
    s: &State,
    p: &Params,
    f: &Firm,
    profit: Cents,
    cash: Cents,
    wages_paid: Cents,
    interest_due: Cents,
    amort_due: Cents,
) -> PayoutView {
    PayoutView {
        profit,
        cash,
        payroll: wages_paid,
        debt_service: interest_due + amort_due,
        green_plan: f.green_plan,
        revenue: f.revenue,
        green_subsidy: s.policy.green_subsidy.0,
        payout_ratio: crate::util::cfg(s, p).payout_ratio,
    }
}

pub fn decide_payout(v: &PayoutView, ic: &IncomeParams, ip: &InvestParams) -> Cents {
    let outgoings = v.payroll + v.debt_service;
    let mut reserve = outgoings.scale(ic.cash_reserve_months);
    if v.green_plan {
        // Retained cash for the planned green investment.
        reserve += v
            .revenue
            .scale(ip.green_spend_share * (1.0 - v.green_subsidy));
    }
    let regular = v
        .profit
        .max(Cents::ZERO)
        .scale(v.payout_ratio)
        .min((v.cash - reserve).max(Cents::ZERO));
    // Idle cash well above what payroll and debt service need is distributed too.
    let idle_line = reserve + outgoings.scale(ic.excess_cash_months);
    let excess = (v.cash - regular - idle_line)
        .max(Cents::ZERO)
        .scale(ic.excess_payout_rate);
    regular + excess
}

// ---- Mechanisms (causality graph stage 1): declarations of the plain code below; they change no behaviour. ----

/// Firms pay workers the wage, scaled down if cash falls short; the pay lands in the workers' households (`income::run`).
pub const WAGE_PAYMENT: MechanismSpec = MechanismSpec {
    id: M::WagePayment,
    system: SystemId::Income,
    inputs: &[Q::Wage, Q::Workers, Q::Cash],
    outputs: &[Q::Payroll, Q::Savings],
    effects: &[
        Fx::up(Q::Wage, Q::Payroll).system(),
        Fx::up(Q::Workers, Q::Payroll).system(),
        Fx::up(Q::Cash, Q::Payroll).system(),
        Fx::up(Q::Wage, Q::Savings).system(),
        Fx::up(Q::Workers, Q::Savings).system(),
        Fx::up(Q::Cash, Q::Savings).system(),
    ],
    params: &[],
    rule: "pay_up_to_cash",
};

/// Dividends and rent are paid to their owners' households (`income::run`).
pub const HOUSEHOLD_INCOME_FLOWS: MechanismSpec = MechanismSpec {
    id: M::HouseholdIncomeFlows,
    system: SystemId::Income,
    inputs: &[Q::Dividends, Q::Rent],
    outputs: &[Q::Savings, Q::CapitalIncome],
    effects: &[
        Fx::up(Q::Dividends, Q::Savings).system(),
        Fx::up(Q::Dividends, Q::CapitalIncome).system(),
        Fx::varies(
            Q::Rent,
            Q::Savings,
            "renters pay rent out of savings and their landlords receive it",
        )
        .system(),
        Fx::up(Q::Rent, Q::CapitalIncome).system(),
    ],
    params: &[],
    rule: "dividends_and_rent_received",
};

/// Profit is revenue less wages, interest, the carbon charge and profit tax (`income::run`).
pub const FIRM_PROFIT: MechanismSpec = MechanismSpec {
    id: M::FirmProfit,
    system: SystemId::Income,
    inputs: &[
        Q::Revenue,
        Q::Payroll,
        Q::DebtService,
        Q::CarbonPrice,
        Q::Footprint,
        Q::ProfitTaxRate,
    ],
    outputs: &[Q::Profit],
    effects: &[
        Fx::up(Q::Revenue, Q::Profit).system(),
        Fx::down(Q::Payroll, Q::Profit).system(),
        Fx::down(Q::DebtService, Q::Profit).system(),
        Fx::down(Q::CarbonPrice, Q::Profit).system(),
        Fx::down(Q::Footprint, Q::Profit).system(),
        Fx::down(Q::ProfitTaxRate, Q::Profit).system(),
    ],
    params: &[],
    rule: "revenue_less_costs_and_taxes",
};

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let n_h = s.households.len();
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
        let dividends = decide_payout(
            &payout_view(s, p, f, profit, cash, wages_paid, interest_due, amort_due),
            &p.income,
            &p.invest,
        );
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
