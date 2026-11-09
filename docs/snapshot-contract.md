# Snapshot Contract v0.1 (draft)

The boundary between the Rust simulation and the web front end. Companion to `docs/ui-design.md` (module `web/src/contract` and `web/src/sim-port`) and `SPEC.md` 13.1. Drafted from the code at commit `5382c4a` (`sim-types`, `sim-engine`, `sim-metrics`).

## 1. What the code gives us today, and what it lacks

**Already good**
- `preview()` and `submit_turn()` share one evaluation routine, so the plan tray's costs always match what a submit charges. `CommandCost` carries PC, money, lead time, effective turn, lobbying opposition and a block reason.
- `PolicyGapSummary` (nominal vs effective) supports "you set 50, expect about 31 in force".
- `Event::Commuted` supports the commuting animation; `TraceRecord` supports "why?" for emissions, income and lobbying gaps.
- Unit newtypes serialise as plain JSON numbers (`#[serde(transparent)]`), and IDs are plain integers.
- `submit_turn_with` already takes a per-tick callback, so per-month frames are possible without changing the model.

**Gaps (each needs Rust work, listed in section 8)**

| # | Gap | Consequence for the UI |
|---|---|---|
| G1 | **There is no map in the simulation.** `Command::Place { kind, count }` has no tile. Housing and energy are aggregate capacities. | The 12 x 12 grid and building positions must be owned by the UI (section 5). |
| G2 | **Only three things are buildable:** `PublicHousing`, `FossilPlant`, `RenewablePlant`. The spec's roads, clinic/school, forest planting, homes and firms are not commands. Firms are founded by class mobility and households are fixed cohorts. | The build palette has 3 items today. See decision B. |
| G3 | `snapshot()` returns the **whole internal `State`** (income history, counters, plan fields). | The UI would depend on internals that will change. Needs a stable view type. |
| G4 | **Events are one flat list per turn**, with no month. | Playback cannot sequence three months. |
| G5 | Rust enums serialise in serde's default tagged form (`{"SetPolicy":{"CarbonPrice":5000}}`); `outcome` is a tuple `(turn, Outcome)`. | Awkward and fragile for TypeScript. Needs a friendlier wire format. |
| G6 | **No lever metadata:** label, unit, min, max, step, and the direction arrows for the effect preview. The params hold `base_cost`, `lead_time`, `unit`, `min_size` only. | Sliders and arrows need a table. |
| G7 | **No per-turn history** in the state (only a CSV writer). | Sparklines and save/load of charts need a history series. |
| G8 | Firms have no display name (households have `label`). | Name from sector and slot, or add a label. |

## 2. Principles
1. **A separate wire layer.** A new crate `sim-api` owns the JSON shapes below and converts from `sim-types`. `sim-types` and the model are not changed to suit the UI.
2. **Plain numbers.** Money in **dollars** (float, converted from integer cents at the boundary; exact bookkeeping stays in Rust). Rates and shares are fractions (0.25 = 25%). Emissions in tonnes. People as numbers.
3. **Friendly tagged unions** (`{ "type": "hired", ... }`), lower snake case keys, no tuples.
4. **Versioned.** Every snapshot and save carries `schemaVersion`.
5. **Read-only to the UI.** The UI sends commands; it never mutates a snapshot.
6. **No logic in the contract.** Types only. The Rust side can generate the TypeScript from the Rust wire types later.

## 3. The port (what the UI calls)

```ts
interface SimPort {
  init(seed: number): Snapshot;
  levers(): LeverInfo[];                              // static metadata (G6)
  preview(commands: Command[]): CommandPreview[];     // costs without changing state
  submitTurn(commands: Command[]): TurnResult;        // validates, queues, runs 3 ticks
  cancelPending(id: number): { refundPc: number; snapshot: Snapshot };
  exportState(): string;                              // opaque blob for saves
  importState(blob: string): Snapshot;
}
```
Two implementations: `mock` (UI development) and `wasm` (the Rust simulation). Nothing else knows which is running.

## 4. Types (TypeScript sketch)

```ts
type LeverKey =
  | 'carbon_price' | 'tax_wage' | 'tax_profit' | 'tax_capital' | 'wealth_tax'
  | 'wealth_threshold' | 'base_rate' | 'min_wage' | 'benefit' | 'ubi'
  | 'green_subsidy' | 'campaign_cap' | 'emissions_cap' | 'commons_harvest'
  | 'commons_access' | 'bank_capital_ratio' | 'bailout' | 'public_spending'
  | 'job_guarantee';
type InfraKind = 'public_housing' | 'fossil_plant' | 'renewable_plant';
type SystemKey = 'market' | 'regulated' | 'coop';
type ClassKey = 'capitalist' | 'rentier' | 'manager' | 'worker' | 'dependent';
type BlockReason = 'transition_in_progress' | 'nothing_to_remove' | 'same_system'
  | 'lever_unavailable' | 'insufficient_political_capital' | 'insufficient_funds' | 'invalid_command';

type Command =
  | { type: 'set_policy'; lever: LeverKey; value: number | boolean | CommonsAccessWire }
  | { type: 'switch_system'; to: SystemKey }
  | { type: 'place'; kind: InfraKind; count: number; tile?: { x: number; y: number } } // tile is UI-owned, ignored by the sim
  | { type: 'remove'; kind: InfraKind; count: number };
type CommonsAccessWire = { mode: 'open' } | { mode: 'community'; cap: number };
// 'enclosed' is never sent by the player; it arises from lobbying.

interface LeverInfo {
  key: LeverKey; label: string; unit: 'usd' | 'usd_per_tonne' | 'fraction' | 'tonnes' | 'bool' | 'enum';
  min: number; max: number; step: number;
  baseCostPc: number; leadTime: number;               // turns
  lobbied: boolean;                                   // may be weakened by lobbying
  arrows: { metric: MetricKey; direction: 'up' | 'down' }[]; // predicted effect, arrows only
  availableIn: SystemKey[];                           // e.g. campaign_cap not in market
}

interface CommandPreview {
  command: Command; pc: number; money: number; leadTime: number; effectiveTurn: number;
  opposition: number;                                 // lobbying share included in pc
  blocked: BlockReason | null;
}

interface Snapshot {
  schemaVersion: number;
  turn: number;                                       // 1-based; 161 = game over
  tick: number;
  gameOver: null | { turn: number; outcome: 'budget_exhausted' | 'survived' | { collapse: 'legitimacy_zero' | 'mass_unemployment' | 'bank_failure_cascade' } };
  pc: { current: number; cap: number; regenNextTurn: number };
  system: { current: SystemKey; transition: null | { to: SystemKey; monthsLeft: number; total: number } };
  pending: PendingItem[];
  policy: Record<LeverKey, { nominal: number | boolean | string; effective: number | null }>; // effective only for lobbied levers
  netLobby: number; unionDensity: number;
  indicators: Indicators;
  households: HouseholdView[];
  firms: FirmView[];
  bank: BankView; government: GovernmentView; nature: NatureView;
  housing: HousingView; energy: EnergyView;
  history: HistoryPoint[];                            // one per completed turn (G7)
}

interface PendingItem {
  id: number; command: Command; submittedTurn: number; effectiveTurn: number;
  pcCost: number; moneyCost: number;
}

interface Indicators {
  gdpAnnual: number; gdpRealAnnual: number; unemployment: number; employed: number; labourForce: number;
  gini: number; wageShare: number; profitShare: number; avgWellbeing: number;
  remainingBudget: number; budgetShare: number;       // share of B0 left, 0..1
  temperature: number; gpiAnnual: number; moneyInCirculation: number; priceIndex: number;
}

interface HouseholdView {
  id: number; label: string; class: ClassKey; people: number;
  employed: number; labourForce: number; savings: number; debt: number; wealthPerCapita: number;
  income: { wages: number; dividends: number; rent: number; interest: number; benefits: number;
            taxPaid: number; rentPaid: number; loanPayments: number; spending: number };  // monthly
  wellbeing: number; politicalPower: number; footprintTonnes: number; foodMet: number; housedShare: number;
}

interface FirmView {
  id: number; active: boolean; sector: 'goods' | 'food'; name: string;     // name derived (G8)
  owner: { kind: 'household'; id: number } | { kind: 'workers' } | { kind: 'state' } | { kind: 'bank' };
  workers: number; wage: number; price: number; output: number; emissions: number;
  revenue: number; profit: number; capital: number; debt: number; greenTech: number;
}
interface BankView { equity: number; loans: number; deposits: number; defaultRate: number; failed: boolean }
interface GovernmentView { cash: number; debt: number; debtToGdp: number; legitimacy: number; receipts: number; spending: number }
interface NatureView {
  forest: number; commons: number; commonsAccess: 'open' | 'community' | 'enclosed';
  cumulativeTonnes: number; netEmissionsMonthly: number; grossEmissionsMonthly: number; sinkMonthly: number;
}
interface HousingView { privateCap: number; publicCap: number; rentPrivate: number; rentPublic: number; occupancy: number; housedShare: number }
interface EnergyView { fossilCap: number; renewableCap: number; fossilShare: number }
interface HistoryPoint { turn: number; gdpRealAnnual: number; unemployment: number; gini: number;
  emissions: number; budgetShare: number; legitimacy: number; avgWellbeing: number; gpiAnnual: number; debtToGdp: number }

interface TurnResult {
  turn: number;
  accepted: PendingItem[];
  rejected: { index: number; command: Command; reason: BlockReason }[];
  activated: PendingItem[];                           // took effect at the start of this turn
  pcBefore: number; pcAfter: number;
  months: MonthFrame[];                               // exactly 3 (G4)
  policyGaps: { lever: LeverKey; nominal: number; effective: number }[];
  trace: TraceRecord[];
  snapshot: Snapshot;                                 // state after the turn
}

interface MonthFrame {
  month: 0 | 1 | 2;
  events: SimEvent[];
  flows: {                                            // drives playback animation
    commutes:  { household: number; firm: number; people: number }[];
    purchases: { household: number; sector: 'goods' | 'food'; amount: number }[];
    rent:      { household: number; amount: number }[];
    wages:     { household: number; firm: number; amount: number }[];
    emissions: { source: 'firm' | 'households' | 'sink'; id?: number; tonnes: number }[];
  };
}
```

**Events (`SimEvent`)** keep the Rust variants, as `{ type: ... }`: `commuted`, `hired`, `fired`, `loan_defaulted`, `firm_founded`, `firm_closed`, `forest_shrank`, `commons_collapsed`, `commons_enclosed`, `policy_blocked`, `command_activated`, `bank_failed`, `rent_shortfall`, `spending_cut`, `bank_bailout`, `class_changed`, `system_switch_started`, `system_switch_completed`, `budget_exhausted`, `collapse`. Each also gets an `importance` (1 to 3) so the results screen can rank them.

**Trace (`TraceRecord`)** keeps the three Rust variants (`emissions`, `income`, `policy_gap`) and is what the "why?" links read.

## 5. The map is owned by the UI (decision A, recommended)

The simulation has no space. So:
- The **layout** (which tile holds which building) lives in the web `store` and in the save file, next to the sim's opaque state blob. It is presentation, like camera position.
- `place` carries an optional `tile`. The sim ignores it; the UI records it and moves on. `remove` carries no tile; the UI chooses which building to take down.
- Starting layout: a fixed `layout.json` that places the starting cohorts' homes, the two firms, the bank and the government, plus forest and commons tiles. When class mobility founds a firm in a dormant slot, the UI picks a free tile.
- Consequence: **position has no gameplay effect** (no commute distance, no local pollution). If we later want it to, the layout would have to move into the sim. That is a deliberate v1 cut, and it should be stated in the UI so players don't expect it.

## 6. Save and load
Save file = `{ schemaVersion, simState: <opaque string from exportState>, layout, uiPrefs }`. A mismatched `schemaVersion` is detected and refused with a clear message.

## 7. Decisions

**Confirmed:** A (the UI owns the layout; position is cosmetic and the UI says so) and B option 1 (v1 ships with the three real build commands; firms, homes and forest are fixed scenery). C (derived names) is accepted as the working default.

Original options, kept for the record:
- **A. Map ownership:** UI-owned layout, position cosmetic (recommended), or a spatial model in the sim (large scope, not for v1).
- **B. Build palette scope for v1:** only `public_housing`, `fossil_plant`, `renewable_plant` are real commands. Options: (1) ship with those three and show firms, homes and forest as fixed scenery; (2) add roads, clinic/school and forest planting to the simulation as new commands, which means new model rules.
- **C. Names:** firm names derived from sector ("Factory", "Farm") and cohort labels as already set, or richer labels. (The spec chose abstract labels, so derived names are fine.)

## 8. Rust work this contract implies (for a later agent)
1. New crate `sim-api`: wire types (serde `tag = "type"`, snake case), conversion from `State`/`TurnOutput`, and TypeScript type generation.
2. Per-month frames: use the existing `on_tick` callback to build `MonthFrame` events and `flows`.
3. Lever metadata table in `content/` (label, unit, min, max, step, arrows, available systems), exposed by `levers()`.
4. Per-turn history series stored in sim state (so save/load keeps the charts).
5. `exportState` / `importState` via serde JSON of `State` plus RNG state, with a schema check.
6. Event importance ranking.
7. `Outcome` as an object, not a tuple.

## 9. Contract v0.2: resolutions to the scaffold's ten issues

Resolved after the web scaffold (`docs/web-notes.md`, "Contract issues"). Where this section differs from sections 3 and 4, **this section wins**. Resolutions were checked against the Rust code (`sim-engine/src/turn.rs`, `sim-types`).

| # | Issue | Resolution |
|---|---|---|
| 1 | `levers()` cannot describe `switch_system` or the place/remove kinds | Keep `levers()` for the 19 policy levers. Add `actions(): ActionInfo[]` for `switch_system`, `place:<kind>` and `remove:<kind>`: `{ key, label, baseCostPc, leadTime, moneyCost (estimated, dollars; 0 if none), availableIn: SystemKey[] }`. `preview()` remains the authority on actual cost. |
| 2 | `MetricKey` undefined | Define it in the contract as the closed set: `gdp, gpi, unemployment, gini, wage_share, profit_share, wellbeing, emissions, budget, temperature, legitimacy, debt, money, prices, housing, forest, commons`. |
| 3 | `SimEvent` and `TraceRecord` payloads unspecified | `SimEvent` = `{ type, importance: 1 \| 2 \| 3, ...payload }` with the payloads of the Rust variants: `commuted{household,firm,people}`, `hired{firm,people}`, `fired{firm,people}`, `loan_defaulted{firm}`, `firm_founded{firm}`, `firm_closed{firm}`, `forest_shrank{stock}`, `commons_collapsed`, `commons_enclosed{owner}`, `policy_blocked{reason}`, `command_activated{id}`, `bank_failed`, `rent_shortfall{household,amount}`, `spending_cut{shortfall}`, `bank_bailout{amount}`, `class_changed{household,to}`, `system_switch_started{to}`, `system_switch_completed{to}`, `budget_exhausted`, `collapse{reason}`. **No display text in the contract**: the UI formats text from the payload (`panels/format`). `TraceRecord` = `emissions{source:'firm'\|'households'\|'sink', id?, tonnes}` \| `income{household, kind:'wages'\|'dividends'\|'rent'\|'interest'\|'benefits', amount}` \| `policy_gap{lever, nominal, effective, netLobby}`. |
| 4 | `commons_access` typed as enum but numeric min/max/step | `LeverInfo` gets `options?: string[]` for `unit: 'enum'` levers (`['open','community']` for commons access; `min/max/step` are 0 and ignored). The wire value is `CommonsAccessWire`; `policy.nominal` for that lever is the mode string. |
| 5 | Meaning of `pcBefore` / `pcAfter` | `pcBefore` = PC when the turn was submitted, before charging commands. `pcAfter` = PC at the start of the next turn, after charging and regeneration. (Matches Rust.) |
| 6 | `regenNextTurn`, `gameOver.turn` | `regenNextTurn` = `pc_base x legitimacy` as a float (not rounded; the UI rounds for display). `gameOver.turn` = the turn on which the game ended (161 when the run was survived). |
| 7 | `policyGaps` numeric only | Intended. Only numeric levers can be lobbied (carbon price, wealth tax, profit tax, emissions cap), so bool and enum levers never appear in `policyGaps`. `LeverInfo.lobbied` is true only for those. |
| 8 | No calendar | `Snapshot.game = { startYear, firstTurn: 1, lastTurn: 160, turnsPerYear: 4 }`. The UI derives quarter and year labels from it and hard-codes nothing. |
| 9 | When money is charged | Charged **at activation**, not at queueing (Rust: `PendingCommand.money_cost` is "the money the command will cost when it takes effect"; pending builds count as committed against available funds). Cancelling refunds **PC only** (50%); no money was charged yet. `CommandPreview.money` is the amount that will be charged on activation. |
| 10 | `bailout` | A **persistent boolean policy toggle** ("the government will bail the bank out if it fails"), as in Rust (`Policy.bailout: bool`). It is not a one-off and must not reset on activation. |
