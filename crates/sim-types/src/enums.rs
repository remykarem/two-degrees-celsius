use crate::ids::*;
use crate::state::Policy;
use crate::units::*;
use serde::{Deserialize, Serialize};

/// Derived class label (S9). Never assigned by hand after setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Class {
    Capitalist,
    Rentier,
    Manager,
    Worker,
    Dependent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Sector {
    Goods,
    Food,
}

/// Who owns a firm; decides where dividends go (S4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Owner {
    Household(HouseholdId),
    Workers,
    State,
    Bank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomicSystem {
    Market,
    Regulated,
    Coop,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommonsAccess {
    Open,
    Enclosed(HouseholdId),
    /// Harvest capped at `cap` times maximum sustainable yield.
    Community {
        cap: Rate,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfraKind {
    PublicHousing,
    FossilPlant,
    RenewablePlant,
}

/// One policy lever change. Money levers are in cents, shares in model units.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PolicyChange {
    /// Cents per tonne CO2.
    CarbonPrice(Cents),
    TaxWage(Rate),
    TaxProfit(Rate),
    TaxCapital(Rate),
    /// Annual rate on net wealth above the threshold.
    WealthTax(Rate),
    WealthThreshold(Cents),
    /// Annual base interest rate.
    BaseRate(Rate),
    MinWage(Cents),
    /// Monthly benefit per unemployed or dependent person.
    Benefit(Cents),
    /// Monthly universal basic income per person.
    Ubi(Cents),
    /// Share of green investment paid by the government.
    GreenSubsidy(Rate),
    /// Max share of capital income that may be spent on lobbying.
    CampaignCap(Rate),
    /// Per-firm monthly emissions cap in tonnes (0 = none).
    EmissionsCap(Tonnes),
    CommonsHarvest(Rate),
    CommonsAccess(CommonsAccess),
    BankCapitalRatio(Rate),
    Bailout(bool),
    /// Government goods purchases as a share of monthly GDP (the G lever).
    PublicSpending(Rate),
    /// Share of the unemployed the government hires at the minimum wage.
    JobGuarantee(Rate),
}

impl PolicyChange {
    /// Key used in content tables and plan files.
    pub fn key(&self) -> &'static str {
        match self {
            PolicyChange::CarbonPrice(_) => "carbon_price",
            PolicyChange::TaxWage(_) => "tax_wage",
            PolicyChange::TaxProfit(_) => "tax_profit",
            PolicyChange::TaxCapital(_) => "tax_capital",
            PolicyChange::WealthTax(_) => "wealth_tax",
            PolicyChange::WealthThreshold(_) => "wealth_threshold",
            PolicyChange::BaseRate(_) => "base_rate",
            PolicyChange::MinWage(_) => "min_wage",
            PolicyChange::Benefit(_) => "benefit",
            PolicyChange::Ubi(_) => "ubi",
            PolicyChange::GreenSubsidy(_) => "green_subsidy",
            PolicyChange::CampaignCap(_) => "campaign_cap",
            PolicyChange::EmissionsCap(_) => "emissions_cap",
            PolicyChange::CommonsHarvest(_) => "commons_harvest",
            PolicyChange::CommonsAccess(_) => "commons_access",
            PolicyChange::BankCapitalRatio(_) => "bank_capital_ratio",
            PolicyChange::Bailout(_) => "bailout",
            PolicyChange::PublicSpending(_) => "public_spending",
            PolicyChange::JobGuarantee(_) => "job_guarantee",
        }
    }

    /// The new value in friendly units (dollars, tonnes, fractions); `None` for
    /// non-numeric levers, whose size of change is 1.
    pub fn value(&self) -> Option<f64> {
        Some(match *self {
            PolicyChange::CarbonPrice(c)
            | PolicyChange::WealthThreshold(c)
            | PolicyChange::MinWage(c)
            | PolicyChange::Benefit(c)
            | PolicyChange::Ubi(c) => c.dollars(),
            PolicyChange::TaxWage(r)
            | PolicyChange::TaxProfit(r)
            | PolicyChange::TaxCapital(r)
            | PolicyChange::WealthTax(r)
            | PolicyChange::BaseRate(r)
            | PolicyChange::GreenSubsidy(r)
            | PolicyChange::CampaignCap(r)
            | PolicyChange::CommonsHarvest(r)
            | PolicyChange::BankCapitalRatio(r)
            | PolicyChange::PublicSpending(r)
            | PolicyChange::JobGuarantee(r) => r.0,
            PolicyChange::EmissionsCap(t) => t.0,
            PolicyChange::CommonsAccess(_) | PolicyChange::Bailout(_) => return None,
        })
    }

    /// Does raising (or for the emissions cap, lowering) this lever hurt the capital-owning classes?
    /// Those changes face lobbying opposition (S9b) and cost more political capital.
    pub fn opposed_by_capital(&self, active: &Policy) -> bool {
        match *self {
            PolicyChange::CarbonPrice(c) => c > active.carbon_price,
            PolicyChange::WealthTax(r) => r > active.wealth_tax,
            PolicyChange::TaxProfit(r) => r > active.tax_profit,
            PolicyChange::EmissionsCap(t) => {
                t.0 > 0.0 && (active.emissions_cap.0 <= 0.0 || t < active.emissions_cap)
            }
            _ => false,
        }
    }
}

impl Command {
    /// Key of the lever table in `content/params.json` (`turn.levers`).
    pub fn lever_key(&self) -> &'static str {
        match self {
            Command::SetPolicy(c) => c.key(),
            Command::SwitchSystem(_) => "switch_system",
            Command::Place { kind, .. } => match kind {
                InfraKind::PublicHousing => "place_public_housing",
                InfraKind::FossilPlant => "place_fossil_plant",
                InfraKind::RenewablePlant => "place_renewable_plant",
            },
            Command::Remove { kind, .. } => match kind {
                InfraKind::PublicHousing => "remove_public_housing",
                InfraKind::FossilPlant => "remove_fossil_plant",
                InfraKind::RenewablePlant => "remove_renewable_plant",
            },
        }
    }
}

/// Every key the lever table must define.
pub const LEVER_KEYS: [&str; 26] = [
    "carbon_price",
    "tax_wage",
    "tax_profit",
    "tax_capital",
    "wealth_tax",
    "wealth_threshold",
    "base_rate",
    "min_wage",
    "benefit",
    "ubi",
    "green_subsidy",
    "campaign_cap",
    "emissions_cap",
    "commons_harvest",
    "commons_access",
    "bank_capital_ratio",
    "bailout",
    "public_spending",
    "job_guarantee",
    "switch_system",
    "place_public_housing",
    "place_fossil_plant",
    "place_renewable_plant",
    "remove_public_housing",
    "remove_fossil_plant",
    "remove_renewable_plant",
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Place { kind: InfraKind, count: u32 },
    Remove { kind: InfraKind, count: u32 },
    SetPolicy(PolicyChange),
    SwitchSystem(EconomicSystem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollapseReason {
    LegitimacyZero,
    MassUnemployment,
    BankFailureCascade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    BudgetExhausted,
    Collapse(CollapseReason),
    Survived,
}

/// Which simulation system produced a change (used in checks and error messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SystemId {
    Policy,
    Labour,
    Production,
    Nature,
    Goods,
    Income,
    Credit,
    Government,
    Investment,
    Class,
    Lobbying,
    Wellbeing,
    Metrics,
    Commands,
}

impl SystemId {
    pub fn name(self) -> &'static str {
        match self {
            SystemId::Policy => "policy",
            SystemId::Labour => "S1 labour",
            SystemId::Production => "S2 production",
            SystemId::Nature => "S7 natural capital",
            SystemId::Goods => "S3 goods and housing",
            SystemId::Income => "S4 income distribution",
            SystemId::Credit => "S5 credit and money",
            SystemId::Government => "S8 government",
            SystemId::Investment => "S6 investment",
            SystemId::Class => "S9 class and mobility",
            SystemId::Lobbying => "S9b lobbying",
            SystemId::Wellbeing => "S10 wellbeing",
            SystemId::Metrics => "metrics",
            SystemId::Commands => "commands",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmissionSource {
    Firm(FirmId),
    Households,
    Sink,
}

/// A holder of a deposit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Account {
    Household(HouseholdId),
    Firm(FirmId),
    Government,
}

/// What a transfer is for; the engine uses it to record household income by type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowKind {
    Wage,
    Dividend,
    Rent,
    Benefit,
    Tax,
    Purchase,
    Lobby,
    /// Neutral reallocation (mobility, subsidies, liquidation).
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GdpPart {
    Investment,
    GovServices,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Commuted {
        household: HouseholdId,
        firm: FirmId,
        people: People,
    },
    Hired {
        firm: FirmId,
        people: People,
    },
    Fired {
        firm: FirmId,
        people: People,
    },
    LoanDefaulted {
        firm: FirmId,
    },
    FirmFounded {
        firm: FirmId,
    },
    FirmClosed {
        firm: FirmId,
    },
    ForestShrank {
        stock: Units,
    },
    CommonsCollapsed,
    CommonsEnclosed {
        owner: HouseholdId,
    },
    PolicyBlocked {
        reason: BlockReason,
    },
    /// A pending command became active.
    CommandActivated {
        id: u32,
    },
    BankFailed,
    BankBailout {
        amount: Cents,
    },
    ClassChanged {
        household: HouseholdId,
        to: Class,
    },
    SystemSwitchStarted {
        to: EconomicSystem,
    },
    SystemSwitchCompleted {
        to: EconomicSystem,
    },
    BudgetExhausted,
    Collapse {
        reason: CollapseReason,
    },
}

/// Contribution records that explain why a number changed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TraceRecord {
    Emissions {
        source: EmissionSource,
        tonnes: Tonnes,
    },
    Income {
        household: HouseholdId,
        kind: IncomeKind,
        amount: Cents,
    },
    /// Lobbying erosion of a lever: nominal vs effective and the capital share of the pressure.
    PolicyGap {
        lever: Lever,
        nominal: f64,
        effective: f64,
        net_lobby: Rate,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncomeKind {
    Wages,
    Dividends,
    Rent,
    Interest,
    Benefits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lever {
    CarbonPrice,
    WealthTax,
    ProfitTax,
    EmissionsCap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
    TransitionInProgress,
    NothingToRemove,
    SameSystem,
    LeverUnavailable,
    InsufficientPoliticalCapital,
    InsufficientFunds,
    InvalidCommand,
}
