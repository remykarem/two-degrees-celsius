//! Closed key sets of the contract (snake_case strings on the wire) and their conversions
//! from and to the `sim-types` enums.
use serde::{Deserialize, Serialize};
use sim_types as st;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeverKey {
    CarbonPrice,
    TaxWage,
    TaxProfit,
    TaxCapital,
    WealthTax,
    WealthThreshold,
    BaseRate,
    MinWage,
    Benefit,
    Ubi,
    GreenSubsidy,
    CampaignCap,
    EmissionsCap,
    CommonsHarvest,
    CommonsAccess,
    BankCapitalRatio,
    Bailout,
    PublicSpending,
    JobGuarantee,
}

impl LeverKey {
    pub const ALL: [LeverKey; 19] = [
        LeverKey::CarbonPrice,
        LeverKey::TaxWage,
        LeverKey::TaxProfit,
        LeverKey::TaxCapital,
        LeverKey::WealthTax,
        LeverKey::WealthThreshold,
        LeverKey::BaseRate,
        LeverKey::MinWage,
        LeverKey::Benefit,
        LeverKey::Ubi,
        LeverKey::GreenSubsidy,
        LeverKey::CampaignCap,
        LeverKey::EmissionsCap,
        LeverKey::CommonsHarvest,
        LeverKey::CommonsAccess,
        LeverKey::BankCapitalRatio,
        LeverKey::Bailout,
        LeverKey::PublicSpending,
        LeverKey::JobGuarantee,
    ];

    /// The key used in `content/params.json` (`turn.levers`); identical to the wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            LeverKey::CarbonPrice => "carbon_price",
            LeverKey::TaxWage => "tax_wage",
            LeverKey::TaxProfit => "tax_profit",
            LeverKey::TaxCapital => "tax_capital",
            LeverKey::WealthTax => "wealth_tax",
            LeverKey::WealthThreshold => "wealth_threshold",
            LeverKey::BaseRate => "base_rate",
            LeverKey::MinWage => "min_wage",
            LeverKey::Benefit => "benefit",
            LeverKey::Ubi => "ubi",
            LeverKey::GreenSubsidy => "green_subsidy",
            LeverKey::CampaignCap => "campaign_cap",
            LeverKey::EmissionsCap => "emissions_cap",
            LeverKey::CommonsHarvest => "commons_harvest",
            LeverKey::CommonsAccess => "commons_access",
            LeverKey::BankCapitalRatio => "bank_capital_ratio",
            LeverKey::Bailout => "bailout",
            LeverKey::PublicSpending => "public_spending",
            LeverKey::JobGuarantee => "job_guarantee",
        }
    }

    pub fn from_policy(c: &st::PolicyChange) -> LeverKey {
        use st::PolicyChange as P;
        match c {
            P::CarbonPrice(_) => LeverKey::CarbonPrice,
            P::TaxWage(_) => LeverKey::TaxWage,
            P::TaxProfit(_) => LeverKey::TaxProfit,
            P::TaxCapital(_) => LeverKey::TaxCapital,
            P::WealthTax(_) => LeverKey::WealthTax,
            P::WealthThreshold(_) => LeverKey::WealthThreshold,
            P::BaseRate(_) => LeverKey::BaseRate,
            P::MinWage(_) => LeverKey::MinWage,
            P::Benefit(_) => LeverKey::Benefit,
            P::Ubi(_) => LeverKey::Ubi,
            P::GreenSubsidy(_) => LeverKey::GreenSubsidy,
            P::CampaignCap(_) => LeverKey::CampaignCap,
            P::EmissionsCap(_) => LeverKey::EmissionsCap,
            P::CommonsHarvest(_) => LeverKey::CommonsHarvest,
            P::CommonsAccess(_) => LeverKey::CommonsAccess,
            P::BankCapitalRatio(_) => LeverKey::BankCapitalRatio,
            P::Bailout(_) => LeverKey::Bailout,
            P::PublicSpending(_) => LeverKey::PublicSpending,
            P::JobGuarantee(_) => LeverKey::JobGuarantee,
        }
    }

    pub fn from_lever(l: st::Lever) -> LeverKey {
        match l {
            st::Lever::CarbonPrice => LeverKey::CarbonPrice,
            st::Lever::WealthTax => LeverKey::WealthTax,
            st::Lever::ProfitTax => LeverKey::TaxProfit,
            st::Lever::EmissionsCap => LeverKey::EmissionsCap,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfraKind {
    PublicHousing,
    FossilPlant,
    RenewablePlant,
    Road,
}

impl InfraKind {
    pub const ALL: [InfraKind; 4] = [
        InfraKind::PublicHousing,
        InfraKind::FossilPlant,
        InfraKind::RenewablePlant,
        InfraKind::Road,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            InfraKind::PublicHousing => "public_housing",
            InfraKind::FossilPlant => "fossil_plant",
            InfraKind::RenewablePlant => "renewable_plant",
            InfraKind::Road => "road",
        }
    }
    pub fn to_sim(self) -> st::InfraKind {
        match self {
            InfraKind::PublicHousing => st::InfraKind::PublicHousing,
            InfraKind::FossilPlant => st::InfraKind::FossilPlant,
            InfraKind::RenewablePlant => st::InfraKind::RenewablePlant,
            InfraKind::Road => st::InfraKind::Road,
        }
    }
    pub fn from_sim(k: st::InfraKind) -> InfraKind {
        match k {
            st::InfraKind::PublicHousing => InfraKind::PublicHousing,
            st::InfraKind::FossilPlant => InfraKind::FossilPlant,
            st::InfraKind::RenewablePlant => InfraKind::RenewablePlant,
            st::InfraKind::Road => InfraKind::Road,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemKey {
    Market,
    Regulated,
    Coop,
}

impl SystemKey {
    pub const ALL: [SystemKey; 3] = [SystemKey::Market, SystemKey::Regulated, SystemKey::Coop];
    pub fn to_sim(self) -> st::EconomicSystem {
        match self {
            SystemKey::Market => st::EconomicSystem::Market,
            SystemKey::Regulated => st::EconomicSystem::Regulated,
            SystemKey::Coop => st::EconomicSystem::Coop,
        }
    }
    pub fn from_sim(s: st::EconomicSystem) -> SystemKey {
        match s {
            st::EconomicSystem::Market => SystemKey::Market,
            st::EconomicSystem::Regulated => SystemKey::Regulated,
            st::EconomicSystem::Coop => SystemKey::Coop,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassKey {
    Capitalist,
    Rentier,
    Manager,
    Worker,
    Dependent,
}

impl ClassKey {
    pub fn from_sim(c: st::Class) -> ClassKey {
        match c {
            st::Class::Capitalist => ClassKey::Capitalist,
            st::Class::Rentier => ClassKey::Rentier,
            st::Class::Manager => ClassKey::Manager,
            st::Class::Worker => ClassKey::Worker,
            st::Class::Dependent => ClassKey::Dependent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    TransitionInProgress,
    NothingToRemove,
    SameSystem,
    LeverUnavailable,
    InsufficientPoliticalCapital,
    InsufficientFunds,
    InvalidCommand,
    TileOccupied,
    TileInvalid,
}

impl BlockReason {
    pub fn from_sim(r: st::BlockReason) -> BlockReason {
        match r {
            st::BlockReason::TransitionInProgress => BlockReason::TransitionInProgress,
            st::BlockReason::NothingToRemove => BlockReason::NothingToRemove,
            st::BlockReason::SameSystem => BlockReason::SameSystem,
            st::BlockReason::LeverUnavailable => BlockReason::LeverUnavailable,
            st::BlockReason::InsufficientPoliticalCapital => {
                BlockReason::InsufficientPoliticalCapital
            }
            st::BlockReason::InsufficientFunds => BlockReason::InsufficientFunds,
            st::BlockReason::InvalidCommand => BlockReason::InvalidCommand,
            st::BlockReason::TileOccupied => BlockReason::TileOccupied,
            st::BlockReason::TileInvalid => BlockReason::TileInvalid,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKey {
    Gdp,
    Gpi,
    Unemployment,
    Gini,
    WageShare,
    ProfitShare,
    Wellbeing,
    Emissions,
    Budget,
    Temperature,
    Legitimacy,
    Debt,
    Money,
    Prices,
    Housing,
    Forest,
    Commons,
    AvgIncome,
    MedianIncome,
    TopWealthShare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollapseReason {
    LegitimacyZero,
    MassUnemployment,
    BankFailureCascade,
}

impl CollapseReason {
    pub fn from_sim(r: st::CollapseReason) -> CollapseReason {
        match r {
            st::CollapseReason::LegitimacyZero => CollapseReason::LegitimacyZero,
            st::CollapseReason::MassUnemployment => CollapseReason::MassUnemployment,
            st::CollapseReason::BankFailureCascade => CollapseReason::BankFailureCascade,
        }
    }
}

/// Sector of a firm (`goods`, `food` or `services`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectorKey {
    Goods,
    Food,
    Services,
}

impl SectorKey {
    pub fn from_sim(s: st::Sector) -> SectorKey {
        match s {
            st::Sector::Goods => SectorKey::Goods,
            st::Sector::Food => SectorKey::Food,
            st::Sector::Services => SectorKey::Services,
        }
    }
}

/// The game outcome as the contract writes it: a bare string, or `{ "collapse": reason }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GameOutcome {
    Plain(PlainOutcome),
    Collapse { collapse: CollapseReason },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlainOutcome {
    BudgetExhausted,
    Survived,
}

impl GameOutcome {
    pub fn from_sim(o: st::Outcome) -> GameOutcome {
        match o {
            st::Outcome::BudgetExhausted => GameOutcome::Plain(PlainOutcome::BudgetExhausted),
            st::Outcome::Survived => GameOutcome::Plain(PlainOutcome::Survived),
            st::Outcome::Collapse(r) => GameOutcome::Collapse {
                collapse: CollapseReason::from_sim(r),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKindKey {
    Empty,
    Home,
    PublicHousing,
    Firm,
    Bank,
    TownHall,
    FossilPlant,
    RenewablePlant,
    Road,
    Forest,
    Commons,
}

impl TileKindKey {
    pub fn from_sim(k: st::TileKind) -> TileKindKey {
        match k {
            st::TileKind::Empty => TileKindKey::Empty,
            st::TileKind::Home => TileKindKey::Home,
            st::TileKind::PublicHousing => TileKindKey::PublicHousing,
            st::TileKind::Firm => TileKindKey::Firm,
            st::TileKind::Bank => TileKindKey::Bank,
            st::TileKind::TownHall => TileKindKey::TownHall,
            st::TileKind::FossilPlant => TileKindKey::FossilPlant,
            st::TileKind::RenewablePlant => TileKindKey::RenewablePlant,
            st::TileKind::Road => TileKindKey::Road,
            st::TileKind::Forest => TileKindKey::Forest,
            st::TileKind::Commons => TileKindKey::Commons,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeBandKey {
    Child,
    Adult,
    Retired,
}

impl AgeBandKey {
    pub fn from_sim(a: st::AgeBand) -> AgeBandKey {
        match a {
            st::AgeBand::Child => AgeBandKey::Child,
            st::AgeBand::Adult => AgeBandKey::Adult,
            st::AgeBand::Retired => AgeBandKey::Retired,
        }
    }
}
