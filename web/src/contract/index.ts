// Snapshot contract v0.1 (docs/snapshot-contract.md section 4). Types only: no logic, no imports.

export type LeverKey =
  | 'carbon_price' | 'tax_wage' | 'tax_profit' | 'tax_capital' | 'wealth_tax'
  | 'wealth_threshold' | 'base_rate' | 'min_wage' | 'benefit' | 'ubi'
  | 'green_subsidy' | 'campaign_cap' | 'emissions_cap' | 'commons_harvest'
  | 'commons_access' | 'bank_capital_ratio' | 'bailout' | 'public_spending'
  | 'job_guarantee';
export type InfraKind = 'public_housing' | 'fossil_plant' | 'renewable_plant';
export type SystemKey = 'market' | 'regulated' | 'coop';
export type ClassKey = 'capitalist' | 'rentier' | 'manager' | 'worker' | 'dependent';
export type BlockReason =
  | 'transition_in_progress' | 'nothing_to_remove' | 'same_system'
  | 'lever_unavailable' | 'insufficient_political_capital' | 'insufficient_funds' | 'invalid_command';

export type MetricKey =
  | 'gdp' | 'gpi' | 'unemployment' | 'gini' | 'wage_share' | 'profit_share' | 'wellbeing' | 'emissions'
  | 'budget' | 'temperature' | 'legitimacy' | 'debt' | 'money' | 'prices' | 'housing' | 'forest' | 'commons';

export type CommonsAccessWire = { mode: 'open' } | { mode: 'community'; cap: number };

export type Command =
  | { type: 'set_policy'; lever: LeverKey; value: number | boolean | CommonsAccessWire }
  | { type: 'switch_system'; to: SystemKey }
  | { type: 'place'; kind: InfraKind; count: number; tile?: { x: number; y: number } }
  | { type: 'remove'; kind: InfraKind; count: number };

export interface LeverInfo {
  key: LeverKey;
  label: string;
  unit: 'usd' | 'usd_per_tonne' | 'fraction' | 'tonnes' | 'bool' | 'enum';
  min: number;
  max: number;
  step: number;
  baseCostPc: number;
  leadTime: number;
  lobbied: boolean;
  arrows: { metric: MetricKey; direction: 'up' | 'down' }[];
  availableIn: SystemKey[];
  /** For unit 'enum' levers: the modes in order (min/max/step are 0 and ignored). */
  options?: string[];
}

export type ActionKey = 'switch_system' | `place:${InfraKind}` | `remove:${InfraKind}`;
export interface ActionInfo {
  key: ActionKey;
  label: string;
  baseCostPc: number;
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
}

export interface HouseholdView {
  id: number; label: string; class: ClassKey; people: number;
  employed: number; labourForce: number; savings: number; debt: number; wealthPerCapita: number;
  income: {
    wages: number; dividends: number; rent: number; interest: number; benefits: number;
    taxPaid: number; rentPaid: number; loanPayments: number; spending: number;
  };
  wellbeing: number; politicalPower: number; footprintTonnes: number; foodMet: number; housedShare: number;
}

export type FirmOwner =
  | { kind: 'household'; id: number }
  | { kind: 'workers' }
  | { kind: 'state' }
  | { kind: 'bank' };

export interface FirmView {
  id: number; active: boolean; sector: 'goods' | 'food'; name: string;
  owner: FirmOwner;
  workers: number; wage: number; price: number; output: number; emissions: number;
  revenue: number; profit: number; capital: number; debt: number; greenTech: number;
}

export interface BankView { equity: number; loans: number; deposits: number; defaultRate: number; failed: boolean }
export interface GovernmentView {
  cash: number; debt: number; debtToGdp: number; legitimacy: number; receipts: number; spending: number;
}
export interface NatureView {
  forest: number; commons: number; commonsAccess: 'open' | 'community' | 'enclosed';
  cumulativeTonnes: number; netEmissionsMonthly: number; grossEmissionsMonthly: number; sinkMonthly: number;
}
export interface HousingView {
  privateCap: number; publicCap: number; rentPrivate: number; rentPublic: number; occupancy: number; housedShare: number;
}
export interface EnergyView { fossilCap: number; renewableCap: number; fossilShare: number }

export interface HistoryPoint {
  turn: number; gdpRealAnnual: number; unemployment: number; gini: number;
  emissions: number; budgetShare: number; legitimacy: number; avgWellbeing: number; gpiAnnual: number; debtToGdp: number;
}

export interface PolicyEntry { nominal: number | boolean | string; effective: number | null }

export interface GameCalendar { startYear: number; firstTurn: number; lastTurn: number; turnsPerYear: number }

export interface Snapshot {
  schemaVersion: number;
  game: GameCalendar;
  turn: number;
  tick: number;
  gameOver: null | { turn: number; outcome: GameOutcome };
  pc: { current: number; cap: number; regenNextTurn: number };
  system: { current: SystemKey; transition: null | { to: SystemKey; monthsLeft: number; total: number } };
  pending: PendingItem[];
  policy: Record<LeverKey, PolicyEntry>;
  netLobby: number;
  unionDensity: number;
  indicators: Indicators;
  households: HouseholdView[];
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

export interface MonthFlows {
  commutes: { household: number; firm: number; people: number }[];
  purchases: { household: number; sector: 'goods' | 'food'; amount: number }[];
  rent: { household: number; amount: number }[];
  wages: { household: number; firm: number; amount: number }[];
  emissions: { source: 'firm' | 'households' | 'sink'; id?: number; tonnes: number }[];
}

export interface MonthFrame { month: 0 | 1 | 2; events: SimEvent[]; flows: MonthFlows }

export type CollapseReason = 'legitimacy_zero' | 'mass_unemployment' | 'bank_failure_cascade';
type Importance = { importance: 1 | 2 | 3 };

/** Type + importance + payload. No display text: the UI formats it. */
export type SimEvent = Importance & (
  | { type: 'commuted'; household: number; firm: number; people: number }
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
  | { type: 'emissions'; source: 'firm' | 'households' | 'sink'; id?: number; tonnes: number }
  | { type: 'income'; household: number; kind: 'wages' | 'dividends' | 'rent' | 'interest' | 'benefits'; amount: number }
  | { type: 'policy_gap'; lever: LeverKey; nominal: number; effective: number; netLobby: number };
