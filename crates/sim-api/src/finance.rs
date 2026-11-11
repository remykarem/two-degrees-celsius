//! `snapshot.finance` and `snapshot.energy` (contract section 14). Flow values come from the
//! ledger (the same transactions that reconcile with the balances), so every number can be
//! traced to entries: monthly for the latest tick, with the trailing quarter (the latest three
//! ticks, fewer early in the game) in `quarter`. Stocks (debt, cash, equity) come from the state.
use crate::command::Tile;
use crate::ledger::{FinEntry, LedgerStore};
use crate::snapshot::{EstateFinance, EstateMoney};
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types::{Cents, HouseholdId, LedgerAccount, LedgerKind, PlantKind, Posting};

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovRevenue {
    pub wage_tax: f64,
    pub profit_tax: f64,
    pub capital_tax: f64,
    pub wealth_tax: f64,
    pub carbon: f64,
    pub other: f64,
}

impl GovRevenue {
    pub fn total(&self) -> f64 {
        self.wage_tax
            + self.profit_tax
            + self.capital_tax
            + self.wealth_tax
            + self.carbon
            + self.other
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovSpending {
    pub benefits: f64,
    pub ubi: f64,
    pub public_spending: f64,
    pub job_guarantee: f64,
    pub upkeep: f64,
    pub subsidy: f64,
    pub interest: f64,
    pub builds: f64,
    pub other: f64,
}

impl GovSpending {
    pub fn total(&self) -> f64 {
        self.benefits
            + self.ubi
            + self.public_spending
            + self.job_guarantee
            + self.upkeep
            + self.subsidy
            + self.interest
            + self.builds
            + self.other
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(rename = "FinanceHouseholdIncome"))]
pub struct HouseholdIncome {
    pub wages: f64,
    pub dividends: f64,
    pub rent: f64,
    pub interest: f64,
    pub benefits: f64,
    /// Contract v1.9: the part of `wages` paid by a household to its staff (the estate's pilot).
    #[serde(default)]
    pub domestic_wages: f64,
}

/// The flow values of a period (a month, or the trailing quarter), in dollars.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceFlows {
    pub gov_revenue: GovRevenue,
    pub gov_spending: GovSpending,
    pub household_income: HouseholdIncome,
    pub household_taxes: f64,
    pub rent_paid: f64,
    pub loan_payments: f64,
    pub household_spending: f64,
    pub firm_revenue: f64,
    pub wage_bill: f64,
    pub firm_profit: f64,
    pub investment: f64,
    pub bank_interest_income: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernmentFinance {
    pub revenue: GovRevenue,
    pub spending: GovSpending,
    /// Monthly surplus (revenue minus spending, interest included even when capitalised): the
    /// change in the government's net position in the month.
    pub balance: f64,
    /// Contract v1.4 addition: the treasury's cash.
    pub cash: f64,
    pub debt: f64,
    pub debt_to_gdp: f64,
    pub legitimacy: f64,
    pub basis: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankFinance {
    pub equity: f64,
    pub loans: f64,
    pub deposits: f64,
    pub capital_ratio: f64,
    pub capital_ratio_limit: f64,
    pub headroom: f64,
    pub default_rate: f64,
    pub interest_income: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdFinance {
    pub income: HouseholdIncome,
    pub taxes: f64,
    pub rent_paid: f64,
    pub loan_payments: f64,
    pub spending: f64,
    pub saving_rate: f64,
    pub debt: f64,
    pub net_worth: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmFinance {
    pub revenue: f64,
    pub wage_bill: f64,
    pub profit: f64,
    pub investment: f64,
    pub debt: f64,
    pub cash: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoneyFinance {
    pub supply: f64,
    pub created_this_turn: f64,
    pub destroyed_this_turn: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finance {
    pub government: GovernmentFinance,
    pub bank: BankFinance,
    pub households: HouseholdFinance,
    pub firms: FirmFinance,
    pub money: MoneyFinance,
    pub price_index: f64,
    pub wage_share: f64,
    pub profit_share: f64,
    /// Trailing-quarter totals of the same flows (contract v1.4, wire addition).
    pub quarter: FinanceFlows,
}

/// Running sums in cents.
#[derive(Clone, Copy, Debug, Default)]
struct Sums {
    rev: [i64; 6],
    spend: [i64; 9],
    inc: [i64; 5],
    hh_taxes: i64,
    rent_paid: i64,
    loan_payments: i64,
    hh_spending: i64,
    firm_revenue: i64,
    wage_bill: i64,
    firm_costs: i64,
    investment: i64,
    bank_interest: i64,
    created: i64,
    destroyed: i64,
    hh_net: i64,
    domestic_wages: i64,
}

// Indices into `rev` and `spend`.
const WAGE_TAX: usize = 0;
const PROFIT_TAX: usize = 1;
const CAPITAL_TAX: usize = 2;
const WEALTH_TAX: usize = 3;
const CARBON: usize = 4;
const REV_OTHER: usize = 5;
const BENEFITS: usize = 0;
const UBI: usize = 1;
const PUBLIC: usize = 2;
const JG: usize = 3;
const UPKEEP: usize = 4;
const SUBSIDY: usize = 5;
const INTEREST: usize = 6;
const BUILDS: usize = 7;
const SP_OTHER: usize = 8;
// Household income.
const WAGES: usize = 0;
const DIVIDENDS: usize = 1;
const RENT: usize = 2;
const INT: usize = 3;
const BENEFIT: usize = 4;

fn is_hh(a: LedgerAccount) -> bool {
    matches!(a, LedgerAccount::Household(_))
}
fn is_firm(a: LedgerAccount) -> bool {
    matches!(a, LedgerAccount::Firm(_))
}

impl Sums {
    fn add(&mut self, e: &FinEntry) {
        let a = e.amount.0;
        let gov = LedgerAccount::Government;
        // Money created and destroyed: the same classification as the engine's MoneyLedger.
        match e.posting {
            Posting::CreditCreate | Posting::BankPays => self.created += a,
            Posting::CreditRepay | Posting::BankReceives => self.destroyed += a,
            Posting::Transfer | Posting::WriteOff | Posting::DebtToBank => {}
        }
        self.hh_net += e.household_net();
        match e.posting {
            Posting::Transfer => {
                if e.to == gov {
                    let i = match (e.kind, e.detail) {
                        (LedgerKind::Tax, "wage tax") => WAGE_TAX,
                        (LedgerKind::Tax, "profit tax") => PROFIT_TAX,
                        (LedgerKind::Tax, "capital tax") => CAPITAL_TAX,
                        (LedgerKind::Tax, "wealth tax") => WEALTH_TAX,
                        (LedgerKind::Tax, "carbon price") => CARBON,
                        _ => REV_OTHER,
                    };
                    self.rev[i] += a;
                }
                if e.from == gov {
                    let i = match (e.kind, e.detail) {
                        (LedgerKind::Benefit, "ubi") => UBI,
                        (LedgerKind::Benefit, _) => BENEFITS,
                        (LedgerKind::Purchase, _) => PUBLIC,
                        (LedgerKind::Wage, "job guarantee") => JG,
                        (LedgerKind::Upkeep, _) => UPKEEP,
                        (LedgerKind::Subsidy, _) => SUBSIDY,
                        (LedgerKind::Build, _) => BUILDS,
                        _ => SP_OTHER,
                    };
                    self.spend[i] += a;
                }
                if is_hh(e.to) {
                    match e.kind {
                        // Labour income of every kind: firms' wages, the job guarantee,
                        // public works, subsidies paid out as work, lobbying fees.
                        LedgerKind::Wage
                        | LedgerKind::Lobby
                        | LedgerKind::Build
                        | LedgerKind::Upkeep
                        | LedgerKind::Subsidy => {
                            self.inc[WAGES] += a;
                            if e.kind == LedgerKind::Wage && e.detail == "domestic wages" {
                                self.domestic_wages += a;
                            }
                        }
                        LedgerKind::Dividend => self.inc[DIVIDENDS] += a,
                        LedgerKind::Rent => self.inc[RENT] += a,
                        LedgerKind::Benefit => self.inc[BENEFIT] += a,
                        _ => {}
                    }
                }
                if is_hh(e.from) {
                    match e.kind {
                        LedgerKind::Tax => self.hh_taxes += a,
                        LedgerKind::Rent => self.rent_paid += a,
                        LedgerKind::Purchase => self.hh_spending += a,
                        _ => {}
                    }
                }
                if is_firm(e.to) && e.kind == LedgerKind::Purchase {
                    self.firm_revenue += a;
                }
                if is_firm(e.from) {
                    match (e.kind, e.detail) {
                        (LedgerKind::Wage, "wages") => {
                            self.wage_bill += a;
                            self.firm_costs += a;
                        }
                        (LedgerKind::Wage, "investment labour") => self.investment += a,
                        (LedgerKind::Purchase, _) => self.investment += a,
                        (LedgerKind::Tax, _) => self.firm_costs += a,
                        _ => {}
                    }
                }
            }
            Posting::BankPays => {
                if is_hh(e.to) {
                    match e.kind {
                        LedgerKind::Interest => self.inc[INT] += a,
                        LedgerKind::Dividend => self.inc[DIVIDENDS] += a,
                        _ => {}
                    }
                }
            }
            Posting::BankReceives => {
                if e.kind == LedgerKind::Interest {
                    self.bank_interest += a;
                    if is_hh(e.from) {
                        self.loan_payments += a;
                    }
                    if is_firm(e.from) {
                        self.firm_costs += a;
                    }
                    if e.from == gov {
                        self.spend[INTEREST] += a;
                    }
                }
            }
            Posting::CreditRepay => {
                if is_hh(e.from) {
                    self.loan_payments += a;
                }
            }
            Posting::DebtToBank => {
                if e.kind == LedgerKind::Interest {
                    self.spend[INTEREST] += a;
                    self.bank_interest += a;
                } else {
                    self.spend[SP_OTHER] += a;
                }
            }
            Posting::CreditCreate | Posting::WriteOff => {}
        }
    }

    fn flows(&self) -> FinanceFlows {
        let d = |c: i64| Cents(c).dollars();
        let r = &self.rev;
        let s = &self.spend;
        let i = &self.inc;
        FinanceFlows {
            gov_revenue: GovRevenue {
                wage_tax: d(r[WAGE_TAX]),
                profit_tax: d(r[PROFIT_TAX]),
                capital_tax: d(r[CAPITAL_TAX]),
                wealth_tax: d(r[WEALTH_TAX]),
                carbon: d(r[CARBON]),
                other: d(r[REV_OTHER]),
            },
            gov_spending: GovSpending {
                benefits: d(s[BENEFITS]),
                ubi: d(s[UBI]),
                public_spending: d(s[PUBLIC]),
                job_guarantee: d(s[JG]),
                upkeep: d(s[UPKEEP]),
                subsidy: d(s[SUBSIDY]),
                interest: d(s[INTEREST]),
                builds: d(s[BUILDS]),
                other: d(s[SP_OTHER]),
            },
            household_income: HouseholdIncome {
                wages: d(i[WAGES]),
                dividends: d(i[DIVIDENDS]),
                rent: d(i[RENT]),
                interest: d(i[INT]),
                benefits: d(i[BENEFIT]),
                domestic_wages: d(self.domestic_wages),
            },
            household_taxes: d(self.hh_taxes),
            rent_paid: d(self.rent_paid),
            loan_payments: d(self.loan_payments),
            household_spending: d(self.hh_spending),
            firm_revenue: d(self.firm_revenue),
            wage_bill: d(self.wage_bill),
            firm_profit: d(self.firm_revenue - self.firm_costs),
            investment: d(self.investment),
            bank_interest_income: d(self.bank_interest),
        }
    }

    fn income_total(&self) -> i64 {
        self.inc.iter().sum()
    }
}

impl FinEntry<'_> {
    /// The effect of this entry on household balances (sum over households), in cents.
    fn household_net(&self) -> i64 {
        sim_types::effects_of(self.posting, self.from, self.to, self.amount)
            .into_iter()
            .filter(|(a, _)| is_hh(*a))
            .map(|(_, p)| p.balance.0)
            .sum()
    }
}

/// Sums of the entries of ticks `from..=to`.
fn sums(store: &LedgerStore, from: u32, to: u32) -> Sums {
    let mut s = Sums::default();
    for e in store.window(from, to) {
        s.add(&e);
    }
    s
}

/// The flows of one turn (all three ticks), for the history points.
pub fn turn_flows(store: &LedgerStore, turn: u32) -> FinanceFlows {
    let first = turn.saturating_sub(crate::ledger::FIRST_TURN) * 3;
    sums(store, first, first + 2).flows()
}

pub fn build(sim: &Sim, store: &LedgerStore) -> Finance {
    let s = &sim.state;
    let zero = Sums::default();
    let (month, quarter, turn_sums) = match store.last_tick() {
        Some(t) => (
            sums(store, t, t),
            sums(store, t.saturating_sub(2), t),
            sums(store, (t / 3) * 3, t),
        ),
        None => (zero, zero, zero),
    };
    let m = month.flows();
    let kappa = s.policy.bank_capital_ratio.0;
    let equity = s.bank.equity.dollars();
    let loans = s.bank.loans.dollars();
    let headroom = if kappa > 0.0 {
        (equity.max(0.0) / kappa - loans).max(0.0)
    } else {
        0.0
    };
    let income = month.income_total();
    let wealth: f64 = sim_metrics::household_wealth(s, &sim.params).iter().sum();
    let profit_pos = m.firm_profit.max(0.0);
    let labour_and_profit = m.wage_bill + profit_pos;
    let (wage_share, profit_share) = if labour_and_profit > 0.0 {
        (
            m.wage_bill / labour_and_profit,
            profit_pos / labour_and_profit,
        )
    } else {
        (0.0, 0.0)
    };
    Finance {
        government: GovernmentFinance {
            revenue: m.gov_revenue,
            spending: m.gov_spending,
            balance: m.gov_revenue.total() - m.gov_spending.total(),
            cash: s.gov.cash.dollars(),
            debt: s.gov.debt.dollars(),
            debt_to_gdp: crate::history::debt_to_gdp(s),
            legitimacy: s.gov.legitimacy.0,
            basis: "month".to_string(),
        },
        bank: BankFinance {
            equity,
            loans,
            deposits: s.bank.deposits.dollars(),
            // Equity over loans; with no loans the bank is unconstrained, shown as 1.
            capital_ratio: if loans > 0.0 { equity / loans } else { 1.0 },
            capital_ratio_limit: kappa,
            headroom,
            default_rate: s.bank.default_rate.0,
            interest_income: m.bank_interest_income,
        },
        households: HouseholdFinance {
            income: m.household_income,
            taxes: m.household_taxes,
            rent_paid: m.rent_paid,
            loan_payments: m.loan_payments,
            spending: m.household_spending,
            saving_rate: if income > 0 {
                month.hh_net as f64 / income as f64
            } else {
                0.0
            },
            debt: Cents::sum(s.households.iter().map(|h| h.debt)).dollars(),
            net_worth: wealth / 100.0,
        },
        firms: FirmFinance {
            revenue: m.firm_revenue,
            wage_bill: m.wage_bill,
            profit: m.firm_profit,
            investment: m.investment,
            debt: Cents::sum(s.firms.iter().map(|f| f.debt)).dollars(),
            cash: Cents::sum(s.firms.iter().map(|f| f.cash)).dollars(),
        },
        money: MoneyFinance {
            supply: s.bank.deposits.dollars(),
            created_this_turn: Cents(turn_sums.created).dollars(),
            destroyed_this_turn: Cents(turn_sums.destroyed).dollars(),
        },
        price_index: s.indicators.price_index,
        wage_share,
        profit_share,
        quarter: quarter.flows(),
    }
}

// ---------------------------------------------------------------- energy

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(rename = "EnergyPlantKind"))]
pub enum PlantKindKey {
    Fossil,
    Renewable,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantSupply {
    pub tile: Tile,
    pub kind: PlantKindKey,
    pub capacity: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyDemand {
    pub firms: f64,
    pub households: f64,
    pub total: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantEmissions {
    pub tile: Tile,
    pub tonnes: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyView {
    pub unit: String,
    pub supply: Vec<PlantSupply>,
    pub demand: EnergyDemand,
    pub served: f64,
    pub intensity: f64,
    pub renewable_share: f64,
    pub emissions_by_plant: Vec<PlantEmissions>,
}

pub fn energy(sim: &Sim, unit: &str) -> EnergyView {
    let s = &sim.state;
    let fossil = s.capacity(PlantKind::Fossil);
    let renewable = s.capacity(PlantKind::Renewable);
    let total = s.energy.demand.0;
    // Households' pre-rationing demand: what they were delivered over the served share. With
    // nothing served (no supply) it is recomputed from the model's own demand function, which
    // reads last month's consumption (so it can differ slightly from what the tick used).
    let households = if s.energy.served.0 > 0.0 {
        s.households.iter().map(|h| h.energy_use.0).sum::<f64>() / s.energy.served.0
    } else {
        sim_model::production::household_demand(s, &sim.params, 1.0)
            .iter()
            .sum()
    };
    let households = households.min(total);
    EnergyView {
        unit: unit.to_string(),
        supply: s
            .plants
            .iter()
            .map(|p| PlantSupply {
                tile: Tile::from_sim(p.tile),
                kind: match p.kind {
                    PlantKind::Fossil => PlantKindKey::Fossil,
                    PlantKind::Renewable => PlantKindKey::Renewable,
                },
                capacity: p.capacity.0,
            })
            .collect(),
        demand: EnergyDemand {
            firms: total - households,
            households,
            total,
        },
        served: s.energy.served.0,
        intensity: s.energy.grid_intensity.0,
        renewable_share: if fossil + renewable > 0.0 {
            // `+ 0.0` turns an empty sum's -0.0 into 0.0 (it would print as "-0.0").
            renewable / (fossil + renewable) + 0.0
        } else {
            0.0
        },
        emissions_by_plant: s
            .plants
            .iter()
            .map(|p| PlantEmissions {
                tile: Tile::from_sim(p.tile),
                tonnes: p.emissions.0,
            })
            .collect(),
    }
}

/// The estate household's money from the ledger (contract v1.9): the latest month and the
/// trailing twelve months (fewer early in the game).
pub fn estate_finance(hh: HouseholdId, store: &LedgerStore) -> EstateFinance {
    let me = LedgerAccount::Household(hh);
    let Some(t) = store.last_tick() else {
        return EstateFinance {
            month: EstateMoney::default(),
            last12: EstateMoney::default(),
            months: 0,
            wealth_tax_owed: 0.0,
        };
    };
    let sum = |from: u32, to: u32| {
        let (mut div, mut wage, mut wealth, mut carbon) = (0i64, 0i64, 0i64, 0i64);
        for e in store.window(from, to) {
            let a = e.amount.0;
            if e.to == me && e.kind == LedgerKind::Dividend {
                div += a;
            }
            if e.from == me && e.posting == Posting::Transfer {
                match (e.kind, e.detail) {
                    (LedgerKind::Wage, "domestic wages") => wage += a,
                    (LedgerKind::Tax, "wealth tax") => wealth += a,
                    (LedgerKind::Tax, "carbon price") => carbon += a,
                    _ => {}
                }
            }
        }
        let d = |c: i64| Cents(c).dollars();
        EstateMoney {
            dividends: d(div),
            wage_paid: d(wage),
            wealth_tax: d(wealth),
            carbon_charge: d(carbon),
        }
    };
    let from = t.saturating_sub(11);
    EstateFinance {
        month: sum(t, t),
        last12: sum(from, t),
        months: t - from + 1,
        wealth_tax_owed: 0.0, // filled by `estate_view` (needs the state)
    }
}
