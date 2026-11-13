//! Decision points (`docs/features/decision-points.md`): data only. A decision is a pure function in
//! `sim-model` over a read-only view; its `DecisionSpec` declares what it reads, what it writes and
//! which parameters it uses, so that "who decides what" and a future causality view can be generated
//! from code. Labels, units and prose live in `content/decisions.json` (AGENTS.md rule 9).

use crate::enums::SystemId;

/// Declares a closed enum with a snake_case wire name per variant, `ALL` (declaration order) and
/// `as_str`. One macro for three enums: the variant list is written once, so the list, the
/// wire name and `ALL` cannot disagree.
macro_rules! named_enum {
    ($(#[$m:meta])* pub enum $name:ident { $($(#[$vm:meta])* $v:ident => $s:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name { $($(#[$vm])* $v),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$v),+];
            /// The wire name (snake_case).
            pub const fn as_str(self) -> &'static str {
                match self { $($name::$v => $s),+ }
            }
        }
    };
}

named_enum! {
    /// Who makes a decision, or whose physics a mechanism describes. Every owner kind except Weather has registered decisions (Nature is not an agent; its one decision is the commons enclosure); Weather owns mechanisms and quantities only (causality graph stage 1).
    pub enum OwnerKind {
        Firm => "firm",
        Household => "household",
        Bank => "bank",
        Government => "government",
        Estate => "estate",
        Nature => "nature",
        Weather => "weather",
    }
}

named_enum! {
    /// Every decision registered in `sim-model::decisions::DECISIONS`.
    pub enum DecisionId {
        FirmHire => "firm_hire",
        FirmFire => "firm_fire",
        FirmWage => "firm_wage",
        FirmPrice => "firm_price",
        FirmGreen => "firm_green",
        FirmCapex => "firm_capex",
        FirmPayout => "firm_payout",
        FirmExit => "firm_exit",
        HouseholdFoundFirm => "household_found_firm",
        HouseholdSpend => "household_spend",
        HouseholdShop => "household_shop",
        HouseholdLobby => "household_lobby",
        FirmExpect => "firm_expect",
        BankHeadroom => "bank_headroom",
        BankLend => "bank_lend",
        BankSpread => "bank_spread",
        BankDividend => "bank_dividend",
        BankRescue => "bank_rescue",
        GovernmentFund => "government_fund",
        GovernmentCut => "government_cut",
        GovernmentLegitimacy => "government_legitimacy",
        EstatePilot => "estate_pilot",
        EstateFlight => "estate_flight",
        NatureEnclosure => "nature_enclosure",
    }
}

impl DecisionId {
    /// The owner is derived from the id (an exhaustive match), never stored, so the two cannot disagree.
    pub const fn owner(self) -> OwnerKind {
        match self {
            DecisionId::FirmHire
            | DecisionId::FirmFire
            | DecisionId::FirmWage
            | DecisionId::FirmPrice
            | DecisionId::FirmGreen
            | DecisionId::FirmCapex
            | DecisionId::FirmPayout
            | DecisionId::FirmExit
            | DecisionId::FirmExpect => OwnerKind::Firm,
            DecisionId::HouseholdFoundFirm
            | DecisionId::HouseholdSpend
            | DecisionId::HouseholdShop
            | DecisionId::HouseholdLobby => OwnerKind::Household,
            DecisionId::BankHeadroom
            | DecisionId::BankLend
            | DecisionId::BankSpread
            | DecisionId::BankDividend
            | DecisionId::BankRescue => OwnerKind::Bank,
            DecisionId::GovernmentFund
            | DecisionId::GovernmentCut
            | DecisionId::GovernmentLegitimacy => OwnerKind::Government,
            DecisionId::EstatePilot | DecisionId::EstateFlight => OwnerKind::Estate,
            DecisionId::NatureEnclosure => OwnerKind::Nature,
        }
    }
}

named_enum! {
    /// Every mechanism registered in `sim-model::decisions::MECHANISMS`: plain code (physics and
    /// accounting) that connects the decisions' quantities (causality graph stage 1). A mechanism
    /// is declared, never chosen: it changes no behaviour.
    pub enum MechanismId {
        ProductionTechnology => "production_technology",
        TechnologyGrowth => "technology_growth",
        CapitalAccumulation => "capital_accumulation",
        EffectiveLabourSupply => "effective_labour_supply",
        FarmHealthFactor => "farm_health_factor",
        ClimateDamage => "climate_damage",
        DroughtLoss => "drought_loss",
        FireSmoke => "fire_smoke",
        TransitionLoss => "transition_loss",
        ProductionOutput => "production_output",
        FoodCap => "food_cap",
        InventoryStock => "inventory_stock",
        ElectricityDemand => "electricity_demand",
        ElectricityRationing => "electricity_rationing",
        GridIntensityMix => "grid_intensity_mix",
        FirmEmissions => "firm_emissions",
        PlantEmissionsOutput => "plant_emissions_output",
        EmissionsAccounting => "emissions_accounting",
        CarbonBudget => "carbon_budget",
        ForestSinkUptake => "forest_sink_uptake",
        ForestStockChange => "forest_stock_change",
        CommonsStockChange => "commons_stock_change",
        FireBurnsForest => "fire_burns_forest",
        ClimateStressCurve => "climate_stress_curve",
        DroughtOnset => "drought_onset",
        FireIgnition => "fire_ignition",
        WellbeingScore => "wellbeing_score",
        InequalityGini => "inequality_gini",
        LastDisposableRecord => "last_disposable_record",
        WagePayment => "wage_payment",
        HouseholdIncomeFlows => "household_income_flows",
        FirmProfit => "firm_profit",
        TaxCollection => "tax_collection",
        GoodsSettlement => "goods_settlement",
        UnitCostBuildup => "unit_cost_buildup",
        HousingRent => "housing_rent",
        HousingMoves => "housing_moves",
        LabourMatching => "labour_matching",
        UnemploymentRate => "unemployment_rate",
    }
}

impl MechanismId {
    /// The owner is derived from the id (an exhaustive match), like a decision's, so the two cannot
    /// disagree. Production physics belong to firms, the stocks and the grid's emissions to nature,
    /// drought and fire to weather.
    pub const fn owner(self) -> OwnerKind {
        use MechanismId as M;
        match self {
            M::ProductionTechnology
            | M::TechnologyGrowth
            | M::CapitalAccumulation
            | M::EffectiveLabourSupply
            | M::FarmHealthFactor
            | M::ClimateDamage
            | M::DroughtLoss
            | M::FireSmoke
            | M::TransitionLoss
            | M::ProductionOutput
            | M::FoodCap
            | M::InventoryStock
            | M::ElectricityDemand
            | M::FirmEmissions
            | M::WagePayment
            | M::FirmProfit
            | M::GoodsSettlement
            | M::UnitCostBuildup => OwnerKind::Firm,
            M::ElectricityRationing
            | M::GridIntensityMix
            | M::PlantEmissionsOutput
            | M::EmissionsAccounting
            | M::CarbonBudget
            | M::ForestSinkUptake
            | M::ForestStockChange
            | M::CommonsStockChange
            | M::FireBurnsForest => OwnerKind::Nature,
            M::ClimateStressCurve | M::DroughtOnset | M::FireIgnition => OwnerKind::Weather,
            M::WellbeingScore
            | M::InequalityGini
            | M::LastDisposableRecord
            | M::HouseholdIncomeFlows
            | M::HousingRent
            | M::HousingMoves
            | M::LabourMatching
            | M::UnemploymentRate => OwnerKind::Household,
            M::TaxCollection => OwnerKind::Government,
        }
    }
}

named_enum! {
    /// The vocabulary of state variables and flows that decisions read or write.
    pub enum QuantityId {
        ExpectedSales => "expected_sales",
        Inventory => "inventory",
        InventoryCover => "inventory_cover",
        Workers => "workers",
        SkilledWorkers => "skilled_workers",
        EffectiveLabour => "effective_labour",
        CapitalOutput => "capital_output",
        WeatherFactor => "weather_factor",
        ResourceFactor => "resource_factor",
        TransitionFactor => "transition_factor",
        WantedWorkers => "wanted_workers",
        Vacancies => "vacancies",
        OverTargetMonths => "over_target_months",
        Unemployment => "unemployment",
        Wage => "wage",
        MinWage => "min_wage",
        SectorTfpGrowth => "sector_tfp_growth",
        Price => "price",
        Markup => "markup",
        UnitCost => "unit_cost",
        Footprint => "footprint",
        CarbonPrice => "carbon_price",
        Revenue => "revenue",
        EmissionsCap => "emissions_cap",
        GreenSubsidy => "green_subsidy",
        TreasuryCash => "treasury_cash",
        Cash => "cash",
        CashReserve => "cash_reserve",
        GreenTech => "green_tech",
        GreenPlan => "green_plan",
        Profit => "profit",
        ProfitTrend => "profit_trend",
        Capital => "capital",
        CapitalPerSale => "capital_per_sale",
        LoanRate => "loan_rate",
        Distressed => "distressed",
        InvestHurdle => "invest_hurdle",
        GreenCashShare => "green_cash_share",
        InvestmentPlan => "investment_plan",
        LoanRequest => "loan_request",
        Payroll => "payroll",
        DebtService => "debt_service",
        PayoutRatio => "payout_ratio",
        Dividends => "dividends",
        Debt => "debt",
        PaymentShortfall => "payment_shortfall",
        DistressMonths => "distress_months",
        FirmOpen => "firm_open",
        SectorReturn => "sector_return",
        BaseRate => "base_rate",
        RandomDraw => "random_draw",
        NewFirm => "new_firm",
        Savings => "savings",
        LastDisposable => "last_disposable",
        NetWealth => "net_wealth",
        FoodPrice => "food_price",
        FoodNeed => "food_need",
        HouseholdSize => "household_size",
        SpendingPropensity => "spending_propensity",
        WealthSpendingRate => "wealth_spending_rate",
        ConsumptionBudget => "consumption_budget",
        DemandUnits => "demand_units",
        UnitsSold => "units_sold",
        CapitalIncome => "capital_income",
        CampaignCap => "campaign_cap",
        LobbySpend => "lobby_spend",
        BankEquity => "bank_equity",
        BankLoans => "bank_loans",
        BankDeposits => "bank_deposits",
        CapitalRatio => "capital_ratio",
        CreditHeadroom => "credit_headroom",
        FirmAssets => "firm_assets",
        LoanApproved => "loan_approved",
        CreditSpreadAdd => "credit_spread_add",
        BankDividend => "bank_dividend",
        BailoutPolicy => "bailout_policy",
        BankFailed => "bank_failed",
        PolicyRate => "policy_rate",
        CommittedSpending => "committed_spending",
        NextMonthPurchases => "next_month_purchases",
        TreasuryTaxes => "treasury_taxes",
        PublicDebt => "public_debt",
        Borrowing => "borrowing",
        Repayment => "repayment",
        SpendingCutRatio => "spending_cut_ratio",
        Legitimacy => "legitimacy",
        Wellbeing => "wellbeing",
        Inequality => "inequality",
        PublicDebtRatio => "public_debt_ratio",
        Bailout => "bailout",
        TransitionResistance => "transition_resistance",
        PilotEmployed => "pilot_employed",
        PilotUnpaidMonths => "pilot_unpaid_months",
        PilotFree => "pilot_free",
        EstateSavings => "estate_savings",
        Flight => "flight",
        EnclosureChance => "enclosure_chance",
        CommonsOpen => "commons_open",
        CommonsEnclosed => "commons_enclosed",
        // Causality graph stage 1: state the mechanisms read and write.
        ForestStock => "forest_stock",
        CommonsStock => "commons_stock",
        CommonsCommunity => "commons_community",
        CommonsHarvest => "commons_harvest",
        CumulativeEmissions => "cumulative_emissions",
        GrossEmissions => "gross_emissions",
        ForestSink => "forest_sink",
        NetEmissions => "net_emissions",
        DirectEmissions => "direct_emissions",
        PlantEmissions => "plant_emissions",
        GridIntensity => "grid_intensity",
        RenewableShare => "renewable_share",
        PlantCapacity => "plant_capacity",
        EnergyDemand => "energy_demand",
        ElectricityServed => "electricity_served",
        ClimateStress => "climate_stress",
        Drought => "drought",
        Fire => "fire",
        ResilienceSpending => "resilience_spending",
        SystemChange => "system_change",
        Output => "output",
        Technology => "technology",
        Consumption => "consumption",
        Rent => "rent",
        Occupancy => "occupancy",
        ProfitTaxRate => "profit_tax_rate",
        WageTaxRate => "wage_tax_rate",
        CapitalTaxRate => "capital_tax_rate",
        WealthTaxRate => "wealth_tax_rate",
        WealthThreshold => "wealth_threshold",
    }
}

/// The declaration of one decision. `inputs` and `outputs` are the edges of the causal graph;
/// `params` are dotted paths into `content/params.json`; `rule` is the id of the default rule
/// (its words are in `content/decisions.json`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionSpec {
    pub id: DecisionId,
    pub system: SystemId,
    pub inputs: &'static [QuantityId],
    pub outputs: &'static [QuantityId],
    pub params: &'static [&'static str],
    pub rule: &'static str,
}

impl DecisionSpec {
    pub const fn owner(&self) -> OwnerKind {
        self.id.owner()
    }
}

/// The declaration of one mechanism: plain physics or accounting code that no decision makes but
/// that links decisions' quantities (production, emissions, weather, wellbeing, rent, taxes,
/// matching). Same shape as a decision's: `inputs` and `outputs` are graph edges, `params` are
/// dotted paths into `content/params.json` the code reads, `rule` is the id of its words in
/// `content/decisions.json`. It is read-only documentation of code: declaring one changes no
/// behaviour, and `tests/mechanism_params.rs` checks the declared parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MechanismSpec {
    pub id: MechanismId,
    pub system: SystemId,
    pub inputs: &'static [QuantityId],
    pub outputs: &'static [QuantityId],
    pub params: &'static [&'static str],
    pub rule: &'static str,
}

impl MechanismSpec {
    pub const fn owner(&self) -> OwnerKind {
        self.id.owner()
    }
}
