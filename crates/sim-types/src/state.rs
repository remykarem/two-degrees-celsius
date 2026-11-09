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

/// One of the 20 people. Work, skill, commuting and wellbeing live here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub label: String,
    pub household: HouseholdId,
    pub age_band: AgeBand,
    pub skill: Rate,
    /// Adults who look for work. Owners and the like are adults outside the labour force.
    pub in_labour_force: bool,
    /// Wage this person asks for; the cheapest are hired first.
    pub reservation: Cents,
    pub job: Option<FirmId>,
    /// In the government job guarantee (employed, paid the minimum wage).
    pub jg: bool,
    /// Monthly pay actually received this tick.
    pub wage: Cents,
    pub commute_tiles: u32,
    pub commute_minutes: u32,
    /// Share of a full working month actually productive, after commute losses.
    pub effective_hours: Rate,
    pub wellbeing: Rate,
}

impl Person {
    /// In work: a firm job or the job guarantee.
    pub fn employed(&self) -> bool {
        self.job.is_some() || self.jg
    }
    pub fn unemployed(&self) -> bool {
        self.in_labour_force && !self.employed()
    }
}

/// A household of 2 to 4 people. Money, spending, saving, rent and loans live here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Household {
    pub id: HouseholdId,
    pub label: String,
    pub members: Vec<PersonId>,
    /// `None`: the household has no home.
    pub home: Option<HomeId>,
    pub class: Class,
    pub savings: Cents,
    pub debt: Cents,
    pub income: IncomeFlows,
    /// Last six months of (capital income, gross income).
    pub income_hist: Vec<(Cents, Cents)>,
    pub last_disposable: Cents,
    pub food_met: Rate,
    pub consumption: Cents,
    /// Mean wellbeing of the members.
    pub wellbeing: Rate,
    pub political_power: Rate,
    /// Direct emissions plus the energy emissions attributed to the household's electricity.
    pub footprint: Tonnes,
    /// Electricity used this month.
    pub energy_use: Units,
    /// Net worth: savings, homes owned, firm and bank equity, minus debt.
    pub wealth: Cents,
    pub arrears: bool,
    /// Consecutive months in which the rent was not paid in full.
    pub arrears_months: u32,
}

/// A home on the map. Each home holds at most one household.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Home {
    pub id: HomeId,
    pub tile: Tile,
    pub owner: HomeOwner,
    pub household: Option<HouseholdId>,
    /// Monthly rent charged to a household that does not own the home.
    pub rent: Cents,
    pub value: Cents,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub id: PlantId,
    pub kind: PlantKind,
    pub tile: Tile,
    /// Energy units per month the plant can deliver.
    pub capacity: Units,
    pub output: Units,
    /// Monthly emissions (zero for renewables).
    pub emissions: Tonnes,
}

/// One cell of the map: what stands there and the id of that thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapTile {
    pub kind: TileKind,
    /// Home, firm or plant id; `None` for empty, scenery and roads.
    pub id: Option<u32>,
}

impl MapTile {
    pub const EMPTY: MapTile = MapTile {
        kind: TileKind::Empty,
        id: None,
    };
}

/// The simulation's map: row-major cells, at most one thing per tile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Map {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<MapTile>,
}

impl Map {
    pub fn new(width: u8, height: u8) -> Map {
        Map {
            width,
            height,
            cells: vec![MapTile::EMPTY; width as usize * height as usize],
        }
    }
    pub fn in_bounds(&self, t: Tile) -> bool {
        t.x < self.width && t.y < self.height
    }
    pub fn idx(&self, t: Tile) -> usize {
        t.y as usize * self.width as usize + t.x as usize
    }
    pub fn get(&self, t: Tile) -> Option<MapTile> {
        self.in_bounds(t).then(|| self.cells[self.idx(t)])
    }
    pub fn kind_at(&self, t: Tile) -> Option<TileKind> {
        self.get(t).map(|c| c.kind)
    }
    pub fn set(&mut self, t: Tile, c: MapTile) {
        if self.in_bounds(t) {
            let i = self.idx(t);
            self.cells[i] = c;
        }
    }
    /// All tiles in row-major order.
    pub fn tiles(&self) -> impl Iterator<Item = Tile> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| Tile { x, y }))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Firm {
    pub id: FirmId,
    pub active: bool,
    pub sector: Sector,
    pub tile: Tile,
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
    /// Process (direct) emissions per unit of output, before green technology.
    pub intensity_base: Rate,
    /// Electricity per unit of output, before green technology (energy efficiency).
    pub energy_per_unit: Rate,
    pub green_tech: Rate,
    pub tfp: Rate,
    pub output: Units,
    /// Direct (process) emissions this month.
    pub emissions: Tonnes,
    /// Emissions the firm answers for: direct plus the energy emissions of the electricity it
    /// uses, at the grid's average intensity. This is the basis of the carbon price. The energy
    /// part is physically emitted by plants and counted once, there.
    pub footprint: Tonnes,
    pub energy_use: Units,
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
    /// Exponentially smoothed sales in units (the basis of production and employment).
    pub expected_sales: Units,
    /// Price markup over unit cost, set at the start so the initial price is on target.
    pub markup: Rate,
    /// Capital per unit of expected sales (accelerator target), set at the start.
    pub capital_per_sale: f64,
    /// Spent this tick on capital goods bought from the goods sector.
    pub capex: Cents,
    /// Profit tax actually paid this tick.
    pub tax_paid: Cents,
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
    /// Goods purchases that were budgeted but could not be funded this tick.
    pub unfunded: Cents,
    /// Running total of unfunded spending and unpaid interest.
    pub unfunded_total: Cents,
    /// Real receipts this tick (taxes and carbon revenue).
    pub receipts: Cents,
    pub pending_build: Vec<(InfraKind, Tile)>,
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
    /// Market rent of a privately owned home that is rented out.
    pub rent_private: Cents,
    pub rent_public: Cents,
    /// Occupied homes over all homes.
    pub occupancy: Rate,
    pub unhoused_households: u32,
}

/// The power system this month.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Energy {
    /// Electricity firms and households want.
    pub demand: Units,
    /// Total plant capacity.
    pub supply: Units,
    pub delivered: Units,
    /// Share of demand that is served (1 when supply is enough).
    pub served: Rate,
    /// Tonnes of CO2 per energy unit delivered (fossil share of capacity x the fossil factor).
    pub grid_intensity: Rate,
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
    pub public_spending: Cents,
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
    /// Exponentially smoothed unemployment: with 10 people in the labour force one job is ten
    /// points, so wages, legitimacy and the collapse test read this slower measure.
    pub unemployment_smooth: Rate,
    pub employed: People,
    pub labour_force: People,
    pub gini: Rate,
    /// Mean monthly income per person (household gross income shared over its members).
    pub avg_income_pp: Cents,
    pub median_income_pp: Cents,
    /// Wealth of the wealthiest household over the positive wealth of all households.
    pub top_wealth_share: Rate,
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
    pub persons: Vec<Person>,
    pub households: Vec<Household>,
    pub firms: Vec<Firm>,
    pub homes: Vec<Home>,
    pub plants: Vec<Plant>,
    pub next_home_id: u32,
    pub next_plant_id: u32,
    pub map: Map,
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
        People(self.persons.len() as f64)
    }
    pub fn person(&self, id: PersonId) -> &Person {
        &self.persons[id.idx()]
    }
    pub fn home(&self, id: HomeId) -> Option<&Home> {
        self.homes.iter().find(|h| h.id == id)
    }
    pub fn plant(&self, id: PlantId) -> Option<&Plant> {
        self.plants.iter().find(|p| p.id == id)
    }
    /// Capacity of all plants of a kind.
    pub fn capacity(&self, kind: PlantKind) -> f64 {
        self.plants
            .iter()
            .filter(|p| p.kind == kind)
            .map(|p| p.capacity.0)
            .sum()
    }
    /// Labour force and the number of those in work (firm job or job guarantee).
    pub fn labour_force(&self) -> (f64, f64) {
        let lf = self.persons.iter().filter(|p| p.in_labour_force).count();
        let emp = self
            .persons
            .iter()
            .filter(|p| p.in_labour_force && p.employed())
            .count();
        (emp as f64, lf as f64)
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
