//! S5 credit and money. Lending creates money, principal repayment destroys it,
//! interest moves value from borrowers to the bank. Defaults write off loans.
//! This is, with government borrowing in S8, the only place money is created or destroyed.

use crate::util::{cfg, monthly};
use sim_params::{CreditParams, Params};
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::Effect as Fx;
use sim_types::QuantityId as Q;
use sim_types::*;

// ---- Decision: a firm that cannot pay for long enough closes ----

pub const FIRM_EXIT: DecisionSpec = DecisionSpec {
    id: D::FirmExit,
    system: SystemId::Credit,
    inputs: &[Q::PaymentShortfall, Q::DistressMonths, Q::Debt],
    outputs: &[Q::DistressMonths, Q::FirmOpen],
    effects: &[
        Fx::up(Q::PaymentShortfall, Q::DistressMonths),
        Fx::down(Q::PaymentShortfall, Q::FirmOpen),
        Fx::up(Q::DistressMonths, Q::DistressMonths),
        Fx::down(Q::DistressMonths, Q::FirmOpen),
        Fx::none(Q::Debt, Q::DistressMonths, "the debt decides only whether a firm that is distressed long enough closes, not how many months it has been distressed"),
        Fx::down(Q::Debt, Q::FirmOpen),
    ],
    params: &["credit.default_after_months"],
    rule: "default_after_months",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExitView {
    /// Interest or scheduled principal or wages went unpaid this month.
    pub payment_shortfall: bool,
    pub distress_months: u32,
    /// Debt left after this month's payments.
    pub debt: Cents,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExitChoice {
    pub distress_months: u32,
    /// The firm defaults and closes. (Who gets what is left is accounting and stays in `run`.)
    pub closes: bool,
}

pub fn decide_exit(v: &ExitView, cp: &CreditParams) -> ExitChoice {
    let distress_months = if v.payment_shortfall {
        v.distress_months + 1
    } else {
        0
    };
    ExitChoice {
        distress_months,
        closes: distress_months >= cp.default_after_months && v.debt > Cents::ZERO,
    }
}

// ---- Decisions of the bank: how much it can lend, to whom, at what rate, and what it pays out ----

pub const BANK_HEADROOM: DecisionSpec = DecisionSpec {
    id: D::BankHeadroom,
    system: SystemId::Credit,
    inputs: &[Q::BankEquity, Q::BankLoans, Q::CapitalRatio],
    outputs: &[Q::CreditHeadroom],
    effects: &[
        Fx::up(Q::BankEquity, Q::CreditHeadroom),
        Fx::down(Q::BankLoans, Q::CreditHeadroom),
        Fx::down(Q::CapitalRatio, Q::CreditHeadroom),
    ],
    params: &[],
    rule: "capital_ratio_limit",
};

/// The bank's position after this month's repayments, defaults and any rescue (`run` keeps the
/// running figures), and the minimum capital ratio the policy sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadroomView {
    pub bank_equity: Cents,
    pub bank_loans: Cents,
    pub capital_ratio: f64,
}

pub fn headroom_view(s: &State, equity: Cents, loans: Cents) -> HeadroomView {
    HeadroomView {
        bank_equity: equity,
        bank_loans: loans,
        capital_ratio: s.policy.bank_capital_ratio.0,
    }
}

/// How many more dollars the bank may lend: equity divided by the required ratio, less what it
/// has already lent (never negative).
pub fn decide_headroom(v: &HeadroomView) -> Cents {
    let max_loans = if v.capital_ratio > 0.0 {
        v.bank_equity.max(Cents::ZERO).scale(1.0 / v.capital_ratio)
    } else {
        Cents::ZERO
    };
    (max_loans - v.bank_loans).max(Cents::ZERO)
}

pub const BANK_LEND: DecisionSpec = DecisionSpec {
    id: D::BankLend,
    system: SystemId::Credit,
    inputs: &[Q::LoanRequest, Q::CreditHeadroom, Q::FirmAssets, Q::Debt],
    outputs: &[Q::LoanApproved],
    effects: &[
        Fx::up(Q::LoanRequest, Q::LoanApproved),
        Fx::up(Q::CreditHeadroom, Q::LoanApproved),
        Fx::up(Q::FirmAssets, Q::LoanApproved),
        Fx::down(Q::Debt, Q::LoanApproved),
    ],
    params: &["credit.max_firm_leverage"],
    rule: "headroom_and_leverage_cap",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LendView {
    pub loan_request: Cents,
    /// What the bank has left to lend this month (falls as loans are approved).
    pub credit_headroom: Cents,
    /// Cents: capital at its price plus cash.
    pub firm_assets: f64,
    pub debt: Cents,
}

/// A firm's assets in cents: capital at the price of capital, plus cash.
pub fn firm_assets(p: &Params, f: &Firm, cash: Cents) -> f64 {
    f.capital.0 * (p.invest.capital_price_usd * 100.0) + cash.as_f64()
}

/// `credit_headroom`, the firm's `debt` and `cash` after this month's payments are the bank's
/// running figures that `run` passes in.
pub fn lend_view(p: &Params, f: &Firm, headroom: Cents, debt: Cents, cash: Cents) -> LendView {
    LendView {
        loan_request: f.loan_request,
        credit_headroom: headroom,
        firm_assets: firm_assets(p, f, cash),
        debt,
    }
}

/// Approve the request up to the bank's headroom and up to the firm's leverage cap.
pub fn decide_lend(v: &LendView, cp: &CreditParams) -> Cents {
    let lev_cap = Cents::from_f64((cp.max_firm_leverage * v.firm_assets).max(0.0)) - v.debt;
    v.loan_request
        .min(v.credit_headroom)
        .min(lev_cap.max(Cents::ZERO))
        .max(Cents::ZERO)
}

pub const BANK_SPREAD: DecisionSpec = DecisionSpec {
    id: D::BankSpread,
    system: SystemId::Credit,
    inputs: &[Q::PolicyRate, Q::Debt, Q::FirmAssets, Q::CreditSpreadAdd],
    outputs: &[Q::LoanRate],
    effects: &[
        Fx::up(Q::PolicyRate, Q::LoanRate).lag(1),
        Fx::up(Q::Debt, Q::LoanRate).lag(1),
        Fx::down(Q::FirmAssets, Q::LoanRate).lag(1),
        Fx::up(Q::CreditSpreadAdd, Q::LoanRate).lag(1),
    ],
    params: &["credit.spread_base", "credit.spread_leverage"],
    rule: "base_plus_leverage_spread",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpreadView {
    pub base_rate: f64,
    /// The firm's debt after this month's new loan.
    pub debt: Cents,
    pub firm_assets: f64,
    /// The economic system's extra spread (a system setting, not a parameter).
    pub credit_spread_add: f64,
}

pub fn spread_view(s: &State, p: &Params, f: &Firm, debt: Cents, cash: Cents) -> SpreadView {
    SpreadView {
        base_rate: s.policy.base_rate.0,
        debt,
        firm_assets: firm_assets(p, f, cash),
        credit_spread_add: cfg(s, p).credit_spread_add,
    }
}

/// The yearly rate on the firm's debt from next month: base rate plus a base spread plus a spread
/// that grows with its leverage (debt over assets).
pub fn decide_spread(v: &SpreadView, cp: &CreditParams) -> Rate {
    let lev = safe_div(v.debt.as_f64(), v.firm_assets, 0.0);
    Rate(v.base_rate + cp.spread_base + cp.spread_leverage * lev + v.credit_spread_add)
}

pub const BANK_DIVIDEND: DecisionSpec = DecisionSpec {
    id: D::BankDividend,
    system: SystemId::Credit,
    inputs: &[Q::BankEquity, Q::BankLoans, Q::CapitalRatio],
    outputs: &[Q::BankDividend],
    effects: &[
        Fx::up(Q::BankEquity, Q::BankDividend),
        Fx::down(Q::BankLoans, Q::BankDividend),
        Fx::down(Q::CapitalRatio, Q::BankDividend),
    ],
    params: &["credit.bank_buffer", "credit.bank_payout"],
    rule: "pay_out_excess_equity",
};

pub fn dividend_view(s: &State, equity: Cents, loans: Cents) -> HeadroomView {
    headroom_view(s, equity, loans)
}

/// Equity above the required buffer (the capital ratio plus a safety margin) is partly paid to the
/// bank's owner.
pub fn decide_bank_dividend(v: &HeadroomView, cp: &CreditParams) -> Cents {
    let required = v.bank_loans.scale(v.capital_ratio * (1.0 + cp.bank_buffer));
    let excess = (v.bank_equity - required).max(Cents::ZERO);
    excess.scale(cp.bank_payout)
}

pub const BANK_RESCUE: DecisionSpec = DecisionSpec {
    id: D::BankRescue,
    system: SystemId::Credit,
    inputs: &[
        Q::BankEquity,
        Q::BankLoans,
        Q::BankDeposits,
        Q::BailoutPolicy,
    ],
    outputs: &[Q::BankEquity, Q::BankFailed],
    effects: &[
        Fx::down(Q::BankEquity, Q::BankEquity),
        Fx::down(Q::BankEquity, Q::BankFailed),
        Fx::up(Q::BankLoans, Q::BankEquity),
        Fx::none(Q::BankLoans, Q::BankFailed, "whether the bank fails depends on its equity going negative and on the bailout choice, not on loans directly"),
        Fx::none(Q::BankDeposits, Q::BankEquity, "deposits set the depositors' haircut, not the bailout"),
        Fx::none(Q::BankDeposits, Q::BankFailed, "deposits set the depositors' haircut, not whether the bank fails"),
        Fx::up(Q::BailoutPolicy, Q::BankEquity),
        Fx::down(Q::BailoutPolicy, Q::BankFailed),
    ],
    params: &["credit.bailout_target_ratio", "credit.haircut_margin"],
    rule: "bailout_or_haircut",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RescueView {
    pub bank_equity: Cents,
    pub bank_loans: Cents,
    pub bank_deposits: Cents,
    /// The player's policy: the treasury recapitalises a failing bank.
    pub bailout_policy: bool,
}

pub fn rescue_view(s: &State, equity: Cents, loans: Cents, deposits: Cents) -> RescueView {
    RescueView {
        bank_equity: equity,
        bank_loans: loans,
        bank_deposits: deposits,
        bailout_policy: s.policy.bailout,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RescueChoice {
    /// Equity is not negative: nothing to do.
    None,
    /// New money restores equity to the target ratio of loans.
    Bailout { amount: Cents },
    /// No bailout: the bank fails and depositors lose this share.
    Haircut { rate: Rate },
}

pub fn decide_rescue(v: &RescueView, cp: &CreditParams) -> RescueChoice {
    if !v.bank_equity.is_negative() {
        RescueChoice::None
    } else if v.bailout_policy {
        let target = v.bank_loans.scale(cp.bailout_target_ratio);
        RescueChoice::Bailout {
            amount: target - v.bank_equity,
        }
    } else {
        RescueChoice::Haircut {
            rate: Rate(
                (-v.bank_equity.as_f64()) / v.bank_deposits.as_f64().max(1.0) * cp.haircut_margin,
            ),
        }
    }
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let cp = &p.credit;
    let base = s.policy.base_rate.0;
    // Running bank position, mirroring what the engine will apply.
    let mut eq = s.bank.equity;
    let mut loans = s.bank.loans;
    let mut dep = s.bank.deposits;
    let mut defaults = 0u32;
    let active = s.firms.iter().filter(|f| f.active).count().max(1);

    // ---- Firms: interest, scheduled principal, defaults ----
    let mut debt_after: Vec<Cents> = vec![Cents::ZERO; s.firms.len()];
    let mut cash_after: Vec<Cents> = vec![Cents::ZERO; s.firms.len()];
    let mut closed = vec![false; s.firms.len()];
    for f in s.firms.iter().filter(|f| f.active) {
        let acct = Account::Firm(f.id);
        let interest_due = f.debt.scale(monthly(f.loan_rate.0));
        let pay_i = interest_due.min(f.cash);
        let principal_due = f.amort.min(f.debt);
        let pay_p = principal_due.min(f.cash - pay_i);
        d.money.push(MoneyOp::PayInterest {
            from: acct,
            amount: pay_i,
        });
        d.money.push(MoneyOp::Repay {
            from: acct,
            amount: pay_p,
        });
        eq += pay_i;
        dep -= pay_i + pay_p;
        loans -= pay_p;
        let cash = f.cash - pay_i - pay_p;
        let debt = f.debt - pay_p;
        let exit = decide_exit(
            &ExitView {
                payment_shortfall: pay_i < interest_due
                    || pay_p < principal_due
                    || f.wage_shortfall,
                distress_months: f.distress_months,
                debt,
            },
            cp,
        );
        let distress = exit.distress_months;
        if exit.closes {
            // Default: cash goes to the bank first, the rest of the debt is written off,
            // anything left goes to the owner. The firm closes.
            let repay = cash.min(debt);
            let write_off = debt - repay;
            d.money.push(MoneyOp::Repay {
                from: acct,
                amount: repay,
            });
            d.money.push(MoneyOp::WriteOff {
                who: acct,
                amount: write_off,
            });
            loans -= repay + write_off;
            dep -= repay;
            eq -= write_off;
            let residual = cash - repay;
            match f.owner {
                Owner::Household(h) => d.transfer_as(
                    acct,
                    Account::Household(h),
                    residual,
                    FlowKind::Neutral,
                    Purpose::DefaultResidual,
                ),
                Owner::Bank => d.transfer_as(
                    acct,
                    Account::Household(s.bank.owner),
                    residual,
                    FlowKind::Neutral,
                    Purpose::DefaultResidual,
                ),
                Owner::State | Owner::Workers => d.transfer_as(
                    acct,
                    Account::Government,
                    residual,
                    FlowKind::Neutral,
                    Purpose::DefaultResidual,
                ),
            }
            for x in crate::util::workers_of(s, f.id) {
                d.changes.push(Delta::Person(
                    x.id,
                    PersonPatch {
                        job: Some(None),
                        ..PersonPatch::default()
                    },
                ));
                d.events.push(Event::PersonFired {
                    person: x.id,
                    firm: f.id,
                });
            }
            d.firm(
                f.id,
                FirmPatch {
                    active: Some(false),
                    capital: Some(Units::ZERO),
                    inventory: Some(Units::ZERO),
                    workers: Some(People::ZERO),
                    output: Some(Units::ZERO),
                    emissions: Some(Tonnes::ZERO),
                    footprint: Some(Tonnes::ZERO),
                    energy_use: Some(Units::ZERO),
                    amort: Some(Cents::ZERO),
                    plan_invest: Some(Cents::ZERO),
                    loan_request: Some(Cents::ZERO),
                    distress_months: Some(0),
                    closed_tick: Some(Some(s.tick)),
                    ..FirmPatch::default()
                },
            );
            d.events.push(Event::LoanDefaulted { firm: f.id });
            d.events.push(Event::FirmClosed { firm: f.id });
            defaults += 1;
            closed[f.id.idx()] = true;
        } else {
            debt_after[f.id.idx()] = debt;
            cash_after[f.id.idx()] = cash;
            d.firm(
                f.id,
                FirmPatch {
                    distress_months: Some(distress),
                    ..FirmPatch::default()
                },
            );
        }
    }

    // ---- Households: interest and amortisation on their loans ----
    let h_rate = monthly(base + cp.spread_base);
    for h in &s.households {
        let acct = Account::Household(h.id);
        let due_i = h.debt.scale(h_rate);
        let pay_i = due_i.min(h.savings);
        let due_p = h.debt.scale(cp.household_amort);
        let pay_p = due_p.min(h.savings - pay_i).min(h.debt);
        d.money.push(MoneyOp::PayInterest {
            from: acct,
            amount: pay_i,
        });
        d.money.push(MoneyOp::Repay {
            from: acct,
            amount: pay_p,
        });
        eq += pay_i;
        dep -= pay_i + pay_p;
        loans -= pay_p;
        let arrears = pay_i < due_i || pay_p < due_p;
        if arrears != h.arrears {
            d.household(
                h.id,
                HouseholdPatch {
                    arrears: Some(arrears),
                    ..HouseholdPatch::default()
                },
            );
        }
    }

    // ---- Government debt service ----
    let gov_due = s.gov.debt.scale(monthly(base));
    let gov_pay = gov_due.min(s.gov.cash);
    d.money.push(MoneyOp::PayInterest {
        from: Account::Government,
        amount: gov_pay,
    });
    eq += gov_pay;
    dep -= gov_pay;
    // Interest the treasury cannot pay is added to its debt, and reported.
    let unpaid = gov_due - gov_pay;
    if unpaid > Cents::ZERO {
        d.money.push(MoneyOp::CapitalizeInterest { amount: unpaid });
        eq += unpaid;
        loans += unpaid;
        d.changes.push(Delta::Gov(GovPatch {
            unfunded_add: Some(unpaid),
            ..GovPatch::default()
        }));
    }

    // ---- Interest on deposits (households) ----
    let r_d = (base - cp.deposit_spread).max(0.0);
    for h in &s.households {
        let due = h.savings.scale(monthly(r_d));
        let pay = due.min(eq.max(Cents::ZERO));
        d.money.push(MoneyOp::DepositInterest {
            to: Account::Household(h.id),
            amount: pay,
        });
        eq -= pay;
        dep += pay;
    }

    // ---- Failure check before any new lending or dividends ----
    match decide_rescue(&rescue_view(s, eq, loans, dep), cp) {
        RescueChoice::None => {}
        RescueChoice::Bailout { amount } => {
            d.money.push(MoneyOp::Bailout { amount });
            eq += amount;
            loans += amount;
            d.changes.push(Delta::Bank(BankPatch {
                bailout_this_tick: Some(true),
                ..BankPatch::default()
            }));
            d.events.push(Event::BankBailout { amount });
        }
        RescueChoice::Haircut { rate } => {
            d.money.push(MoneyOp::Haircut { rate });
            d.changes.push(Delta::Bank(BankPatch {
                failed: Some(true),
                ..BankPatch::default()
            }));
            d.events.push(Event::BankFailed);
            // The haircut restores equity to at least zero; the exact figure is checked by the engine.
            eq = Cents::ZERO.max(eq);
        }
    }

    // ---- New lending to firms, within the capital ratio ----
    let mut headroom = decide_headroom(&headroom_view(s, eq, loans));
    let mut lent = Cents::ZERO;
    d.trace.push(TraceRecord::CreditRoom { headroom });
    for f in s.firms.iter().filter(|f| f.active) {
        let i = f.id.idx();
        if closed[i] {
            continue;
        }
        let approve = decide_lend(&lend_view(p, f, headroom, debt_after[i], cash_after[i]), cp);
        d.trace.push(TraceRecord::CreditDecision {
            firm: f.id,
            requested: f.loan_request.max(Cents::ZERO),
            approved: approve,
        });
        if approve > Cents::ZERO {
            d.money.push(MoneyOp::Lend {
                to: Account::Firm(f.id),
                amount: approve,
            });
            headroom -= approve;
            loans += approve;
            dep += approve;
            lent += approve;
            debt_after[i] += approve;
            cash_after[i] += approve;
        }
        let rate = decide_spread(&spread_view(s, p, f, debt_after[i], cash_after[i]), cp);
        d.firm(
            f.id,
            FirmPatch {
                loan_request: Some(Cents::ZERO),
                loan_rate: Some(rate),
                ..FirmPatch::default()
            },
        );
    }

    // ---- Bank dividend: equity above the required buffer goes to the bank's owner ----
    let dividend = decide_bank_dividend(&dividend_view(s, eq, loans), cp);
    if dividend > Cents::ZERO {
        d.money.push(MoneyOp::BankDividend {
            to: s.bank.owner,
            amount: dividend,
        });
    }

    let rate_now = defaults as f64 / active as f64;
    d.changes.push(Delta::Bank(BankPatch {
        default_rate: Some(Rate(
            cp.default_rate_old_weight * s.bank.default_rate.0
                + cp.default_rate_new_weight * rate_now,
        )),
        lent_this_tick: Some(lent),
        ..BankPatch::default()
    }));
    d
}
