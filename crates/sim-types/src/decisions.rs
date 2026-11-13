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
    /// Who makes a decision. Every owner kind has registered decisions (Nature is not an agent; its one decision is the commons enclosure).
    pub enum OwnerKind {
        Firm => "firm",
        Household => "household",
        Bank => "bank",
        Government => "government",
        Estate => "estate",
        Nature => "nature",
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
