# Ecological Economics Simulation Game: Spec (v0.1)

Working title: *TBD*
Status: draft for discussion. No code written yet.

---

## 1. Concept

A 3D-ish, rotatable, single-town simulation in which the player acts as the town's **planner**. The player places things on the map, sets policy, and can **switch the economic system itself** (for example from market capitalism to cooperatives or state ownership). The town is made of agents who earn, spend, borrow, pollute and change class over time.

**Goal:** keep cumulative emissions under the town's share of the 1.5°C carbon budget, while keeping the town livable (employment, wellbeing, inequality within limits).

**Tone:** critical. The default rules should make visible how profit-driven growth, debt, inequality and political capture push emissions up. Alternative systems should show different trade-offs, not a magic fix.

**Fidelity:** "feels plausible", not predictive. Numbers are tunable placeholders, not calibrated data.

---

## 2. Scope (v1)

| Dimension | Decision |
|---|---|
| World | One town on a tile grid |
| Time | Monthly tick; a run is 40 years = 480 ticks. About 30 min at 1x (about 4 s per tick), 8 to 15 min at 2x to 4x. All tunable. |
| Agents | 10 individually-behaving agents (see section 5) |
| Money | Banks create money by lending (endogenous money) |
| Win/lose | Stay within the carbon budget while meeting livability floors |
| Systems | Switchable economic system (see section 8) |
| Class | Emergent, with mobility (see section 6) |
| Platform | Browser, static site, no backend |

**Out of scope for v1:** multiple towns, trade, real-world calibration, multiplayer, saving to a server, large agent counts.

---

## 3. Vocabulary

| Term | Meaning in this game |
|---|---|
| **Agent** | Something that decides: household, firm, bank, government |
| **Infrastructure** | Placed things that enable or boost others: road, public housing, power plant |
| **Natural capital** | Ecosystem stocks with a regeneration rate and carrying capacity: forest, soil, water, atmosphere |
| **Commons** | A common-pool resource: non-excludable but depletable (a shared forest or fishery) |
| **Stock** | A quantity that accumulates (money, atmospheric CO2, forest biomass) |
| **Flow** | A quantity per month (wages, emissions, regrowth) |
| **Externality** | A cost to others that the actor does not pay (emissions) |
| **Policy lever** | A global setting the player controls (tax, cap, subsidy, interest rate) |
| **Economic system** | A bundle of ownership and distribution rules the player can switch between |
| **Carbon budget** | The total CO2 the town may emit over the run |
| **Class** | Position in production: who owns capital and who sells labour |

---

## 4. Player actions

1. **Place** entities on the map (build cost, upkeep, attributes).
2. **Govern** by setting policy levers.
3. **Switch system** (expensive, slow to take effect, politically contested).
4. **Inspect** any agent or tile to see its attributes, income sources, and recent decisions. Inspection is **read-only**: the player never edits an agent's attributes directly during play. All influence is indirect, through placement, policy and system switching. (A separate sandbox mode could allow direct edits later.)
5. **Rotate** the map in 90° steps (four views), pan and zoom.
6. **Control time**: pause, 1x, 2x, 4x.

---

## 5. Entities

### 5.1 Agents (10 to start)

| # | Agent | Starting class / role |
|---|---|---|
| 1 | Household A | Capitalist (owns the factory) |
| 2 | Household B | Landlord (owns housing) |
| 3 | Household C | Manager / professional |
| 4 | Household D | Worker |
| 5 | Household E | Worker |
| 6 | Household F | Unemployed / dependent |
| 7 | Factory | High output, high emissions |
| 8 | Farm or shop | Lower emissions, a natural-capital user |
| 9 | Bank | Lends, creates money, can fail |
| 10 | Government | Taxes, spends, sets policy as directed by the player |

### 5.2 Infrastructure (not agents)

Road, public housing, power plant (fossil / renewable), school or clinic (wellbeing). Each has a build cost, upkeep, capacity, and effects on agents (commute time, rent, emissions).

### 5.3 Natural capital (not agents)

Forest (CO2 sink and resource), commons (shared resource), atmosphere (CO2 stock), soil/water (optional for v1).

### 5.4 Attributes (limited set per type)

**Household:** `class`, `savings`, `debt`, `assets` (firms, housing owned), `income` split into wages / profit / rent / interest / benefits, `propensity_to_consume`, `carbon_footprint`, `political_power`, `wellbeing`, `employed_at`.

**Firm:** `owner`, `sector`, `revenue`, `costs`, `wage_bill`, `profit`, `employees`, `output`, `emissions`, `co2_intake` (if any), `investment`, `price`, `wage_offered`, `ownership_model`.

**Bank:** `reserves`, `loans`, `deposits`, `interest_rate`, `default_rate`.

**Government:** `tax_revenue`, `spending`, `debt`, `legitimacy`.

**Infrastructure / natural capital:** `emissions`, `co2_intake`, `capacity`, `upkeep`, `regen_rate`, `stock`, `carrying_capacity`.

---

## 6. Class model

Class comes from **relationship to production**, not from wealth. Wealth is an outcome.

| Class | Defined by | Main income |
|---|---|---|
| Capitalist | Owns firms, land or a bank | Profit, rent, interest |
| Manager / professional | Sells skilled labour, may hold some assets | Salary plus some capital income |
| Worker | Owns only their labour | Wages |
| Dependent | No production income | Benefits, charity |

**Mobility rules (v1, simple):**
- A worker with enough savings can **found a firm** (becomes capitalist), with some probability and risk of failure.
- A capitalist whose firm fails and who has debt **falls** to manager or worker.
- A worker laid off becomes **unemployed**, and long-term unemployment lowers skill and wellbeing.
- Class is **recomputed each tick** from income sources and ownership, so it is never assigned by hand after setup.

---

## 7. Capitalist dynamics to model (default system)

1. **Profit motive and growth imperative:** firms that do not grow lose market share.
2. **Wage vs profit split:** the central distribution tension.
3. **Accumulation:** capital income compounds, so inequality widens unless policy intervenes.
4. **Debt and interest:** loans create money and must be repaid with interest, which pushes the economy to keep growing.
5. **Externalities:** firms ignore emissions and commons depletion unless priced or capped.
6. **Reserve army of labour:** unemployment suppresses wages.
7. **Political capture:** wealth buys `political_power`, which is used to lobby against policy (carbon taxes, wealth taxes).
8. **Enclosure:** wealthy agents can buy or privatise the commons.

---

## 8. Switchable economic systems

A system is a **bundle of rule overrides**, not a separate game. Each changes who owns, who decides, and who gets surplus.

| System | Ownership | Surplus goes to | Investment decided by | Expected trade-off |
|---|---|---|---|---|
| **Market capitalism** (default) | Private | Owners | Owners, for profit | Fast growth, high inequality, high emissions |
| **Regulated capitalism / Green New Deal** | Private + rules | Owners, after tax | Owners + state incentives | Lower emissions, political resistance |
| **Worker cooperatives** | Workers | Workers | Workers | Lower inequality, slower capital formation |
| **State ownership of key sectors** | State | Public budget | Planner (player) | Emissions controllable, risk of inefficiency |
| **Degrowth / steady-state** | Mixed | Mixed | Capped by ecological limits | Low emissions, output and jobs pressure |

**Switching costs:** a transition period (several ticks to years), a temporary productivity drop, and resistance from affected classes (political power).

Each system is a configuration object that changes: where profit goes, how firms set wages and investment, which policy levers exist, and what the agents' decision rules are. v1 should ship with **market capitalism plus two alternatives**. Suggested: regulated capitalism and worker cooperatives. Add the rest after the engine is stable.

---

## 9. Metrics (side panel)

| Group | Metrics |
|---|---|
| **Economic** | GDP, money in circulation, unemployment, debt, profit share vs wage share |
| **Inequality** | Gini coefficient, wealth of top vs bottom agent, class mix |
| **Ecological** | CO2 emitted this month, **remaining carbon budget**, atmospheric CO2, forest / commons stock |
| **Social** | Average wellbeing, housing, political stability / legitimacy |
| **Contrast** | **GDP vs a genuine-progress indicator** (GDP adjusted for inequality and ecological damage) |

Show time-series sparklines and a few headline numbers. The contrast metric is the main teaching device.

---

## 10. Win / lose

- **Lose:** the cumulative emissions budget is exhausted, or the town collapses (unemployment or inequality beyond a threshold, bank failure cascade, legitimacy at zero).
- **Win:** reach the end of the run (target year) with the budget intact and livability floors met.
- **Scoring (optional):** a final score combining budget left, wellbeing, and equality, so "win" has degrees.

**Carbon budget approach:** give the town a per-capita share of the remaining 1.5°C global budget, scaled to the town. Display a derived temperature estimate for context. This is more defensible than treating one town as the whole world.

---

## 11. Simulation: the monthly tick

Order matters. Suggested sequence per tick:

1. **Policy and system update** (apply player changes, transition effects)
2. **Labour market:** firms post wages and vacancies, households with no job look, matching happens
3. **Production:** firms produce output based on workers and capital, and emit CO2
4. **Natural capital update:** forest and commons regrow or deplete, atmosphere accumulates emissions, sinks absorb
5. **Goods market:** households buy goods, firms receive revenue
6. **Income distribution:** firms pay wages, rent, interest and profit or dividends according to the active system
7. **Credit:** banks lend, create money, collect repayments, mark defaults
8. **Government:** collect taxes, pay benefits, spend on infrastructure
9. **Agent decisions:** firms invest, hire or fire, set price and wage; households save, consume, move, start firms
10. **Class and political update:** recompute class, `political_power`, lobbying outcomes
11. **Metrics and win/lose checks**
12. **Emit events for animation** (who moved where, what flowed)

**Stock-flow consistency:** money should never appear or vanish except through bank lending and repayment. A running check that total assets equal total liabilities will catch most bugs early.

---

## 12. Visuals and interaction

- **View:** 3D-ish tile map, orthographic or low-perspective camera, **four 90° rotation snaps**.
- **Style:** procedural, low-poly, toy-like geometry (similar in spirit to learn-esp32), no model files.
- **Animations (presentation only, driven by tick events):**
  - households travelling home to work, shops, and back
  - goods moving from factories to shops
  - money flows (wages, rent, loan payments) as small tokens
  - smoke or CO2 particles over emitters, tree or forest health changes
- **Inspector:** click any entity to see attributes, income breakdown and its last decision.
- **Layout:** see section 12.1.

**Rule:** the simulation never depends on the animation. The animation reads from the simulation.

---

### 12.1 UI design

Desktop-first, one screen, no page navigation. The map is always visible.

```
+------------------------------------------------------------------+
| Jan 2031 | ⏸ 1x 2x 4x | Carbon budget [██████░░░░] 61% | System: Market capitalism |
+--------+-----------------------------------------+---------------+
| BUILD  |                                         | METRICS       |
| palette|          3D MAP (rotatable)             | Economy  Eco  |
|        |      [lens: Emissions | Class | Wealth] | GDP     ▁▂▃▅  |
| Homes  |                                         | GPI     ▃▃▂▂  |
| Firms  |                                         | Gini    ▁▂▃▄  |
| Infra  |                       +-------------+   | ...           |
| Nature |                       | INSPECTOR   |   |               |
|        |                       | (selected)  |   |               |
+--------+-----------------------+-------------+---+---------------+
| EVENT LOG / TIMELINE: "Factory laid off 3 workers" · "Carbon tax passed" |
+------------------------------------------------------------------+
```

**Top bar:** date, pause and speed, the **carbon budget gauge** (the most important number in the game, always visible), and the current economic system (click to switch).

**Left rail, build palette:** placeable things grouped as Homes, Firms, Infrastructure, Nature. Select, then click a tile. Shows cost and a short effect preview on hover.

**Right panel, metrics:** tabs for Economy, Ecology and Society. Each metric is a headline number plus a sparkline. GDP and the genuine-progress indicator (GPI) sit side by side. Clicking a metric opens "why is this changing?" (see 13.2).

**Floating inspector:** appears on selecting an agent or tile. Shows attributes, income split (wages / profit / rent / interest), class, and its last few decisions. Read-only.

**Policy drawer:** slides in from the right. Sliders for tax rates, carbon cap and price, interest rate, minimum wage and so on. Each change shows a predicted direction of effect (up/down arrows), not exact numbers.

**System switcher:** a modal comparing the available systems, with costs, transition time and who will resist. Confirmation is required, since switching is expensive.

**Map lenses:** overlays that recolour the map: emissions heat, class colours, wealth, employment, money flows. Lenses are how the player *sees* the invisible economy.

**View bar:** four rotation buttons (also Q / E keys), zoom, and reset, in the style of learn-esp32's view bar.

**Event log / timeline:** a scrollable log of notable events and policy changes, with the carbon-budget runway chart (when will the budget run out at the current rate?).

**End screen:** whether win or lose, shows a short causal story ("emissions rose after the power plant was built because rents pushed workers to commute...") generated from the causal trace (see 13.2).

**Onboarding:** a guided first scenario that pauses at key moments to explain one concept each.

**Accessibility:** colour-blind-safe palettes (never colour alone for class or status), keyboard control for camera and pause, a light theme first and dark later.

## 13. Technical approach (decided: Option B, Rust simulation + TypeScript web front end)

Decided. The **simulation is written in Rust** and runs natively for headless runs and tests, then compiles to **WebAssembly (WASM)** for the browser. The **front end is TypeScript** (Vite, Svelte, Three.js). Chosen because the simulation is the risky part of the project, Rust's enums and type system suit the many data shapes (class, ownership, economic system, commands, events), and Rust-first avoids writing the model twice. (This replaces the earlier Option A, TypeScript throughout.)

| Layer | Choice | Reason |
|---|---|---|
| Simulation language | **Rust** (workspace of small crates, see 13.1) | Enums for the data models, no silent `NaN`/`undefined`, strong typing of money and IDs, deterministic, fast tests |
| Simulation target | Native binary (CLI, tests) and **WASM** (browser) | Same code in both; one golden snapshot checks both |
| Front-end build | **Vite**, static deploy (Vercel) | Matches the reference sites |
| 3D | **Three.js**, procedural low-poly, orthographic camera | Lightweight, no assets to ship |
| UI panels | **Svelte 5** (React is an acceptable alternative) | Metrics, policy and inspector are ordinary reactive UI; see the library table below |
| State | Owned by the simulation; the UI reads snapshots and events | Clean separation |
| Persistence | Save = serialised state JSON in `localStorage` | No backend needed |
| Tests | `cargo test` for the simulation (invariants, sign tests, golden snapshots) | The model is the product |

**Key structural idea:** the simulation is a set of crates with no knowledge of rendering or UI. The front end only sends commands in and reads state snapshots and events out, across a small JSON boundary. See 13.1.

---

**Libraries (we are not building everything from scratch):**

| Need | Use | Notes |
|---|---|---|
| 3D rendering | **Three.js**, used directly (not react-three-fiber) | `InstancedMesh` for people, goods and particles. Built-in `Raycaster` for tile picking. |
| UI framework | **Svelte 5** | Small, reactive. Simulation state lives outside it; panels subscribe to a store. |
| Accessible UI parts | **Bits UI** (Svelte) or Radix (React) | Modals, sliders, tabs, tooltips with keyboard and screen-reader support. |
| Charts | Hand-rolled SVG sparklines + **uPlot** for the carbon runway chart | uPlot is tiny and fast for time series. |
| Icons | **Lucide** | |
| Styling | Plain CSS with variables (as the reference sites do), or Tailwind | Light theme first. |
| Tweening | `@tweenjs/tween.js`, or none | Camera rotation and flow motion are simple. |
| Tests | **Vitest** for front-end logic; **Playwright** later for UI smoke tests | The simulation is tested with `cargo test`. |
| Rust crates | `serde` + `serde_json` (data), `wasm-bindgen` (browser glue), `libm` (identical math on native and WASM), `clap` (CLI), `insta` (snapshot tests), `proptest` (property tests) | Standard choices; each is used by one crate only (see 13.1). |

**Custom (the game-specific parts):** the tile grid and entity meshes, the 90° camera rotation, the map lenses, the flow animations driven by tick events, and the whole simulation.

**Deliberately not used:** a game engine (Phaser, Babylon, Godot web export). They bring their own scene, UI and state models, which would fight the clean simulation boundary in 13.1.

### 13.1 Module architecture (a Rust workspace of small crates)

Modularity rule: **each crate does one job, depends only on the crates above it in this list, and can be tested alone.** Dependencies point one way, so a rule change never touches the engine or the front end.

```
repo/
  Cargo.toml                workspace
  crates/
    sim-types/    data only: IDs, enums, state structs, Command, Event. No logic.
    sim-rng/      seeded PCG32 random number generator. Tiny, no dependencies.
    sim-params/   parameter structs, loading and validation from content/ JSON
    sim-model/    the causal rules: one module per system (see 13.2). Pure functions.
    sim-metrics/  derived indicators (GDP, Gini, GPI, budget, temperature)
    sim-checks/   invariants: money conservation, finite numbers, non-negative stocks
    sim-engine/   tick loop, system ordering, command handling, event and trace collection
    sim-cli/      binary: headless runs, CSV output, scripted scenarios   (milestone 1)
    sim-wasm/     thin wasm-bindgen wrapper for the browser               (milestone 4)
  content/        data, not code: params, economic-system configs, scenarios, causal map (JSON)
  tests/          scenario runs and golden snapshots
  web/            Vite + Svelte + Three.js front end; consumes sim-wasm
```

Only `sim-cli` does file I/O. Only `sim-wasm` knows about the browser. The other crates are pure and have no I/O, clock, threads or global state.

**Where Rust enums carry the data models** (this is why Option B fits):

| Enum | Variants (sketch) | Used for |
|---|---|---|
| `Class` | `Capitalist`, `Rentier`, `Manager`, `Worker`, `Dependent` | Derived class label (S9) |
| `Sector` | `Goods`, `Food` | Firm output type |
| `Owner` | `Household(HouseholdId)`, `Workers`, `State`, `Bank` | Who owns a firm; decides where dividends go (S4) |
| `EconomicSystem` | `Market`, `Regulated`, `Coop`, later `State`, `Degrowth` | Selects rule overrides (section 8) |
| `CommonsAccess` | `Open`, `Enclosed(HouseholdId)`, `Community { cap }` | Commons rule (S7) |
| `Command` | `Place`, `Remove`, `SetPolicy`, `SwitchSystem`, `SetSpeed` | Everything the player can do |
| `Event` | `Commuted`, `Hired`, `Fired`, `LoanDefaulted`, `ForestShrank`, `PolicyBlocked` ... | What the front end animates and logs |
| `Loss` / `Outcome` | `BudgetExhausted`, `Collapse(Reason)`, `Survived` | End-of-run result |

`match` must be exhaustive, so adding a new economic system or event makes the compiler list every place that needs updating.

**The boundary is a small, serialisable API:**

```
init(config, seed)           -> Sim
step(&mut sim, commands)     -> StepOutput { events, trace }
snapshot(&sim)               -> Snapshot        (state view + metrics, JSON)
```

- **Commands in:** place, remove, set policy, switch system, set speed.
- **Events out:** "household 4 commuted to firm 7", "forest stock fell", "bank loan defaulted". The renderer animates these.
- **Trace out:** contribution records that explain *why* a number changed (see 13.2).
- **In the browser:** the simulation state lives inside WASM memory. Each tick the front end receives a JSON snapshot and the events. The data is small, so the cost is negligible.
- A `schema_version` field on the snapshot and on saved games lets old saves be detected.

**Rules for the simulation crates** (enforced by the compiler and by lints where possible):
1. **State is plain data:** structs, `Vec`s, numbers and enums, all `Clone + Serialize + Deserialize`. No `Rc`, `RefCell`, trait objects or references between entities in state.
2. **Typed IDs:** `HouseholdId(u32)`, `FirmId(u32)` and so on. Entities live in `Vec`s and refer to each other by ID, never by reference.
3. **Typed quantities:** money is `Cents(i64)` with checked arithmetic (so conservation checks are exact); tonnes of CO2, rates and shares are separate newtypes over `f64`. The compiler stops you adding dollars to tonnes. No `f32`.
4. **Seeded RNG from `sim-rng`** (PCG32). The `rand` crate and thread-local randomness are not used. Same seed gives the same run.
5. **Deterministic iteration:** `HashMap` and `HashSet` are banned in simulation crates (Clippy `disallowed_types`). Use `Vec` or `BTreeMap`, iterating in ID order.
6. **Systems are pure functions:** `fn system(&State, &Params, &mut Rng) -> Deltas`. They read a snapshot of the state and return changes; the engine applies the changes in a fixed order. No system mutates state another system still needs this tick.
7. **Same math on native and WASM:** use the `libm` crate for `pow`, `exp`, `ln` and trig, so results match bit for bit on both targets.
8. **Numbers stay sane:** after every system the engine runs `sim-checks`: all values finite (no `NaN` or infinity), stocks non-negative, money conserved. A violation stops the run and names the system and tick that caused it. Division by a possibly zero value goes through a helper that makes the zero case explicit.
9. **`#![forbid(unsafe_code)]`** in all simulation crates.
10. **Golden snapshots:** a seeded 480-tick run is stored with `insta`. It runs on native and in WASM and must match exactly. This is the contract that the browser is running the same model as the tests.

### 13.2 The causal model

Yes, it lives in one place: the `sim-model` crate. It is the heart of the game, and it is kept separate from the tick loop (`sim-engine`).

**Structure:**
- **One module per system**, each a small pure function with a single responsibility: `labour-market`, `production`, `emissions`, `natural-capital`, `goods-market`, `income-distribution`, `credit`, `government`, `firm-decisions`, `household-decisions`, `class-update`, `political-power`.
- **The engine runs them in a fixed order** (the 12-step tick in section 11).
- **Parameters are data** in `content/` (for example "propensity to consume by class", "regrowth rate"), so tuning needs no code changes.
- **Economic systems are rule overrides:** the `EconomicSystem` enum selects a parameter set and, where needed, which rule a step uses (for example, a `match` on `Owner` decides where firm profit goes). Adding a system means adding a variant and letting the compiler point to what is missing.

**The causal map as data.** Alongside the code, keep a declarative file listing the intended cause-and-effect links:

```
carbon_price      --(-)-->  firm_emissions        delay: 3 months
firm_profit       --(+)-->  investment
investment        --(+)-->  output  --(+)--> emissions
unemployment      --(-)-->  wages
wealth            --(+)-->  political_power --(-)--> carbon_price
```

This one file is used for three things:
1. **Design:** we discuss and review the loops (reinforcing and balancing) before coding them.
2. **Tests:** check that the implemented model actually shows the intended signs (raise the carbon price in a test run and see emissions fall).
3. **The "why?" explainer:** the UI can show which links are driving a metric.

**Trace.** Each system records *contributions* as it runs (for example, emissions by source, income by type). That lets the UI answer "why did emissions rise this year?" and generate the end-screen story without guessing.

**Status:** a first draft of the equations is in `docs/causal-model.md` (ten systems, the loop table, calibration targets and invariant tests). Parameters are placeholders to be tuned in milestone 1. That document also lists three modelling decisions (D1 to D3) that need confirming.

## 14. Milestones

1. **Headless model (no graphics).** A *headless run* executes the simulation with no map, UI or browser: a command (for example `sim --seed 42`) runs 480 ticks, optionally with scripted player actions ("raise the carbon price in year 5"), and prints or writes a CSV of the key series (GDP, unemployment, Gini, emissions, remaining budget, bank equity).
   - **Why first:** it tests whether the economy behaves sensibly before any graphics exist (no collapse in year 3, no money from nowhere), it is where parameters get tuned toward the targets in `docs/causal-model.md` section 5, and it becomes the test suite (invariants, sign tests, golden snapshot).
   - **Contents:** the Rust workspace (`sim-types`, `sim-rng`, `sim-params`, `sim-checks`, `sim-engine`, `sim-model`, `sim-metrics`, `sim-cli`), then the ten systems added one at a time, checking the numbers after each. Command: `cargo run -p sim-cli -- --seed 42`.
   - **Done when:** a seeded 40-year run prints a summary, invariant tests pass, and the default run lands near the calibration targets.
2. **Natural capital and carbon budget:** emissions, sinks, forest, budget depletion.
3. **Class, mobility and political power.**
4. **`sim-wasm` glue, tile map and static 3D view** with rotation and entity placement. Includes running the golden snapshot in WASM.
5. **Metrics panel, inspector and policy panel.**
6. **Animations** driven by tick events.
7. **Economic system switching** (market, regulated, cooperative).
8. **Balance pass and win/lose tuning.**
9. **Polish:** onboarding, scenarios, save/load, sound.

The main risk is that the model produces boring or degenerate behaviour (everything collapses or nothing happens). Doing milestone 1 first without graphics is how we find out cheaply.

---

## 15. Risks and open questions

**Risks**
- Balancing a model with many interacting loops; it may need many tuning passes.
- Over-engineering the economics before the game is fun.
- The "critical" tone could turn preachy; the alternatives must have real costs.
- Class mobility and system switching multiply the number of rules.
- Rust/WASM friction at milestone 4 (build setup, debugging across the boundary). Mitigated by keeping the boundary small and JSON-based, and by running the golden snapshot in WASM early.

**Open questions**
1. ~~How long should a run take?~~ Decided: 40 years, 480 ticks, about 30 min at 1x.
2. ~~Direct agent editing?~~ Decided: indirect only during play.
3. What does the player see when they lose: a summary screen explaining which loops caused the failure?
4. Should random events (floods, recessions, price shocks) exist?
5. ~~Named agents or abstract labels?~~ Decided: abstract labels.
6. What age range or audience is the game for? That decides how much to explain.

---

## 16. Next steps

1. Review this spec and mark anything wrong or missing.
2. Settle the remaining open questions in section 15.
3. ~~Confirm decisions D1 to D3~~ Confirmed: cohort households, 2 dormant firm slots, closed economy (see `docs/causal-model.md`).
4. Start milestone 1: the headless model as a Rust workspace, following the rules in 13.1. Not started; no code has been written yet.
