//! Parameter structs, loading and validation. Values live in `content/` JSON, never in code.
//! This crate does no file I/O: callers pass JSON text.
//!
//! Units: unless a name says otherwise, rates are per month, shares are 0..1,
//! money is in dollars (`*_usd`), converted to `Cents` by the engine at init.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sim_types::{Class, EconomicSystem, Sector};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunParams {
    /// Length of a game in turns.
    pub turns: u32,
    /// Monthly ticks per turn (one quarter).
    pub ticks_per_turn: u32,
    /// Settling months run before tick 0 so the run starts from a working economy.
    pub warmup_ticks: u32,
    pub stop_on_loss: bool,
    /// Any payment that cannot be made in full is a check failure when set (tests set it).
    pub strict_flows: bool,
    /// Shortfall (dollars) tolerated per tick before `strict_flows` fires: rounding only.
    pub flow_tolerance_usd: f64,
}

/// Political-capital cost and lead time of one kind of command.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeverCfg {
    pub base_cost: f64,
    /// Turns between queueing and taking effect.
    pub lead_time: u32,
    /// Change that counts as size 1 (friendly units: dollars, fractions, tonnes).
    pub unit: f64,
    /// Smallest size charged for any change.
    pub min_size: f64,
}

/// Turn structure, political capital and policy delay (SPEC 4.1, model doc section 7).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnParams {
    pub pc_start: f64,
    pub pc_cap: f64,
    /// Regeneration per turn is `pc_base * legitimacy`.
    pub pc_base: f64,
    /// Share of PC returned when a pending command is cancelled.
    pub cancel_refund: f64,
    /// Weight of a still-pending carbon price in firms' expectations.
    pub announce_discount: f64,
    pub levers: std::collections::BTreeMap<String, LeverCfg>,
}

impl TurnParams {
    pub fn lever(&self, key: &str) -> &LeverCfg {
        &self.levers[key]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabourParams {
    /// Expected sales growth aspiration per month.
    pub growth_aspiration: f64,
    pub fire_after_months: u32,
    pub max_fire_share: f64,
    /// Largest share of its current workforce a firm can add in one month.
    pub max_hire_share: f64,
    pub skill_gain: f64,
    pub skill_loss: f64,
    /// Share of a firm's jobs that are skilled (filled by premium cohorts).
    pub skilled_share: f64,
    /// Months of sales held as target inventory cover.
    pub inventory_cover: f64,
    pub inventory_adjust: f64,
    /// Output per worker assumed for a firm with no workers.
    pub min_workers: f64,
    /// Wage change per month = `wage_k * (u_ref - unemployment)`, capped at `wage_cap`.
    pub wage_k: f64,
    pub u_ref: f64,
    pub wage_cap: f64,
    /// Weight of the newest month in expected sales (exponential average).
    pub expected_smoothing: f64,
    /// A firm can always hire at least this many people in a month.
    pub min_hire: f64,
    pub skill_min: f64,
    pub skill_max: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionParams {
    pub alpha_k: f64,
    pub beta_n: f64,
    pub farm_health_exp: f64,
    pub climate_damage: f64,
    /// Monthly TFP growth by sector.
    pub tfp_growth_goods: f64,
    pub tfp_growth_food: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassShares {
    pub capitalist: f64,
    pub rentier: f64,
    pub manager: f64,
    pub worker: f64,
    pub dependent: f64,
}

impl ClassShares {
    pub fn get(&self, c: Class) -> f64 {
        match c {
            Class::Capitalist => self.capitalist,
            Class::Rentier => self.rentier,
            Class::Manager => self.manager,
            Class::Worker => self.worker,
            Class::Dependent => self.dependent,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoodsParams {
    /// Share of the gap between price and (1 + markup) x unit cost closed per month.
    pub price_adjust: f64,
    /// Smallest price (dollars), a guard against dividing by zero.
    pub min_price_usd: f64,
    pub holding_cost: f64,
    pub price_elasticity: f64,
    /// Propensity to consume out of last month's disposable income, by class.
    pub propensity: ClassShares,
    pub food_need_units_pc: f64,
    /// Monthly consumption out of net wealth by the capitalist and rentier classes.
    pub wealth_consumption_rate: f64,
    pub rent_up: f64,
    pub rent_down: f64,
    pub occupancy_trigger: f64,
    pub min_rent_usd: f64,
    pub housing_depreciation: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomeParams {
    /// Months of wage bill and debt service a firm keeps before paying dividends.
    pub cash_reserve_months: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreditParams {
    pub spread_base: f64,
    pub spread_leverage: f64,
    pub firm_term_months: f64,
    pub household_amort: f64,
    /// Deposit rate = base rate minus this.
    pub deposit_spread: f64,
    pub default_after_months: u32,
    /// Share of equity above the required buffer paid to the bank owner each month.
    pub bank_payout: f64,
    /// Equity kept above the minimum required by the capital ratio.
    pub bank_buffer: f64,
    pub bailout_target_ratio: f64,
    pub max_firm_leverage: f64,
    /// Not implemented in v1: must be false. Reserved for household borrowing.
    pub household_credit: bool,
    /// Depositor haircut = shortfall / deposits x this margin, so equity is restored.
    pub haircut_margin: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvestParams {
    pub retain_share: f64,
    /// Share of the gap between desired and actual capital closed per month (accelerator).
    pub accelerator: f64,
    /// Expansion investment is at most this share of capital value per month.
    pub max_invest_share: f64,
    /// Share of the landlord's savings that can go to new housing in a month.
    pub housing_budget_share: f64,
    pub depreciation: f64,
    pub capital_price_usd: f64,
    pub green_step: f64,
    pub green_cost_share: f64,
    pub green_payback_years: f64,
    pub green_max: f64,
    /// Only invest in capital if the firm's cash is above this many months of payroll.
    pub min_cash_months: f64,
    pub housing_cost_usd: f64,
    pub housing_hurdle: f64,
    pub housing_target_occupancy: f64,
    /// Excess return over the hurdle at which investment reaches its maximum.
    pub return_scale: f64,
    /// Weight of the newest month in smoothed profit and in the investment plan.
    pub smoothing: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NatureParams {
    pub forest_growth: f64,
    pub forest_max: f64,
    pub commons_growth: f64,
    pub commons_max: f64,
    pub sink_sigma: f64,
    pub farm_harvest_commons: f64,
    pub farm_harvest_forest: f64,
    pub enclosure_harvest_multiplier: f64,
    pub enclosure_fee_usd: f64,
    pub community_coord_cost: f64,
    pub forest_clear_per_build: f64,
    pub household_footprint_base: f64,
    pub household_footprint_consumption: f64,
    /// Consumption per capita (dollars) at which the consumption term is scaled.
    pub footprint_ref_consumption_usd: f64,
    /// Carbon budget B0 in tonnes for the whole run.
    pub budget_b0: f64,
    pub t0: f64,
    pub t_limit: f64,
    pub atmosphere_start: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovParams {
    pub cash_target_months: f64,
    pub repay_share: f64,
    pub housing_upkeep_usd: f64,
    pub plant_upkeep_usd: f64,
    pub public_housing_cost_usd: f64,
    pub plant_cost_usd: f64,
    pub plant_capacity: f64,
    pub legit_wellbeing: f64,
    pub legit_unemployment: f64,
    pub legit_unemployment_threshold: f64,
    pub legit_gini: f64,
    pub legit_gini_threshold: f64,
    pub legit_bailout: f64,
    pub legit_debt: f64,
    pub collapse_unemployment: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassParams {
    pub capital_share_threshold: f64,
    pub rentier_share_threshold: f64,
    pub skill_threshold: f64,
    /// Managers earn more than this multiple of the average base wage.
    pub skilled_wage_ratio: f64,
    /// A cohort counts as employed when at least this share of its labour force works.
    pub employed_share_min: f64,
    /// Months of income history the class function averages over.
    pub history_months: usize,
    pub startup_cost_usd: f64,
    pub startup_buffer_usd: f64,
    pub startup_capital_share: f64,
    pub founders_per_firm: f64,
    pub mobility_rate: f64,
    pub fall_fraction: f64,
    pub min_cohort_weight: f64,
    pub startup_hurdle_floor: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LobbyParams {
    /// Largest net lobbying pressure.
    pub max_net: f64,
    pub spend_rate: f64,
    pub k: f64,
    pub labour_u: f64,
    pub union_adjust: f64,
    pub union_min: f64,
    pub union_max: f64,
    pub union_target_unemployment: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WellbeingParams {
    pub needs_usd_pc: f64,
    pub w_consumption: f64,
    pub w_employment: f64,
    pub w_housing: f64,
    pub w_environment: f64,
    pub w_security: f64,
    pub security_months: f64,
    pub smoothing: f64,
    /// Monthly gross emissions (tonnes) at which local pollution is total.
    pub pollution_scale: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsParams {
    pub scc_usd: f64,
    pub depletion_usd_per_unit: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DividendRule {
    Owner,
    Employees,
    State,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommonsRule {
    Open,
    Community,
}

/// A bundle of rule overrides (SPEC section 8, causal model section 3).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemConfig {
    pub name: String,
    pub dividend_rule: DividendRule,
    pub payout_ratio: f64,
    /// Multiplier on the minimum wage lever.
    pub min_wage_multiplier: f64,
    pub invest_hurdle: f64,
    pub credit_spread_add: f64,
    pub lambda_max: f64,
    pub union_floor: f64,
    pub commons_rule: CommonsRule,
    pub enclosure_prob: f64,
    pub transition_months: u32,
    pub transition_output_penalty: f64,
    /// Legitimacy cost per month of a transition, from affected classes.
    pub resistance: f64,
    /// Green subsidy raised to at least this when the system is adopted.
    pub green_subsidy_floor: f64,
    /// Share of retained cash a firm devotes to green investment.
    pub green_cash_share: f64,
    /// Campaign-finance cap enforced when the system is adopted (1.0 = none).
    pub campaign_cap: f64,
    /// Carbon price floor (dollars per tonne) enforced when adopted.
    pub carbon_price_floor_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemsParams {
    pub market: SystemConfig,
    pub regulated: SystemConfig,
    pub coop: SystemConfig,
}

impl SystemsParams {
    pub fn get(&self, s: EconomicSystem) -> &SystemConfig {
        match s {
            EconomicSystem::Market => &self.market,
            EconomicSystem::Regulated => &self.regulated,
            EconomicSystem::Coop => &self.coop,
        }
    }
}

// ---------- initial town ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortInit {
    pub label: String,
    pub class: Class,
    pub weight: f64,
    pub labour_share: f64,
    pub reservation_usd: f64,
    pub wage_premium: f64,
    pub nonlabour_benefit: bool,
    pub renter_share: f64,
    pub skill: f64,
    pub savings_usd: f64,
    pub debt_usd: f64,
    /// Owner-occupied housing value (counts as wealth only).
    pub home_value_usd: f64,
    /// Disposable income per month assumed for the month before the run starts.
    pub initial_income_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmInit {
    pub sector: Sector,
    pub active: bool,
    pub founder: usize,
    pub capital_units: f64,
    pub cash_usd: f64,
    pub debt_usd: f64,
    pub wage_usd: f64,
    pub price_usd: f64,
    pub inventory_units: f64,
    pub intensity_base: f64,
    /// Output the firm should produce at the initial state; TFP is derived from it.
    pub initial_output: f64,
    /// Capital investment planned for the first month.
    pub plan_invest_usd: f64,
    /// Employment per cohort index.
    pub employment: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInit {
    pub carbon_price_usd: f64,
    pub tax_wage: f64,
    pub tax_profit: f64,
    pub tax_capital: f64,
    pub wealth_tax: f64,
    pub wealth_threshold_usd: f64,
    pub base_rate: f64,
    pub min_wage_usd: f64,
    pub benefit_usd: f64,
    pub ubi_usd: f64,
    pub green_subsidy: f64,
    pub campaign_cap: f64,
    pub emissions_cap: f64,
    pub commons_harvest: f64,
    pub bank_capital_ratio: f64,
    pub bailout: bool,
    pub public_spending_usd: f64,
    pub job_guarantee: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialTown {
    pub cohorts: Vec<CohortInit>,
    pub firms: Vec<FirmInit>,
    pub bank_owner: usize,
    pub gov_cash_usd: f64,
    pub gov_debt_usd: f64,
    pub legitimacy: f64,
    pub forest: f64,
    pub commons: f64,
    pub private_housing: f64,
    pub public_housing: f64,
    pub rent_private_usd: f64,
    pub rent_public_usd: f64,
    pub fossil_capacity: f64,
    pub renewable_capacity: f64,
    pub union_density: f64,
    pub policy: PolicyInit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub run: RunParams,
    pub turn: TurnParams,
    pub labour: LabourParams,
    pub production: ProductionParams,
    pub goods: GoodsParams,
    pub income: IncomeParams,
    pub credit: CreditParams,
    pub invest: InvestParams,
    pub nature: NatureParams,
    pub government: GovParams,
    pub class: ClassParams,
    pub lobby: LobbyParams,
    pub wellbeing: WellbeingParams,
    pub metrics: MetricsParams,
    pub systems: SystemsParams,
    pub initial: InitialTown,
}

/// The pieces of JSON content that make up a full parameter set.
pub struct ContentJson<'a> {
    pub params: &'a str,
    pub initial_town: &'a str,
    pub market: &'a str,
    pub regulated: &'a str,
    pub coop: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoreParams {
    run: RunParams,
    turn: TurnParams,
    labour: LabourParams,
    production: ProductionParams,
    goods: GoodsParams,
    income: IncomeParams,
    credit: CreditParams,
    invest: InvestParams,
    nature: NatureParams,
    government: GovParams,
    class: ClassParams,
    lobby: LobbyParams,
    wellbeing: WellbeingParams,
    metrics: MetricsParams,
}

impl Params {
    /// Parse the content files and validate.
    pub fn from_content(c: &ContentJson) -> Result<Params, String> {
        let core: CoreParams =
            serde_json::from_str(c.params).map_err(|e| format!("params.json: {e}"))?;
        let initial: InitialTown =
            serde_json::from_str(c.initial_town).map_err(|e| format!("initial_town.json: {e}"))?;
        let market: SystemConfig =
            serde_json::from_str(c.market).map_err(|e| format!("systems/market.json: {e}"))?;
        let regulated: SystemConfig = serde_json::from_str(c.regulated)
            .map_err(|e| format!("systems/regulated.json: {e}"))?;
        let coop: SystemConfig =
            serde_json::from_str(c.coop).map_err(|e| format!("systems/coop.json: {e}"))?;
        let p = Params {
            run: core.run,
            turn: core.turn,
            labour: core.labour,
            production: core.production,
            goods: core.goods,
            income: core.income,
            credit: core.credit,
            invest: core.invest,
            nature: core.nature,
            government: core.government,
            class: core.class,
            lobby: core.lobby,
            wellbeing: core.wellbeing,
            metrics: core.metrics,
            systems: SystemsParams {
                market,
                regulated,
                coop,
            },
            initial,
        };
        p.validate()?;
        Ok(p)
    }

    /// Check ranges and internal consistency of the initial town.
    pub fn validate(&self) -> Result<(), String> {
        let mut errs: Vec<String> = Vec::new();
        for k in sim_types::LEVER_KEYS {
            match self.turn.levers.get(k) {
                None => errs.push(format!("turn.levers is missing '{k}'")),
                Some(l) if !(l.unit > 0.0 && l.base_cost >= 0.0 && l.min_size >= 0.0) => errs.push(
                    format!("turn.levers.{k} has a non-positive unit or negative cost"),
                ),
                Some(_) => {}
            }
        }
        if self.credit.household_credit {
            errs.push("credit.household_credit is not implemented in v1; it must be false".into());
        }
        let mut pos = |name: &str, v: f64| {
            if !(v.is_finite() && v > 0.0) {
                errs.push(format!("{name} must be positive and finite (got {v})"));
            }
        };
        pos("run.turns", self.run.turns as f64);
        pos("run.ticks_per_turn", self.run.ticks_per_turn as f64);
        pos("turn.pc_cap", self.turn.pc_cap);
        pos("labour.wage_cap", self.labour.wage_cap);
        pos("production.alpha_k", self.production.alpha_k);
        pos("production.beta_n", self.production.beta_n);
        pos("nature.forest_max", self.nature.forest_max);
        pos("nature.commons_max", self.nature.commons_max);
        pos("nature.budget_b0", self.nature.budget_b0);
        pos("invest.capital_price_usd", self.invest.capital_price_usd);
        pos("credit.firm_term_months", self.credit.firm_term_months);
        pos("class.founders_per_firm", self.class.founders_per_firm);
        let mut share = |name: &str, v: f64| {
            if !(0.0..=1.0).contains(&v) {
                errs.push(format!("{name} must be in [0, 1] (got {v})"));
            }
        };
        share("invest.retain_share", self.invest.retain_share);
        share("lobby.spend_rate", self.lobby.spend_rate);
        share(
            "class.capital_share_threshold",
            self.class.capital_share_threshold,
        );
        share("labour.skilled_share", self.labour.skilled_share);
        for (n, s) in [
            ("market", &self.systems.market),
            ("regulated", &self.systems.regulated),
            ("coop", &self.systems.coop),
        ] {
            if !(0.0..=1.0).contains(&s.payout_ratio) {
                errs.push(format!("systems.{n}.payout_ratio out of range"));
            }
            if !(0.0..=1.0).contains(&s.lambda_max) {
                errs.push(format!("systems.{n}.lambda_max out of range"));
            }
        }
        let t = &self.initial;
        let n = t.cohorts.len();
        if n == 0 {
            errs.push("initial.cohorts is empty".into());
        }
        if t.firms.len() != sim_types::MAX_FIRMS {
            errs.push(format!(
                "initial.firms must have {} slots",
                sim_types::MAX_FIRMS
            ));
        }
        if t.bank_owner >= n {
            errs.push("initial.bank_owner out of range".into());
        }
        for (i, f) in t.firms.iter().enumerate() {
            if f.founder >= n {
                errs.push(format!("firm {i}: founder out of range"));
            }
            if f.employment.len() != n {
                errs.push(format!(
                    "firm {i}: employment must have one entry per cohort"
                ));
            }
        }
        for (i, c) in t.cohorts.iter().enumerate() {
            if c.weight <= 0.0 {
                errs.push(format!("cohort {i}: weight must be positive"));
            }
            let emp: f64 = t
                .firms
                .iter()
                .map(|f| f.employment.get(i).copied().unwrap_or(0.0))
                .sum();
            if emp > c.weight * c.labour_share + 1e-9 {
                errs.push(format!("cohort {i}: employment exceeds labour force"));
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("; "))
        }
    }
}
