use crate::enums::*;
use crate::ids::*;
use crate::ledger::Purpose;
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
        /// Why (labels the ledger only; behaviour never reads it).
        purpose: Purpose,
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
    /// Unpaid government interest added to its debt (bank loans and equity rise). Money unchanged.
    CapitalizeInterest { amount: Cents },
    /// Savers lose `rate` of every deposit after a bank failure. Destroys money.
    Haircut { rate: Rate },
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
    pub footprint: Option<Tonnes>,
    pub energy_use: Option<Units>,
    pub tile: Option<Tile>,
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
    pub expected_sales: Option<Units>,
    pub markup: Option<Rate>,
    pub capital_per_sale: Option<f64>,
    pub capex: Option<Cents>,
    pub tax_paid: Option<Cents>,
    pub carbon_cost: Option<Cents>,
    pub profit_ema: Option<Cents>,
}

/// Partial update of a person. `None` means unchanged.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PersonPatch {
    pub skill: Option<Rate>,
    pub job: Option<Option<FirmId>>,
    pub jg: Option<bool>,
    pub wage: Option<Cents>,
    pub commute_tiles: Option<u32>,
    pub commute_minutes: Option<u32>,
    pub effective_hours: Option<Rate>,
    pub wellbeing: Option<Rate>,
}

/// Partial update of a household. `None` means unchanged.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HouseholdPatch {
    pub home: Option<Option<HomeId>>,
    pub class: Option<Class>,
    pub wellbeing: Option<Rate>,
    pub political_power: Option<Rate>,
    pub last_disposable: Option<Cents>,
    pub push_income_hist: Option<(Cents, Cents)>,
    pub food_met: Option<Rate>,
    pub consumption: Option<Cents>,
    pub footprint: Option<Tonnes>,
    pub energy_use: Option<Units>,
    pub wealth: Option<Cents>,
    pub arrears: Option<bool>,
    pub arrears_months: Option<u32>,
}

/// Partial update of a home.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HomePatch {
    pub household: Option<Option<HouseholdId>>,
    pub rent: Option<Cents>,
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
    pub unfunded: Option<Cents>,
    pub unfunded_add: Option<Cents>,
    pub receipts: Option<Cents>,
    pub tax_revenue: Option<Cents>,
    pub spending: Option<Cents>,
    pub pending_build: Option<Vec<(InfraKind, Tile)>>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Delta {
    Person(PersonId, PersonPatch),
    Firm(FirmId, FirmPatch),
    Household(HouseholdId, HouseholdPatch),
    Home(HomeId, HomePatch),
    /// A new home or plant, and the tile it occupies. The map is updated by `SetTile`.
    AddHome(Home),
    AddPlant(Plant),
    SetTile(Tile, MapTile),
    /// New output and emissions of a plant.
    PlantOutput {
        id: PlantId,
        output: Units,
        emissions: Tonnes,
    },
    Bank(BankPatch),
    Gov(GovPatch),
    Nature(Nature),
    Weather(Weather),
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
        self.transfer_as(from, to, amount, kind, Purpose::Plain);
    }
    /// A transfer with a ledger purpose (same behaviour as `transfer`).
    pub fn transfer_as(
        &mut self,
        from: Account,
        to: Account,
        amount: Cents,
        kind: FlowKind,
        purpose: Purpose,
    ) {
        if amount > Cents::ZERO && from != to {
            self.money.push(MoneyOp::Transfer {
                from,
                to,
                amount,
                kind,
                purpose,
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
