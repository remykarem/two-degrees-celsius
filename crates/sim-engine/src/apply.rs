//! Applies a system's deltas to the state, in a fixed order: money operations first,
//! then field changes, then events and trace. Returns the money ledger for the checks.

use sim_checks::MoneyLedger;
use sim_params::Params;
use sim_types::*;

macro_rules! set_fields {
    ($target:expr, $patch:expr; $($f:ident),* $(,)?) => {
        $( if let Some(v) = $patch.$f { $target.$f = v; } )*
    };
}

fn balance(s: &State, a: Account) -> Cents {
    match a {
        Account::Household(h) => s.households[h.idx()].savings,
        Account::Firm(f) => s.firms[f.idx()].cash,
        Account::Government => s.gov.cash,
    }
}

fn add_balance(s: &mut State, a: Account, d: Cents) {
    match a {
        Account::Household(h) => s.households[h.idx()].savings += d,
        Account::Firm(f) => s.firms[f.idx()].cash += d,
        Account::Government => s.gov.cash += d,
    }
}

fn debt_of(s: &State, a: Account) -> Cents {
    match a {
        Account::Household(h) => s.households[h.idx()].debt,
        Account::Firm(f) => s.firms[f.idx()].debt,
        Account::Government => s.gov.debt,
    }
}

fn add_debt(s: &mut State, a: Account, d: Cents) {
    match a {
        Account::Household(h) => s.households[h.idx()].debt += d,
        Account::Firm(f) => s.firms[f.idx()].debt += d,
        Account::Government => s.gov.debt += d,
    }
}

/// Totals the engine reports for diagnostics.
#[derive(Clone, Copy, Debug, Default)]
pub struct ApplyStats {
    /// Money that a payer could not cover and that was therefore not paid.
    pub clamped: Cents,
}

pub fn apply(
    s: &mut State,
    p: &Params,
    d: Deltas,
    events: &mut Vec<Event>,
    trace: &mut Vec<TraceRecord>,
    stats: &mut ApplyStats,
) -> MoneyLedger {
    let mut ledger = MoneyLedger::default();
    for op in d.money {
        apply_money(s, p, op, &mut ledger, stats);
    }
    for c in d.changes {
        apply_change(s, c);
    }
    events.extend(d.events);
    trace.extend(d.trace);
    ledger
}

fn apply_money(s: &mut State, p: &Params, op: MoneyOp, l: &mut MoneyLedger, st: &mut ApplyStats) {
    match op {
        MoneyOp::Transfer {
            from,
            to,
            amount,
            kind,
        } => {
            let amt = amount.min(balance(s, from)).max(Cents::ZERO);
            st.clamped += amount - amt;
            if amt == Cents::ZERO {
                return;
            }
            add_balance(s, from, -amt);
            add_balance(s, to, amt);
            record_flow(s, from, to, amt, kind);
        }
        MoneyOp::Lend { to, amount } => {
            if amount <= Cents::ZERO {
                return;
            }
            add_balance(s, to, amount);
            add_debt(s, to, amount);
            s.bank.loans += amount;
            s.bank.deposits += amount;
            l.created += amount;
            if let Account::Firm(f) = to {
                let term = p.credit.firm_term_months;
                s.firms[f.idx()].amort += amount.scale(1.0 / term);
            }
        }
        MoneyOp::Repay { from, amount } => {
            let amt = amount
                .min(balance(s, from))
                .min(debt_of(s, from))
                .max(Cents::ZERO);
            if amt == Cents::ZERO {
                return;
            }
            add_balance(s, from, -amt);
            add_debt(s, from, -amt);
            s.bank.loans -= amt;
            s.bank.deposits -= amt;
            l.destroyed += amt;
            if let Account::Household(h) = from {
                s.households[h.idx()].income.loan_payments += amt;
            }
        }
        MoneyOp::PayInterest { from, amount } => {
            let amt = amount.min(balance(s, from)).max(Cents::ZERO);
            st.clamped += amount - amt;
            if amt == Cents::ZERO {
                return;
            }
            add_balance(s, from, -amt);
            s.bank.equity += amt;
            s.bank.deposits -= amt;
            l.destroyed += amt;
            if let Account::Household(h) = from {
                s.households[h.idx()].income.loan_payments += amt;
            }
        }
        MoneyOp::DepositInterest { to, amount } => {
            let amt = amount.min(s.bank.equity).max(Cents::ZERO);
            if amt == Cents::ZERO {
                return;
            }
            add_balance(s, to, amt);
            s.bank.equity -= amt;
            s.bank.deposits += amt;
            l.created += amt;
            if let Account::Household(h) = to {
                s.households[h.idx()].income.interest_received += amt;
            }
        }
        MoneyOp::BankDividend { to, amount } => {
            let amt = amount.min(s.bank.equity).max(Cents::ZERO);
            if amt == Cents::ZERO {
                return;
            }
            s.households[to.idx()].savings += amt;
            s.bank.equity -= amt;
            s.bank.deposits += amt;
            l.created += amt;
            s.households[to.idx()].income.dividends += amt;
        }
        MoneyOp::WriteOff { who, amount } => {
            let amt = amount.min(debt_of(s, who)).max(Cents::ZERO);
            add_debt(s, who, -amt);
            s.bank.loans -= amt;
            s.bank.equity -= amt;
        }
        MoneyOp::Bailout { amount } => {
            s.gov.debt += amount;
            s.bank.loans += amount;
            s.bank.equity += amount;
        }
        MoneyOp::Haircut { rate } => {
            let r = rate.0.clamp(0.0, 1.0);
            let mut total = Cents::ZERO;
            for h in &mut s.households {
                let loss = Cents((h.savings.0 as f64 * r).floor() as i64);
                h.savings -= loss;
                total += loss;
            }
            for f in &mut s.firms {
                let loss = Cents((f.cash.0 as f64 * r).floor() as i64);
                f.cash -= loss;
                total += loss;
            }
            let loss = Cents((s.gov.cash.0 as f64 * r).floor() as i64);
            s.gov.cash -= loss;
            total += loss;
            s.bank.equity += total;
            s.bank.deposits -= total;
            l.destroyed += total;
        }
        MoneyOp::MoveDebt { from, to, amount } => {
            let amt = amount.min(s.households[from.idx()].debt).max(Cents::ZERO);
            s.households[from.idx()].debt -= amt;
            s.households[to.idx()].debt += amt;
        }
    }
}

/// Record household income by type, so the inspector and class function can read it.
fn record_flow(s: &mut State, from: Account, to: Account, amt: Cents, kind: FlowKind) {
    if let Account::Household(h) = to {
        let inc = &mut s.households[h.idx()].income;
        match kind {
            FlowKind::Wage | FlowKind::Lobby => inc.wages += amt,
            FlowKind::Dividend => inc.dividends += amt,
            FlowKind::Rent => inc.rent_received += amt,
            FlowKind::Benefit => inc.benefits += amt,
            FlowKind::Tax | FlowKind::Purchase | FlowKind::Neutral => {}
        }
    }
    if let Account::Household(h) = from {
        let inc = &mut s.households[h.idx()].income;
        match kind {
            FlowKind::Tax => inc.tax_paid += amt,
            FlowKind::Rent => inc.rent_paid += amt,
            FlowKind::Purchase => inc.spending += amt,
            FlowKind::Lobby => inc.lobby_spent += amt,
            FlowKind::Wage | FlowKind::Dividend | FlowKind::Benefit | FlowKind::Neutral => {}
        }
    }
}

fn apply_change(s: &mut State, c: Delta) {
    match c {
        Delta::Employment { h, f, n } => s.households[h.idx()].employment[f.idx()] = n,
        Delta::Firm(id, p) => {
            let f = &mut s.firms[id.idx()];
            set_fields!(f, p; active, owner, capital, workers, wage, price, inventory, output,
                emissions, green_tech, tfp, sales_hist, sales_units, revenue, profit, unfilled,
                over_target_months, distress_months, wage_shortfall, plan_invest, loan_request,
                loan_rate, amort, green_plan, closed_tick, excess_demand, carbon_cost, profit_ema);
        }
        Delta::Household(id, p) => {
            let h = &mut s.households[id.idx()];
            set_fields!(h, p; weight, skill, class, wellbeing, political_power, last_disposable,
                food_met, housed_share, consumption, footprint, wealth_pc, arrears);
            if let Some(x) = p.push_income_hist {
                h.income_hist.push(x);
                if h.income_hist.len() > 6 {
                    h.income_hist.remove(0);
                }
            }
        }
        Delta::Bank(p) => {
            set_fields!(s.bank, p; default_rate, failed, bailout_this_tick, lent_this_tick);
        }
        Delta::Gov(p) => {
            set_fields!(s.gov, p; legitimacy, purchases_budget, tax_revenue, spending, pending_build);
        }
        Delta::Nature(n) => s.nature = n,
        Delta::Housing(h) => s.housing = h,
        Delta::Energy(e) => s.energy = e,
        Delta::Effective(e) => s.effective = e,
        Delta::System(x) => s.system = x,
        Delta::Transition(t) => s.transition = t,
        Delta::OutputPenalty(r) => s.output_penalty = r,
        Delta::UnionDensity(r) => s.union_density = r,
        Delta::NetLobby(r) => s.net_lobby = r,
        Delta::Gdp(part, c) => match part {
            GdpPart::Investment => s.gdp_parts.investment += c,
            GdpPart::GovServices => s.gdp_parts.gov_services += c,
        },
        Delta::Outcome(o) => s.outcome = o,
        Delta::SetPolicy(c) => set_policy(s, c),
        Delta::ClearPendingCommons => s.nature.pending_clearing = Units::ZERO,
        Delta::ResetFlows => {
            for h in &mut s.households {
                h.income = IncomeFlows::default();
            }
            s.gdp_parts = GdpParts::default();
            s.bank.bailout_this_tick = false;
            s.bank.lent_this_tick = Cents::ZERO;
        }
    }
}

/// Set one policy lever.
pub fn set_policy(s: &mut State, c: PolicyChange) {
    let pol = &mut s.policy;
    match c {
        PolicyChange::CarbonPrice(v) => pol.carbon_price = v.max(Cents::ZERO),
        PolicyChange::TaxWage(v) => pol.tax_wage = clamp01(v),
        PolicyChange::TaxProfit(v) => pol.tax_profit = clamp01(v),
        PolicyChange::TaxCapital(v) => pol.tax_capital = clamp01(v),
        PolicyChange::WealthTax(v) => pol.wealth_tax = clamp01(v),
        PolicyChange::WealthThreshold(v) => pol.wealth_threshold = v.max(Cents::ZERO),
        PolicyChange::BaseRate(v) => pol.base_rate = Rate(v.0.clamp(0.0, 0.5)),
        PolicyChange::MinWage(v) => pol.min_wage = v.max(Cents::ZERO),
        PolicyChange::Benefit(v) => pol.benefit = v.max(Cents::ZERO),
        PolicyChange::Ubi(v) => pol.ubi = v.max(Cents::ZERO),
        PolicyChange::GreenSubsidy(v) => pol.green_subsidy = clamp01(v),
        PolicyChange::CampaignCap(v) => pol.campaign_cap = clamp01(v),
        PolicyChange::EmissionsCap(v) => pol.emissions_cap = v.max(Tonnes::ZERO),
        PolicyChange::CommonsHarvest(v) => pol.commons_harvest = Rate(v.0.clamp(0.0, 10.0)),
        PolicyChange::CommonsAccess(a) => s.nature.commons_access = a,
        PolicyChange::BankCapitalRatio(v) => pol.bank_capital_ratio = Rate(v.0.clamp(0.01, 1.0)),
        PolicyChange::Bailout(b) => pol.bailout = b,
        PolicyChange::PublicSpending(v) => pol.public_spending = clamp01(v),
        PolicyChange::JobGuarantee(v) => pol.job_guarantee = clamp01(v),
    }
}

fn clamp01(r: Rate) -> Rate {
    Rate(r.0.clamp(0.0, 1.0))
}
