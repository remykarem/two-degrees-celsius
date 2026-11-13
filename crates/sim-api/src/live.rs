//! Contract v2.9 (causality graph stage 0): the live value of a graph quantity.
//!
//! `binding(QuantityId)` is an exhaustive `match`, so a new quantity cannot be added without
//! deciding where its live number comes from (a snapshot field, a sum or mean over firms or
//! households, a lever) or why it has none. A bound quantity serves its current value on
//! `snapshot.quantities`; the catalog serves, per quantity, the `path` of the snapshot field it was
//! read from, how it was aggregated, or the reason it has no value. Values are read from the
//! typed snapshot, never recomputed from the state, so a graph node can never disagree with the
//! panel that shows the same number. Words (aggregate names, reasons, trend words) are content
//! (`content/decisions.json` `live`); the web only formats.
use crate::history::HistoryPoint;
use crate::keys::LeverKey;
use crate::snapshot::{CommonsAccessKey, PilotStatus, PolicyScalar, Snapshot};
use serde::{Deserialize, Serialize};
use sim_types::QuantityId;

/// How the town-level number is made from the agents' own values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aggregate {
    /// One number for the whole town already (an indicator, the bank, the government).
    Town,
    /// Sum over active firms or over households.
    Sum,
    /// Mean over active firms.
    Mean,
    /// The current setting of a policy lever (the effective one for lobbied levers).
    Lever,
    /// Yes or no, served as 1 or 0.
    Flag,
}

impl Aggregate {
    pub const ALL: [Aggregate; 5] = [
        Aggregate::Town,
        Aggregate::Sum,
        Aggregate::Mean,
        Aggregate::Lever,
        Aggregate::Flag,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Aggregate::Town => "town",
            Aggregate::Sum => "sum",
            Aggregate::Mean => "mean",
            Aggregate::Lever => "lever",
            Aggregate::Flag => "flag",
        }
    }
}

/// Why a quantity has no town-level live value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unbound {
    /// A working variable inside one agent; the town has no single number for it.
    Private,
    /// Computed fresh inside a rule each month and not stored, so there is nothing to read.
    Computed,
    /// Exists in the simulation state but the snapshot does not publish a town figure for it.
    NotPublished,
    /// A fixed setting or yes/no flag of a rule that is not shown as a moving number.
    Fixed,
    /// A random draw.
    Random,
}

impl Unbound {
    pub const ALL: [Unbound; 5] = [
        Unbound::Private,
        Unbound::Computed,
        Unbound::NotPublished,
        Unbound::Fixed,
        Unbound::Random,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Unbound::Private => "private",
            Unbound::Computed => "computed",
            Unbound::NotPublished => "not_published",
            Unbound::Fixed => "fixed",
            Unbound::Random => "random",
        }
    }
}

/// Where a bound quantity's number comes from.
pub struct Bound {
    /// A JSON path into the snapshot (`indicators.unemployment`, `firms[].price`,
    /// `markets[food].price`). A test resolves every one on a real snapshot.
    pub path: &'static str,
    pub aggregate: Aggregate,
    /// The lever this quantity is the setting of (so a graph can say "you can change this").
    pub lever: Option<LeverKey>,
    pub read: fn(&Snapshot) -> Option<f64>,
    /// The same figure at the end of the previous turn (or the turn before it when this turn has not started), where the history keeps that series.
    pub previous: Option<fn(&HistoryPoint) -> Option<f64>>,
}

pub enum Binding {
    Bound(Bound),
    Unbound(Unbound),
}

fn flag(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

fn active_firms(s: &Snapshot) -> impl Iterator<Item = &crate::snapshot::FirmView> {
    s.firms.iter().filter(|f| f.active)
}

fn firm_sum(s: &Snapshot, f: fn(&crate::snapshot::FirmView) -> f64) -> Option<f64> {
    let mut it = active_firms(s).peekable();
    it.peek()?;
    Some(it.map(f).sum())
}

fn firm_mean(s: &Snapshot, f: fn(&crate::snapshot::FirmView) -> f64) -> Option<f64> {
    let n = active_firms(s).count();
    (n > 0).then(|| active_firms(s).map(f).sum::<f64>() / n as f64)
}

fn lever_value(s: &Snapshot, k: LeverKey) -> Option<f64> {
    let e = s.policy.get(&k)?;
    match (&e.nominal, e.effective) {
        (_, Some(x)) => Some(x),
        (PolicyScalar::Number(x), None) => Some(*x),
        (PolicyScalar::Bool(b), None) => Some(flag(*b)),
        (PolicyScalar::Mode(_), None) => None,
    }
}

fn market_price(s: &Snapshot, id: &str) -> Option<f64> {
    s.markets.iter().find(|m| m.id == id)?.price
}

fn town(
    path: &'static str,
    read: fn(&Snapshot) -> Option<f64>,
    previous: Option<fn(&HistoryPoint) -> Option<f64>>,
) -> Binding {
    Binding::Bound(Bound {
        path,
        aggregate: Aggregate::Town,
        lever: None,
        read,
        previous,
    })
}

fn agg(path: &'static str, aggregate: Aggregate, read: fn(&Snapshot) -> Option<f64>) -> Binding {
    Binding::Bound(Bound {
        path,
        aggregate,
        lever: None,
        read,
        previous: None,
    })
}

macro_rules! lever {
    ($key:expr, $path:literal) => {
        Binding::Bound(Bound {
            path: $path,
            aggregate: Aggregate::Lever,
            lever: Some($key),
            read: |s| lever_value(s, $key),
            previous: None,
        })
    };
}

/// The live-value source of every quantity. Exhaustive on purpose: adding a `QuantityId` is a
/// compile error until it is bound or given a reason.
pub fn binding(q: QuantityId) -> Binding {
    use QuantityId as Q;
    use Unbound::*;
    match q {
        // Firms (summed or averaged over active firms).
        Q::Workers => agg("firms[].workers", Aggregate::Sum, |s| {
            firm_sum(s, |f| f.workers)
        }),
        Q::Wage => agg("firms[].wage", Aggregate::Mean, |s| {
            firm_mean(s, |f| f.wage)
        }),
        Q::Price => agg("firms[].price", Aggregate::Mean, |s| {
            firm_mean(s, |f| f.price)
        }),
        Q::Revenue => agg("firms[].revenue", Aggregate::Sum, |s| {
            firm_sum(s, |f| f.revenue)
        }),
        Q::Profit => agg("firms[].profit", Aggregate::Sum, |s| {
            firm_sum(s, |f| f.profit)
        }),
        Q::Capital => agg("firms[].capital", Aggregate::Sum, |s| {
            firm_sum(s, |f| f.capital)
        }),
        Q::Debt => agg("firms[].debt", Aggregate::Sum, |s| firm_sum(s, |f| f.debt)),
        Q::GreenTech => agg("firms[].greenTech", Aggregate::Mean, |s| {
            firm_mean(s, |f| f.green_tech)
        }),
        // Households.
        Q::Savings => agg("households[].savings", Aggregate::Sum, |s| {
            Some(s.households.iter().map(|h| h.savings).sum())
        }),
        Q::NetWealth => agg("households[].netWorth", Aggregate::Sum, |s| {
            Some(s.households.iter().map(|h| h.net_worth).sum())
        }),
        // Town indicators.
        Q::Unemployment => town(
            "indicators.unemployment",
            |s| Some(s.indicators.unemployment),
            Some(|h| Some(h.unemployment)),
        ),
        Q::Inequality => town(
            "indicators.gini",
            |s| Some(s.indicators.gini),
            Some(|h| Some(h.gini)),
        ),
        Q::Wellbeing => town(
            "indicators.avgWellbeing",
            |s| Some(s.indicators.avg_wellbeing),
            Some(|h| Some(h.avg_wellbeing)),
        ),
        // Government and bank.
        Q::Legitimacy => town(
            "government.legitimacy",
            |s| Some(s.government.legitimacy),
            Some(|h| Some(h.legitimacy)),
        ),
        Q::PublicDebtRatio => town(
            "government.debtToGdp",
            |s| Some(s.government.debt_to_gdp),
            Some(|h| Some(h.debt_to_gdp)),
        ),
        Q::PublicDebt => town("government.debt", |s| Some(s.government.debt), None),
        Q::TreasuryCash => town("government.cash", |s| Some(s.government.cash), None),
        Q::BankEquity => town("bank.equity", |s| Some(s.bank.equity), None),
        Q::BankLoans => town("bank.loans", |s| Some(s.bank.loans), None),
        Q::BankDeposits => town("bank.deposits", |s| Some(s.bank.deposits), None),
        Q::BankFailed => Binding::Bound(Bound {
            path: "bank.failed",
            aggregate: Aggregate::Flag,
            lever: None,
            read: |s| Some(flag(s.bank.failed)),
            previous: None,
        }),
        // Markets.
        Q::FoodPrice => town(
            "markets[food].price",
            |s| market_price(s, "food"),
            Some(|h| h.price_food),
        ),
        Q::LoanRate => town(
            "markets[credit].price",
            |s| market_price(s, "credit"),
            Some(|h| h.loan_rate_avg),
        ),
        // Levers (the effective setting where the lever is lobbied).
        Q::CarbonPrice => lever!(LeverKey::CarbonPrice, "policy.carbon_price.effective"),
        Q::EmissionsCap => lever!(LeverKey::EmissionsCap, "policy.emissions_cap.effective"),
        Q::MinWage => lever!(LeverKey::MinWage, "policy.min_wage.nominal"),
        Q::GreenSubsidy => lever!(LeverKey::GreenSubsidy, "policy.green_subsidy.nominal"),
        Q::CampaignCap => lever!(LeverKey::CampaignCap, "policy.campaign_cap.nominal"),
        Q::CapitalRatio => lever!(
            LeverKey::BankCapitalRatio,
            "policy.bank_capital_ratio.nominal"
        ),
        Q::PolicyRate => lever!(LeverKey::BaseRate, "policy.base_rate.nominal"),
        Q::BailoutPolicy => Binding::Bound(Bound {
            path: "policy.bailout.nominal",
            aggregate: Aggregate::Flag,
            lever: Some(LeverKey::Bailout),
            read: |s| lever_value(s, LeverKey::Bailout),
            previous: None,
        }),
        // Nature and the estate.
        Q::CommonsOpen => Binding::Bound(Bound {
            path: "nature.commonsAccess",
            aggregate: Aggregate::Flag,
            lever: None,
            read: |s| Some(flag(s.nature.commons_access == CommonsAccessKey::Open)),
            previous: None,
        }),
        Q::CommonsEnclosed => Binding::Bound(Bound {
            path: "nature.commonsAccess",
            aggregate: Aggregate::Flag,
            lever: None,
            read: |s| Some(flag(s.nature.commons_access == CommonsAccessKey::Enclosed)),
            previous: None,
        }),
        Q::Flight => Binding::Bound(Bound {
            path: "estate.jet.flyingThisMonth",
            aggregate: Aggregate::Flag,
            lever: None,
            read: |s| Some(flag(s.estate.as_ref()?.jet.flying_this_month)),
            previous: None,
        }),
        Q::PilotEmployed => Binding::Bound(Bound {
            path: "estate.pilot.status",
            aggregate: Aggregate::Flag,
            lever: None,
            read: |s| {
                let p = s.estate.as_ref()?.pilot.as_ref()?;
                Some(flag(p.status == PilotStatus::Employed))
            },
            previous: None,
        }),
        // Private working variables of one agent.
        Q::ExpectedSales
        | Q::Inventory
        | Q::InventoryCover
        | Q::SkilledWorkers
        | Q::WantedWorkers
        | Q::Vacancies
        | Q::OverTargetMonths
        | Q::CashReserve
        | Q::Cash
        | Q::ProfitTrend
        | Q::CapitalPerSale
        | Q::GreenPlan
        | Q::Distressed
        | Q::InvestHurdle
        | Q::GreenCashShare
        | Q::InvestmentPlan
        | Q::LoanRequest
        | Q::Payroll
        | Q::DebtService
        | Q::PayoutRatio
        | Q::Dividends
        | Q::PaymentShortfall
        | Q::DistressMonths
        | Q::FirmOpen
        | Q::NewFirm
        | Q::LastDisposable
        | Q::FoodNeed
        | Q::HouseholdSize
        | Q::ConsumptionBudget
        | Q::DemandUnits
        | Q::UnitsSold
        | Q::CapitalIncome
        | Q::LobbySpend
        | Q::SectorReturn
        | Q::PilotUnpaidMonths
        | Q::PilotFree
        | Q::EstateSavings
        | Q::FirmAssets
        | Q::LoanApproved
        | Q::BankDividend => Binding::Unbound(Private),
        // Computed fresh inside a rule each month.
        Q::EffectiveLabour
        | Q::CapitalOutput
        | Q::WeatherFactor
        | Q::ResourceFactor
        | Q::TransitionFactor
        | Q::UnitCost
        | Q::Markup
        | Q::Footprint
        | Q::CreditHeadroom
        | Q::CreditSpreadAdd
        | Q::SectorTfpGrowth
        | Q::EnclosureChance
        | Q::TransitionResistance
        | Q::NextMonthPurchases
        | Q::CommittedSpending
        | Q::SpendingCutRatio => Binding::Unbound(Computed),
        // In the state, not published as a town figure.
        Q::TreasuryTaxes | Q::Borrowing | Q::Repayment | Q::Bailout | Q::BaseRate => {
            Binding::Unbound(NotPublished)
        }
        // Settings of a household or rule, fixed in the game's parameters.
        Q::SpendingPropensity | Q::WealthSpendingRate => Binding::Unbound(Fixed),
        Q::RandomDraw => Binding::Unbound(Random),
    }
}

/// The live value of one quantity on a snapshot (`None` if unbound or nothing to average).
pub fn value(q: QuantityId, s: &Snapshot) -> Option<f64> {
    match binding(q) {
        Binding::Bound(b) => (b.read)(s).filter(|v| v.is_finite()),
        Binding::Unbound(_) => None,
    }
}

/// The wire's trend id from two values and the steady band (a fraction).
pub fn trend(now: f64, before: f64, steady_band: f64) -> &'static str {
    let scale = now.abs().max(before.abs());
    if scale == 0.0 || (now - before).abs() <= steady_band * scale {
        "steady"
    } else if now > before {
        "rising"
    } else {
        "falling"
    }
}

/// Contract v2.9: the current value of one bound quantity.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantityLive {
    /// The `QuantityId` wire name (`catalog().quantities`).
    pub id: String,
    pub value: f64,
    /// The same figure at the end of the previous turn; only where the history keeps the series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous: Option<f64>,
    /// `rising`, `steady` or `falling` against `previous`, the same ids and band as the Markets
    /// price direction (words: `catalog().markets.directions`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trend: Option<String>,
}

/// Every bound quantity that has a value now, in `QuantityId::ALL` order.
pub fn build(s: &Snapshot, history: &[HistoryPoint], steady_band: f64) -> Vec<QuantityLive> {
    // Compare with the end of the previous turn: when no month of the current turn has been
    // played yet the latest history point *is* now, so the one before it is the baseline.
    let last = if s.turn_phase.months_done == 0 {
        history.len().checked_sub(2).and_then(|i| history.get(i))
    } else {
        history.last()
    };
    QuantityId::ALL
        .iter()
        .filter_map(|q| {
            let Binding::Bound(b) = binding(*q) else {
                return None;
            };
            let v = (b.read)(s).filter(|v| v.is_finite())?;
            let previous = last.and_then(|h| b.previous.and_then(|f| f(h)));
            Some(QuantityLive {
                id: q.as_str().to_string(),
                value: v,
                previous,
                trend: previous.map(|p| trend(v, p, steady_band).to_string()),
            })
        })
        .collect()
}
