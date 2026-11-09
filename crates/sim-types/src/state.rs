use crate::enums::*;
use crate::ids::*;
use crate::units::*;
use serde::{Deserialize, Serialize};

/// Monthly income and outgoings of a cohort, by type. Reset at the start of each tick.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IncomeFlows {
    pub wages: Cents,
    pub dividends: Cents,
    pub rent_received: Cents,
    pub interest_received: Cents,
    pub benefits: Cents,
    pub tax_paid: Cents,
    pub rent_paid: Cents,
    pub loan_payments: Cents,
    pub spending: Cents,
    pub lobby_spent: Cents,
}

impl IncomeFlows {
    pub fn capital_income(&self) -> Cents {
        self.dividends + self.rent_received + self.interest_received
    }
    pub fn gross_income(&self) -> Cents {
        self.wages + self.capital_income() + self.benefits
    }
    pub fn disposable(&self) -> Cents {
        self.gross_income() - self.tax_paid - self.rent_paid - self.loan_payments
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Household {
    pub id: HouseholdId,
    pub label: String,
    pub class: Class,
    /// People this cohort stands for.
    pub weight: People,
    pub labour_share: Rate,
    pub reservation: Cents,
    pub wage_premium: Rate,
    /// Non-labour members (children, retired) receive benefits.
    pub nonlabour_benefit: bool,
    pub renter_share: Rate,
    pub home_value: Cents,
    pub savings: Cents,
    pub debt: Cents,
    pub skill: Rate,
    /// Employed people per firm slot.
    pub employment: Vec<People>,
    pub income: IncomeFlows,
    /// Last six months of (capital income, gross income).
    pub income_hist: Vec<(Cents, Cents)>,
    pub last_disposable: Cents,
    pub food_met: Rate,
    pub housed_share: Rate,
    pub consumption: Cents,
    pub wellbeing: Rate,
    pub political_power: Rate,
    pub footprint: Tonnes,
    pub wealth_pc: Cents,
    pub arrears: bool,
}

impl Household {
    pub fn employed(&self) -> People {
        People(self.employment.iter().map(|p| p.0).sum())
    }
    pub fn labour_force(&self) -> People {
        self.weight * self.labour_share.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Firm {
    pub id: FirmId,
    pub active: bool,
    pub sector: Sector,
    pub owner: Owner,
    /// The household who founded the firm; owner when the system is private.
    pub founder: HouseholdId,
    pub capital: Units,
    pub cash: Cents,
    pub debt: Cents,
    /// Scheduled monthly principal repayment.
    pub amort: Cents,
    pub workers: People,
    pub wage: Cents,
    pub price: Cents,
    pub inventory: Units,
    pub intensity_base: Rate,
    pub green_tech: Rate,
    pub tfp: Rate,
    pub output: Units,
    pub emissions: Tonnes,
    pub sales_units: Units,
    /// Units sold in each of the last three months.
    pub sales_hist: [f64; 3],
    pub revenue: Cents,
    pub profit: Cents,
    pub carbon_cost: Cents,
    /// Smoothed profit, used by investment decisions.
    pub profit_ema: Cents,
    pub unfilled: bool,
    pub over_target_months: u32,
    pub distress_months: u32,
    pub wage_shortfall: bool,
    pub plan_invest: Cents,
    pub loan_request: Cents,
    pub loan_rate: Rate,
    pub green_plan: bool,
    pub excess_demand: Rate,
    pub closed_tick: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bank {
    pub owner: HouseholdId,
    pub equity: Cents,
    pub loans: Cents,
    pub deposits: Cents,
    pub default_rate: Rate,
    pub failed: bool,
    pub bailout_this_tick: bool,
    pub lent_this_tick: Cents,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Government {
    pub cash: Cents,
    pub debt: Cents,
    pub legitimacy: Rate,
    pub tax_revenue: Cents,
    pub spending: Cents,
    /// Goods and food purchases planned for the next goods market.
    pub purchases_budget: Cents,
    pub pending_build: Vec<(InfraKind, u32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nature {
    pub forest: Units,
    pub commons: Units,
    pub commons_access: CommonsAccess,
    /// Cumulative net emissions CE.
    pub cumulative: Tonnes,
    pub net_emissions: Tonnes,
    pub gross_emissions: Tonnes,
    pub sink: Tonnes,
    pub atmosphere: Tonnes,
    pub harvest_commons: Units,
    pub harvest_forest: Units,
    /// Monthly decline of forest plus commons stock (zero if growing).
    pub depletion: Units,
    /// Forest to clear next update because of construction.
    pub pending_clearing: Units,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Housing {
    pub owner: HouseholdId,
    pub private_cap: Units,
    pub public_cap: Units,
    pub rent_private: Cents,
    pub rent_public: Cents,
    pub occupancy: Rate,
    pub housed_share: Rate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Energy {
    pub fossil_cap: Units,
    pub renewable_cap: Units,
}

impl Energy {
    /// Fossil share of the power mix.
    pub fn grid_factor(&self) -> f64 {
        safe_div(
            self.fossil_cap.0,
            self.fossil_cap.0 + self.renewable_cap.0,
            1.0,
        )
    }
}

/// Policy levers set by the player (nominal values).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub carbon_price: Cents,
    pub tax_wage: Rate,
    pub tax_profit: Rate,
    pub tax_capital: Rate,
    pub wealth_tax: Rate,
    pub wealth_threshold: Cents,
    pub base_rate: Rate,
    pub min_wage: Cents,
    pub benefit: Cents,
    pub ubi: Cents,
    pub green_subsidy: Rate,
    pub campaign_cap: Rate,
    pub emissions_cap: Tonnes,
    pub commons_harvest: Rate,
    pub bank_capital_ratio: Rate,
    pub bailout: bool,
    pub public_spending: Rate,
    /// Share of the unemployed the government employs at the minimum wage.
    pub job_guarantee: Rate,
}

/// Policy as actually applied after lobbying (S9b).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectivePolicy {
    pub carbon_price: Cents,
    pub wealth_tax: Rate,
    pub tax_profit: Rate,
    pub emissions_cap: Tonnes,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub from: EconomicSystem,
    pub to: EconomicSystem,
    pub months_left: u32,
    pub total: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Indicators {
    pub gdp_month: Cents,
    pub gdp_annual: Cents,
    pub gdp_real_annual: f64,
    pub unemployment: Rate,
    pub employed: People,
    pub labour_force: People,
    pub gini: Rate,
    pub wage_share: Rate,
    pub profit_share: Rate,
    pub avg_wellbeing: Rate,
    pub remaining_budget: Tonnes,
    pub temperature: f64,
    pub gpi_annual: f64,
    pub money: Cents,
    pub price_index: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GdpParts {
    pub investment: Cents,
    pub gov_services: Cents,
}

/// A queued player command waiting for its lead time to end.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingCommand {
    pub id: u32,
    pub command: Command,
    pub submitted_turn: u32,
    pub effective_turn: u32,
    pub pc_cost: f64,
    /// Money the command will cost when it takes effect (builds).
    pub money_cost: Cents,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub schema_version: u32,
    pub tick: u32,
    /// Current turn, 1-based. 161 means the game is over.
    pub turn: u32,
    pub political_capital: f64,
    /// Commands queued but not yet active. Systems never read these (except firms
    /// reading an announced carbon price).
    pub pending: Vec<PendingCommand>,
    pub next_pending_id: u32,
    pub system: EconomicSystem,
    pub transition: Option<Transition>,
    /// Temporary productivity penalty during a system transition.
    pub output_penalty: Rate,
    pub households: Vec<Household>,
    pub firms: Vec<Firm>,
    pub bank: Bank,
    pub gov: Government,
    pub nature: Nature,
    pub housing: Housing,
    pub energy: Energy,
    pub policy: Policy,
    pub effective: EffectivePolicy,
    pub union_density: Rate,
    pub net_lobby: Rate,
    pub indicators: Indicators,
    pub gdp_parts: GdpParts,
    pub outcome: Option<(u32, Outcome)>,
}

impl State {
    pub fn total_people(&self) -> People {
        People(self.households.iter().map(|h| h.weight.0).sum())
    }
    /// Money in circulation: every non-bank deposit.
    pub fn total_deposits(&self) -> Cents {
        Cents::sum(self.households.iter().map(|h| h.savings))
            + Cents::sum(self.firms.iter().map(|f| f.cash))
            + self.gov.cash
    }
    pub fn total_debts(&self) -> Cents {
        Cents::sum(self.households.iter().map(|h| h.debt))
            + Cents::sum(self.firms.iter().map(|f| f.debt))
            + self.gov.debt
    }
}
