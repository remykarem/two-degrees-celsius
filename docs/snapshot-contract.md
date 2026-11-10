> **Status: PARTLY HISTORICAL.** Sections 1-8 are the v0.1 draft (UI owns the map, cohort households) and are superseded. Sections 9-11 are current, section 11 (v1.0, Town v2) wins; the code (`web/src/contract/index.ts`, `crates/sim-api`) and `docs/fixtures/` are the final authority. Index: `docs/README.md`.

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

## 10. Contract v0.3: alignment with the real simulation

From the independent review of the web app (mock vs the real Rust crates). Where this section differs from sections 3, 4 and 9, **this section wins**. **The real simulation and its `content/` files are the source of truth for numbers; the mock must read them, never invent its own.**

1. **Buildings are derived counts.** `Snapshot.buildings = { publicHousing, fossilPlant, renewablePlant }`, computed by the wire layer from sim capacity (for example `round(fossil_cap / plant_capacity)`; `private_cap` housing is not a building). The UI derives its tile layout from these counts plus the `households` and `firms` lists (homes from cohorts, firms from active slots, bank, government, forest, commons). The UI reconciles its stored layout whenever the counts change: it adds tiles for new buildings and removes tiles for removed ones. The initial layout is no longer hand-authored to match the sim.
2. **Costs come from `preview()` only.** The UI must not gate or label a lever using `baseCostPc` alone. Real PC cost is `base x max(|change| / unit, min_size) x (1 + opposition)`, measured against the latest pending value for that lever. `LeverInfo` therefore carries `costModel: { base: number; unit: number; minSize: number }` for hints, and the UI shows "about N PC" only as an estimate; the authoritative figure is `CommandPreview.pc`.
3. **Units.** `LeverInfo.unit` becomes one of `'usd' | 'usd_per_month' | 'usd_per_tonne' | 'fraction' | 'tonnes_per_month' | 'bool' | 'enum'`. Add `zeroMeansNone?: boolean` (true for `emissions_cap`: 0 means no cap, and the UI must say "no cap", not "strictest"). The real defaults and ranges come from `content/` (for example `min_wage` is dollars per month; `job_guarantee` is a fraction; `public_spending` is dollars per month).
4. **Activation timing.** Pending commands activate at the **start of the next `submitTurn`** (Rust: `activate_due` runs before the ticks). So after turn N, the items that will fire at the start of the next submit have `effectiveTurn == snapshot.turn`, and the UI's "coming next turn" list is exactly those. `TurnResult.activated` lists items that took effect at the start of the turn just played.
5. **Money for builds.** `CommandPreview.money` is what will be charged when the build is paid for in the sim (the wire layer must verify from `Gov.pending_build` and `commands.rs` whether that is at activation or at completion, and document it in `docs/sim-api-notes.md`). Cancelling refunds PC only (50%).
6. **Game over.** The wire layer halts at the first `Outcome` (the Rust run has `stop_on_loss = false`). `Snapshot.gameOver` is set; `submitTurn` after game over returns an error result.
7. **Errors.** Every `SimPort` call that can fail returns `{ ok: true, value } | { ok: false, error: { code: string; message: string } }`. A Rust `Violation` becomes an error result, never a thrown exception. The UI shows a recoverable error screen.
8. **Flows may be empty.** `MonthFrame.flows` arrays may be empty or partial (the Rust side builds them from per-tick data and may start with only `commutes` and `emissions`). The UI must degrade gracefully. Purchases and rent carry no counterparty; the scene picks cosmetic endpoints.
9. **`availableIn`, `lever_unavailable`, ranges.** `availableIn` mirrors the Rust `validate()` rules (for example `campaign_cap` and community commons are not available in Market; enclosure is never player-available). `preview()` returns `invalid_command` for out-of-range values. A second `switch_system` while one is pending or in progress returns `transition_in_progress`.
10. **Collapse outcomes** `mass_unemployment` and `bank_failure_cascade` are real and must be reachable in the mock.
11. **Mock sync.** The mock's numbers (PC start/cap/regen, per-lever costs and lead times, build costs, building capacity per unit, starting policy, transition lengths, carbon budget, calendar) are generated from `content/*.json` by a script (`web`: `npm run sync-mock`), and a test fails if the generated file is stale.
12. **Fixtures.** The Rust wire layer writes sample JSON to `docs/fixtures/` (`snapshot-turn1.json`, `turn-result.json`, `levers.json`, `actions.json`) so the web side can test its types against real output.

## 11. Contract v1.0: Town v2 (breaking; supersedes sections 5 and 7 and any `households` cohort shape)

Approved design: `docs/town-v2.md`. The map is now owned by the simulation. Types below extend the TypeScript sketch in section 4; where they conflict, **this section wins**. The exact wire shapes will be finalised by the `sim-api` agent, with fixtures in `docs/fixtures/`, and any difference recorded in `docs/sim-api-notes.md`.

- **Removed:** cohort-style `HouseholdView` (class, `people` count, per-cohort income) and the "UI owns the layout" rule (section 5). `Snapshot.buildings` counts remain as a summary.
- **New snapshot fields:** `map: { width: 12; height: 12; tiles: Tile[] }`, `people: PersonView[]`, `households: HouseholdView[]` (new shape), `homes: HomeView[]`.
```ts
type TileKind = 'empty' | 'home' | 'public_housing' | 'firm' | 'bank' | 'town_hall'
  | 'fossil_plant' | 'renewable_plant' | 'road' | 'forest' | 'commons';
interface Tile { x: number; y: number; kind: TileKind; ref?: number /* home/firm id */; buildable: boolean }
interface PersonView { id: number; label: string; household: number; ageBand: 'child'|'adult'|'retired';
  skill: number; job: { firm: number } | null; wage: number; commuteTiles: number; commuteMinutes: number; wellbeing: number }
interface HouseholdView { id: number; label: string; members: number[]; home: number | null; class: ClassKey;
  savings: number; debt: number; netWorth: number; income: { wages: number; dividends: number; rent: number; interest: number; benefits: number; taxPaid: number; rentPaid: number; loanPayments: number; spending: number };
  politicalPower: number; arrearsMonths: number }
interface HomeView { id: number; tile: { x: number; y: number }; owner: number /* household id */ | 'public'; household: number | null; rent: number }
```
- **Commands:** `place { kind: 'public_housing'|'fossil_plant'|'renewable_plant'|'road'; tile: {x,y} }` (tile now required and honoured), `remove { tile: {x,y} }`. `BlockReason` gains `tile_occupied`, `tile_invalid`. `CommandPreview` gains `tileValid: boolean` and `tileReason?: string`; the UI may also call a `validTiles(kind): {x,y}[]` helper to shade the grid.
- **Sector:** `'goods' | 'food' | 'services'`. **MetricKey** gains `avg_income`, `median_income`, `top_wealth_share`.
- **Indicators** gain `avgIncomePerPerson`, `medianIncomePerPerson`, `topHouseholdWealthShare`. `HistoryPoint` gains the same three.
- **Events** gain `household_moved{household, to}`, `household_evicted{household}`, `person_hired{person, firm}`, `person_fired{person, firm}`, `purchase{household, firm, amount}`; `commuted` carries `person`.
- **MonthFrame.flows:** `commutes` are per person (with a tile path), `purchases` are real (household to firm).
- **Nature:** each plant tile reports `emissionsMonthly`; `TraceRecord.emissions` source adds `'plant'` with the tile.
- **Versioning:** `schemaVersion` becomes 2. Old saves are refused with a clear message.

## 12. Contract v1.1: live-turn calls for Pressure mode (additive; `schemaVersion` stays 2)

Design: `docs/town-v3.md`. `submitTurn` is unchanged and remains the Relaxed-mode call. Three new calls split it; **run in order they must be bit-identical to `submitTurn`** (same state, events, trace, history, RNG).

```ts
interface SimPort {  // additions
  beginTurn(commands: Command[]): Result<TurnStart>;   // charge + queue commands, activate due items
  advanceMonth(): Result<MonthResult>;                  // run exactly one tick (3 per turn)
  finishTurn(): Result<TurnResult>;                     // policy gaps, PC regen, turn += 1, history point
}
interface TurnStart  { turn: number; accepted: PendingItem[]; rejected: { index: number; command: Command; reason: BlockReason }[];
                       activated: PendingItem[]; pcBefore: number; snapshot: Snapshot }
interface MonthResult { month: 0 | 1 | 2; frame: MonthFrame; snapshot: Snapshot }
// finishTurn returns the same TurnResult as submitTurn (months[] holds the three frames accumulated since beginTurn)
interface Snapshot { /* addition */ turnPhase: { open: boolean; monthsDone: 0 | 1 | 2 | 3 } }
```
- **State machine:** `beginTurn` is legal only when no turn is open (`open: false`); `advanceMonth` only when open and `monthsDone < 3`; `finishTurn` only when open and `monthsDone == 3`. `submitTurn` only when no turn is open. Otherwise `{ ok:false, error:{ code:'turn_state', message } }` and state is unchanged.
- **Halting:** if an outcome occurs during a month, `advanceMonth` returns the month with `snapshot.gameOver` set; further `advanceMonth`/`finishTurn` calls complete the turn bookkeeping if possible, else return `game_over`.
- **Saves:** `exportState` is legal only when no turn is open (a turn boundary); otherwise `turn_state`. `importState` always yields `turnPhase.open: false`.
- **Previews:** `preview(commands)` is legal at any time and costs commands as for the next `beginTurn` (it does not see the turn in progress).
- Fixtures: add `turn-start.json` and `month-result.json` to `docs/fixtures/`.

## 13. Contract v1.2: weather (fires and drought) (additive; `schemaVersion` bumps only if the saved State layout changes)

Design: `docs/town-v4.md`; rules: `docs/causal-model.md` section 11.
```ts
interface Snapshot { /* additions */
  weather: { stress: number /* 0..1 */; drought: null | { severity: number; monthsLeft: number };
             fires: { tile: { x: number; y: number }; monthsLeft: number }[];
             forecast: { fireRisk: 'low'|'medium'|'high'; droughtRisk: 'low'|'medium'|'high' } };
  nature: { /* additions */ forestHealth: number; commonsHealth: number; farmHealth: number /* 0..1 */ };
}
type TileKind = /* existing */ | 'fire_station';
interface Tile { /* addition */ state?: 'standing' | 'burning' | 'regrowing' /* forest tiles only */ }
type LeverKey = /* existing */ | 'resilience_spending';
type InfraKind = /* existing */ | 'fire_station';
type MetricKey = /* existing */ | 'forest_health' | 'commons_health' | 'farm_health' | 'climate_stress';
```
- **Events** (each with `importance`): `fire_started{tile}`, `fire_spread{from,to}`, `fire_out{tile}`, `forest_burned{tile}`, `drought_started{severity,months}`, `drought_ended`, optional `home_damaged{home}`.
- **TraceRecord.emissions** source gains `'fire_sink_loss'` (tonnes of sink lost to fire).
- **HistoryPoint** gains `climateStress`, `forestHealth`, `commonsHealth`, `farmHealth`.
- `placeable` kinds gain `fire_station`; `levers.json` gains `resilience_spending`; the fixtures gain a weather sample.
- If the `State` layout changes for saves, bump `schemaVersion` to 3 and refuse older saves with a clear message.

## 14. Contract v1.4: finance, energy and ledgers (additive; `schemaVersion` bumps to 4 because the saved wrapper blob gains the ledger)

Design: `docs/town-v5.md`. Money on the wire is dollars (as elsewhere).
```ts
type AccountRef =
  | { kind: 'household'; id: number } | { kind: 'firm'; id: number }
  | { kind: 'government' } | { kind: 'bank' } | { kind: 'external'; label: string } // e.g. 'bank credit creation'
type LedgerKind = 'wage'|'dividend'|'rent'|'benefit'|'tax'|'purchase'|'lobby'|'interest'|'loan_new'|'loan_repay'
  | 'loan_writeoff'|'build'|'upkeep'|'subsidy'|'bailout'|'wealth_consumption'|'neutral';
interface LedgerEntry { turn: number; month: 0|1|2; tick: number; from: AccountRef; to: AccountRef; amount: number;
  kind: LedgerKind; detail?: string /* e.g. 'income tax', 'food' */; memo: string; balanceAfter: number /* of the queried account */ }
interface LedgerSummaryRow { turn: number; month: 0|1|2; counterparty: AccountRef; kind: LedgerKind; inflow: number; outflow: number; count: number }
interface LedgerPage { account: AccountRef; openingBalance: number /* at turn 0 */; entries: LedgerEntry[] /* detail window */;
  summary: LedgerSummaryRow[] /* rolled-up older periods */; closingBalance: number; reconciles: boolean; detailFromTurn: number }
interface PersonLedger { person: number; household: number; earnings: LedgerEntry[] /* wages earned, credited to the household */; events: SimEvent[] }
interface SimPort { /* additions */
  ledger(account: AccountRef, query?: { fromTurn?: number; toTurn?: number; kind?: LedgerKind }): Result<LedgerPage>;
  personLedger(person: number): Result<PersonLedger>;
}
interface Snapshot { /* additions */
  finance: {
    government: { revenue: Record<'wageTax'|'profitTax'|'capitalTax'|'wealthTax'|'carbon'|'other', number>;
                  spending: Record<'benefits'|'ubi'|'publicSpending'|'jobGuarantee'|'upkeep'|'subsidy'|'interest'|'builds'|'other', number>;
                  balance: number; debt: number; debtToGdp: number; legitimacy: number; basis: 'month' };
    bank: { equity: number; loans: number; deposits: number; capitalRatio: number; capitalRatioLimit: number; headroom: number; defaultRate: number; interestIncome: number };
    households: { income: Record<'wages'|'dividends'|'rent'|'interest'|'benefits', number>; taxes: number; rentPaid: number; loanPayments: number; spending: number; savingRate: number; debt: number; netWorth: number };
    firms: { revenue: number; wageBill: number; profit: number; investment: number; debt: number; cash: number };
    money: { supply: number; createdThisTurn: number; destroyedThisTurn: number };
    priceIndex: number; wageShare: number; profitShare: number;
  };
  energy: { unit: string /* 'MWh' from config */; supply: { tile: { x: number; y: number }; kind: 'fossil'|'renewable'; capacity: number }[];
            demand: { firms: number; households: number; total: number }; served: number /* 0..1 */; intensity: number /* tCO2 per unit */;
            renewableShare: number; emissionsByPlant: { tile: { x: number; y: number }; tonnes: number }[] };
}
```
- Finance flow values are **monthly** for the latest tick with trailing-quarter totals alongside (`*Quarter` fields added by the wire agent; record in `docs/sim-api-notes.md`).
- `ledger` is legal at any time; entries for the open turn are included. A page's `reconciles` is true iff opening + all entries and roll-ups = closing.
- `HistoryPoint` gains `govRevenue`, `govSpending`, `bankCapitalRatio`, `renewableShare`, `gridServed`.
- Fixtures: add `ledger-household.json`, `ledger-government.json`, `finance.json`, `energy.json`.

## 15. Contract v1.5: the model explorer (additive; queued after v1.4)

Design: `docs/town-v5.md` section 10.
```ts
interface AssumptionParam { path: string /* e.g. 'energy.hh_energy_base_pc' */; label: string; value: number | string | boolean; unit: string }
interface AssumptionExample { label: string; inputs: Record<string, number | string>; result: { value: number; unit: string }; note?: string /* computed by the model's own function */ }
interface AssumptionTopic {
  id: string; title: string;
  group: 'emissions_energy'|'people_households'|'production_prices'|'nature_climate'|'weather'|'government_policy'|'finance_credit'|'class_politics'|'scoring'|'leaves_out';
  summary: string; formula: string; params: AssumptionParam[]; examples: AssumptionExample[];
  status: 'placeholder' | 'tuned' | 'rough_real_world'; statusNote: string; leavesOut: string[]; code: string[] /* Rust file paths, for developers */;
  related: string[] /* topic ids */; usedBy: string[] /* UI anchors: metric keys, lever keys, panel ids */ }
interface SimPort { assumptions(): Result<AssumptionTopic[]> }
```
- Values come from the live `Params`; examples come from the model's functions; both are produced in Rust. The web must not hard-code any number or formula text from the model.
- `usedBy` lets the UI place "How is this calculated?" links (a `MetricKey`, `LeverKey`, or panel id).

## 16. Contract v1.6: breakdowns, policies and industry (additive; queued with v1.5)

Design: `docs/town-v5.md` section 11. All values are computed in Rust; the web never recomputes model math.
```ts
interface BreakdownInput { label: string; value: number; unit: string; source?: string /* metric key or ledger kind */ }
interface Breakdown { id: string; title: string; formula: string; inputs: BreakdownInput[]; result: { value: number; unit: string };
  parts?: { label: string; value: number; unit: string; note?: string }[] /* components that sum to the result */; sumsToResult?: boolean }
interface Snapshot { /* additions */ breakdowns: {
  gdp: Breakdown; gpi: Breakdown; wageShare: Breakdown; profitShare: Breakdown; debtToGdp: Breakdown;
  gini: Breakdown & { lorenz: { populationShare: number; wealthShare: number }[]; byHousehold: { household: number; people: number; netWorth: number }[] };
  legitimacy: Breakdown & { terms: { id: 'wellbeing'|'unemployment'|'inequality'|'debt'|'bailout'; label: string; input: number; threshold?: number; weight: number; contribution: number }[]; trend: number };
  capitalRatio: Breakdown; 
  emissions: { month: EmissionsLine; turn: EmissionsLine; cumulative: { tonnes: number; budget: number; remaining: number; share: number }; reconciles: boolean; note: string };
  taxes: { id: 'income'|'profit'|'capital'|'wealth'|'carbon'|'gst'; label: string; appliesTo: string; rateNominal: number | null; rateEffective: number | null; revenueMonth: number; revenueQuarter: number; modelled: boolean }[];
}; }
interface EmissionsLine { firms: number; households: number; plants: number; grossTotal: number; sink: number /* negative */; net: number; headline: number }
// policies and industry
interface PolicyRow { lever: LeverKey; label: string; group: string; initial: number|boolean|string; active: number|boolean|string;
  effective: number | null; pending: { id: number; value: number|boolean|string; effectiveTurn: number } | null; lastChangeTurn: number | null; weakenedBy: number | null /* share */ }
interface PolicyEvent { turn: number; lever: LeverKey; kind: 'queued'|'activated'|'cancelled'|'rejected'|'weakened'; from: unknown; to: unknown; reason?: string }
interface Snapshot { /* additions */ policies: { rows: PolicyRow[]; timeline: PolicyEvent[] };
  industry: { firms: { firm: number; name: string; sector: 'goods'|'food'|'services'; output: number; price: number; workers: number; wage: number; revenue: number; profit: number; emissions: number;
                       ownership: { owner: 'household'|'workers'|'state'|'bank'; household?: number; share: number }[]; stateShare: number }[];
              infrastructure: { kind: 'road'|'public_housing'|'fossil_plant'|'renewable_plant'|'fire_station'; count: number; buildCost: number; upkeepMonthly: number }[];
              publicGoods: { id: string; label: string; spendingMonthly: number; buys: string }[];
              note: string /* e.g. no state shareholding in current systems */ } }
```
- A label change: the tax currently keyed `tax_wage` is labelled **"Income tax"** (key unchanged). `gst` is listed with `modelled: false`.
- A Rust test asserts every breakdown's `parts` sum to `result` and the emissions `reconciles` flag; State hashes must be unchanged.
