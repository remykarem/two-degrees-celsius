use crate::enums::*;
use crate::ids::*;
use crate::state::*;
use crate::units::*;
use serde::{Deserialize, Serialize};

/// A money operation. Systems return these; the engine applies them in order.
/// Everything except `Transfer`/`Haircut`-free flows is a credit operation that
/// creates or destroys money and may only come from S5 or S8.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoneyOp {
    /// Deposit to deposit. Conserves money. Clamped to the payer's balance.
    Transfer {
        from: Account,
        to: Account,
        amount: Cents,
        kind: FlowKind,
    },
    /// New loan: borrower deposit and bank loans both rise. Creates money.
    Lend { to: Account, amount: Cents },
    /// Principal repayment: deposit and loan fall. Destroys money.
    Repay { from: Account, amount: Cents },
    /// Interest to the bank: deposit falls, bank equity rises. Destroys money.
    PayInterest { from: Account, amount: Cents },
    /// Interest on deposits: bank equity falls, deposit rises. Creates money.
    DepositInterest { to: Account, amount: Cents },
    /// Bank dividend to its owner: equity falls, deposit rises. Creates money.
    BankDividend { to: HouseholdId, amount: Cents },
    /// Loan write-off: loan and equity fall. Money unchanged.
    WriteOff { who: Account, amount: Cents },
    /// Government bonds swapped for bank equity. Money unchanged.
    Bailout { amount: Cents },
    /// Savers lose `rate` of every deposit after a bank failure. Destroys money.
    Haircut { rate: Rate },
    /// Debt moves with people between cohorts. Money unchanged.
    MoveDebt {
        from: HouseholdId,
        to: HouseholdId,
        amount: Cents,
    },
}

/// Partial update of a firm. `None` means unchanged.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FirmPatch {
    pub active: Option<bool>,
    pub owner: Option<Owner>,
    pub capital: Option<Units>,
    pub workers: Option<People>,
    pub wage: Option<Cents>,
    pub price: Option<Cents>,
    pub inventory: Option<Units>,
    pub output: Option<Units>,
    pub emissions: Option<Tonnes>,
    pub green_tech: Option<Rate>,
    pub tfp: Option<Rate>,
    pub sales_hist: Option<[f64; 3]>,
    pub sales_units: Option<Units>,
    pub revenue: Option<Cents>,
    pub profit: Option<Cents>,
    pub unfilled: Option<bool>,
    pub over_target_months: Option<u32>,
    pub distress_months: Option<u32>,
    pub wage_shortfall: Option<bool>,
    pub plan_invest: Option<Cents>,
    pub loan_request: Option<Cents>,
    pub loan_rate: Option<Rate>,
    pub amort: Option<Cents>,
    pub green_plan: Option<bool>,
    pub closed_tick: Option<Option<u32>>,
    pub excess_demand: Option<Rate>,
    pub carbon_cost: Option<Cents>,
    pub profit_ema: Option<Cents>,
}

/// Partial update of a household cohort. `None` means unchanged.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HouseholdPatch {
    pub weight: Option<People>,
    pub skill: Option<Rate>,
    pub class: Option<Class>,
    pub wellbeing: Option<Rate>,
    pub political_power: Option<Rate>,
    pub last_disposable: Option<Cents>,
    pub push_income_hist: Option<(Cents, Cents)>,
    pub food_met: Option<Rate>,
    pub housed_share: Option<Rate>,
    pub consumption: Option<Cents>,
    pub footprint: Option<Tonnes>,
    pub wealth_pc: Option<Cents>,
    pub arrears: Option<bool>,
}

/// Partial update of the bank's statistics (stocks change only through `MoneyOp`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BankPatch {
    pub default_rate: Option<Rate>,
    pub failed: Option<bool>,
    pub bailout_this_tick: Option<bool>,
    pub lent_this_tick: Option<Cents>,
}

/// Partial update of government bookkeeping (cash and debt change only through `MoneyOp`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GovPatch {
    pub legitimacy: Option<Rate>,
    pub purchases_budget: Option<Cents>,
    pub tax_revenue: Option<Cents>,
    pub spending: Option<Cents>,
    pub pending_build: Option<Vec<(InfraKind, u32)>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Delta {
    Employment {
        h: HouseholdId,
        f: FirmId,
        n: People,
    },
    Firm(FirmId, FirmPatch),
    Household(HouseholdId, HouseholdPatch),
    Bank(BankPatch),
    Gov(GovPatch),
    Nature(Nature),
    Housing(Housing),
    Energy(Energy),
    Effective(EffectivePolicy),
    System(EconomicSystem),
    Transition(Option<Transition>),
    OutputPenalty(Rate),
    UnionDensity(Rate),
    NetLobby(Rate),
    Gdp(GdpPart, Cents),
    Outcome(Option<(u32, Outcome)>),
    ClearPendingCommons,
    /// Change a policy lever (used when a system is adopted or a lever is forced).
    SetPolicy(PolicyChange),
    /// Reset the per-tick flow accumulators (income, gdp parts) at the start of a tick.
    ResetFlows,
}

/// What a system returns. The engine applies `money` first, then `changes`, in order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Deltas {
    pub money: Vec<MoneyOp>,
    pub changes: Vec<Delta>,
    pub events: Vec<Event>,
    pub trace: Vec<TraceRecord>,
}

impl Deltas {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn transfer(&mut self, from: Account, to: Account, amount: Cents, kind: FlowKind) {
        if amount > Cents::ZERO && from != to {
            self.money.push(MoneyOp::Transfer {
                from,
                to,
                amount,
                kind,
            });
        }
    }
    pub fn firm(&mut self, f: FirmId, p: FirmPatch) {
        self.changes.push(Delta::Firm(f, p));
    }
    pub fn household(&mut self, h: HouseholdId, p: HouseholdPatch) {
        self.changes.push(Delta::Household(h, p));
    }
}
