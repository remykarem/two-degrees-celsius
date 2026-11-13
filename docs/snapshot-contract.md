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

## 17. Contract v1.7: the agents catalogue (additive)

Design: `docs/town-v5-3.md`. A new call, `agents()` (wasm `WasmSim.agents`, `SimPort.agents()`); `assumptions()` and the snapshot are unchanged. No `SCHEMA_VERSION` bump, no state change (state hashes unchanged). Source: the `agents` section of `content/assumptions.json`, resolved in `crates/sim-api/src/assumptions.rs` against the live `Params` (every `path` must exist, or the load fails).
```ts
type Distribution = 'bernoulli' | 'uniform' | 'uniform_index';
interface AgentVariant { id: ClassKey; label: string; decision: string; rule: string }   // household classes
interface AgentKindInfo { id: string; title: string; actor: 'agent' | 'non_agent'; summary: string; decides: string[]; observes: string[]; rule: string;
  params: AssumptionParam[]; variants: AgentVariant[] /* only households */; classParams: AssumptionParam[]; code: string[] }
interface RandomDraw { id: string; event: string; system: string; distribution: Distribution; probability: string; when: string; params: AssumptionParam[]; code: string }
interface AgentsCatalog { kinds: AgentKindInfo[]; randomness: RandomDraw[]; deterministic: string[] }
interface SimPort { agents(): Result<AgentsCatalog> }
```
- Kinds: households, firms, bank, government, lobbyists (agents); nature, weather (not agents). Market, Regulated and Coop are economic systems, not firm kinds.
- `randomness` lists every RNG draw in `sim-model`. A Rust test (`randomness_table_matches_the_draw_sites_in_the_model`) counts `r.chance(`, `r.next_f64(` and `r.below(` in the source and fails when the table and the code disagree (the `drought_size` entry counts as two draws).
- Param paths may start with `systems.<market|regulated|coop>.` (the economic-system files); the mock resolves them from `content/systems/`.
- Fixture: `docs/fixtures/agents.json`. The mock's catalogue is generated from `content/` by `npm run sync-mock` and adds one line to `deterministic` saying it does not run the rules.

## 18. Contract v1.8: the UI catalog and thin-renderer fields (additive; no `SCHEMA_VERSION` bump, no state change)

Design: `docs/town-v6.md`. The web renders; rules, thresholds and explanatory text live in `content/` and arrive here. All changes are additive, so older readers keep working.

```ts
interface GameCalendar { /* ... */ ticksPerTurn: number }                          // params run.ticks_per_turn (3)
interface Weather { /* ... */ coverageRadius: number; smokeRadius: number }       // params weather.coverage_radius (3), smoke_radius (2), in tiles
interface Tile { /* ... */ fireRisk?: number }                                     // standing forest tiles: odds a fire reaches this tile next month
interface CommandPreview { /* ... */ expectedEffective?: number }                 // carbon_price, tax_profit, wealth_tax, emissions_cap: value in force at today's net lobbying
interface Runway { perTurn: number; runOutTurn?: number; runsOutBeforeEnd: boolean }
interface Breakdowns { emissions: { runway: Runway; /* ... */ } }
interface SimPort { catalog(): Result<Catalog> }                                   // wasm WasmSim.catalog
```
- `Catalog { bands, leverGroups, text, systems }` comes from `content/ui.json`, resolved against the live params (`crates/sim-api/src/catalog.rs`). `bands`: crisis thresholds (`legitimacyBelow`, `budgetBelow`), health bands (`min`, `id`, `word`), stress and severity word bands, `communityCap`, `capitalTightFactor`, `budgetMeter`. `leverGroups`: the order and blurb of the groups that `LeverInfo.group` names. `text`: authored sentences and labels by group and key, with `{name}` placeholders the web fills. `systems`: one card per economic system, with numbers read from `params.systems.*` (transition months, output penalty, `lambda_max`, hurdle, payout, spread, wage floor, green-subsidy floor).
- `Tile.fireRisk` is `sim_model::weather::tile_fire_risk`: the monthly ignition odds shared over the standing tiles (ignition picks one uniformly), combined with the spread odds from each burning orthogonal neighbour. It reads state only and draws no random numbers; it ignores fires that burn out first.
- `CommandPreview.expectedEffective` is `sim_model::lobbying::expected_effective` (`nominal x (1 - net_lobby)`; a cap is divided, and 0 stays "no cap"). A Rust test checks it against the policy gap the lobbying system records. The web states no lobbying claim without it.
- `breakdowns.emissions.runway`: `perTurn` is the latest month's net emissions (not below zero) times `ticksPerTurn`; `runOutTurn` is the turn on which the remaining budget would be used up at that rate.
- Params: `government.bank_cascade_unemployment` (0.2) replaces a literal in the engine's `check_outcome`. The params hash changes, so older saves are refused (`params_mismatch`). The state hash is unchanged.
- Fixture: `docs/fixtures/catalog.json`. The mock serves that real reply (`npm run sync-mock` copies it) and builds its lever table from `content/levers.json`.

## 19. Contract v1.9: the elite estate (Town v7; additive; `schemaVersion` stays 5, the model `State` schema goes 3 to 4)

Design: `docs/town-v7.md` (approved). Everything is additive, so older readers keep working. The wire `schemaVersion` stays 5; the model `State` gained `estate`, `Person.staff_of` and new tile kinds, so `sim_types::SCHEMA_VERSION` is 4 and any save from before Town v7 is refused (state schema check, and `params_mismatch`). The state hash changed (a new household, tiles, the Farm's owner, the draw): goldens were re-blessed on purpose.

```ts
type TileKind = /* ... */ | 'estate_grounds' | 'runway';                           // fixed scenery; place/remove on them is tile_invalid
interface HouseholdView { /* ... */ tag?: string;                                   // catalog.tags.estate_owner ("Estate owner"), the estate household only
                          wealthBand?: { id: string; label: string } }               // every household; from catalog.wealthBands and net worth
interface PersonView { /* ... */ employer?: { kind: 'household'; household: number; label: string } }  // the pilot; `job` stays null
interface Snapshot { /* ... */ estate?: Estate }                                    // absent when the content has no estate
interface Estate {
  household: number; home: number; groundsTiles: TileRef[]; runwayTiles: TileRef[] /* take-off order */; hangarTile: TileRef; jetName: string;
  jet: { value: number; flyingThisMonth: boolean };
  lastFlight?: { turn: number; month: 0 | 1 | 2; tonnes: number }; flightsTotal: number; tonnesTotal: number;   // counted from the start of play (settling flights are not counted)
  flightTonnes: number; householdEmissionsT: number;                                // params estate.flight_tonnes, household_emissions_t
  pilot?: { person: number; wage: number; status: 'employed' | 'let_go'; unpaidMonths: number; unpaidMonthsMax: number };
  finance: { month: EstateMoney; last12: EstateMoney; months: number; wealthTaxOwed: number };            // from the ledger: dividends, wagePaid, wealthTax, carbonCharge (dollars)
}
// events (importance 2, 3 for pilot_let_go and 2 for pilot_hired): { type: 'private_jet_flight', household, tonnes } | { type: 'pilot_let_go', person } | { type: 'pilot_hired', person }
interface MonthFlows { /* ... */ staffCommutes?: { person; household; employer; from: TileRef | null; to: TileRef /* hangar */; tiles; minutes; wage }[] }   // omitted when empty
type EmissionSource = /* ... */ | 'aviation';                                      // frames' flows.emissions, trace and TickRecord
interface EmissionsLine { /* ... */ aviation: number }                              // breakdowns.emissions.month/turn; grossTotal = firms + households + plants + aviation
interface FinanceFlows { householdIncome: { /* ... */ domesticWages: number } }     // the part of `wages` that a household paid to its staff; month and quarter
interface Breakdowns { gini: { byHousehold: { /* ... */ parts: { id: 'net_cash'|'homes'|'jet'|'bank'|'firms'; value: number }[] }[] } }   // zero parts omitted
interface Catalog { /* ... */ tags: Record<string, string>; wealthBands: { min: number | null; id: string; label: string }[] /* richest first; the last has min null */ }
```
- Rules (crates `sim-model` `estate.rs`, `income.rs`, `nature.rs`; see `docs/causal-model.md` section 12): one Bernoulli draw a month while the estate exists (taken even with the pilot gone); a flight needs an employed pilot when the content has one; it adds `flight_tonnes` as `aviation`, and charges `effective.carbon_price x tonnes` from the estate's savings (`Purpose::CarbonTax`, capped by savings). The estate household's direct emissions are `estate.household_emissions_t` (0.5), replacing the per-person 0.12. The pilot is paid `estate.pilot_wage_usd` household to household (`Purpose::DomesticWages`, ledger detail `domestic wages`, kind `wage`); short pay for `pilot_unpaid_months_max` months in a row lets the pilot go (`staff_of` cleared); rehire when the estate's savings cover `pilot_rehire_months_of_wages` months of wages and the pilot has no other job.
- The pilot is `employed()` (`staff_of`), in the labour force, class Worker. `indicators.labourForce` is 11 and `employed` 10 at the start (unemployment 9.1%).
- Ledger: a domestic wage is a household to household `wage` entry, so it appears in both ledgers; `finance.households.income.wages` counts it once as income. `ledger_kind` text and the Finance rows are unchanged; the web shows `domesticWages` as its own row.
- Also in this version: `catalog().text.estate` (group `estate`) holds the wording for the employer line, the three events and the pilot status; `assumptions()` has the topic `private_jet`; `agents()` has the kind `estate` and the draw `estate_flight` (10 draw sites in `sim-model`).
- Fixtures: `docs/fixtures/*` regenerated. The mock carries the same fields from `content/` (`npm run sync-mock`; its flights and dismissal are a labelled copy in `web/src/sim-port/mock/estate.ts`).

## 20. Contract v2.0: sources as lists and a generated TS contract (Platform cleanup; BREAKING; `SCHEMA_VERSION` 5 to 6)

Design: `docs/taxonomy.md` (approved), sections 5.1, 5.2, 7 steps 1, 2 and 6, 8.1 and 8.2. No Town version number: this is the "Platform cleanup" track. No model change: the state hash is unchanged (`cargo run -p sim-cli --example statehash` byte-identical before and after). The saved blob did not change shape; the wire version is bumped because snapshot shapes broke (AGENTS.md rule), so a save made by an older web is refused with `schema_mismatch`. This section supersedes the `EmissionsLine` of section 16 and 19, the `householdIncome` of sections 14 and 19, and `HouseholdView.income` of section 9.

```ts
interface EmissionSourceTotal { id: string; label: string; category: 'industry'|'homes'|'energy'|'aviation'|'waste'; tonnes: number }
interface EmissionsLine {                      // breakdowns.emissions.month / .turn
  sources: EmissionSourceTotal[];              // ids: firms, households, plants, aviation (closed set in Rust); content order
  grossTotal: number; sink: number; net: number; headline: number }   // grossTotal = sum of sources.tonnes; net = grossTotal + sink
interface IncomeSource { id: string; label: string; category: 'labour'|'capital'|'transfer'; partOf?: string; amount: number }
interface HouseholdView { income: { sources: IncomeSource[]; taxPaid; rentPaid; loanPayments; spending } }   // five sources: wages, dividends, rent, interest, benefits
interface FinanceFlows { householdIncome: IncomeSource[] }          // also finance.households.income; adds domestic_wages (partOf wages, not added to a total)
interface SourceLine { /* ... */ category?: EmissionCategory }      // breakdowns.emissions.monthLines; none for the sink and the fire line
interface Catalog { /* ... */ emissionSources: EmissionSourceDef[]; incomeSources: IncomeSourceDef[] }   // id, label, category, showWhenZero / partOf
```
- Where the words live: `content/ui.json` `emission_sources` (`id`, `label`, `category`, `showWhenZero`) and `income_sources` (`id`, `label`, `category`, `partOf`, `views`). `crates/sim-api/src/sources.rs` checks them at load (every model id listed once, no unknown id, a part comes after its parent) and builds the lists; `Meta.sources` carries them. A source equal to 0 is left out of a line unless `showWhenZero` (firms, households and plants always show; aviation shows only when it flew).
- Totals reconcile: `sources` add up to `grossTotal` (`breakdowns.emissions.reconciles` now also checks this, and a Rust test and `web/tests/sources.test.ts` pin it). Income totals are the entries without `partOf`.
- Removed fields: `EmissionsLine.{firms,households,plants,aviation}`, `HouseholdView.income.{wages,dividends,rent,interest,benefits}`, `FinanceFlows.householdIncome.{wages,dividends,rent,interest,benefits,domesticWages}`. The web reads lists by id through `web/src/store/sources.ts`; the per-source named fields are gone from `web/src` (the mock keeps named fields internally and converts at the boundary, `mock/sources.ts`; it is replaced in phase 2).
- Internal types are unchanged (`IncomeFlows`, `EmissionSource`, `TickRecord`): the lists are built at the wire, so nothing hashed or saved moved.
- Vocabulary (display only): the households emission source reads "Homes (heating, transport)", the income source "Rent received".
- Also in this version: the TypeScript contract is **generated** from the Rust wire types (`web/src/contract/generated.ts`, ts-rs behind the `ts` feature of `sim-api`, see `docs/how-to.md` "Regenerate the TS contract"). `web/src/contract/index.ts` is now a thin file of aliases and the few non-wire types. Shape changes of the generator: dropped types and renamed ones are listed in `docs/known-issues.md` (PC-2). Dead wire in the web removed: the aggregate `hired` and `fired` events (never sent by the real wire).
- Fixtures, wasm goldens, `catalog.json` and the mock data were re-blessed (intended). The mock's `schemaVersion` is 6; the wasm loader refuses a package that does not say 6.

## 21. Contract v2.1: the entity registry (Platform cleanup phase 2; ADDITIVE, `SCHEMA_VERSION` stays 6)

Design: `docs/taxonomy.md` section 5.5 (registry v0) and section 7 steps 3 and 4. One new field, `Catalog.entities`, from `content/entities.json`; the state hash is unchanged. `Catalog.text.infra_label` (content/ui.json) is removed: infra labels are the entity labels.

```ts
interface EntityKind {
  id: string;                         // unique: person, estate_owner, pilot, household, estate_household, firm, bank, government, home, estate_house, public_housing,
                                      //   estate_grounds, runway, hangar, jet, road, fossil_plant, renewable_plant, fire_station, forest, commons, empty
  kind: 'actor' | 'facility' | 'stock'; category: 'people'|'firms'|'finance'|'government'|'housing'|'energy'|'transport'|'safety'|'estate'|'nature';
  label: string; plural: string;
  roles: Role[];                      // located owned emitter sink payer payee employer income_source energy_producer energy_consumer hazard_exposed hazard_mitigator buildable lever_target labelled
  tileKinds: TileKindKey[];           // map tile kinds it stands on; each wire tile kind belongs to exactly one entity
  footprint: [number, number][];      // tiles of one instance relative to its anchor; non-empty iff `located`
  infraKind?: InfraKindKey;           // set iff `buildable`; each InfraKind belongs to exactly one entity
  variantOf?: string; group?: string; // the pilot is a variant of person; fossil and renewable plants share group power_plant (attributes.fuel)
  owner: 'none'|'household'|'public'|'state'|'workers'|'bank'|'varies';
  attributes: Record<string, string>;
  sceneModel: Record<string, string[]>; // model ids by variant key (default required); several ids are alternatives picked by tile position
  labels: Record<string, string>;     // wording by state: burning, title_burning, closed ({name}), vacant, occupied
  inspector: InspectorSection[];      // the sections the inspector shows, in order (closed set; the web has one builder per id)
}
```
- Validation at load (`crates/sim-api/src/entities.rs`, tests `crates/sim-api/tests/entities.rs`): unique ids; closed enums for kind, category, role, owner and section; `labelled` on every entity; `located` iff a footprint; `buildable` iff an `infraKind`; every wire `TileKind` and `InfraKind` claimed exactly once; `variantOf` names another entity; an `emissions` section needs the `emitter` role; a `sceneModel` has a `default`. Footprints agree with the initial map (a test counts tiles). Unknown fields are refused.
- Scene model ids are web names; `web/tests/entities.test.ts` checks every id exists in `scene/models.ts`. An id the build does not know draws a plain block (`facility_generic`), so a facility added only in content still shows.
- Not in this version: `Purpose` categories (taxonomy step 5) and hazards as entries.

## 22. Contract v2.2: the net worth statement (Town v8; ADDITIVE, `SCHEMA_VERSION` stays 6)

Design: `docs/town-v8.md`. The state hash is unchanged (`cargo run -p sim-cli --example statehash` identical before and after). Types: `crates/sim-api/src/networth.rs`, generated into `web/src/contract/generated.ts`.

- `breakdowns.gini.byHousehold[].statement: NetWorthStatement` = `{lines, assets, liabilities, netWorth, people, headline, reconciles}`. A line is `{id, label, category: 'asset'|'liability', value, perPerson, count?, howId?}` in dollars; zero lines are omitted; liabilities are positive magnitudes. `headline` is `household_wealth` (the Gini's figure) and `reconciles` is true when `assets - liabilities` equals it up to float rounding (1e-9 of the largest magnitude).
- `breakdowns.gini.byHousehold[].holder`: `households | firm_owners | bank | estate` (priority in that reverse order: estate, then the bank's owner, then owners of or shareholders in a firm, then the rest).
- `breakdowns.gini.town: TownNetWorth` = `{holders: NetWorthHolder[], headline, holdersTotal, reconciles}`; a holder is `{id, label, note, inHeadline, households, people, assets, liabilities, netWorth, howId?}`. The `inHeadline` holders add up to `headline`; `government` (cash minus debt, plus state-owned firms) is a memo row with `inHeadline: false`.
- `catalog.netWorthLines`, `catalog.netWorthHolders`: the definitions from `content/ui.json` (`net_worth_lines`, `net_worth_holders`), validated against the closed id sets `LINE_IDS` and `HOLDER_IDS`. Authored text: `catalog.text.networth`. `howId` is an explorer anchor listed in `usedBy` of the `net_worth` and `net_worth_firms` topics.
- `byHousehold[].parts` (v1.9) is unchanged in shape and now derived from the same lines.
- Fixtures, the wasm golden and the recording were re-blessed (intended: the new fields).

## 23. Contract v2.3: the Markets view (feature `docs/features/markets.md`, scope A; `SCHEMA_VERSION` 7)

The state hash is unchanged (`cargo run -p sim-cli --example statehash` identical before and after, all 18 plan and seed rows). `SCHEMA_VERSION` goes 6 to 7 because the saved wrapper log and `HistoryPoint` grew: a save from schema 6 is refused (`schema_mismatch`). Types: `crates/sim-api/src/markets.rs`, generated into `web/src/contract/generated.ts`.

- **`Snapshot.markets: MarketView[]`**, one card per market of the latest month, in content order; **empty until the first month has been played** (the figures come from that month's records). A `MarketView` is `{id, label, category, mechanisms[], blurb, unit, amount: count|money|energy, demand, supply, traded, unmet, unsold, demandLabel, supplyLabel, tradedLabel, demandText, supplyText, tradedText, gapWords[], gapKind: none|short|glut|both, verdict, priceKind: money|rate|none, priceLabel, priceUnit, price?, priceText?, priceDirection, priceDirectionWord, priceCause, priceCauseWord, priceNote, priceSeries[], drivers[], short[], howId, reconciles}`.
- **The two identities, for every market and every unit:** `demand = traded + unmet` and `supply = traded + unsold`. Goods: demand is what buyers would have bought at each shop's price before its stock limit (`sold + max(wanted - sold, 0)` per shop), supply the shelf stock at the start of the goods step. Labour (people): demand = firm jobs + open postings, supply = people in the labour force (domestic staff excluded), traded = people in a firm job, unsold = jobless (includes the job guarantee). Housing: households against homes, traded = housed households = occupied homes. Credit (dollars): requested against the bank's headroom before the month's decisions, traded = approved. Energy: demand against plant capacity, traded = delivered.
- **Price:** `price` is dollars (a shelf price, a monthly wage, a monthly rent) or, for `priceKind: rate`, an annual fraction. `priceText` is the finished string: a loan rate is written as the dollars a year that $100 of borrowing costs, so no percentage appears on the page. `priceDirection` is `rising | steady | falling | unknown | none`, from the last two points of `priceSeries` (the end of each finished turn from `history`, plus the open month in live mode) against `catalog.markets.steadyBand`. `priceCause` is what the model really lets move this price: `cost` (goods: cost-plus, demand never enters), `slack` (wages), `scarcity` (rent), `risk` (loan rate), `none`.
- **`short[]`** is `{id, label, count, entity?}`; `entity` is `{kind: firm|household|home, id}` for the web to select. `drivers[]` is `{id, label, value}` with the value already written.
- **`reconciles`** is the AND of the two identities and market-specific cross-checks against the end state (traced sales equal each shop's stored `sales_units`; a shop's end inventory equals stock minus sold; firm jobs equal the sum of `workers`; occupied homes equal housed households; approved credit equals `bank.lentThisTick`; delivered energy equals demand times `served`).
- **`HistoryPoint`** gains optional `priceFood`, `priceGoods`, `priceServices`, `wageAvg`, `rentPrivate`, `loanRateAvg`.
- **`catalog.markets: MarketsContent`** (from `content/ui.json` `markets`): `steadyBand`, `gapNone`, `fullyMet`, `directions`, `causes`, `categories[]`, `mechanisms[]` (the four coordination mechanisms: market, rules and planning, firm, community rules, each with a blurb and an example) and `list[]` (the market definitions: label, category, mechanisms, units, price words, verdict sentences, chip and driver sentences, `howId`). Validated at load against the closed id set `MARKET_IDS` and the chip and driver ids the code can produce. `catalog.systems[].coordination` is a sentence on which mechanisms each economic system leans on. Authored text: `catalog.text.markets`.
- **Model side (output only):** `TraceRecord::{GoodsFirm, Vacancies, CreditRoom, CreditDecision, RentBurden}` emitted by `goods`, `labour` and `credit`, kept per tick in the wrapper log (`TickRecord.markets`). The wire `TraceRecord` omits them. Nothing in the model reads them.
- Fixtures, the wasm golden and the recording were re-blessed (intended: the new fields, the schema version and the content text).

## 24. Contract v2.4: market trades (feature `docs/features/market-trades.md`; ADDITIVE, `SCHEMA_VERSION` stays 7)

The state hash is unchanged (`cargo run -p sim-cli --example statehash` identical before and after). The saved ledger gained a `trades` field that defaults to empty, so schema 7 saves from before still load. Types: `crates/sim-api/src/trades.rs`, `markets.rs`; generated into `web/src/contract/generated.ts`.

- **Call:** `marketTrades(market, query?) -> Result<MarketTrades>` (wasm `market_trades(market, queryJson)`; `SimPort.marketTrades`). Legal at any time. `query` is `TradesQuery {turn?, month?, entity?}`: absent means the latest month; a `turn` without `month` is the whole turn; `entity` is a `MarketEntity` and limits the rows to trades that party took part in. An unknown market is `bad_request`. The recorded demo returns `unsupported`.
- **`MarketTrades`:** `{market, label, intro, empty, uncounted, period, periods[], columns[], rows[], totals, aggregate?, reconciles?}`. `period` and `periods[]` are `TradePeriod {turn, month?, rolled, label}` (newest first; a turn as a whole, then its months while the turn is in the detail window; older turns are `rolled: true`, whole only). `columns[]` is `TradeColumn {id, label, help, numeric}`. `rows[]` is `TradeRow {cells[], counted}` with one `TradeCell {text, sort?, entity?}` per column; `entity` is a `MarketEntity {kind: firm|household|home|person|government|bank, id}` the web turns into an inspector link; `sort` is the number to sort by. `totals` is `{count, units, money, text, cells[]}` over the counted rows. `aggregate {traded, text}` and `reconciles` are present only for the latest month with no `entity` filter: the counted rows add up to the card's `traded` (units for goods and credit, rows for labour and housing).
- **Rows per market:** food, goods, services: one purchase per buyer and shop (buyer, seller, units, unit price, total, note, month); labour: one row per job, plus jobs lost and open jobs (not counted) and the guarantee and household staff (not counted); housing: one row per home plus households with no home (not counted); credit: one row per firm that asked (asked, approved, rate, outcome); energy: no columns, no rows.
- **Text** is the content's (`content/ui.json` `markets.trades`, `markets.list[].trades`, `text.markets.trades_*`), validated at load. No percentages: a loan rate is "$4.85 a year per $100".
- **Model side (output only):** `TraceRecord::Trade {sector, buyer, firm, units, unitPrice, amount}` from `goods`; the wire `TraceRecord` omits it. Jobs, homes and loans are read from the state after each tick and the tick's hire/fire events and `CreditDecision` records.
- **Storage:** in the ledger store (not `State`): full rows for the last `ledger.detail_turns` turns, older turns summed per (market, parties, note, turn). Saved with the game.
- New fixtures `docs/fixtures/market-trades-food.json` and `market-trades-labour.json`; golden `catalog` and `export` hashes and the recording re-blessed on purpose.

## 25. Contract v2.4.1: rules moved out of the web (ADDITIVE, `SCHEMA_VERSION` stays 7)

Review finding F8 (thin renderer). The state hash is unchanged.

- `energy.rationed: bool`: true when `served` is below `params.energy.rationed_below` (0.999). Computed by `sim_model::production::is_rationed`, the same rule that fires the `ElectricityRationed` event. The web no longer holds the threshold.
- `game.temperatureLimit: number`: `params.nature.t_limit` (1.5), for the New game text. Years are `lastTurn / turnsPerYear`.
- Catalog `bands.estimateToleranceShare`, `bands.estimateToleranceUsd` (when the table cost and the preview disagree) and `bands.flowMinUsd` (smallest money flow drawn in the flow diagram), from `content/ui.json`.
- Catalog `text.rules.budget_spent`, `text.rules.budget_hint`, `text.rules.start_pitch` (`{years}`, `{limit}`).
- The web reads the budget total from `breakdowns.emissions.cumulative.budget` instead of dividing `remainingBudget` by `budgetShare`.
- `CONTRACT_VERSION` ("2.4.1"; the draft `docs/features/inflation.md` keeps v2.5 for itself) and `SCHEMA_VERSION` are emitted into the generated `web/src/contract/generated.ts`; the wasm loader compares against them (F7).

## 26. SCHEMA_VERSION 8: the weather has its own random stream (review finding F3; no wire shape change except one field)

- `State.weather.rng` (`sim_types::RngStream {state, inc}`), state schema 4 to 5; wire `SCHEMA_VERSION` 7 to 8. Saves from before are refused with `schema_mismatch` ("start a new game"). The recorded demo, fixtures, wasm golden and the insta snapshot were re-blessed on purpose: THE STATE HASH CHANGED.
- Seeding: `Rng::new(run seed, WEATHER_STREAM)` at the end of the warm-up (`sim_engine::init`, `reset_clock`); `WEATHER_STREAM = 0x57454154`. The shared stream is stream 54.
- `AgentsCatalog.randomness[].stream`: `"weather"` or `"shared"` (shown in the Explorer's randomness table).
- Import bound: the 64-bit words of an `rng` object inside the saved state are exempt from the integer bound (`persist::MAX_SAVED_INTEGER`).

## 27. Contract v2.6: the sustainable-yield view and the food cap (feature `docs/features/farm-from-nature.md`; ADDITIVE, `SCHEMA_VERSION` stays 8)

- `Snapshot.natureYield: YieldView[]` (always present; commons first, then forest; `crates/sim-api/src/yields.rs`). A `YieldView` is `{id, title, blurb, formula, headline, overLine, verdict, monthsToCollapse?, rows[], parts[], change, changeUnit, changeLabel, series[], seriesLabels[], howId, access?, reconciles}`.
  - `rows[]` are `{key, label, value?, unit, text}`: stock now, regrowth a month at this stock, best possible regrowth (maximum sustainable yield), taken this month, taken compared with regrown, over the sustainable line, time until collapse at this rate, most food the stock can supply next month, and (commons) the food the town wanted, (forest) the CO2 the forest absorbs. Labels, units and sentences are `content/ui.json` `nature_yield`; the web names no row. `value` is absent when there is no figure (no limit, nothing shrinking). The ratio is worded "X taken for every Y regrown", never a percentage.
  - "At this stock" figures look forward one month: regrowth is what the stock would regrow from its size now (with the drought now), taken is this month's harvest, `parts` = regrowth and minus taken, and they sum to `change` (what happens next month at this rate); `reconciles` checks that and that the food cap is finite and not negative.
  - `series[]` is `{stock, regrowth, taken}` per finished turn from the history, then the month in progress (so it differs from the doc's "monthly series": history is per turn).
  - `access` (commons only): `{id: open|enclosed|community, label, mechanism, mechanismLabel}`, the access rule and its coordination-mechanism tag (open and enclosed are "market", community is "community rules").
- `HistoryPoint` gains optional `commonsStock`, `commonsRegrowth`, `commonsTaken`, `forestStock`, `forestRegrowth`, `forestTaken` (absent in an older hand-built history; no schema bump).
- Markets: `MarketDef`/`MarketView.link?: {label, target}` (the food card links to `panel:ecology`); the food market has a new "who is short" chip `commons_limit` (shown when the stocks left cap next month's food below what the farm made this month; its count is the cap).
- `CONTRACT_VERSION` is "2.6.0". `docs/features/inflation.md` still proposes v2.5 for itself (not built); v2.5 is skipped.
- The model change behind it (no wire type): production caps the food output by last month's stocks (`sim-metrics::food_cap`, params `nature.farm_cap_share_commons` and `nature.farm_cap_share_forest`). No `State` field, no `SCHEMA_VERSION` bump, no new random draw. `content/params.json` changed, so old saves are invalid (`paramsHash`).

## 28. Contract v2.7: decision points (feature `docs/features/decision-points.md`; ADDITIVE, `SCHEMA_VERSION` stays 8)

- `Catalog.decisions: DecisionInfo[]` and `Catalog.quantities: QuantityInfo[]` (`crates/sim-api/src/decisions.rs`, text from `content/decisions.json`, structure from `sim_model::decisions::DECISIONS`).
  - `DecisionInfo` is `{id, title, owner, agentKind, system, decides, rule: {id, label, text}, inputs: QuantityRef[], outputs: QuantityRef[], params: AssumptionParam[]}`. `id` and `owner` are the snake_case `DecisionId` and `OwnerKind`; `agentKind` is the `AgentKindInfo.id` that shows it; `params` carry live values. `QuantityRef` is `{id, label, unit}`; `QuantityInfo` adds `owner` and `summary`.
  - The graph is `inputs -> decision -> outputs`; a quantity one decision writes and another reads links them.
- `AgentKindInfo` (the `agents()` catalogue) is unchanged in shape; for the `firms` kind, `decides`, `observes` and `params` now begin with lines generated from the decisions (hand-written lines for price and investment follow).
- Stage 1 registers three decisions: `firm_hire`, `firm_fire`, `firm_wage` (system `S1 labour`), over 14 quantities.
- `CONTRACT_VERSION` is "2.7.0". Fixtures `catalog.json`, `agents.json`, the golden's `agents` and `catalog` hashes and the recording changed; the state hashes did not.

## 29. Contract v2.8: causal graph and Agents notes (feature `docs/features/decision-points.md` section 15; ADDITIVE, `SCHEMA_VERSION` stays 8)

- `Catalog.causal: CausalGraph` = `{upstreamDepth, roles: CausalRole[], nodes: CausalNode[], edges: CausalEdge[]}` (`crates/sim-api/src/decisions.rs`; structure from `sim_model::causal`, words from `content/decisions.json` `causal`).
  - `CausalNode` is `{id, kind, role, refId, label, owner, selfLoop, component, upstream}`. `id` is `quantity:<QuantityId>` or `decision:<DecisionId>`; `kind` is `quantity` or `decision`; `role` is `source` (no decision writes it), `derived`, `sink` (read only by the decisions that write it) or `decision`; `component` is the weakly connected component, numbered in node order; `upstream` is the quantity node ids that can change this quantity, nearest first, up to `upstreamDepth` decisions back (empty for sources and decisions). Nodes are the quantities in `QuantityId` order, then the decisions in registry order.
  - `CausalEdge` is `{from, to, kind, via, selfLoop}`: `reads` (quantity -> decision), `writes` (decision -> quantity), `drives` (quantity -> quantity, `via` the decision node id). `selfLoop` is true when the decision both reads and writes that quantity. Order: reads and writes per decision in registry order, then the drives edges.
  - `CausalRole` is `{id, label, text}` for the four roles.
- `AgentKindInfo.notes: string[]`: documented exceptions per Agents kind (`content/decisions.json` `kind_notes`). `decides`, `observes`, `params` and `rule` are now generated for every kind that owns registered decisions (`rule` is the decisions' rule texts, one per line); a kind with no decision (lobbyists, weather) has empty `decides` and its own `observes`, `params` and `rule`.
- `CONTRACT_VERSION` is "2.8.0". Fixtures `catalog.json`, `agents.json`, the golden's `agents` and `catalog` hashes and the recording changed; the state hashes did not.

## 30. Contract v2.9: live values and quantity links (feature `docs/features/causality-graph.md` stage 0; ADDITIVE, `SCHEMA_VERSION` stays 8)

- `Snapshot.quantities: QuantityLive[]` = `{id, value, previous?, trend?}` (`crates/sim-api/src/live.rs`): the town-level live value of every graph quantity that has one, in `QuantityId::ALL` order. `id` is the `QuantityId` wire name. `previous` and `trend` (`rising`, `steady`, `falling`; words `catalog().markets.directions`, band `markets.steadyBand`) exist only where the history keeps the series; `previous` is the end of the previous turn (the turn before when no month of this turn has been played).
- `Catalog.quantities[].live: LiveBinding` = `{bound, path?, aggregate?, lever?, hasTrend, unbound?}`. A bound quantity has the snapshot `path` it is read from (`indicators.unemployment`, `firms[].price`, `markets[food].price`), an `aggregate` id (`town`, `sum`, `mean`, `lever`, `flag`) and, if it is a lever setting, the `LeverKey`. An unbound one names its reason (`private`, `computed`, `not_published`, `fixed`, `random`). `Catalog.causal.live = {aggregates, unbound}` (`CausalRole[]`) holds the words (`content/decisions.json` `causal.live`).
- `BreakdownInput.quantityId?`, `MarketView.priceQuantity?`, `MarketDriver.quantityId?`: the catalog quantity an input, a market price or a driver stands for, where the registry has one.
- `CONTRACT_VERSION` is "2.9.0". `content/ui.json` `text.misc.what_changed_*` (the "Why is this changing?" window). Fixtures `catalog.json`, `agents.json`, `snapshot-turn1.json`, `breakdowns.json`, the turn-result and month fixtures, the golden's snapshot, breakdown, catalog and agents hashes and the recording changed; the state hashes did not.

