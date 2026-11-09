//! S5 credit and money. Lending creates money, principal repayment destroys it,
//! interest moves value from borrowers to the bank. Defaults write off loans.
//! This is, with government borrowing in S8, the only place money is created or destroyed.

use crate::util::{cfg, monthly};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let cp = &p.credit;
    let c = cfg(s, p);
    let base = s.policy.base_rate.0;
    let kappa = s.policy.bank_capital_ratio.0;
    let kp = p.invest.capital_price_usd * 100.0;
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
        let short = pay_i < interest_due || pay_p < principal_due || f.wage_shortfall;
        let distress = if short { f.distress_months + 1 } else { 0 };
        if distress >= cp.default_after_months && debt > Cents::ZERO {
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
                Owner::Household(h) => {
                    d.transfer(acct, Account::Household(h), residual, FlowKind::Neutral)
                }
                Owner::Bank => d.transfer(
                    acct,
                    Account::Household(s.bank.owner),
                    residual,
                    FlowKind::Neutral,
                ),
                Owner::State | Owner::Workers => {
                    d.transfer(acct, Account::Government, residual, FlowKind::Neutral)
                }
            }
            for h in &s.households {
                if h.employment[f.id.idx()].0 > 0.0 {
                    d.changes.push(Delta::Employment {
                        h: h.id,
                        f: f.id,
                        n: People::ZERO,
                    });
                }
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
    if eq.is_negative() {
        if s.policy.bailout {
            let target = loans.scale(cp.bailout_target_ratio);
            let amount = target - eq;
            d.money.push(MoneyOp::Bailout { amount });
            eq += amount;
            loans += amount;
            d.changes.push(Delta::Bank(BankPatch {
                bailout_this_tick: Some(true),
                ..BankPatch::default()
            }));
            d.events.push(Event::BankBailout { amount });
        } else {
            let rate = (-eq.as_f64()) / dep.as_f64().max(1.0) * cp.haircut_margin;
            d.money.push(MoneyOp::Haircut { rate: Rate(rate) });
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
    let max_loans = if kappa > 0.0 {
        eq.max(Cents::ZERO).scale(1.0 / kappa)
    } else {
        Cents::ZERO
    };
    let mut headroom = (max_loans - loans).max(Cents::ZERO);
    let mut lent = Cents::ZERO;
    for f in s.firms.iter().filter(|f| f.active) {
        let i = f.id.idx();
        if closed[i] {
            continue;
        }
        let assets = f.capital.0 * kp + cash_after[i].as_f64();
        let lev_cap = Cents::from_f64((cp.max_firm_leverage * assets).max(0.0)) - debt_after[i];
        let approve = f
            .loan_request
            .min(headroom)
            .min(lev_cap.max(Cents::ZERO))
            .max(Cents::ZERO);
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
        let lev = safe_div(
            debt_after[i].as_f64(),
            f.capital.0 * kp + cash_after[i].as_f64(),
            0.0,
        );
        let rate = base + cp.spread_base + cp.spread_leverage * lev + c.credit_spread_add;
        d.firm(
            f.id,
            FirmPatch {
                loan_request: Some(Cents::ZERO),
                loan_rate: Some(Rate(rate)),
                ..FirmPatch::default()
            },
        );
    }

    // ---- Bank dividend: equity above the required buffer goes to the bank's owner ----
    let required = loans.scale(kappa * (1.0 + cp.bank_buffer));
    let excess = (eq - required).max(Cents::ZERO);
    let dividend = excess.scale(cp.bank_payout);
    if dividend > Cents::ZERO {
        d.money.push(MoneyOp::BankDividend {
            to: s.bank.owner,
            amount: dividend,
        });
    }

    let rate_now = defaults as f64 / active as f64;
    d.changes.push(Delta::Bank(BankPatch {
        default_rate: Some(Rate(0.9 * s.bank.default_rate.0 + 0.1 * rate_now)),
        lent_this_tick: Some(lent),
        ..BankPatch::default()
    }));
    d
}
