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
    /// The single largest clamp this tick: what it was for and by how much.
    pub worst: Option<(&'static str, Cents)>,
}

impl ApplyStats {
    fn clamp(&mut self, what: &'static str, short: Cents) {
        if short > Cents::ZERO {
            self.clamped += short;
            if self.worst.is_none_or(|(_, w)| short > w) {
                self.worst = Some((what, short));
            }
        }
    }
}

/// What the engine records while applying deltas: every money movement as a `Transaction`
/// (with the amount actually moved, after clamping) and wages earned by persons.
#[derive(Clone, Debug, Default)]
pub struct Capture {
    pub transactions: Vec<Transaction>,
    pub earnings: Vec<Earning>,
}

impl Capture {
    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        tick: u32,
        from: LedgerAccount,
        to: LedgerAccount,
        amount: Cents,
        kind: LedgerKind,
        detail: &'static str,
        posting: Posting,
    ) {
        if amount != Cents::ZERO {
            self.transactions.push(Transaction::new(
                tick, from, to, amount, kind, detail, posting,
            ));
        }
    }
}

pub fn apply(
    s: &mut State,
    p: &Params,
    d: Deltas,
    events: &mut Vec<Event>,
    trace: &mut Vec<TraceRecord>,
    stats: &mut ApplyStats,
    cap: &mut Capture,
) -> MoneyLedger {
    let mut ledger = MoneyLedger::default();
    for op in d.money {
        apply_money(s, p, op, &mut ledger, stats, cap);
    }
    for c in d.changes {
        capture_earning(s, &c, cap);
        apply_change(s, c, p.class.history_months);
    }
    events.extend(d.events);
    trace.extend(d.trace);
    ledger
}

/// Wages a person earns this tick come from `Delta::Person` patches with a wage.
fn capture_earning(s: &State, c: &Delta, cap: &mut Capture) {
    if let Delta::Person(id, patch) = c {
        if let Some(w) = patch.wage.filter(|w| *w > Cents::ZERO) {
            let x = &s.persons[id.idx()];
            let job = patch.job.unwrap_or(x.job);
            cap.earnings.push(Earning {
                tick: s.tick,
                person: *id,
                household: x.household,
                firm: job,
                amount: w,
            });
        }
    }
}

/// Apply one money operation and record what it did. Every balance in the model changes here
/// and nowhere else; the recorded transactions are the ledger of the tick.
fn apply_money(
    s: &mut State,
    p: &Params,
    op: MoneyOp,
    l: &mut MoneyLedger,
    st: &mut ApplyStats,
    cap: &mut Capture,
) {
    let tick = s.tick;
    use LedgerAccount as A;
    match op {
        MoneyOp::Transfer {
            from,
            to,
            amount,
            kind,
            purpose,
        } => {
            let amt = amount.min(balance(s, from)).max(Cents::ZERO);
            st.clamp(flow_name(kind), amount - amt);
            if amt == Cents::ZERO {
                return;
            }
            add_balance(s, from, -amt);
            add_balance(s, to, amt);
            record_flow(s, from, to, amt, kind);
            let (lk, detail) = purpose.resolve(kind);
            cap.push(
                tick,
                from.into(),
                to.into(),
                amt,
                lk,
                detail,
                Posting::Transfer,
            );
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
            cap.push(
                tick,
                A::External(External::CreditCreation),
                to.into(),
                amount,
                LedgerKind::LoanNew,
                "new loan",
                Posting::CreditCreate,
            );
        }
        MoneyOp::Repay { from, amount } => {
            let amt = amount
                .min(balance(s, from))
                .min(debt_of(s, from))
                .max(Cents::ZERO);
            st.clamp("repayment", amount.min(debt_of(s, from)) - amt);
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
            cap.push(
                tick,
                from.into(),
                A::External(External::CreditDestruction),
                amt,
                LedgerKind::LoanRepay,
                "loan repayment",
                Posting::CreditRepay,
            );
        }
        MoneyOp::PayInterest { from, amount } => {
            let amt = amount.min(balance(s, from)).max(Cents::ZERO);
            st.clamp("interest", amount - amt);
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
            cap.push(
                tick,
                from.into(),
                A::Bank,
                amt,
                LedgerKind::Interest,
                "loan interest",
                Posting::BankReceives,
            );
        }
        MoneyOp::DepositInterest { to, amount } => {
            let amt = amount.min(s.bank.equity).max(Cents::ZERO);
            st.clamp("deposit interest", amount - amt);
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
            cap.push(
                tick,
                A::Bank,
                to.into(),
                amt,
                LedgerKind::Interest,
                "deposit interest",
                Posting::BankPays,
            );
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
            cap.push(
                tick,
                A::Bank,
                A::Household(to),
                amt,
                LedgerKind::Dividend,
                "bank dividend",
                Posting::BankPays,
            );
        }
        MoneyOp::WriteOff { who, amount } => {
            let amt = amount.min(debt_of(s, who)).max(Cents::ZERO);
            add_debt(s, who, -amt);
            s.bank.loans -= amt;
            s.bank.equity -= amt;
            cap.push(
                tick,
                A::Bank,
                who.into(),
                amt,
                LedgerKind::LoanWriteoff,
                "loan written off",
                Posting::WriteOff,
            );
        }
        MoneyOp::Bailout { amount } => {
            s.gov.debt += amount;
            s.bank.loans += amount;
            s.bank.equity += amount;
            cap.push(
                tick,
                A::Government,
                A::Bank,
                amount,
                LedgerKind::Bailout,
                "bank bailout",
                Posting::DebtToBank,
            );
        }
        MoneyOp::CapitalizeInterest { amount } => {
            let amt = amount.max(Cents::ZERO);
            s.gov.debt += amt;
            s.bank.loans += amt;
            s.bank.equity += amt;
            cap.push(
                tick,
                A::Government,
                A::Bank,
                amt,
                LedgerKind::Interest,
                "unpaid interest added to debt",
                Posting::DebtToBank,
            );
        }
        MoneyOp::Haircut { rate } => {
            // Losses round up to a whole cent (never beyond the balance), so rounding can never
            // leave the bank a few cents short of the equity the haircut was sized to restore.
            let r = rate.0.clamp(0.0, 1.0);
            let mut total = Cents::ZERO;
            let mut losses: Vec<(LedgerAccount, Cents)> = Vec::new();
            for h in &mut s.households {
                let loss =
                    Cents(((h.savings.0 as f64 * r).ceil() as i64).clamp(0, h.savings.0.max(0)));
                h.savings -= loss;
                total += loss;
                losses.push((A::Household(h.id), loss));
            }
            for f in &mut s.firms {
                let loss = Cents(((f.cash.0 as f64 * r).ceil() as i64).clamp(0, f.cash.0.max(0)));
                f.cash -= loss;
                total += loss;
                losses.push((A::Firm(f.id), loss));
            }
            let loss =
                Cents(((s.gov.cash.0 as f64 * r).ceil() as i64).clamp(0, s.gov.cash.0.max(0)));
            s.gov.cash -= loss;
            total += loss;
            losses.push((A::Government, loss));
            for (who, loss) in losses {
                cap.push(
                    tick,
                    who,
                    A::Bank,
                    loss,
                    LedgerKind::Neutral,
                    "bank failure haircut",
                    Posting::BankReceives,
                );
            }
            s.bank.equity += total;
            s.bank.deposits -= total;
            l.destroyed += total;
        }
    }
}

/// Record household income by type, so the inspector and class function can read it.
fn flow_name(k: FlowKind) -> &'static str {
    match k {
        FlowKind::Wage => "wage",
        FlowKind::Dividend => "dividend",
        FlowKind::Rent => "rent",
        FlowKind::Benefit => "benefit",
        FlowKind::Tax => "tax",
        FlowKind::Purchase => "purchase",
        FlowKind::Lobby => "lobby",
        FlowKind::Neutral => "neutral",
    }
}

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

fn apply_change(s: &mut State, c: Delta, hist_len: usize) {
    match c {
        Delta::Person(id, p) => {
            let x = &mut s.persons[id.idx()];
            set_fields!(x, p; skill, job, jg, staff_of, wage, commute_tiles, commute_minutes,
                effective_hours, wellbeing);
        }
        Delta::Firm(id, p) => {
            let f = &mut s.firms[id.idx()];
            set_fields!(f, p; active, owner, capital, workers, wage, price, inventory, output,
                emissions, footprint, energy_use, tile, green_tech, tfp, sales_hist, sales_units,
                revenue, profit, unfilled, over_target_months, distress_months, wage_shortfall,
                plan_invest, loan_request, loan_rate, amort, green_plan, closed_tick,
                excess_demand, carbon_cost, profit_ema, expected_sales, capex, tax_paid, markup,
                capital_per_sale);
        }
        Delta::Household(id, p) => {
            let h = &mut s.households[id.idx()];
            set_fields!(h, p; home, class, wellbeing, political_power, last_disposable,
                food_met, consumption, footprint, energy_use, wealth, arrears, arrears_months);
            if let Some(x) = p.push_income_hist {
                h.income_hist.push(x);
                if h.income_hist.len() > hist_len {
                    h.income_hist.remove(0);
                }
            }
        }
        Delta::Home(id, p) => {
            if let Some(h) = s.homes.iter_mut().find(|h| h.id == id) {
                set_fields!(h, p; household, rent);
            }
        }
        Delta::AddHome(h) => {
            s.next_home_id = s.next_home_id.max(h.id.0 + 1);
            s.homes.push(h);
        }
        Delta::AddPlant(p) => {
            s.next_plant_id = s.next_plant_id.max(p.id.0 + 1);
            s.plants.push(p);
        }
        Delta::SetTile(t, c) => s.map.set(t, c),
        Delta::PlantOutput {
            id,
            output,
            emissions,
        } => {
            if let Some(p) = s.plants.iter_mut().find(|p| p.id == id) {
                p.output = output;
                p.emissions = emissions;
            }
        }
        Delta::Bank(p) => {
            set_fields!(s.bank, p; default_rate, failed, bailout_this_tick, lent_this_tick);
        }
        Delta::Gov(p) => {
            set_fields!(s.gov, p; legitimacy, unfunded, receipts, tax_revenue, spending, pending_build);
            if let Some(x) = p.unfunded_add {
                s.gov.unfunded_total += x;
            }
        }
        Delta::Nature(n) => s.nature = n,
        Delta::Weather(w) => s.weather = w,
        Delta::Estate(e) => s.estate = Some(e),
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
            for x in &mut s.persons {
                x.wage = Cents::ZERO;
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
        PolicyChange::PublicSpending(v) => pol.public_spending = v.max(Cents::ZERO),
        PolicyChange::JobGuarantee(v) => pol.job_guarantee = clamp01(v),
        PolicyChange::ResilienceSpending(v) => pol.resilience_spending = v.max(Cents::ZERO),
    }
}

fn clamp01(r: Rate) -> Rate {
    Rate(r.0.clamp(0.0, 1.0))
}
