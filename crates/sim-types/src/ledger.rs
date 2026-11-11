//! The transaction ledger vocabulary (Town v5). Every money movement the engine applies in a
//! tick is described as a `Transaction`. Nothing here is part of `State`: transactions are
//! output (like events), the engine checks them against balances every tick, and the wire
//! layer stores them.
//!
//! # Balances and postings
//! An account has a `Position { balance, debt }`:
//! * household: `balance` = savings, `debt` = debt;
//! * firm: `balance` = cash, `debt` = debt;
//! * government: `balance` = cash, `debt` = debt;
//! * bank: `balance` = equity, `debt` = loans (what it is owed).
//!
//! Net position (the sector-balance identity of `sim-checks`) is `balance - debt` for the
//! first three and `balance` for the bank. A transaction's `posting` says exactly how it moves
//! the positions of its two accounts (see [`Posting`] and [`Transaction::effects`]).
use crate::enums::{FlowKind, Sector};
use crate::ids::{FirmId, HouseholdId, PersonId};
use crate::state::State;
use crate::units::Cents;
use serde::{Deserialize, Serialize};

/// Money that enters or leaves the town's books without a counterparty inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum External {
    /// New loans create deposits.
    CreditCreation,
    /// Repayments destroy deposits.
    CreditDestruction,
}

impl External {
    pub fn label(self) -> &'static str {
        match self {
            External::CreditCreation => "bank credit creation",
            External::CreditDestruction => "bank credit destruction",
        }
    }
}

/// One side of a transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LedgerAccount {
    Household(HouseholdId),
    Firm(FirmId),
    Government,
    Bank,
    External(External),
}

impl LedgerAccount {
    pub fn label(self) -> String {
        match self {
            LedgerAccount::Household(h) => format!("household {}", h.0),
            LedgerAccount::Firm(f) => format!("firm {}", f.0),
            LedgerAccount::Government => "government".to_string(),
            LedgerAccount::Bank => "bank".to_string(),
            LedgerAccount::External(e) => e.label().to_string(),
        }
    }
}

impl From<crate::enums::Account> for LedgerAccount {
    fn from(a: crate::enums::Account) -> Self {
        match a {
            crate::enums::Account::Household(h) => LedgerAccount::Household(h),
            crate::enums::Account::Firm(f) => LedgerAccount::Firm(f),
            crate::enums::Account::Government => LedgerAccount::Government,
        }
    }
}

/// What a transaction is (the closed set of the contract).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LedgerKind {
    Wage,
    Dividend,
    Rent,
    Benefit,
    Tax,
    Purchase,
    Lobby,
    Interest,
    LoanNew,
    LoanRepay,
    LoanWriteoff,
    Build,
    Upkeep,
    Subsidy,
    Bailout,
    WealthConsumption,
    Neutral,
}

impl LedgerKind {
    pub const ALL: [LedgerKind; 17] = [
        LedgerKind::Wage,
        LedgerKind::Dividend,
        LedgerKind::Rent,
        LedgerKind::Benefit,
        LedgerKind::Tax,
        LedgerKind::Purchase,
        LedgerKind::Lobby,
        LedgerKind::Interest,
        LedgerKind::LoanNew,
        LedgerKind::LoanRepay,
        LedgerKind::LoanWriteoff,
        LedgerKind::Build,
        LedgerKind::Upkeep,
        LedgerKind::Subsidy,
        LedgerKind::Bailout,
        LedgerKind::WealthConsumption,
        LedgerKind::Neutral,
    ];
}

/// Why a deposit-to-deposit transfer happens, finer than `FlowKind`. `Plain` means "just the
/// flow kind". Purposes only label the ledger; the model's behaviour never reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Purpose {
    #[default]
    Plain,
    Wages,
    FirmDividend,
    StateFirmDividend,
    Rent,
    PublicRent,
    CommonsFee,
    WageTax,
    CapitalTax,
    WealthTax,
    ProfitTax,
    CarbonTax,
    Benefits,
    Ubi,
    JobGuarantee,
    Upkeep,
    Build,
    GreenSubsidy,
    InvestmentLabour,
    RepairLabour,
    StartupFunding,
    StartupWorks,
    DefaultResidual,
    LobbySpend,
    Purchase(Sector),
    CapitalGoods,
    PublicPurchases,
    /// Town v7: the estate pays its pilot (household to household).
    DomesticWages,
}

impl Purpose {
    /// Ledger kind and detail label, given the flow kind the model used.
    pub fn resolve(self, flow: FlowKind) -> (LedgerKind, &'static str) {
        use LedgerKind as K;
        match self {
            Purpose::Plain => match flow {
                FlowKind::Wage => (K::Wage, "wages"),
                FlowKind::Dividend => (K::Dividend, "dividends"),
                FlowKind::Rent => (K::Rent, "rent"),
                FlowKind::Benefit => (K::Benefit, "benefit"),
                FlowKind::Tax => (K::Tax, "tax"),
                FlowKind::Purchase => (K::Purchase, "purchase"),
                FlowKind::Lobby => (K::Lobby, "lobbying"),
                FlowKind::Neutral => (K::Neutral, "transfer"),
            },
            Purpose::Wages => (K::Wage, "wages"),
            Purpose::FirmDividend => (K::Dividend, "firm dividend"),
            Purpose::StateFirmDividend => (K::Dividend, "state firm dividend"),
            Purpose::Rent => (K::Rent, "rent"),
            Purpose::PublicRent => (K::Rent, "public housing rent"),
            Purpose::CommonsFee => (K::Rent, "commons fee"),
            Purpose::WageTax => (K::Tax, "wage tax"),
            Purpose::CapitalTax => (K::Tax, "capital tax"),
            Purpose::WealthTax => (K::Tax, "wealth tax"),
            Purpose::ProfitTax => (K::Tax, "profit tax"),
            Purpose::CarbonTax => (K::Tax, "carbon price"),
            Purpose::Benefits => (K::Benefit, "benefit"),
            Purpose::Ubi => (K::Benefit, "ubi"),
            Purpose::JobGuarantee => (K::Wage, "job guarantee"),
            Purpose::Upkeep => (K::Upkeep, "upkeep"),
            Purpose::Build => (K::Build, "build"),
            Purpose::GreenSubsidy => (K::Subsidy, "green subsidy"),
            Purpose::InvestmentLabour => (K::Wage, "investment labour"),
            Purpose::RepairLabour => (K::Wage, "fire repair labour"),
            Purpose::StartupFunding => (K::Neutral, "startup funding"),
            Purpose::StartupWorks => (K::Wage, "startup construction"),
            Purpose::DefaultResidual => (K::Neutral, "default residual"),
            Purpose::LobbySpend => (K::Lobby, "lobbying"),
            Purpose::Purchase(Sector::Food) => (K::Purchase, "food"),
            Purpose::Purchase(Sector::Goods) => (K::Purchase, "goods"),
            Purpose::Purchase(Sector::Services) => (K::Purchase, "services"),
            Purpose::CapitalGoods => (K::Purchase, "capital goods"),
            Purpose::PublicPurchases => (K::Purchase, "public spending"),
            Purpose::DomesticWages => (K::Wage, "domestic wages"),
        }
    }
}

/// How a transaction moves the positions of its two accounts. This is the whole ledger
/// mapping of the money operations: one posting per `MoneyOp` variant (a test enumerates them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Posting {
    /// Deposit moves: `from` balance falls, `to` balance rises (`MoneyOp::Transfer`).
    Transfer,
    /// New loan: `to` balance and debt rise, bank loans rise (`Lend`).
    CreditCreate,
    /// Principal repaid: `from` balance and debt fall, bank loans fall (`Repay`).
    CreditRepay,
    /// `from` pays the bank: `from` balance falls, bank equity rises (`PayInterest`, `Haircut`).
    BankReceives,
    /// The bank pays `to`: `to` balance rises, bank equity falls (`DepositInterest`, `BankDividend`).
    BankPays,
    /// Debt forgiven: `to` debt falls, bank loans and equity fall (`WriteOff`).
    WriteOff,
    /// Government debt handed to the bank: `from` (government) debt rises, bank loans and equity
    /// rise (`Bailout`, `CapitalizeInterest`).
    DebtToBank,
}

/// An account's position (see the module docs for what each field means per account).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub balance: Cents,
    pub debt: Cents,
}

impl Position {
    pub const ZERO: Position = Position {
        balance: Cents::ZERO,
        debt: Cents::ZERO,
    };
}

impl std::ops::Add for Position {
    type Output = Position;
    fn add(self, o: Position) -> Position {
        Position {
            balance: self.balance + o.balance,
            debt: self.debt + o.debt,
        }
    }
}

impl std::ops::AddAssign for Position {
    fn add_assign(&mut self, o: Position) {
        *self = *self + o;
    }
}

/// One money movement applied by the engine, with the amount actually moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Transaction {
    pub tick: u32,
    pub from: LedgerAccount,
    pub to: LedgerAccount,
    pub amount: Cents,
    pub kind: LedgerKind,
    /// Short label inside the kind, e.g. "wage tax", "food".
    pub detail: &'static str,
    pub memo: String,
    pub posting: Posting,
}

impl Transaction {
    pub fn new(
        tick: u32,
        from: LedgerAccount,
        to: LedgerAccount,
        amount: Cents,
        kind: LedgerKind,
        detail: &'static str,
        posting: Posting,
    ) -> Transaction {
        let memo = format!("{detail}: {} -> {}", from.label(), to.label());
        Transaction {
            tick,
            from,
            to,
            amount,
            kind,
            detail,
            memo,
            posting,
        }
    }

    /// The change in each account's position caused by this transaction (external accounts
    /// have no position and never appear).
    pub fn effects(&self) -> Vec<(LedgerAccount, Position)> {
        effects_of(self.posting, self.from, self.to, self.amount)
    }
}

/// See [`Transaction::effects`].
pub fn effects_of(
    posting: Posting,
    from: LedgerAccount,
    to: LedgerAccount,
    a: Cents,
) -> Vec<(LedgerAccount, Position)> {
    let bal = |b: Cents| Position {
        balance: b,
        debt: Cents::ZERO,
    };
    let bank = LedgerAccount::Bank;
    match posting {
        Posting::Transfer => vec![(from, bal(-a)), (to, bal(a))],
        Posting::CreditCreate => vec![
            (
                to,
                Position {
                    balance: a,
                    debt: a,
                },
            ),
            (
                bank,
                Position {
                    balance: Cents::ZERO,
                    debt: a,
                },
            ),
        ],
        Posting::CreditRepay => vec![
            (
                from,
                Position {
                    balance: -a,
                    debt: -a,
                },
            ),
            (
                bank,
                Position {
                    balance: Cents::ZERO,
                    debt: -a,
                },
            ),
        ],
        Posting::BankReceives => vec![(from, bal(-a)), (bank, bal(a))],
        Posting::BankPays => vec![(bank, bal(-a)), (to, bal(a))],
        Posting::WriteOff => vec![
            (
                to,
                Position {
                    balance: Cents::ZERO,
                    debt: -a,
                },
            ),
            (
                bank,
                Position {
                    balance: -a,
                    debt: -a,
                },
            ),
        ],
        Posting::DebtToBank => vec![
            (
                from,
                Position {
                    balance: Cents::ZERO,
                    debt: a,
                },
            ),
            (
                bank,
                Position {
                    balance: a,
                    debt: a,
                },
            ),
        ],
    }
}

/// Wages a person earned this tick (credited to their household). Output of the engine like
/// transactions; the person ledger is built from these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Earning {
    pub tick: u32,
    pub person: PersonId,
    pub household: HouseholdId,
    pub firm: Option<FirmId>,
    pub amount: Cents,
}

/// Positions of every account at one moment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balances {
    pub households: Vec<Position>,
    pub firms: Vec<Position>,
    pub government: Position,
    pub bank: Position,
}

impl Balances {
    pub fn of(s: &State) -> Balances {
        Balances {
            households: s
                .households
                .iter()
                .map(|h| Position {
                    balance: h.savings,
                    debt: h.debt,
                })
                .collect(),
            firms: s
                .firms
                .iter()
                .map(|f| Position {
                    balance: f.cash,
                    debt: f.debt,
                })
                .collect(),
            government: Position {
                balance: s.gov.cash,
                debt: s.gov.debt,
            },
            bank: Position {
                balance: s.bank.equity,
                debt: s.bank.loans,
            },
        }
    }

    pub fn get(&self, a: LedgerAccount) -> Option<Position> {
        match a {
            LedgerAccount::Household(h) => self.households.get(h.idx()).copied(),
            LedgerAccount::Firm(f) => self.firms.get(f.idx()).copied(),
            LedgerAccount::Government => Some(self.government),
            LedgerAccount::Bank => Some(self.bank),
            LedgerAccount::External(_) => None,
        }
    }

    /// Add `e` to an account's position (external accounts have none and are ignored).
    pub fn add(&mut self, a: LedgerAccount, e: Position) {
        if let Some(p) = self.slot(a) {
            *p += e;
        }
    }

    fn slot(&mut self, a: LedgerAccount) -> Option<&mut Position> {
        match a {
            LedgerAccount::Household(h) => self.households.get_mut(h.idx()),
            LedgerAccount::Firm(f) => self.firms.get_mut(f.idx()),
            LedgerAccount::Government => Some(&mut self.government),
            LedgerAccount::Bank => Some(&mut self.bank),
            LedgerAccount::External(_) => None,
        }
    }

    /// Apply a transaction's effects.
    pub fn post(&mut self, t: &Transaction) {
        for (a, e) in t.effects() {
            if let Some(p) = self.slot(a) {
                *p += e;
            }
        }
    }

    /// Every account with its position, in a fixed order.
    pub fn accounts(&self) -> Vec<(LedgerAccount, Position)> {
        let mut v = Vec::with_capacity(self.households.len() + self.firms.len() + 2);
        for (i, p) in self.households.iter().enumerate() {
            v.push((LedgerAccount::Household(HouseholdId(i as u32)), *p));
        }
        for (i, p) in self.firms.iter().enumerate() {
            v.push((LedgerAccount::Firm(FirmId(i as u32)), *p));
        }
        v.push((LedgerAccount::Government, self.government));
        v.push((LedgerAccount::Bank, self.bank));
        v
    }
}

/// Split `total` into parts proportional to `weights` (all but the last rounded down, the last
/// takes the remainder), so the parts always sum to `total` exactly. Weights that sum to zero
/// give one part: everything to the first weight's label.
pub fn split_by_weights<T: Copy>(total: Cents, weights: &[(Cents, T)]) -> Vec<(Cents, T)> {
    let sum: i128 = weights.iter().map(|w| i128::from(w.0 .0.max(0))).sum();
    let Some(last) = weights.len().checked_sub(1) else {
        return Vec::new();
    };
    if sum == 0 || total <= Cents::ZERO {
        return vec![(total, weights[0].1)];
    }
    let mut left = total;
    let mut out = Vec::with_capacity(weights.len());
    for (i, (w, tag)) in weights.iter().enumerate() {
        if i == last {
            out.push((left, *tag));
        } else {
            let part = Cents((i128::from(total.0) * i128::from(w.0.max(0)) / sum) as i64);
            left -= part;
            out.push((part, *tag));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_preserves_the_total() {
        let parts = split_by_weights(
            Cents(1001),
            &[(Cents(300), 'a'), (Cents(700), 'b'), (Cents(1), 'c')],
        );
        assert_eq!(Cents::sum(parts.iter().map(|p| p.0)), Cents(1001));
        let one = split_by_weights(Cents(5), &[(Cents::ZERO, 'x'), (Cents::ZERO, 'y')]);
        assert_eq!(one, vec![(Cents(5), 'x')]);
    }

    #[test]
    fn every_posting_nets_to_the_expected_total() {
        use LedgerAccount as A;
        let (h, g) = (A::Household(HouseholdId(0)), A::Government);
        let a = Cents(100);
        let net = |p: Posting, from, to| -> (Cents, Cents) {
            let e = effects_of(p, from, to, a);
            (
                Cents::sum(e.iter().map(|x| x.1.balance)),
                Cents::sum(e.iter().map(|x| x.1.debt)),
            )
        };
        // Transfers conserve; credit creation adds deposits and loans together.
        assert_eq!(net(Posting::Transfer, h, g), (Cents::ZERO, Cents::ZERO));
        assert_eq!(
            net(
                Posting::CreditCreate,
                A::External(External::CreditCreation),
                h
            ),
            (a, a + a)
        );
        assert_eq!(
            net(Posting::BankReceives, h, A::Bank),
            (Cents::ZERO, Cents::ZERO)
        );
    }
}
