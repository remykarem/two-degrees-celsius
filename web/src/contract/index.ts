// Snapshot contract v1.0, Town v2 (docs/snapshot-contract.md section 11 wins over sections 3-10). Types only: no logic, no imports.

/** Snapshot.schemaVersion for contract v1.0. */
export type SchemaVersion = 2;

export type LeverKey =
  | 'carbon_price' | 'tax_wage' | 'tax_profit' | 'tax_capital' | 'wealth_tax'
  | 'wealth_threshold' | 'base_rate' | 'min_wage' | 'benefit' | 'ubi'
  | 'green_subsidy' | 'campaign_cap' | 'emissions_cap' | 'commons_harvest'
  | 'commons_access' | 'bank_capital_ratio' | 'bailout' | 'public_spending'
  | 'job_guarantee';
export type InfraKind = 'public_housing' | 'fossil_plant' | 'renewable_plant' | 'road';
export interface TileRef { x: number; y: number }
export type SystemKey = 'market' | 'regulated' | 'coop';
export type ClassKey = 'capitalist' | 'rentier' | 'manager' | 'worker' | 'dependent';
export type BlockReason =
  | 'transition_in_progress' | 'nothing_to_remove' | 'same_system'
  | 'lever_unavailable' | 'insufficient_political_capital' | 'insufficient_funds' | 'invalid_command'
  | 'tile_occupied' | 'tile_invalid';

export type MetricKey =
  | 'gdp' | 'gpi' | 'unemployment' | 'gini' | 'wage_share' | 'profit_share' | 'wellbeing' | 'emissions'
  | 'avg_income' | 'median_income' | 'top_wealth_share'
  | 'budget' | 'temperature' | 'legitimacy' | 'debt' | 'money' | 'prices' | 'housing' | 'forest' | 'commons';

export type CommonsAccessWire = { mode: 'open' } | { mode: 'community'; cap: number };

export type Command =
  | { type: 'set_policy'; lever: LeverKey; value: number | boolean | CommonsAccessWire }
  | { type: 'switch_system'; to: SystemKey }
  | { type: 'place'; kind: InfraKind; tile: TileRef }
  | { type: 'remove'; tile: TileRef };

export type LeverUnit = 'usd' | 'usd_per_month' | 'usd_per_tonne' | 'fraction' | 'tonnes_per_month' | 'bool' | 'enum';
export interface CostModel { base: number; unit: number; minSize: number }

export interface LeverInfo {
  key: LeverKey;
  label: string;
  unit: LeverUnit;
  min: number;
  max: number;
  step: number;
  /** Base PC cost of a minimum-size change. A hint only: the real cost is base x max(|change| / unit, minSize) x (1 + opposition). */
  baseCostPc: number;
  /** For hints ("about N PC"). CommandPreview.pc is the authority. */
  costModel: CostModel;
  /** True for levers where 0 means "off" (emissions_cap): the UI says "no cap", not "strictest". */
  zeroMeansNone?: boolean;
  leadTime: number;
  lobbied: boolean;
  arrows: { metric: MetricKey; direction: 'up' | 'down' }[];
  availableIn: SystemKey[];
  /** For unit 'enum' levers: the modes in order (min/max/step are 0 and ignored). */
  options?: string[];
  /** Enum options that are only available in some systems (for example commons 'community' is not available in Market). Not in section 10: see docs/web-notes-align.md. */
  optionAvailableIn?: Record<string, SystemKey[]>;
}

/** Costs are per building kind; a `remove` command names only a tile, the sim prices it by what stands there. */
export type ActionKey = 'switch_system' | `place:${InfraKind}` | `remove:${InfraKind}`;
export interface ActionInfo {
  key: ActionKey;
  label: string;
  baseCostPc: number;
  /** The real wire now sends it (base, unit, min size from params). Kept optional for older saves and the mock. When absent, cost = baseCostPc exactly (unit 1, min size 1). */
  costModel?: CostModel;
  leadTime: number;
  /** Estimated dollars; 0 if none. preview() is the authority on actual cost. */
  moneyCost: number;
  availableIn: SystemKey[];
}

export interface CommandPreview {
  command: Command;
  pc: number;
  money: number;
  leadTime: number;
  effectiveTurn: number;
  opposition: number;
  blocked: BlockReason | null;
  /** For place and remove: can the tile take the command? True for other commands. */
  tileValid: boolean;
  /** Why not, when tileValid is false (a BlockReason code on the real wire). */
  tileReason?: string;
}

export interface PendingItem {
  id: number;
  command: Command;
  submittedTurn: number;
  effectiveTurn: number;
  pcCost: number;
  moneyCost: number;
}

export type GameOutcome =
  | 'budget_exhausted'
  | 'survived'
  | { collapse: 'legitimacy_zero' | 'mass_unemployment' | 'bank_failure_cascade' };

export interface Indicators {
  gdpAnnual: number; gdpRealAnnual: number; unemployment: number; employed: number; labourForce: number;
  gini: number; wageShare: number; profitShare: number; avgWellbeing: number;
  remainingBudget: number; budgetShare: number;
  temperature: number; gpiAnnual: number; moneyInCirculation: number; priceIndex: number;
  avgIncomePerPerson: number; medianIncomePerPerson: number; topHouseholdWealthShare: number;
}

export type TileKind = 'empty' | 'home' | 'public_housing' | 'firm' | 'bank' | 'town_hall'
  | 'fossil_plant' | 'renewable_plant' | 'road' | 'forest' | 'commons';
export interface Tile {
  x: number; y: number; kind: TileKind;
  /** Home id for home/public_housing tiles, firm id for firm tiles. */
  ref?: number;
  buildable: boolean;
  /** Plant tiles only: tonnes per month this plant emits. */
  emissionsMonthly?: number;
}
export interface GameMap { width: 12; height: 12; tiles: Tile[] }

export type AgeBand = 'child' | 'adult' | 'retired';
export interface PersonView {
  id: number; label: string; household: number; ageBand: AgeBand; skill: number;
  job: { firm: number } | null; wage: number; commuteTiles: number; commuteMinutes: number; wellbeing: number;
}

export interface HouseholdView {
  id: number; label: string; members: number[]; home: number | null; class: ClassKey;
  savings: number; debt: number; netWorth: number;
  income: {
    wages: number; dividends: number; rent: number; interest: number; benefits: number;
    taxPaid: number; rentPaid: number; loanPayments: number; spending: number;
  };
  politicalPower: number; arrearsMonths: number;
}

export interface HomeView {
  id: number; tile: TileRef;
  /** Owning household id, or 'public'. */
  owner: number | 'public';
  household: number | null;
  rent: number;
}

export type Sector = 'goods' | 'food' | 'services';

export type FirmOwner =
  | { kind: 'household'; id: number }
  | { kind: 'workers' }
  | { kind: 'state' }
  | { kind: 'bank' };

export interface FirmView {
  id: number; active: boolean; sector: Sector; name: string;
  owner: FirmOwner;
  workers: number; wage: number; price: number; output: number; emissions: number;
  revenue: number; profit: number; capital: number; debt: number; greenTech: number;
}

export interface BankView { equity: number; loans: number; deposits: number; defaultRate: number; failed: boolean }
export interface GovernmentView {
  cash: number; debt: number; debtToGdp: number; legitimacy: number; receipts: number; spending: number;
}
export interface NatureView {
  /** Shares of the maximum stock (nature.forest_max / commons_max in params), 0..1. The wire layer divides; see docs/integration-notes.md. */
  forest: number; commons: number;
  /** Raw stocks in the model's unit. Real wire only. */
  forestStock?: number; commonsStock?: number;
  commonsAccess: 'open' | 'community' | 'enclosed';
  cumulativeTonnes: number; netEmissionsMonthly: number; grossEmissionsMonthly: number; sinkMonthly: number;
}
export interface HousingView {
  privateCap: number; publicCap: number; rentPrivate: number; rentPublic: number; occupancy: number; housedShare: number;
}
export interface EnergyView { fossilCap: number; renewableCap: number; fossilShare: number }

export interface HistoryPoint {
  turn: number; gdpRealAnnual: number; unemployment: number; gini: number;
  emissions: number; budgetShare: number; legitimacy: number; avgWellbeing: number; gpiAnnual: number; debtToGdp: number;
  avgIncomePerPerson: number; medianIncomePerPerson: number; topHouseholdWealthShare: number;
}

export interface PolicyEntry { nominal: number | boolean | string; effective: number | null }

/** Summary counts (the tiles are the truth). */
export interface Buildings { publicHousing: number; fossilPlant: number; renewablePlant: number; road?: number }

export interface GameCalendar { startYear: number; firstTurn: number; lastTurn: number; turnsPerYear: number }

export interface Snapshot {
  schemaVersion: SchemaVersion;
  game: GameCalendar;
  turn: number;
  tick: number;
  /** The map, owned by the simulation. */
  map: GameMap;
  /** Summary counts of built things. */
  buildings: Buildings;
  /** True when the numbers come from the web mock rather than the real simulation. The UI shows a badge. Not in section 10. */
  mock?: boolean;
  gameOver: null | { turn: number; outcome: GameOutcome };
  pc: { current: number; cap: number; regenNextTurn: number };
  system: { current: SystemKey; transition: null | { to: SystemKey; monthsLeft: number; total: number } };
  pending: PendingItem[];
  policy: Record<LeverKey, PolicyEntry>;
  netLobby: number;
  unionDensity: number;
  indicators: Indicators;
  people: PersonView[];
  households: HouseholdView[];
  homes: HomeView[];
  firms: FirmView[];
  bank: BankView;
  government: GovernmentView;
  nature: NatureView;
  housing: HousingView;
  energy: EnergyView;
  history: HistoryPoint[];
}

export interface RejectedCommand { index: number; command: Command; reason: BlockReason }
export interface PolicyGap { lever: LeverKey; nominal: number; effective: number }

export interface TurnResult {
  turn: number;
  accepted: PendingItem[];
  rejected: RejectedCommand[];
  activated: PendingItem[];
  pcBefore: number;
  pcAfter: number;
  months: MonthFrame[];
  policyGaps: PolicyGap[];
  trace: TraceRecord[];
  snapshot: Snapshot;
}

/** Any array may be missing or empty. Commutes are per person with a tile path; purchases are real (household to firm). */
export interface MonthFlows {
  commutes?: { person: number; household: number; firm: number; path: TileRef[] }[];
  purchases?: { household: number; firm: number; sector: Sector; amount: number }[];
  rent?: { household: number; amount: number }[];
  wages?: { household: number; firm: number; amount: number }[];
  emissions?: { source: 'firm' | 'households' | 'sink' | 'plant'; id?: number; tile?: TileRef; tonnes: number }[];
}

/** Section 10.7: every fallible SimPort call returns this; Rust violations become error results, never exceptions. */
export interface SimError { code: string; message: string }
export type Result<T> = { ok: true; value: T } | { ok: false; error: SimError };

export interface MonthFrame { month: 0 | 1 | 2; events: SimEvent[]; flows: MonthFlows }

export type CollapseReason = 'legitimacy_zero' | 'mass_unemployment' | 'bank_failure_cascade';
type Importance = { importance: 1 | 2 | 3 };

/** Type + importance + payload. No display text: the UI formats it. */
export type SimEvent = Importance & (
  | { type: 'commuted'; person: number; household: number; firm: number }
  | { type: 'person_hired'; person: number; firm: number }
  | { type: 'person_fired'; person: number; firm: number }
  | { type: 'household_moved'; household: number; to: number }
  | { type: 'household_evicted'; household: number }
  | { type: 'purchase'; household: number; firm: number; amount: number }
  | { type: 'hired'; firm: number; people: number }
  | { type: 'fired'; firm: number; people: number }
  | { type: 'loan_defaulted'; firm: number }
  | { type: 'firm_founded'; firm: number }
  | { type: 'firm_closed'; firm: number }
  | { type: 'forest_shrank'; stock: number }
  | { type: 'commons_collapsed' }
  | { type: 'commons_enclosed'; owner: number }
  | { type: 'policy_blocked'; reason: BlockReason }
  | { type: 'command_activated'; id: number }
  | { type: 'bank_failed' }
  | { type: 'rent_shortfall'; household: number; amount: number }
  | { type: 'spending_cut'; shortfall: number }
  | { type: 'bank_bailout'; amount: number }
  | { type: 'class_changed'; household: number; to: ClassKey }
  | { type: 'system_switch_started'; to: SystemKey }
  | { type: 'system_switch_completed'; to: SystemKey }
  | { type: 'budget_exhausted' }
  | { type: 'collapse'; reason: CollapseReason }
);
export type SimEventType = SimEvent['type'];

export type TraceRecord =
  | { type: 'emissions'; source: 'firm' | 'households' | 'sink' | 'plant'; id?: number; tile?: TileRef; tonnes: number }
  | { type: 'income'; household: number; kind: 'wages' | 'dividends' | 'rent' | 'interest' | 'benefits'; amount: number }
  | { type: 'policy_gap'; lever: LeverKey; nominal: number; effective: number; netLobby: number };
