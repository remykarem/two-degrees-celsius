//! Parameter structs, loading and validation. Values live in `content/` JSON, never in code.
//! This crate does no file I/O: callers pass JSON text.
//!
//! Units: unless a name says otherwise, rates are per month, shares are 0..1,
//! money is in dollars (`*_usd`), converted to `Cents` by the engine at init.
#![forbid(unsafe_code)]

pub mod strengths;

use serde::{Deserialize, Serialize};
use sim_types::{AgeBand, Class, EconomicSystem, PlantKind, Sector};

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
    /// Most people a firm can fire in one month.
    pub max_fire_per_month: u32,
    /// Most people a firm can hire in one month.
    pub max_hire_per_month: u32,
    pub skill_gain: f64,
    pub skill_loss: f64,
    /// Share of a firm's jobs that are skilled (filled by skilled people first).
    pub skilled_share: f64,
    /// Months of sales held as target inventory cover (goods and food).
    pub inventory_cover: f64,
    /// Services cannot be stored, so the target cover is normally zero.
    pub services_inventory_cover: f64,
    pub inventory_adjust: f64,
    /// Workers assumed when sizing the output per worker of an empty firm.
    pub min_workers: f64,
    /// Wage change per month = `wage_k * (u_ref - unemployment)`, capped at `wage_cap`.
    pub wage_k: f64,
    pub u_ref: f64,
    pub wage_cap: f64,
    /// Share of the sector's productivity growth that is passed into the nominal wage every month
    /// (so that at `u_ref` real wages follow productivity and prices stay flat). 0 reproduces v0.2.
    pub productivity_passthrough: f64,
    /// Monthly nominal wage growth that firms and workers expect on top of productivity
    /// (the nominal anchor; about 2% a year). Without it the price level drifts downward.
    pub inflation_anchor: f64,
    /// Weight of the newest month in expected sales (exponential average).
    pub expected_smoothing: f64,
    pub skill_min: f64,
    pub skill_max: f64,
    /// Pay of a person = firm wage x (1 + slope x (skill - ref)), never below the floor multiple.
    pub skill_wage_slope: f64,
    pub skill_wage_ref: f64,
    pub skill_wage_floor: f64,
    /// Effective labour per worker assumed when a firm has no workers to measure it from.
    pub default_effective_per_worker: f64,
    /// A firm hires while the workers it needs (in people) exceed its headcount by more than
    /// this, so a tiny gap does not trigger a hire.
    pub hire_slack: f64,
    /// A firm counts as overstaffed only when its headcount exceeds the workers it needs by at
    /// least this many people (a band, so 3 or 4 workers do not flip-flop).
    pub fire_slack: f64,
    /// An active firm always keeps at least this many workers on its books.
    pub min_target_workers: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionParams {
    pub alpha_k: f64,
    pub beta_n: f64,
    pub farm_health_exp: f64,
    pub climate_damage: f64,
    /// The climate damage ratio (cumulative emissions over the budget) is capped here, so
    /// overshoot beyond this multiple of the budget stops adding damage.
    pub climate_ratio_cap: f64,
    /// Monthly TFP growth by sector.
    pub tfp_growth_goods: f64,
    pub tfp_growth_food: f64,
    pub tfp_growth_services: f64,
}

/// Roads, commutes and where people live (docs/town-v2.md sections 4 and 5).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpaceParams {
    /// Minutes to cross a tile without a road.
    pub slow_minutes: u32,
    /// Minutes to cross a road tile.
    pub fast_minutes: u32,
    /// Commute minutes that cost no productivity.
    pub free_minutes: f64,
    /// Share of working time lost per commute minute above the free minutes.
    pub k_commute: f64,
    /// Effective hours never fall below this.
    pub min_effective_hours: f64,
    /// Commute assumed for a worker whose household has no home.
    pub unhoused_commute_tiles: u32,
    pub unhoused_commute_minutes: u32,
}

/// Power plants and electricity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnergyParams {
    /// Tonnes of CO2 per energy unit from fossil plants (renewables emit nothing).
    pub factor_fossil: f64,
    /// Capacity of a newly built plant, energy units a month.
    pub plant_capacity: f64,
    /// Electricity per person per month before the consumption term.
    pub hh_energy_base_pc: f64,
    /// Extra electricity per person at the reference consumption.
    pub hh_energy_consumption_pc: f64,
    pub hh_ref_consumption_usd: f64,
    /// Electricity counts as rationed (event, and the `rationed` flag on the wire) when the
    /// served share falls below this.
    pub rationed_below: f64,
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
    /// Services spoil at once: the share of unsold services that is lost each month.
    pub services_holding_cost: f64,
    pub price_elasticity: f64,
    /// Propensity to consume out of last month's disposable income, by class.
    pub propensity: ClassShares,
    pub food_need_units_pc: f64,
    /// Monthly consumption out of net wealth by the capitalist and rentier classes.
    pub wealth_consumption_rate: f64,
    /// Share of discretionary household spending that goes to services (the rest to goods).
    pub services_share: f64,
    /// Share of the government's purchases (the G lever) that goes to services.
    pub gov_services_share: f64,
    pub rent_up: f64,
    pub rent_down: f64,
    /// Rents rise only while all homes are taken (occupancy at least this).
    pub occupancy_trigger: f64,
    /// Rents stop rising when the average renter pays this share of income in rent.
    pub rent_burden_cap: f64,
    pub min_rent_usd: f64,
    /// A household loses its home after this many months of unpaid rent.
    pub evict_after_months: u32,
    /// A household without a home moves into a vacant home whose rent is at most this share
    /// of its last disposable income.
    pub move_rent_share: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomeParams {
    /// Months of wage bill and debt service a firm keeps before paying dividends.
    pub cash_reserve_months: f64,
    /// Cash above this many months of payroll and debt service (on top of the reserve) is
    /// treated as idle and paid out as dividends at `excess_payout_rate` per month.
    pub excess_cash_months: f64,
    pub excess_payout_rate: f64,
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
    /// The bank's smoothed default rate keeps this weight of last month's value ...
    pub default_rate_old_weight: f64,
    /// ... and takes this weight of this month's default rate.
    pub default_rate_new_weight: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvestParams {
    pub retain_share: f64,
    /// Share of the gap between desired and actual capital closed per month (accelerator).
    pub accelerator: f64,
    /// Expansion investment is at most this share of capital value per month.
    pub max_invest_share: f64,
    pub depreciation: f64,
    pub capital_price_usd: f64,
    pub green_step: f64,
    pub green_cost_share: f64,
    /// Most a firm spends on green investment in one month, as a share of that month's revenue.
    /// A full step costs `12 * green_cost_share` months of revenue, so this spreads it over time.
    pub green_spend_share: f64,
    pub green_payback_years: f64,
    pub green_max: f64,
    /// A firm counts as carbon-capped (so green investment is worthwhile) once its footprint
    /// reaches this share of its emissions cap.
    pub cap_binding_share: f64,
    /// Only invest in capital if the firm's cash is above this many months of payroll.
    pub min_cash_months: f64,
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
    /// Direct emissions per person per month (heating, transport); electricity comes on top.
    pub household_footprint_base: f64,
    /// Carbon budget B0 in tonnes for the whole run.
    pub budget_b0: f64,
    pub t0: f64,
    pub t_limit: f64,
    pub atmosphere_start: f64,
    /// A `ForestShrank` event fires when the forest falls below this share of last tick's stock.
    pub forest_shrank_share: f64,
    /// The commons count as collapsed below this share of `commons_max`.
    pub commons_collapse_share: f64,
    /// Farm-from-nature (docs/features/farm-from-nature.md): the share of last month's commons
    /// stock the farm can physically take in a month (Schaefer-style cap, `h_max = kappa x C`).
    pub farm_cap_share_commons: f64,
    /// The same for the forest stock.
    pub farm_cap_share_forest: f64,
}

/// Town v4 weather: forest fires and drought (docs/town-v4.md).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherParams {
    /// Stress is 0 while the budget remaining is above this share of `budget_b0`.
    pub onset: f64,
    /// Stress is 1 when the budget remaining is at or below this share.
    pub full: f64,
    /// Exponent of the stress curve.
    pub curve: f64,
    /// Development override: when set (0..1) the climate stress is this value from the first
    /// tick, whatever the carbon budget. `null` (the default) means normal play.
    #[serde(default)]
    pub force_stress: Option<f64>,
    /// Monthly drought odds are `drought_base x (1 + drought_stress_gain x s)`.
    pub drought_base: f64,
    pub drought_stress_gain: f64,
    /// Severity is `sev_min + u x (sev_span_base + sev_span_stress x s)`, capped at 1.
    pub sev_min: f64,
    pub sev_span_base: f64,
    pub sev_span_stress: f64,
    /// Months are `dur_min + floor(u x (dur_span_base + dur_span_stress x s))`.
    pub dur_min: f64,
    pub dur_span_base: f64,
    pub dur_span_stress: f64,
    /// Commons regrowth is multiplied by `1 - a_c x d x (1 - community_buffer if community)`.
    pub a_c: f64,
    /// Farm productivity is multiplied by `1 - a_f x d`.
    pub a_f: f64,
    /// Electricity demand (households and firms) is multiplied by `1 + a_e x d`.
    pub a_e: f64,
    /// Share of the commons drought penalty a community-managed commons avoids.
    pub community_buffer: f64,
    /// Monthly ignition odds: `fire_base x (1 + fire_stress_gain x s) x dryness x standing share`.
    pub fire_base: f64,
    pub fire_stress_gain: f64,
    /// Dryness is `1 + dry_commons x (1 - commons health) + dry_drought x d`.
    pub dry_commons: f64,
    pub dry_drought: f64,
    /// Monthly odds that a burning tile spreads to each adjacent standing forest tile.
    pub spread_p: f64,
    /// Spread odds are multiplied by `1 + spread_drought x d`.
    pub spread_drought: f64,
    /// Months a fire burns on a tile.
    pub duration_months: u32,
    /// Share of a tile's forest stock (forest / forest tiles) lost per burning tile per month.
    pub burn_frac: f64,
    /// Months a burnt tile shows as regrowing before it counts as standing again.
    pub regrow_months: u32,
    /// Tiles (Chebyshev) from a burning tile within which people suffer smoke.
    pub smoke_radius: u32,
    /// Wellbeing target penalty for a household whose home is in smoke.
    pub smoke_wellbeing: f64,
    /// Output penalty for a firm in smoke.
    pub smoke_productivity: f64,
    /// Odds per month that a home next to a fire is damaged. Zero = off (the default).
    pub home_damage_prob: f64,
    pub home_repair_cost_usd: f64,
    /// Tiles (Chebyshev) around a fire station that it covers.
    pub coverage_radius: u32,
    /// Share of spread odds removed on a covered tile.
    pub station_spread_cut: f64,
    /// Share of burn duration removed for a fire that starts or spreads on a covered tile.
    pub station_duration_cut: f64,
    pub fire_station_cost_usd: f64,
    pub fire_station_upkeep_usd: f64,
    /// Resilience effect is `max x (1 - exp(-spending / scale))` (diminishing returns).
    pub resilience_scale_usd: f64,
    pub resilience_ignition_cut_max: f64,
    pub resilience_drought_cut_max: f64,
    /// Forecast: expected tiles burning per quarter at which risk reads medium and high.
    pub forecast_fire_medium: f64,
    pub forecast_fire_high: f64,
    /// Forecast: odds of a drought starting within a quarter at which risk reads medium and high.
    pub forecast_drought_medium: f64,
    pub forecast_drought_high: f64,
    /// Extra tiles per spread-odds unit and burning month assumed for the forecast's fire size.
    pub forecast_size_gain: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovParams {
    pub cash_target_months: f64,
    pub repay_share: f64,
    pub housing_upkeep_usd: f64,
    pub plant_upkeep_usd: f64,
    pub road_upkeep_usd: f64,
    pub public_housing_cost_usd: f64,
    pub plant_cost_usd: f64,
    pub road_cost_usd: f64,
    pub legit_wellbeing: f64,
    /// Wellbeing at which it neither raises nor lowers legitimacy.
    pub legit_wellbeing_neutral: f64,
    pub legit_unemployment: f64,
    pub legit_unemployment_threshold: f64,
    pub legit_gini: f64,
    pub legit_gini_threshold: f64,
    pub legit_bailout: f64,
    pub legit_debt: f64,
    pub collapse_unemployment: f64,
    /// A failed bank brings the town down when (smoothed) unemployment is also above this.
    pub bank_cascade_unemployment: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassParams {
    pub capital_share_threshold: f64,
    pub rentier_share_threshold: f64,
    pub skill_threshold: f64,
    /// Managers earn more than this multiple of the average base wage.
    pub skilled_wage_ratio: f64,
    /// Months of income history the class function averages over.
    pub history_months: usize,
    /// Savings a household needs (startup cost plus buffer) to found a firm.
    pub startup_cost_usd: f64,
    pub startup_buffer_usd: f64,
    pub startup_capital_share: f64,
    /// Monthly probability of founding = `mobility_rate` x (sector return - loan rate).
    pub mobility_rate: f64,
    pub startup_hurdle_floor: f64,
    /// A household founds a firm only where demand exceeds supply by at least this share
    /// (the sector's average excess demand); a second firm in a saturated sector would only
    /// split the sales of the first.
    pub startup_min_excess_demand: f64,
    /// A new firm starts with this share of its sector peers' average expected sales.
    pub new_firm_sales_share: f64,
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
    /// Monthly gross emissions (tonnes) at which pollution is total.
    pub pollution_scale: f64,
    /// Wellbeing lost by a person whose commute is `commute_scale_minutes` above the free minutes.
    pub w_commute: f64,
    pub commute_scale_minutes: f64,
    /// Wellbeing lost by living within `pollution_radius` tiles of a fossil plant or the Factory.
    /// Zero switches the effect off.
    pub w_local_pollution: f64,
    pub pollution_radius: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsParams {
    /// Weight of the newest month in the smoothed unemployment rate.
    pub unemployment_smoothing: f64,
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
pub struct PersonInit {
    pub label: String,
    pub age_band: AgeBand,
    pub skill: f64,
    /// Adults who look for work.
    pub labour_force: bool,
    pub reservation_usd: f64,
    /// Index of the firm slot the person works in at the start.
    pub job: Option<usize>,
    /// Town v7: index of the household that employs this person as domestic staff (the jet's
    /// pilot). Must be the estate household.
    #[serde(default)]
    pub staff_of: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdInit {
    pub label: String,
    pub class: Class,
    pub savings_usd: f64,
    pub debt_usd: f64,
    /// Index into `homes` of the home the household lives in; `None` for an unhoused household.
    pub home: Option<usize>,
    /// Disposable income per month assumed for the month before the run starts.
    pub initial_income_usd: f64,
    pub people: Vec<PersonInit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeInit {
    /// `[x, y]`.
    pub tile: [u8; 2],
    /// Index of the owning household; `None` for public housing.
    pub owner: Option<usize>,
    pub value_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlantInit {
    pub kind: PlantKind,
    pub tile: [u8; 2],
    pub capacity: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapInit {
    pub width: u8,
    pub height: u8,
    pub roads: Vec<[u8; 2]>,
    pub forest: Vec<[u8; 2]>,
    pub commons: Vec<[u8; 2]>,
    pub town_hall: [u8; 2],
    pub bank: [u8; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmInit {
    pub sector: Sector,
    pub active: bool,
    pub founder: usize,
    /// Where the firm stands (a dormant slot's reserved site).
    pub tile: [u8; 2],
    pub capital_units: f64,
    pub cash_usd: f64,
    pub debt_usd: f64,
    pub wage_usd: f64,
    pub price_usd: f64,
    pub inventory_units: f64,
    /// Process (direct) tonnes of CO2 per unit of output.
    pub intensity_base: f64,
    /// Electricity units per unit of output.
    pub energy_per_unit: f64,
    /// Output the firm should produce at the initial state; TFP is derived from it.
    pub initial_output: f64,
    /// Capital investment planned for the first month.
    pub plan_invest_usd: f64,
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
    #[serde(default)]
    pub resilience_spending_usd: f64,
}

/// Town v7: the estate's layout and jet, all in content.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateInit {
    /// Index of the estate household.
    pub household: usize,
    /// Index into `homes` of the estate house.
    pub home: usize,
    /// Pool, garden and drive tiles.
    pub grounds: Vec<[u8; 2]>,
    /// Runway tiles in take-off order.
    pub runway: Vec<[u8; 2]>,
    pub hangar_tile: [u8; 2],
    pub jet_name: String,
    pub jet_value_usd: f64,
}

/// Town v7 rules numbers (docs/town-v7.md).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateParams {
    /// Chance of a flight in a month (one draw a month while the estate exists).
    pub flight_prob_monthly: f64,
    /// Tonnes of CO2 per flight.
    pub flight_tonnes: f64,
    /// Direct emissions of the estate household per month; replaces the per-person figure.
    pub household_emissions_t: f64,
    /// Monthly wage the estate pays its pilot.
    pub pilot_wage_usd: f64,
    /// Months of short pay in a row after which the pilot is let go.
    pub pilot_unpaid_months_max: u32,
    /// The pilot is rehired when the estate's savings cover this many months of wages.
    pub pilot_rehire_months_of_wages: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialTown {
    /// Town v7: the elite estate, if the town has one.
    #[serde(default)]
    pub estate: Option<EstateInit>,
    pub map: MapInit,
    pub households: Vec<HouseholdInit>,
    pub homes: Vec<HomeInit>,
    pub plants: Vec<PlantInit>,
    pub firms: Vec<FirmInit>,
    /// Household that owns the bank.
    pub bank_owner: usize,
    pub gov_cash_usd: f64,
    pub gov_debt_usd: f64,
    pub legitimacy: f64,
    pub forest: f64,
    pub commons: f64,
    pub rent_private_usd: f64,
    pub rent_public_usd: f64,
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
    pub space: SpaceParams,
    pub energy: EnergyParams,
    pub goods: GoodsParams,
    pub income: IncomeParams,
    pub credit: CreditParams,
    pub invest: InvestParams,
    pub nature: NatureParams,
    pub weather: WeatherParams,
    pub government: GovParams,
    pub class: ClassParams,
    pub lobby: LobbyParams,
    pub wellbeing: WellbeingParams,
    pub metrics: MetricsParams,
    pub estate: EstateParams,
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
    space: SpaceParams,
    energy: EnergyParams,
    goods: GoodsParams,
    income: IncomeParams,
    credit: CreditParams,
    invest: InvestParams,
    nature: NatureParams,
    weather: WeatherParams,
    government: GovParams,
    class: ClassParams,
    lobby: LobbyParams,
    wellbeing: WellbeingParams,
    metrics: MetricsParams,
    estate: EstateParams,
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
            space: core.space,
            energy: core.energy,
            goods: core.goods,
            income: core.income,
            credit: core.credit,
            invest: core.invest,
            nature: core.nature,
            weather: core.weather,
            government: core.government,
            class: core.class,
            lobby: core.lobby,
            wellbeing: core.wellbeing,
            metrics: core.metrics,
            estate: core.estate,
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

    /// Check ranges and internal consistency of the initial town: every home holds at most one
    /// household, every person has a valid job, and every tile holds at most one thing.
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
        pos("space.slow_minutes", self.space.slow_minutes as f64);
        pos("space.fast_minutes", self.space.fast_minutes as f64);
        pos("energy.plant_capacity", self.energy.plant_capacity);
        pos("weather.curve", self.weather.curve);
        pos(
            "weather.resilience_scale_usd",
            self.weather.resilience_scale_usd,
        );
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
        share("goods.services_share", self.goods.services_share);
        share("goods.gov_services_share", self.goods.gov_services_share);
        share("space.min_effective_hours", self.space.min_effective_hours);
        if self.space.fast_minutes > self.space.slow_minutes {
            errs.push("space.fast_minutes must not exceed space.slow_minutes".into());
        }
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
        let w = &self.weather;
        if let Some(f) = w.force_stress {
            if !(0.0..=1.0).contains(&f) {
                errs.push(format!(
                    "weather.force_stress must be null or in [0, 1] (got {f})"
                ));
            }
        }
        if !(w.onset > w.full && w.full >= 0.0 && w.onset <= 1.0) {
            errs.push("weather: need 0 <= full < onset <= 1".into());
        }
        for (n, v) in [
            ("spread_p", w.spread_p),
            ("burn_frac", w.burn_frac),
            ("community_buffer", w.community_buffer),
            ("station_spread_cut", w.station_spread_cut),
            ("station_duration_cut", w.station_duration_cut),
            ("home_damage_prob", w.home_damage_prob),
            ("resilience_ignition_cut_max", w.resilience_ignition_cut_max),
            ("resilience_drought_cut_max", w.resilience_drought_cut_max),
        ] {
            if !(0.0..=1.0).contains(&v) {
                errs.push(format!("weather.{n} must be in [0, 1] (got {v})"));
            }
        }
        self.validate_town(&mut errs);
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("; "))
        }
    }

    fn validate_town(&self, errs: &mut Vec<String>) {
        let t = &self.initial;
        let n = t.households.len();
        if n == 0 {
            errs.push("initial.households is empty".into());
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
        let (w, h) = (t.map.width, t.map.height);
        // One thing per tile, all inside the map.
        let mut seen: Vec<([u8; 2], &str)> = Vec::new();
        let mut put = |pos: [u8; 2], what: &'static str, errs: &mut Vec<String>| {
            if pos[0] >= w || pos[1] >= h {
                errs.push(format!("{what} at {pos:?} is outside the {w}x{h} map"));
            } else if let Some((_, other)) = seen.iter().find(|(p, _)| *p == pos) {
                errs.push(format!("tile {pos:?} holds both {other} and {what}"));
            } else {
                seen.push((pos, what));
            }
        };
        for r in &t.map.roads {
            put(*r, "a road", errs);
        }
        for r in &t.map.forest {
            put(*r, "forest", errs);
        }
        for r in &t.map.commons {
            put(*r, "commons", errs);
        }
        put(t.map.town_hall, "the town hall", errs);
        put(t.map.bank, "the bank", errs);
        for hm in &t.homes {
            put(hm.tile, "a home", errs);
            if matches!(hm.owner, Some(o) if o >= n) {
                errs.push("a home has an owner out of range".into());
            }
        }
        for pl in &t.plants {
            put(pl.tile, "a plant", errs);
        }
        for f in &t.firms {
            put(f.tile, "a firm site", errs);
            if f.founder >= n {
                errs.push("a firm's founder is out of range".into());
            }
        }
        if let Some(e) = &t.estate {
            if e.household >= n {
                errs.push("estate.household out of range".into());
            } else if t.households[e.household].people.len() != 1 {
                errs.push("the estate household must have exactly one person".into());
            }
            match t.homes.get(e.home) {
                None => errs.push("estate.home does not exist".into()),
                Some(hm) if hm.owner != Some(e.household) => {
                    errs.push("the estate home must be owned by the estate household".into())
                }
                Some(_) => {}
            }
            if t.households.get(e.household).and_then(|x| x.home) != Some(e.home) {
                errs.push("the estate household must live in the estate home".into());
            }
            for q in &e.grounds {
                put(*q, "estate grounds", errs);
            }
            for q in &e.runway {
                put(*q, "a runway tile", errs);
            }
            if e.runway.is_empty() {
                errs.push("estate.runway is empty".into());
            }
            if !e.grounds.contains(&e.hangar_tile) && !e.runway.contains(&e.hangar_tile) {
                errs.push("estate.hangar_tile must be a grounds or runway tile".into());
            }
        }
        // One household per home; every job refers to an active firm slot.
        let mut occupant: Vec<Option<usize>> = vec![None; t.homes.len()];
        for (i, hh) in t.households.iter().enumerate() {
            if !(1..=4).contains(&hh.people.len()) {
                errs.push(format!("household {i} must have 1 to 4 people"));
            }
            if let Some(home) = hh.home {
                match occupant.get_mut(home) {
                    None => errs.push(format!("household {i}: home {home} does not exist")),
                    Some(slot) => {
                        if let Some(other) = *slot {
                            errs.push(format!("home {home} holds households {other} and {i}"));
                        }
                        *slot = Some(i);
                    }
                }
            }
            for p in &hh.people {
                if let Some(o) = p.staff_of {
                    if t.estate.as_ref().map(|e| e.household) != Some(o) {
                        errs.push(format!(
                            "household {i}: {} is staff of household {o}, which is not the estate",
                            p.label
                        ));
                    }
                    if !p.labour_force || p.job.is_some() {
                        errs.push(format!(
                            "household {i}: staff member {} needs labour_force and no firm job",
                            p.label
                        ));
                    }
                    if hh.home.is_none() {
                        errs.push(format!(
                            "household {i}: staff member's household has no home"
                        ));
                    }
                }
                if let Some(f) = p.job {
                    if !t.firms.get(f).is_some_and(|x| x.active) {
                        errs.push(format!(
                            "household {i}: {} works in an inactive or missing firm",
                            p.label
                        ));
                    }
                    if !p.labour_force {
                        errs.push(format!(
                            "household {i}: {} has a job but is outside the labour force",
                            p.label
                        ));
                    }
                }
            }
        }
    }
}
