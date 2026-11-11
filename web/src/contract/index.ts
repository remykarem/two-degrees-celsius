// Snapshot contract v1.4, Town v5 (docs/snapshot-contract.md sections 11 to 14 win over sections 3-10). Types only: no logic, no imports.

/** Snapshot.schemaVersion. 3 since the weather State layout (section 13); 4 since the saved wrapper blob gains the ledger (section 14). Older saves are refused. */
export type SchemaVersion = 4 | 5;

export type LeverKey =
  | 'carbon_price' | 'tax_wage' | 'tax_profit' | 'tax_capital' | 'wealth_tax'
  | 'wealth_threshold' | 'base_rate' | 'min_wage' | 'benefit' | 'ubi'
  | 'green_subsidy' | 'campaign_cap' | 'emissions_cap' | 'commons_harvest'
  | 'commons_access' | 'bank_capital_ratio' | 'bailout' | 'public_spending'
  | 'job_guarantee' | 'resilience_spending';
export type InfraKind = 'public_housing' | 'fossil_plant' | 'renewable_plant' | 'road' | 'fire_station';
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
  | 'budget' | 'temperature' | 'legitimacy' | 'debt' | 'money' | 'prices' | 'housing' | 'forest' | 'commons'
  | 'forest_health' | 'commons_health' | 'farm_health' | 'climate_stress';

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
  /** True for place and remove actions. Real wire only. */
  needsTile?: boolean;
  /** Plain-text rule for which tiles are valid. Real wire only. */
  tileRule?: string;
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
  | 'fossil_plant' | 'renewable_plant' | 'road' | 'forest' | 'commons' | 'fire_station';
/** Contract v1.2: the state of a forest tile. */
export type ForestState = 'standing' | 'burning' | 'regrowing';
export interface Tile {
  x: number; y: number; kind: TileKind;
  /** Home id for home/public_housing tiles, firm id for firm tiles. */
  ref?: number;
  buildable: boolean;
  /** Plant tiles only: tonnes per month this plant emits. */
  emissionsMonthly?: number;
  /** Contract v1.2, forest tiles only. Absent in a package built before v1.2 (read as 'standing'). */
  state?: ForestState;
}
export interface GameMap { width: 12; height: 12; tiles: Tile[] }

export type AgeBand = 'child' | 'adult' | 'retired';
export interface PersonView {
  id: number; label: string; household: number; ageBand: AgeBand; skill: number;
  job: { firm: number } | null; wage: number; commuteTiles: number; commuteMinutes: number; wellbeing: number;
  /** Extras on the real wire (docs/sim-api-notes.md). */
  inLabourForce: boolean; jobGuarantee: boolean; effectiveHours: number;
}

export interface HouseholdView {
  id: number; label: string; members: number[]; home: number | null; class: ClassKey;
  savings: number; debt: number; netWorth: number;
  income: {
    wages: number; dividends: number; rent: number; interest: number; benefits: number;
    taxPaid: number; rentPaid: number; loanPayments: number; spending: number;
  };
  politicalPower: number; arrearsMonths: number;
  /** Extras on the real wire (docs/sim-api-notes.md). */
  wellbeing: number; footprintTonnes: number; foodMet: number;
}

export interface HomeView {
  id: number; tile: TileRef;
  /** Owning household id, or 'public'. */
  owner: number | 'public';
  household: number | null;
  rent: number;
  /** Extra on the real wire. */
  value: number;
}

/** Extra on the real wire: one entry per plant (also on its tile as `emissionsMonthly`). */
export interface PlantView { id: number; kind: 'fossil' | 'renewable'; tile: TileRef; capacity: number; output: number; emissionsMonthly: number }

export type Sector = 'goods' | 'food' | 'services';

export type FirmOwner =
  | { kind: 'household'; id: number }
  | { kind: 'workers' }
  | { kind: 'state' }
  | { kind: 'bank' };

export interface FirmView {
  id: number; active: boolean; sector: Sector; name: string; tile: TileRef;
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
  /** Contract v1.2, 0..1: the forest and commons shares and the farm's resource-health factor. Absent in a package built before v1.2. */
  forestHealth: number; commonsHealth: number; farmHealth: number;
  cumulativeTonnes: number; netEmissionsMonthly: number; grossEmissionsMonthly: number; sinkMonthly: number;
}
export interface HousingView { homes: number; rentPrivate: number; rentPublic: number; occupancy: number; unhousedHouseholds: number }
/**
 * Contract v1.4 (section 14): the power grid with a unit. `unit` is the label from config (default 'MWh'; the displayed rate is per month).
 * A package built before v1.4 sent a different object under the same key (fossilCap, supply as a number, ...): the wasm loader refuses it.
 */
export interface EnergyView {
  unit: string;
  supply: { tile: TileRef; kind: 'fossil' | 'renewable'; capacity: number }[];
  demand: { firms: number; households: number; total: number };
  /** 0..1. */
  served: number;
  /** Tonnes of CO2 per unit of energy delivered. */
  intensity: number;
  renewableShare: number;
  emissionsByPlant: { tile: TileRef; tonnes: number }[];
}

/** Contract v1.4: one money movement. Money is dollars. */
export type AccountRef =
  | { kind: 'household'; id: number } | { kind: 'firm'; id: number }
  | { kind: 'government' } | { kind: 'bank' } | { kind: 'external'; label: string };
export type LedgerKind = 'wage' | 'dividend' | 'rent' | 'benefit' | 'tax' | 'purchase' | 'lobby' | 'interest' | 'loan_new' | 'loan_repay'
  | 'loan_writeoff' | 'build' | 'upkeep' | 'subsidy' | 'bailout' | 'wealth_consumption' | 'neutral';
export interface LedgerEntry {
  turn: number; month: 0 | 1 | 2; tick: number; from: AccountRef; to: AccountRef; amount: number;
  kind: LedgerKind; detail?: string; memo: string;
  /** Balance of the queried account after this entry (household savings, firm or government cash, bank equity). */
  balanceAfter: number;
  /** Debt of the queried account after this entry (household, firm, government debt; bank loans). */
  debtAfter: number;
}
/** A rolled-up group of older entries. `net` and `debtNet` are the effect on the queried account's balance and debt (inflow and outflow are the gross sides). */
export interface LedgerSummaryRow { turn: number; month: 0 | 1 | 2; counterparty: AccountRef; kind: LedgerKind; inflow: number; outflow: number; count: number; net: number; debtNet: number }
export interface LedgerPage {
  account: AccountRef;
  /** At turn 0. */
  openingBalance: number;
  openingDebt: number;
  /** The detail window (the last `ledger.detail_turns` turns). */
  entries: LedgerEntry[];
  /** Rolled-up older periods. */
  summary: LedgerSummaryRow[];
  closingBalance: number;
  closingDebt: number;
  /** True iff opening + roll-ups + entries = closing, for both balance and debt. */
  reconciles: boolean;
  detailFromTurn: number;
}
/** `eventTurns[i]` is the turn of `events[i]`; `summary` has the earnings older than the detail window (monthly, per firm). */
export interface PersonLedger { person: number; household: number; earnings: LedgerEntry[]; summary: LedgerSummaryRow[]; events: SimEvent[]; eventTurns: number[] }
export interface LedgerQuery { fromTurn?: number; toTurn?: number; kind?: LedgerKind }

/** Contract v1.4: finance aggregates. Flows are for the latest month; `quarter` holds the trailing-quarter totals of the same flows. */
export interface FinanceFlows {
  govRevenue: Record<'wageTax' | 'profitTax' | 'capitalTax' | 'wealthTax' | 'carbon' | 'other', number>;
  govSpending: Record<'benefits' | 'ubi' | 'publicSpending' | 'jobGuarantee' | 'upkeep' | 'subsidy' | 'interest' | 'builds' | 'other', number>;
  householdIncome: Record<'wages' | 'dividends' | 'rent' | 'interest' | 'benefits', number>;
  householdTaxes: number; rentPaid: number; loanPayments: number; householdSpending: number;
  firmRevenue: number; wageBill: number; firmProfit: number; investment: number; bankInterestIncome: number;
}
export interface FinanceView {
  government: {
    revenue: FinanceFlows['govRevenue']; spending: FinanceFlows['govSpending'];
    balance: number; cash: number; debt: number; debtToGdp: number; legitimacy: number; basis: 'month';
  };
  bank: { equity: number; loans: number; deposits: number; capitalRatio: number; capitalRatioLimit: number; headroom: number; defaultRate: number; interestIncome: number };
  households: { income: FinanceFlows['householdIncome']; taxes: number; rentPaid: number; loanPayments: number; spending: number; savingRate: number; debt: number; netWorth: number };
  firms: { revenue: number; wageBill: number; profit: number; investment: number; debt: number; cash: number };
  money: { supply: number; createdThisTurn: number; destroyedThisTurn: number };
  priceIndex: number; wageShare: number; profitShare: number;
  quarter: FinanceFlows;
}

export interface HistoryPoint {
  turn: number; gdpRealAnnual: number; unemployment: number; gini: number;
  emissions: number; budgetShare: number; legitimacy: number; avgWellbeing: number; gpiAnnual: number; debtToGdp: number;
  avgIncomePerPerson: number; medianIncomePerPerson: number; topHouseholdWealthShare: number;
  /** Contract v1.2. */
  climateStress: number; forestHealth: number; commonsHealth: number; farmHealth: number;
  /** Contract v1.4. Optional: absent in a package built before v1.4. */
  govRevenue?: number; govSpending?: number; bankCapitalRatio?: number; renewableShare?: number; gridServed?: number;
}

export interface PolicyEntry { nominal: number | boolean | string; effective: number | null }

/** Summary counts (the tiles are the truth). */
export interface Buildings { publicHousing: number; fossilPlant: number; renewablePlant: number; road: number }

export interface GameCalendar { startYear: number; firstTurn: number; lastTurn: number; turnsPerYear: number }

/** Contract v1.1 (section 12): whether a live turn (beginTurn ... finishTurn) is open, and how many of its 3 months have run. */
export interface TurnPhase { open: boolean; monthsDone: 0 | 1 | 2 | 3 }

/** Contract v1.2 (section 13): climate stress, the drought, the fires and the forecast. */
export type RiskLevel = 'low' | 'medium' | 'high';
export interface Weather {
  /** 0..1. */
  stress: number;
  /** Contract v1.3 (additive, optional: absent in an older package). The stress curve: 0 while the budget share left is above `onset`, 1 at or below `full`, rising along `curve`. */
  onset?: number;
  full?: number;
  curve?: number;
  drought: null | { severity: number; monthsLeft: number };
  fires: { tile: TileRef; monthsLeft: number }[];
  forecast: { fireRisk: RiskLevel; droughtRisk: RiskLevel };
}

export interface Snapshot {
  schemaVersion: SchemaVersion;
  /** Contract v1.1. Absent in a package built before v1.1 (the UI then treats it as closed). */
  turnPhase?: TurnPhase;
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
  plants: PlantView[];
  firms: FirmView[];
  bank: BankView;
  government: GovernmentView;
  nature: NatureView;
  /** Contract v1.2. Absent in a package built before v1.2 (the UI then shows no weather). */
  weather: Weather;
  housing: HousingView;
  /** Contract v1.4. */
  finance: FinanceView;
  energy: EnergyView;
  /** Contract v1.6 (section 16). Optional only because a package built before it lacks them; the UI then says so. */
  breakdowns?: Breakdowns;
  policies?: PolicyBoard;
  industry?: Industry;
  history: HistoryPoint[];
}

export interface RejectedCommand { index: number; command: Command; reason: BlockReason }
export interface PolicyGap { lever: LeverKey; nominal: number; effective: number }

/** Contract v1.1: what beginTurn returns. */
export interface TurnStart {
  turn: number;
  accepted: PendingItem[];
  rejected: RejectedCommand[];
  activated: PendingItem[];
  pcBefore: number;
  snapshot: Snapshot;
}
/** Contract v1.1: what advanceMonth returns. */
export interface MonthResult { month: 0 | 1 | 2; frame: MonthFrame; snapshot: Snapshot }

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

/** Any array may be missing or empty. Commutes are per person (from/to tiles, tiles and minutes; the route is not exposed); purchases are real (household to firm). */
export interface MonthFlows {
  commutes?: { person: number; household: number; firm: number; from: TileRef | null; to: TileRef; tiles: number; minutes: number }[];
  purchases?: { household: number; firm: number; sector: Sector; amount: number }[];
  rent?: { household: number; home: number; amount: number }[];
  wages?: { person: number; household: number; firm: number; amount: number }[];
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
  | { type: 'commuted'; person: number; firm: number; tiles: number; minutes: number }
  | { type: 'built'; kind: InfraKind; tile: TileRef }
  | { type: 'removed'; tile: TileRef }
  | { type: 'electricity_rationed'; served: number }
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
  // contract v1.2 (weather)
  | { type: 'fire_started'; tile: TileRef }
  | { type: 'fire_spread'; from: TileRef; to: TileRef }
  | { type: 'fire_out'; tile: TileRef }
  | { type: 'forest_burned'; tile: TileRef }
  | { type: 'drought_started'; severity: number; months: number }
  | { type: 'drought_ended' }
  | { type: 'home_damaged'; home: number }
);
export type SimEventType = SimEvent['type'];

export type TraceRecord =
  | { type: 'emissions'; source: 'firm' | 'households' | 'sink' | 'plant' | 'fire_sink_loss'; id?: number; tile?: TileRef; tonnes: number }
  | { type: 'income'; household: number; kind: 'wages' | 'dividends' | 'rent' | 'interest' | 'benefits'; amount: number }
  | { type: 'policy_gap'; lever: LeverKey; nominal: number; effective: number; netLobby: number };

/** Contract v1.5 (section 15): the model explorer. */
export interface AssumptionParam { path: string; label: string; value: number | string | boolean; unit: string }
export interface AssumptionExample { label: string; inputs: Record<string, number | string>; result: { value: number; unit: string }; note?: string }
export type AssumptionGroup = 'emissions_energy' | 'people_households' | 'production_prices' | 'nature_climate' | 'weather' | 'government_policy' | 'finance_credit' | 'class_politics' | 'scoring' | 'leaves_out';
export interface AssumptionTopic {
  id: string; title: string; group: AssumptionGroup; summary: string; formula: string;
  params: AssumptionParam[]; examples: AssumptionExample[];
  status: 'placeholder' | 'tuned' | 'rough_real_world'; statusNote: string; leavesOut: string[]; code: string[];
  related: string[]; usedBy: string[];
}

/** Contract v1.7 (section 17): the agents catalogue. */
export type Distribution = 'bernoulli' | 'uniform' | 'uniform_index';
export interface AgentVariant { id: ClassKey; label: string; decision: string; rule: string }
export interface AgentKindInfo {
  id: string; title: string; actor: 'agent' | 'non_agent'; summary: string; decides: string[]; observes: string[]; rule: string;
  params: AssumptionParam[]; variants: AgentVariant[]; classParams: AssumptionParam[]; code: string[];
}
export interface RandomDraw { id: string; event: string; system: string; distribution: Distribution; probability: string; when: string; params: AssumptionParam[]; code: string }
export interface AgentsCatalog { kinds: AgentKindInfo[]; randomness: RandomDraw[]; deterministic: string[] }

/** Contract v1.6 (section 16): breakdowns, policies, industry. */
export interface BreakdownInput { label: string; value: number; unit: string; source?: string }
export interface BreakdownPart { label: string; value: number; unit: string; note?: string }
export interface Breakdown {
  id: string; title: string; formula: string; inputs: BreakdownInput[]; result: { value: number; unit: string };
  parts?: BreakdownPart[]; sumsToResult?: boolean;
}
export interface EmissionsLine { firms: number; households: number; plants: number; grossTotal: number; sink: number; net: number; headline: number }
export interface LegitimacyTerm { id: 'wellbeing' | 'unemployment' | 'inequality' | 'debt' | 'bailout'; label: string; input: number; threshold?: number; weight: number; contribution: number }
export interface TaxRow { id: 'income' | 'profit' | 'capital' | 'wealth' | 'carbon' | 'gst'; label: string; appliesTo: string; rateNominal: number | null; rateEffective: number | null; revenueMonth: number; revenueQuarter: number; modelled: boolean }
export interface Breakdowns {
  gdp: Breakdown; gpi: Breakdown; wageShare: Breakdown; profitShare: Breakdown; debtToGdp: Breakdown;
  gini: Breakdown & { lorenz: { populationShare: number; wealthShare: number }[]; byHousehold: { household: number; people: number; netWorth: number }[] };
  legitimacy: Breakdown & { terms: LegitimacyTerm[]; trend: number };
  capitalRatio: Breakdown;
  emissions: { month: EmissionsLine; turn: EmissionsLine; cumulative: { tonnes: number; budget: number; remaining: number; share: number }; reconciles: boolean; note: string };
  taxes: TaxRow[];
}
export type PolicyValue = number | boolean | string;
export interface PolicyRow {
  lever: LeverKey; label: string; group: string; initial: PolicyValue; active: PolicyValue; effective: number | null;
  pending: { id: number; value: PolicyValue; effectiveTurn: number } | null; lastChangeTurn: number | null; weakenedBy: number | null;
}
export interface PolicyEvent { turn: number; lever: LeverKey; kind: 'queued' | 'activated' | 'cancelled' | 'rejected' | 'weakened'; from: unknown; to: unknown; reason?: string }
export interface PolicyBoard { rows: PolicyRow[]; timeline: PolicyEvent[] }
export interface OwnershipSlice { owner: 'household' | 'workers' | 'state' | 'bank'; household?: number; share: number }
export interface IndustryFirm {
  firm: number; name: string; sector: Sector; output: number; price: number; workers: number; wage: number; revenue: number; profit: number; emissions: number;
  ownership: OwnershipSlice[]; stateShare: number;
}
export interface IndustryInfra { kind: InfraKind; count: number; buildCost: number; upkeepMonthly: number }
export interface IndustryPublicGood { id: string; label: string; spendingMonthly: number; buys: string }
export interface Industry { firms: IndustryFirm[]; infrastructure: IndustryInfra[]; publicGoods: IndustryPublicGood[]; note: string }
