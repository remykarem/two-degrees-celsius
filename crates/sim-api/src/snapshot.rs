//! The `Snapshot` view of the sim state (contract sections 4, 9 and 10). A stable wire type,
//! decoupled from the internal `State`. One function per area.
use crate::command::Tile;
use crate::finance::{EnergyView, Finance};
use crate::history::{budget_share, debt_to_gdp, HistoryPoint};
use crate::keys::{
    AgeBandKey, ClassKey, GameOutcome, LeverKey, PlainOutcome, SectorKey, SystemKey, TileKindKey,
};
use crate::meta::{GameCalendar, Meta};
use crate::sources::{IncomeSource, IncomeView};
use crate::turn::PendingItem;
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types as st;
use std::collections::BTreeMap;

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOver {
    pub turn: u32,
    pub outcome: GameOutcome,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PcView {
    pub current: f64,
    pub cap: f64,
    pub regen_next_turn: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionView {
    pub to: SystemKey,
    pub months_left: u32,
    pub total: u32,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemView {
    pub current: SystemKey,
    pub transition: Option<TransitionView>,
}

/// A lever value: number, boolean or a mode name.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyScalar {
    Bool(bool),
    Number(f64),
    Mode(String),
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyEntry {
    pub nominal: PolicyScalar,
    /// Present only for the four lobbied levers; `null` otherwise.
    pub effective: Option<f64>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Buildings {
    pub public_housing: u32,
    pub fossil_plant: u32,
    pub renewable_plant: u32,
    pub road: u32,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Indicators {
    pub gdp_annual: f64,
    pub gdp_real_annual: f64,
    pub unemployment: f64,
    pub employed: f64,
    pub labour_force: f64,
    pub gini: f64,
    /// Mean monthly income per person (household gross income shared over its members).
    pub avg_income_per_person: f64,
    pub median_income_per_person: f64,
    /// Wealth of the wealthiest household over the positive wealth of all households.
    pub top_household_wealth_share: f64,
    pub wage_share: f64,
    pub profit_share: f64,
    pub avg_wellbeing: f64,
    pub remaining_budget: f64,
    pub budget_share: f64,
    pub temperature: f64,
    pub gpi_annual: f64,
    pub money_in_circulation: f64,
    pub price_index: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdIncome {
    /// Contract v2.0: income by source (`content/ui.json` `income_sources`), not fixed fields.
    pub sources: Vec<IncomeSource>,
    pub tax_paid: f64,
    pub rent_paid: f64,
    pub loan_payments: f64,
    pub spending: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdView {
    pub id: u32,
    pub label: String,
    /// Person ids.
    pub members: Vec<u32>,
    /// Home id, or `null` for an unhoused household.
    pub home: Option<u32>,
    pub class: ClassKey,
    pub savings: f64,
    pub debt: f64,
    pub net_worth: f64,
    pub income: HouseholdIncome,
    pub political_power: f64,
    pub arrears_months: u32,
    pub wellbeing: f64,
    pub footprint_tonnes: f64,
    pub food_met: f64,
    /// Contract v1.9: a short tag from `content/ui.json` `tags` ("Estate owner"); present for the
    /// estate household only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tag: Option<String>,
    /// Contract v1.9: the wealth band of the household's net worth (`content/ui.json`
    /// `wealth_bands`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wealth_band: Option<WealthBandView>,
}

/// Contract v1.9.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WealthBandView {
    pub id: String,
    pub label: String,
}

/// Contract v1.9: who employs a person who has no firm job.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerKind {
    Household,
}

/// Contract v1.9: the employer of a person on a household's payroll (the jet's pilot).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmployerView {
    pub kind: EmployerKind,
    pub household: u32,
    /// The employer household's label.
    pub label: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub firm: u32,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    pub id: u32,
    pub label: String,
    pub household: u32,
    pub age_band: AgeBandKey,
    pub skill: f64,
    /// The firm job, or `null` (unemployed, outside the labour force, or in the job guarantee).
    pub job: Option<JobView>,
    pub wage: f64,
    pub commute_tiles: u32,
    pub commute_minutes: u32,
    pub wellbeing: f64,
    pub in_labour_force: bool,
    /// In the government job guarantee (employed at the minimum wage, no firm).
    pub job_guarantee: bool,
    pub effective_hours: f64,
    /// Contract v1.9: set for domestic staff (`job` stays `null`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub employer: Option<EmployerView>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicOwner {
    Public,
}

/// Owner of a home: a household id, or the string `"public"`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HomeOwnerView {
    Household(u32),
    Public(PublicOwner),
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeView {
    pub id: u32,
    pub tile: Tile,
    pub owner: HomeOwnerView,
    pub household: Option<u32>,
    pub rent: f64,
    pub value: f64,
}

/// One cell of the map.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileView {
    pub x: i32,
    pub y: i32,
    pub kind: TileKindKey,
    /// Home, firm or plant id for those kinds.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "ref")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reference: Option<u32>,
    /// True if the tile is empty (a building could stand there, queued builds aside; use
    /// `validTiles(kind)` for the exact answer).
    pub buildable: bool,
    /// Plant tiles only: tonnes of CO2 emitted per month.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub emissions_monthly: Option<f64>,
    /// Forest tiles only (contract v1.2): standing, burning or regrowing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub state: Option<ForestTileState>,
    /// Standing forest tiles only (contract v1.8): the odds that a fire reaches this tile next
    /// month, from the model's ignition and spread rules (`sim_model::weather::tile_fire_risk`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fire_risk: Option<f64>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForestTileState {
    Standing,
    Burning,
    Regrowing,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskKey {
    Low,
    Medium,
    High,
}

impl RiskKey {
    pub fn from_sim(r: st::RiskLevel) -> RiskKey {
        match r {
            st::RiskLevel::Low => RiskKey::Low,
            st::RiskLevel::Medium => RiskKey::Medium,
            st::RiskLevel::High => RiskKey::High,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DroughtView {
    pub severity: f64,
    pub months_left: u32,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FireView {
    pub tile: Tile,
    pub months_left: u32,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastView {
    pub fire_risk: RiskKey,
    pub drought_risk: RiskKey,
}

/// Contract v1.2: climate stress (0..1), the active drought, burning tiles and the forecast.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherView {
    pub stress: f64,
    /// Contract v1.3 (additive): the stress curve from `content/params.json` `weather`, so the UI
    /// need not hard-code it. Stress is 0 while the budget share left is above `onset`, 1 at or
    /// below `full`, and rises along `curve` in between.
    pub onset: f64,
    pub full: f64,
    pub curve: f64,
    /// Contract v1.8 (additive): how far a fire station's cover reaches, and how far smoke
    /// spreads from a fire, in tiles (`params.weather`).
    pub coverage_radius: u32,
    pub smoke_radius: u32,
    pub drought: Option<DroughtView>,
    pub fires: Vec<FireView>,
    pub forecast: ForecastView,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapView {
    pub width: u32,
    pub height: u32,
    /// Row-major: `tiles[y * width + x]`.
    pub tiles: Vec<TileView>,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlantKindKey {
    Fossil,
    Renewable,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantView {
    pub id: u32,
    pub kind: PlantKindKey,
    pub tile: Tile,
    pub capacity: f64,
    pub output: f64,
    pub emissions_monthly: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FirmOwner {
    Household { id: u32 },
    Workers,
    State,
    Bank,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmView {
    pub id: u32,
    pub active: bool,
    pub sector: SectorKey,
    pub name: String,
    pub owner: FirmOwner,
    pub tile: Tile,
    pub workers: f64,
    pub wage: f64,
    pub price: f64,
    pub output: f64,
    pub emissions: f64,
    pub revenue: f64,
    pub profit: f64,
    pub capital: f64,
    pub debt: f64,
    pub green_tech: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankView {
    pub equity: f64,
    pub loans: f64,
    pub deposits: f64,
    pub default_rate: f64,
    pub failed: bool,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernmentView {
    pub cash: f64,
    pub debt: f64,
    pub debt_to_gdp: f64,
    pub legitimacy: f64,
    pub receipts: f64,
    pub spending: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommonsAccessKey {
    Open,
    Community,
    Enclosed,
}

/// A stock as a share of its maximum, clamped to 0..=1 (0 if the maximum is not positive).
fn share(stock: f64, max: f64) -> f64 {
    if max > 0.0 {
        (stock / max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NatureView {
    /// Forest stock as a share of `nature.forest_max`, 0..=1 (`stock / forest_max`, clamped).
    pub forest: f64,
    /// Commons stock as a share of `nature.commons_max`, 0..=1 (`stock / commons_max`, clamped).
    pub commons: f64,
    /// Raw forest stock (the model's unit).
    pub forest_stock: f64,
    /// Raw commons stock (the model's unit).
    pub commons_stock: f64,
    pub commons_access: CommonsAccessKey,
    pub cumulative_tonnes: f64,
    pub net_emissions_monthly: f64,
    pub gross_emissions_monthly: f64,
    pub sink_monthly: f64,
    /// Contract v1.2: 0..=1. Forest and commons shares, and the farm's resource-health factor.
    pub forest_health: f64,
    pub commons_health: f64,
    pub farm_health: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HousingView {
    pub homes: u32,
    pub rent_private: f64,
    pub rent_public: f64,
    pub occupancy: f64,
    pub unhoused_households: u32,
}

/// Progress inside the current turn (contract v1.1).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnPhaseView {
    pub open: bool,
    pub months_done: u32,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub game: GameCalendar,
    pub turn: u32,
    pub tick: u32,
    pub game_over: Option<GameOver>,
    pub turn_phase: TurnPhaseView,
    pub pc: PcView,
    pub system: SystemView,
    pub map: MapView,
    pub buildings: Buildings,
    pub pending: Vec<PendingItem>,
    #[cfg_attr(feature = "ts", ts(type = "Record<LeverKey, PolicyEntry>"))]
    pub policy: BTreeMap<LeverKey, PolicyEntry>,
    pub net_lobby: f64,
    pub union_density: f64,
    pub indicators: Indicators,
    pub people: Vec<PersonView>,
    pub households: Vec<HouseholdView>,
    pub homes: Vec<HomeView>,
    pub plants: Vec<PlantView>,
    pub firms: Vec<FirmView>,
    pub bank: BankView,
    pub government: GovernmentView,
    pub nature: NatureView,
    pub weather: WeatherView,
    pub housing: HousingView,
    pub energy: EnergyView,
    /// Contract v1.4 (section 14).
    pub finance: Finance,
    /// Contract v1.6 (section 16).
    pub breakdowns: crate::breakdowns::Breakdowns,
    pub policies: crate::policies::Policies,
    pub industry: crate::industry::Industry,
    /// Contract v2.3: one card per market for the latest month (empty before the first month).
    pub markets: Vec<crate::markets::MarketView>,
    /// Contract v2.6: the sustainable-yield view of the commons and the forest (always present).
    pub nature_yield: Vec<crate::yields::YieldView>,
    pub history: Vec<HistoryPoint>,
    /// Contract v2.9: the live value of every graph quantity that has a town-level figure (the rest are
    /// listed with reasons in `catalog().quantities[].live`).
    pub quantities: Vec<crate::live::QuantityLive>,
    /// Contract v1.9: the elite estate; absent when the town has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub estate: Option<EstateView>,
}

/// Contract v1.9: the jet.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JetView {
    pub value: f64,
    /// The jet flew in the latest month (play the flight when this month's frame has a
    /// `private_jet_flight` event).
    pub flying_this_month: bool,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastFlightView {
    pub turn: u32,
    /// 0..=2 within the turn.
    #[cfg_attr(feature = "ts", ts(type = "0 | 1 | 2"))]
    pub month: u8,
    pub tonnes: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotStatus {
    Employed,
    LetGo,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotView {
    pub person: u32,
    /// The monthly wage the estate owes (`params.estate.pilot_wage_usd`).
    pub wage: f64,
    pub status: PilotStatus,
    /// Consecutive months the wage could not be paid in full.
    pub unpaid_months: u32,
    /// Months of short pay after which the pilot is let go.
    pub unpaid_months_max: u32,
}

/// The estate's money, from the ledger: the latest month and the trailing twelve months, in
/// dollars. `months` says how many months the second set covers (fewer early in the game).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstateMoney {
    pub dividends: f64,
    pub wage_paid: f64,
    pub wealth_tax: f64,
    pub carbon_charge: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstateFinance {
    pub month: EstateMoney,
    pub last12: EstateMoney,
    pub months: u32,
    /// Wealth tax the estate owes for the coming month, in dollars: the monthly wealth tax on its
    /// net worth now at the rate in force (the same rule the government system applies). The
    /// tax actually paid is capped by savings, so `owed > month.wealthTax` means paid in part.
    pub wealth_tax_owed: f64,
}

/// Contract v1.9 (Town v7): the elite estate. Tiles come from `content/initial_town.json`; the
/// scene draws what this says.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstateView {
    pub household: u32,
    pub home: u32,
    /// Pool, garden and drive tiles (`estate_grounds` on the map).
    pub grounds_tiles: Vec<Tile>,
    /// Runway tiles in take-off order (`runway` on the map).
    pub runway_tiles: Vec<Tile>,
    pub hangar_tile: Tile,
    pub jet_name: String,
    pub jet: JetView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_flight: Option<LastFlightView>,
    pub flights_total: u32,
    pub tonnes_total: f64,
    /// Tonnes of one flight (`params.estate.flight_tonnes`).
    pub flight_tonnes: f64,
    /// Direct monthly emissions of the estate household (`params.estate.household_emissions_t`).
    pub household_emissions_t: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pilot: Option<PilotView>,
    pub finance: EstateFinance,
}

/// Summary counts, derived from the map.
pub fn buildings(sim: &Sim) -> Buildings {
    let count = |k: st::TileKind| sim.state.map.cells.iter().filter(|c| c.kind == k).count() as u32;
    Buildings {
        public_housing: count(st::TileKind::PublicHousing),
        fossil_plant: count(st::TileKind::FossilPlant),
        renewable_plant: count(st::TileKind::RenewablePlant),
        road: count(st::TileKind::Road),
    }
}

fn forest_state(s: &st::State, t: st::Tile) -> ForestTileState {
    if s.weather.fires.iter().any(|f| f.tile == t) {
        ForestTileState::Burning
    } else if s.weather.regrowing.iter().any(|g| g.tile == t) {
        ForestTileState::Regrowing
    } else {
        ForestTileState::Standing
    }
}

fn weather_view(sim: &Sim) -> WeatherView {
    let s = &sim.state;
    let (fire, drought) = sim_model::weather::forecast(s, &sim.params);
    WeatherView {
        // The metric, so the number always agrees with `climate_stress` in the history.
        stress: s.indicators.climate_stress,
        onset: sim.params.weather.onset,
        full: sim.params.weather.full,
        curve: sim.params.weather.curve,
        coverage_radius: sim.params.weather.coverage_radius,
        smoke_radius: sim.params.weather.smoke_radius,
        drought: s.weather.drought.map(|d| DroughtView {
            severity: d.severity,
            months_left: d.months_left,
        }),
        fires: s
            .weather
            .fires
            .iter()
            .map(|f| FireView {
                tile: Tile::from_sim(f.tile),
                months_left: f.months_left,
            })
            .collect(),
        forecast: ForecastView {
            fire_risk: RiskKey::from_sim(fire),
            drought_risk: RiskKey::from_sim(drought),
        },
    }
}

fn map_view(s: &st::State, params: &sim_params::Params) -> MapView {
    let tiles = s
        .map
        .tiles()
        .map(|t| {
            let c = s.map.cells[s.map.idx(t)];
            let emissions_monthly = match c.kind {
                st::TileKind::FossilPlant | st::TileKind::RenewablePlant => {
                    s.plants.iter().find(|p| p.tile == t).map(|p| p.emissions.0)
                }
                _ => None,
            };
            TileView {
                x: i32::from(t.x),
                y: i32::from(t.y),
                kind: TileKindKey::from_sim(c.kind),
                reference: c.id,
                buildable: c.kind == st::TileKind::Empty,
                emissions_monthly,
                state: (c.kind == st::TileKind::Forest).then(|| forest_state(s, t)),
                fire_risk: sim_model::weather::tile_fire_risk(s, params, t),
            }
        })
        .collect();
    MapView {
        width: u32::from(s.map.width),
        height: u32::from(s.map.height),
        tiles,
    }
}

/// `Some` once the game has ended: the first recorded outcome, or `survived` after the last turn.
pub fn game_over(sim: &Sim) -> Option<GameOver> {
    let tpt = sim.params.run.ticks_per_turn.max(1);
    match sim.state.outcome {
        Some((tick, o)) => Some(GameOver {
            turn: tick / tpt + 1,
            outcome: GameOutcome::from_sim(o),
        }),
        None if sim.state.turn > sim.params.run.turns => Some(GameOver {
            turn: sim.state.turn,
            outcome: GameOutcome::Plain(PlainOutcome::Survived),
        }),
        None => None,
    }
}

fn indicators(sim: &Sim) -> Indicators {
    let i = &sim.state.indicators;
    Indicators {
        gdp_annual: i.gdp_annual.dollars(),
        gdp_real_annual: i.gdp_real_annual,
        unemployment: i.unemployment.0,
        employed: i.employed.0,
        labour_force: i.labour_force.0,
        gini: i.gini.0,
        avg_income_per_person: i.avg_income_pp.dollars(),
        median_income_per_person: i.median_income_pp.dollars(),
        top_household_wealth_share: i.top_wealth_share.0,
        wage_share: i.wage_share.0,
        profit_share: i.profit_share.0,
        avg_wellbeing: i.avg_wellbeing.0,
        remaining_budget: i.remaining_budget.0,
        budget_share: budget_share(&sim.state, &sim.params),
        temperature: i.temperature,
        gpi_annual: i.gpi_annual,
        money_in_circulation: i.money.dollars(),
        price_index: i.price_index,
    }
}

/// Every lever's nominal value and, for lobbied ones, the value in force.
pub fn policy_map(s: &st::State) -> BTreeMap<LeverKey, PolicyEntry> {
    let p = &s.policy;
    let e = &s.effective;
    let num = |v: f64| PolicyScalar::Number(v);
    let plain = |v: f64| PolicyEntry {
        nominal: num(v),
        effective: None,
    };
    let lobbied = |n: f64, eff: f64| PolicyEntry {
        nominal: num(n),
        effective: Some(eff),
    };
    let mode = match s.nature.commons_access {
        st::CommonsAccess::Open => "open",
        st::CommonsAccess::Community { .. } => "community",
        st::CommonsAccess::Enclosed(_) => "enclosed",
    };
    let mut m = BTreeMap::new();
    m.insert(
        LeverKey::CarbonPrice,
        lobbied(p.carbon_price.dollars(), e.carbon_price.dollars()),
    );
    m.insert(LeverKey::TaxWage, plain(p.tax_wage.0));
    m.insert(LeverKey::TaxProfit, lobbied(p.tax_profit.0, e.tax_profit.0));
    m.insert(LeverKey::TaxCapital, plain(p.tax_capital.0));
    m.insert(LeverKey::WealthTax, lobbied(p.wealth_tax.0, e.wealth_tax.0));
    m.insert(
        LeverKey::WealthThreshold,
        plain(p.wealth_threshold.dollars()),
    );
    m.insert(LeverKey::BaseRate, plain(p.base_rate.0));
    m.insert(LeverKey::MinWage, plain(p.min_wage.dollars()));
    m.insert(LeverKey::Benefit, plain(p.benefit.dollars()));
    m.insert(LeverKey::Ubi, plain(p.ubi.dollars()));
    m.insert(LeverKey::GreenSubsidy, plain(p.green_subsidy.0));
    m.insert(LeverKey::CampaignCap, plain(p.campaign_cap.0));
    m.insert(
        LeverKey::EmissionsCap,
        lobbied(p.emissions_cap.0, e.emissions_cap.0),
    );
    m.insert(LeverKey::CommonsHarvest, plain(p.commons_harvest.0));
    m.insert(
        LeverKey::CommonsAccess,
        PolicyEntry {
            nominal: PolicyScalar::Mode(mode.to_string()),
            effective: None,
        },
    );
    m.insert(LeverKey::BankCapitalRatio, plain(p.bank_capital_ratio.0));
    m.insert(
        LeverKey::Bailout,
        PolicyEntry {
            nominal: PolicyScalar::Bool(p.bailout),
            effective: None,
        },
    );
    m.insert(LeverKey::PublicSpending, plain(p.public_spending.dollars()));
    m.insert(LeverKey::JobGuarantee, plain(p.job_guarantee.0));
    m.insert(
        LeverKey::ResilienceSpending,
        plain(p.resilience_spending.dollars()),
    );
    m
}

fn people(sim: &Sim) -> Vec<PersonView> {
    sim.state
        .persons
        .iter()
        .map(|p| PersonView {
            employer: p.staff_of.map(|o| EmployerView {
                kind: EmployerKind::Household,
                household: o.0,
                label: sim.state.households[o.idx()].label.clone(),
            }),
            id: p.id.0,
            label: p.label.clone(),
            household: p.household.0,
            age_band: AgeBandKey::from_sim(p.age_band),
            skill: p.skill.0,
            job: p.job.map(|f| JobView { firm: f.0 }),
            wage: p.wage.dollars(),
            commute_tiles: p.commute_tiles,
            commute_minutes: p.commute_minutes,
            wellbeing: p.wellbeing.0,
            in_labour_force: p.in_labour_force,
            job_guarantee: p.jg,
            effective_hours: p.effective_hours.0,
        })
        .collect()
}

fn households(sim: &Sim, meta: &Meta) -> Vec<HouseholdView> {
    sim.state
        .households
        .iter()
        .map(|h| HouseholdView {
            id: h.id.0,
            label: h.label.clone(),
            members: h.members.iter().map(|m| m.0).collect(),
            home: h.home.map(|x| x.0),
            class: ClassKey::from_sim(h.class),
            savings: h.savings.dollars(),
            debt: h.debt.dollars(),
            net_worth: h.wealth.dollars(),
            income: HouseholdIncome {
                sources: meta
                    .sources
                    .income_list(IncomeView::Household, |id| match id {
                        "wages" => h.income.wages.dollars(),
                        "dividends" => h.income.dividends.dollars(),
                        "rent" => h.income.rent_received.dollars(),
                        "interest" => h.income.interest_received.dollars(),
                        "benefits" => h.income.benefits.dollars(),
                        _ => 0.0, // ids are validated against INCOME_IDS at load
                    }),
                tax_paid: h.income.tax_paid.dollars(),
                rent_paid: h.income.rent_paid.dollars(),
                loan_payments: h.income.loan_payments.dollars(),
                spending: h.income.spending.dollars(),
            },
            political_power: h.political_power.0,
            arrears_months: h.arrears_months,
            wellbeing: h.wellbeing.0,
            footprint_tonnes: h.footprint.0,
            food_met: h.food_met.0,
            tag: (sim.state.estate.as_ref().map(|e| e.household) == Some(h.id))
                .then(|| meta.estate_tag.clone()),
            wealth_band: Some(wealth_band(meta, h.wealth.dollars())),
        })
        .collect()
}

/// The band of a net worth in dollars: the first band whose `min` is at or below it.
fn wealth_band(meta: &Meta, net_worth: f64) -> WealthBandView {
    let b = meta
        .wealth_bands
        .iter()
        .find(|b| b.min.is_none_or(|m| net_worth >= m))
        .or(meta.wealth_bands.last());
    match b {
        Some(b) => WealthBandView {
            id: b.id.clone(),
            label: b.label.clone(),
        },
        None => WealthBandView {
            id: String::new(),
            label: String::new(),
        },
    }
}

fn estate_view(sim: &Sim, ledger: &crate::ledger::LedgerStore) -> Option<EstateView> {
    let s = &sim.state;
    let e = s.estate.as_ref()?;
    let tpt = sim.params.run.ticks_per_turn.max(1);
    let pilot = e.pilot.map(|id| PilotView {
        person: id.0,
        wage: sim.params.estate.pilot_wage_usd,
        status: if s.person(id).staff_of == Some(e.household) {
            PilotStatus::Employed
        } else {
            PilotStatus::LetGo
        },
        unpaid_months: e.unpaid_months,
        unpaid_months_max: sim.params.estate.pilot_unpaid_months_max,
    });
    Some(EstateView {
        household: e.household.0,
        home: e.home.0,
        grounds_tiles: e.grounds.iter().map(|t| Tile::from_sim(*t)).collect(),
        runway_tiles: e.runway.iter().map(|t| Tile::from_sim(*t)).collect(),
        hangar_tile: Tile::from_sim(e.hangar),
        jet_name: e.jet_name.clone(),
        jet: JetView {
            value: e.jet.value.dollars(),
            flying_this_month: e.jet.flying_this_month,
        },
        last_flight: e.last_flight.map(|f| LastFlightView {
            turn: f.tick / tpt + 1,
            month: (f.tick % tpt) as u8,
            tonnes: f.tonnes.0,
        }),
        flights_total: e.flights_total,
        tonnes_total: e.tonnes_total.0,
        flight_tonnes: sim.params.estate.flight_tonnes,
        household_emissions_t: sim.params.estate.household_emissions_t,
        pilot,
        finance: {
            let mut f = crate::finance::estate_finance(e.household, ledger);
            let wealth = sim_metrics::household_wealth(s, &sim.params);
            f.wealth_tax_owed = sim_model::util::wealth_tax_monthly(
                wealth[e.household.idx()],
                s.policy.wealth_threshold,
                s.effective.wealth_tax.0,
            )
            .dollars();
            f
        },
    })
}

fn homes(sim: &Sim) -> Vec<HomeView> {
    sim.state
        .homes
        .iter()
        .map(|h| HomeView {
            id: h.id.0,
            tile: Tile::from_sim(h.tile),
            owner: match h.owner {
                st::HomeOwner::Household(x) => HomeOwnerView::Household(x.0),
                st::HomeOwner::Public => HomeOwnerView::Public(PublicOwner::Public),
            },
            household: h.household.map(|x| x.0),
            rent: h.rent.dollars(),
            value: h.value.dollars(),
        })
        .collect()
}

fn plants(sim: &Sim) -> Vec<PlantView> {
    sim.state
        .plants
        .iter()
        .map(|p| PlantView {
            id: p.id.0,
            kind: match p.kind {
                st::PlantKind::Fossil => PlantKindKey::Fossil,
                st::PlantKind::Renewable => PlantKindKey::Renewable,
            },
            tile: Tile::from_sim(p.tile),
            capacity: p.capacity.0,
            output: p.output.0,
            emissions_monthly: p.emissions.0,
        })
        .collect()
}

/// Firms have no label in the sim (gap G8): name them from sector and slot order.
pub(crate) fn firm_names(firms: &[st::Firm]) -> Vec<String> {
    let mut goods = 0u32;
    let mut food = 0u32;
    let mut services = 0u32;
    firms
        .iter()
        .map(|f| {
            let (base, n) = match f.sector {
                st::Sector::Goods => ("Factory", &mut goods),
                st::Sector::Food => ("Farm", &mut food),
                st::Sector::Services => ("Shop", &mut services),
            };
            *n += 1;
            if *n == 1 {
                base.to_string()
            } else {
                format!("{base} {n}")
            }
        })
        .collect()
}

fn firms(sim: &Sim) -> Vec<FirmView> {
    let names = firm_names(&sim.state.firms);
    sim.state
        .firms
        .iter()
        .zip(names)
        .map(|(f, name)| FirmView {
            id: f.id.0,
            active: f.active,
            sector: SectorKey::from_sim(f.sector),
            name,
            tile: Tile::from_sim(f.tile),
            owner: match f.owner {
                st::Owner::Household(h) => FirmOwner::Household { id: h.0 },
                st::Owner::Workers => FirmOwner::Workers,
                st::Owner::State => FirmOwner::State,
                st::Owner::Bank => FirmOwner::Bank,
            },
            workers: f.workers.0,
            wage: f.wage.dollars(),
            price: f.price.dollars(),
            output: f.output.0,
            emissions: f.emissions.0,
            revenue: f.revenue.dollars(),
            profit: f.profit.dollars(),
            capital: f.capital.0,
            debt: f.debt.dollars(),
            green_tech: f.green_tech.0,
        })
        .collect()
}

/// Build the full snapshot from the sim, the saved history and the metadata.
pub fn build(
    sim: &Sim,
    history: &[HistoryPoint],
    meta: &Meta,
    ledger: &crate::ledger::LedgerStore,
    log: &crate::log::WrapperLog,
) -> Snapshot {
    let markets =
        crate::markets::build(sim, history, log, &meta.sources.markets, &meta.energy_unit);
    let food_wanted = markets.iter().find(|m| m.id == "food").map(|m| m.demand);
    let s = &sim.state;
    let finance = crate::finance::build(sim, ledger, &meta.sources);
    let policy = policy_map(s);
    let tp = &sim.params.turn;
    let mut snap = Snapshot {
        schema_version: crate::SCHEMA_VERSION,
        game: meta.game,
        turn: s.turn,
        tick: s.tick,
        game_over: game_over(sim),
        turn_phase: TurnPhaseView {
            open: sim.turn_phase.open,
            months_done: sim.turn_phase.months_done,
        },
        pc: PcView {
            current: s.political_capital,
            cap: tp.pc_cap,
            regen_next_turn: tp.pc_base * s.gov.legitimacy.0,
        },
        system: SystemView {
            current: SystemKey::from_sim(s.system),
            transition: s.transition.map(|t| TransitionView {
                to: SystemKey::from_sim(t.to),
                months_left: t.months_left,
                total: t.total,
            }),
        },
        map: map_view(s, &sim.params),
        buildings: buildings(sim),
        pending: s.pending.iter().map(PendingItem::from_sim).collect(),
        policies: crate::policies::build(sim, meta, &policy, &log.policy),
        policy,
        net_lobby: s.net_lobby.0,
        union_density: s.union_density.0,
        indicators: indicators(sim),
        people: people(sim),
        households: households(sim, meta),
        homes: homes(sim),
        plants: plants(sim),
        firms: firms(sim),
        bank: BankView {
            equity: s.bank.equity.dollars(),
            loans: s.bank.loans.dollars(),
            deposits: s.bank.deposits.dollars(),
            default_rate: s.bank.default_rate.0,
            failed: s.bank.failed,
        },
        government: GovernmentView {
            cash: s.gov.cash.dollars(),
            debt: s.gov.debt.dollars(),
            debt_to_gdp: debt_to_gdp(s),
            legitimacy: s.gov.legitimacy.0,
            receipts: s.gov.receipts.dollars(),
            spending: s.gov.spending.dollars(),
        },
        nature: NatureView {
            forest: share(s.nature.forest.0, sim.params.nature.forest_max),
            commons: share(s.nature.commons.0, sim.params.nature.commons_max),
            forest_stock: s.nature.forest.0,
            commons_stock: s.nature.commons.0,
            commons_access: match s.nature.commons_access {
                st::CommonsAccess::Open => CommonsAccessKey::Open,
                st::CommonsAccess::Community { .. } => CommonsAccessKey::Community,
                st::CommonsAccess::Enclosed(_) => CommonsAccessKey::Enclosed,
            },
            cumulative_tonnes: s.nature.cumulative.0,
            net_emissions_monthly: s.nature.net_emissions.0,
            gross_emissions_monthly: s.nature.gross_emissions.0,
            sink_monthly: s.nature.sink.0,
            forest_health: s.indicators.forest_health,
            commons_health: s.indicators.commons_health,
            farm_health: s.indicators.farm_health,
        },
        weather: weather_view(sim),
        housing: HousingView {
            homes: s.homes.len() as u32,
            rent_private: s.housing.rent_private.dollars(),
            rent_public: s.housing.rent_public.dollars(),
            occupancy: s.housing.occupancy.0,
            unhoused_households: s.housing.unhoused_households,
        },
        energy: crate::finance::energy(sim, &meta.energy_unit),
        breakdowns: crate::breakdowns::build(sim, &finance, log, &meta.sources),
        industry: crate::industry::build(sim, &finance),
        nature_yield: crate::yields::build(
            sim,
            history,
            &meta.sources.yields,
            &meta.sources.markets.mechanisms,
            food_wanted,
        ),
        markets,
        finance,
        history: history.to_vec(),
        quantities: Vec::new(),
        estate: estate_view(sim, ledger),
    };
    snap.quantities = crate::live::build(&snap, history, meta.sources.markets.steady_band);
    snap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stocks_become_shares_of_their_maximum() {
        assert_eq!(share(50_000.0, 100_000.0), 0.5);
        assert_eq!(share(150_000.0, 100_000.0), 1.0);
        assert_eq!(share(-1.0, 100_000.0), 0.0);
        assert_eq!(share(1.0, 0.0), 0.0);
    }
}
