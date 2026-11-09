# UI Design v0.1

Companion to `SPEC.md` sections 4.1, 12, 12.1 and 13. The game is turn-based (1 turn = 1 quarter). Desktop-first, one screen, the 3D map always visible.

## Decisions
- **Playback** after End turn lasts about 6 to 10 seconds and is **skippable**.
- **Map** is a **12 x 12** tile grid (144 tiles), rotatable in 90 degree steps.
- **Map ownership:** the UI owns the tile layout; the simulation has no space, so position is cosmetic and the UI says so (`docs/snapshot-contract.md` section 5).
- **Build palette v1:** only the three real commands, public housing, fossil plant and renewable plant. Firms, homes and forest are fixed scenery. Roads, clinics, schools and forest planting are out of scope until the simulation supports them.
- **Predicted effects** show **direction arrows only**, not numeric ranges. Ranges may be added later if playtesting shows the preview is too opaque; they would need a forecast run on a copy of the town and must never be wrong enough to feel like a lie.

## The turn loop (UI phases)

| Phase | What the player sees and can do |
|---|---|
| **1. Review** | A briefing card with 3 to 5 lines on what changed last quarter (including policies weakened by lobbying and items that just took effect). Top bar shows political capital (PC) and the carbon budget gauge. Map is live: select, inspect (read-only), switch lenses, rotate. |
| **2. Plan** | Queue commands. **Policy drawer:** moving a slider creates a *draft* with PC cost, lead time ("takes effect in 2 turns"), direction arrows, and an honest note when lobbying will weaken it ("you set 50, expect about 31 in force"). **Build mode:** pick from the palette, a translucent ghost follows the cursor, a click on a valid tile places it; invalid tiles are shaded with a reason on hover. **Plan tray** lists everything queued with PC and money cost and a cancel button for each. Unaffordable moves are greyed out with the reason. |
| **3. End turn** | One button. Only expensive or irreversible moves (system switch, bailout) ask for confirmation. An empty plan is allowed. |
| **4. Playback** | Controls lock, the plan tray collapses, the quarter plays out as animations (commuting, goods, money tokens, smoke). A ticker shows events as they happen. Skippable. |
| **5. Results** | Metric changes with arrows, ranked events, "blocked or weakened by lobbying", "now in force", "coming next turn". Each number has a *why?* link into the causal trace. Then back to Review. |

**Special moments:** a crisis banner (bank failure, commons collapse); the system-switch comparison modal (costs, transition time, who resists); the end screen with the causal story.

## Screen layout

```
+----------------------------------------------------------------------------+
| Q1 2031 · Turn 1/160 | PC 62/100 | Carbon budget [██████░░░░] 61% | System: Market | [End turn] |
+--------+---------------------------------------------+---------------------+
| BUILD  |                                             | METRICS             |
| palette|            3D MAP (12 x 12, rotatable)     | Economy Eco Society |
| Homes  |       [lens: Emissions | Class | Wealth]   | GDP    ▁▂▃▅         |
| Firms  |                                             | GPI    ▃▃▂▂         |
| Infra  |                       +-------------+       | Gini   ▁▂▃▄         |
| Nature |                       | INSPECTOR   |       | ...                 |
|        |                       +-------------+       |                     |
+--------+---------------------------------------------+---------------------+
| PLAN THIS TURN                                  PC 62 → 38   Money −$40k      |
|  ① Carbon price 0 → 50   2 turns   −14 PC  ✕                  [ End turn ▸ ]  |
|  ② Public housing @ (4,7)  3 turns  −6 PC  ✕                                  |
+----------------------------------------------------------------------------+
| EVENT LOG / PENDING: scheduled items and when they take effect                |
+----------------------------------------------------------------------------+
```

Panels: top bar, build palette, metrics (tabs Economy / Ecology / Society with sparklines; GDP beside GPI), floating inspector, policy drawer (slides in from the right), plan tray, event log with pending list, system switcher modal, view bar (rotate, zoom, reset; Q / E keys), map lenses (emissions, class, wealth, employment, money flows).

## Module layout for `web/`

Strict modularity: each module has one job, and dependencies point one way.

```
web/src/
  contract/    TypeScript types for Snapshot, Command, Event, TurnOutput, plus schema version. No logic, no dependencies.
  sim-port/    one interface: init, preview, submitTurn, snapshot
    mock/        fake sim for UI development
    wasm/        real Rust sim (later). Nothing else knows which one is in use.
  store/       game state: phase (review/plan/playback/results), selection, draft plan, snapshot history. No DOM.
  scene/       Three.js only: grid, camera, entity meshes, lenses, flow animations. No Svelte.
  panels/      Svelte only: top bar, metrics, policy drawer, inspector, plan tray, results. No Three.js.
  ui-kit/      design tokens, buttons, sliders, tooltips
  app/         composition root: wires the pieces and runs the phases
```

**Rules**
1. `scene` and `panels` never import each other; they communicate only through `store`.
2. Only `sim-port` knows whether the mock or the real simulation is running.
3. `contract` depends on nothing, so it can later be generated from the Rust types.
4. The simulation never depends on the animation. Playback reads events from `TurnOutput`.
5. Colour is never the only carrier of meaning (class, status). Keyboard control for camera and End turn.

## Build plan (agents)
0. Design and snapshot contract (this document and `docs/snapshot-contract.md`), done with the user, no agents.
1. **Scaffold** (1 builder): Vite + Svelte + Three.js project, contract types, mock sim, app shell, turn loop. Stop and review.
2. **Parallel build** (2 builders): the 3D scene, and the panels. Stop and review.
3. **Review** (1 independent reviewer).
4. **Later:** WASM glue replaces the mock (after the Phase 2 balancing in `docs/balance-backlog.md`).
Builders take browser screenshots as they go. Never more than 2 agents at once.


---

## Town v2 amendment (approved)

See `docs/town-v2.md`. Changes to this document:
- **Map ownership is withdrawn:** the simulation owns the 12 x 12 grid. The UI renders `tiles` from the snapshot; it no longer stores a layout. Tile validity for placement comes from `preview()`.
- **Palette:** public housing, fossil plant, renewable plant and **road**.
- **Inspector:** a **person view** (job, wage, skill, commute, wellbeing) and a **household view** (members, money, assets, class, home) replace the cohort inspector. A household list makes people easy to find.
- **Metrics:** add average income per person, median income, top-household wealth share; unemployment shown as "N of M".
- **Look:** grass-green ground and soft sky gradient, a raised island plot, dirt paths where there are no roads and asphalt where there are. One figure per person.
- **Animation:** purchases are real flows (household to firm); commutes follow roads.
