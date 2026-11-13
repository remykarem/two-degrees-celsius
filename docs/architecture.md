# Architecture

End to end, as implemented (verified against the code at the time of writing). Design intent is in `docs/town-v2.md`; this file is how the pieces fit.

## 1. Data flow

```
content/*.json ──include_str!──> sim-api (content.rs)
   params.json, initial_town.json,         │
   levers.json, systems/*.json             │ sim-params: Params::from_content (validation, deny_unknown_fields)
                                           v
 ┌──────────────────────── Rust, deterministic, no I/O ─────────────────────────┐
 │ sim-types (State, ids, Cents)   sim-rng (PCG32)   sim-params (Params)        │
 │      │                                                                       │
 │ sim-model: 12 pure systems  fn(&State,&Params,&mut Rng)->Deltas              │
 │ sim-metrics: indicators     sim-checks: invariants after every system        │
 │      │                                                                       │
 │ sim-engine: step() tick loop, apply() deltas, turn layer (PC, pending,       │
 │             lead times), submit_turn_with(), Plan, init + warm-up            │
 │             apply_money() records every money movement as a Transaction and  │
 │             step() reconciles opening + transactions = closing (Town v5)     │
 │      │                                                                       │
 │ sim-api: Api = wire JSON (camelCase), wire validation, frames, history,      │
 │          LEDGER (detail window + roll-ups), finance, energy, save/load,      │
 │          fixtures                 <── docs/snapshot-contract.md (the wire)   │
 └──────┬───────────────────────────────────────────────────────────────────────┘
        │ native: sim-cli (headless), tests          wasm: sim-wasm (WasmSim, JSON strings in/out)
        v
 web/src/sim-port      port.ts (SimPort interface)
   ├─ wasm/   loads pkg/ built by `npm run build:wasm`
   └─ replay/ recorded demo: a typed recording of a real run (recording.generated.ts), used only when wasm is unavailable
        │ (only sim-port knows which one runs: selectSimPort())
        v
 web/src/store         game state machine, drafts, selection, save/load, sceneBus
        │                          ▲
        ├──────────> scene/  (Three.js: map, people, flows, lenses)    reads store, writes sceneBus
        └──────────> panels/ (Svelte: top bar, palette, drawer, ...)   reads store, calls game.*
 web/src/app           composes store + scene + panels (App.svelte, SceneHost.svelte, keys, playback)
```

Boundaries are enforced (see `AGENTS.md`). `web/src/contract/` holds the TypeScript types of the wire and is imported by sim-port, store, scene and panels. `generated.ts` is produced from the Rust wire types in `crates/sim-api` (ts-rs behind the `ts` feature; `npm run gen:contract`; a test fails when it is stale); `index.ts` re-exports it and adds only aliases for names the web uses and the few non-wire types (`Result`, `SimError`, `FrameInput`). Only `index.ts` may import, and only `./generated`.

## 2. Crate responsibilities

| Crate | Owns |
|---|---|
| sim-types | `State` (people, households, homes, firms, bank, gov, nature, map, plants, pending, policy, effective policy), `Command`, `Event`, `TraceRecord`, `Delta`/`Deltas`, `SCHEMA_VERSION` (4 since Town v7), `MAX_FIRMS` (4) |
| sim-params | `Params` (run, turn, labour, production, space, energy, goods, income, credit, invest, nature, government, class, lobby, wellbeing, metrics, initial town, 3 systems). Content is passed in as JSON text; no file I/O |
| sim-model | One module per system; `TICK_ORDER`; `space.rs` (map, commute routes); `util.rs` |
| sim-engine | `init` (builds the town, runs 48 warm-up ticks, then `reset_clock`), `step` (returns events, trace, **transactions** and person earnings, and fails if the ledger does not reconcile), `begin_turn` / `advance_month` / `finish_turn`, `submit_turn_with`, `turn.rs` (validate, preview, queue, activate_due, cancel, regenerate), `apply.rs` (the only place a balance changes; records the transactions), `commands.rs`, `plan.rs` |
| sim-api | `Api::{init, reset, snapshot, levers, actions, preview, valid_tiles, submit_turn, begin_turn, advance_month, finish_turn, cancel_pending, export_state, import_state}`; `keys.rs` (closed enums), `meta.rs` (levers/actions and the `ledger`/`energy` config from `content/levers.json` + params), `events.rs` (importance), `frames.rs`, `history.rs`, `ledger.rs` (transaction store, roll-up, pages), `finance.rs` (finance and energy views), `persist.rs`, `fixtures.rs` |
| sim-wasm | `WasmSim` mirrors `Api` with JSON strings. Replies are `{"ok":true,"value":..}` or `{"ok":false,"error":{"code","message"}}`. Panic hook; panics abort in wasm, so `sim-api` validates input instead |

## 3. Turn flow (`Api::submit_turn` -> `sim_engine::submit_turn_with`)

1. **Guard.** `ensure_running`: after a game over, `submit_turn` and `cancel_pending` return `game_over`.
2. **Wire validation** (`meta.rs::to_sim`): out-of-range values, NaN, wrong types, off-map tiles become `invalid_command` / `tile_invalid` rejections. The engine never sees them (it cannot panic on them).
3. **Backup** of state, RNG and clamp counter. Any failure restores it (`violation` error); a failed turn leaves the game untouched.
4. **Queue** (`turn::queue`, shares `evaluate` with `preview`, so a preview always equals what a submit charges). Commands are costed in order; each sees the earlier ones as queued. Per command: `validate` (system rules, availability, transition in progress), `tile_problem`, PC check, funds check (gov cash + bank lending headroom - already queued builds). Accepted commands get a `PendingCommand` with `effective_turn = turn + lead_time` and are **charged PC now**. Rejected ones carry a `BlockReason`.
   - PC cost = `base_cost x max(|change| / unit, min_size) x (1 + opposition)`, where change is measured against the latest pending value of the same lever, and opposition is `net_lobby` for levers capital opposes. All from `params.turn.levers`.
5. **Activate** (`turn::activate_due`): every pending command with `effective_turn <= turn` is applied now (`commands.rs`), before the ticks. Levers change `policy`; builds are pushed to `gov.pending_build`; a system switch starts a transition. `CommandActivated` events are emitted.
6. **Three ticks** (`params.run.ticks_per_turn`), each `step()` (section 4). The api captures a `MonthFrame` per tick (events + flows). Events from steps 4-5 are added to month 0.
7. **Policy gaps** for the four lobbied levers (nominal vs effective), measured at end of turn.
8. **Regenerate PC**: `pc += pc_base x legitimacy`, capped at `pc_cap` (60 start, 100 cap, 8 base). `turn += 1`.
9. **History**: the api appends one `HistoryPoint` per played turn (kept in `Api`, saved in the blob, not in `State`).
   **Ledger (Town v5)**: each tick's `StepOutput.transactions` is recorded in the `Api`'s `LedgerStore` as the ticks run (on a copy, kept only if the turn succeeds; live turns record each month). The store checks the transactions against the state's balances every tick, keeps full detail for `ledger.detail_turns` turns (8) and rolls older turns up to monthly rows. It lives beside the history in the save blob, never in `State`, so the model cannot read it.
10. **Result**: `TurnResult { accepted, rejected, activated, pcBefore, pcAfter, months[3], policyGaps, trace, snapshot }`.

**Live turn (Town v3, contract section 12).** `submit_turn_with` is `begin_turn` (steps 4-5) + `advance_month` x3 (step 6) + `finish_turn` (steps 7-8) in `sim-engine`; progress is `Sim::turn_phase` (`TurnPhase { open, months_done }`, outside `State` and saves). `Api::{begin_turn, advance_month, finish_turn}` run the same steps one call at a time, keeping the accumulating frames, events and trace in a private `Live` struct, and `finish_turn` returns the same `TurnResult` as `submit_turn` (step 9-10). Out-of-order calls return `turn_state` and change nothing; `submit_turn` and `export_state` are refused while a turn is open; `import_state` and `reset` close it. The split path is tested equal to `submit_turn` in `crates/sim-api/tests/turn_split.rs`.

Money for builds: PC is charged at queueing; build **money** is paid when the build activates and the next government tick (S8) pays it out (`gov.pending_build`). Cancelling a pending item refunds 50% of its PC and no money (none was charged). Capacity/tiles appear on activation: the lead time is the whole delay.

Game over: the first `Outcome` (`budget_exhausted`, or a collapse: `legitimacy_zero`, `mass_unemployment`, `bank_failure_cascade`) is recorded by `check_outcome`. The engine runs with `stop_on_loss = false`, so the rest of that turn still plays; the api then refuses further turns. `gameOver.turn` is derived from the outcome tick; a survived game reports turn 161.

## 4. Tick order (`sim-model::TICK_ORDER`, then engine)

Each system returns `Deltas`; `apply()` applies money operations, then field changes, then events and trace; `sim-checks` then runs (state invariants, money conservation, sector balance, unfunded-payment check when `strict_flows`).

Which checks are independent: the second sector-balance equation (saving = firm need + deficit - bank gain) is implied by the first (sector net positions sum to zero before and after), so it adds a clearer message, not coverage. The independent guards are money conservation in `apply`, the per-account ledger reconciliation (`check_ledger`) and the bank balance sheet identities in `check_state`.

| # | `SystemId` | Module | Doc label | Does |
|---|---|---|---|---|
| 1 | Policy | `policy.rs` | S0/system | Resets per-tick flows; advances an economic-system transition |
| 2 | Labour | `labour.rs` | S1 | Individual hiring/firing, wages, job guarantee |
| 3 | Weather | `weather.rs` | S11 | Town v4: climate stress from the budget remaining, drought, forest fires (ignition, spread, burn, regrow), forest stock loss. Draws from the sim RNG (draw order in the module header). Runs **before** production and nature so they see this tick's drought and burnt forest |
| 3a | Estate | `estate.rs` | S12 | Town v7: the estate's jet flight draw (one `r.chance` a month while an estate exists), the carbon charge on a flight, the pilot's dismissal and rehire. Runs after Weather and before Production so Nature totals the flight. The pilot's wage is paid in Income, the estate's direct emissions and the flight's tonnes are totalled in Nature |
| 4 | Production | `production.rs` | S2 | Output from effective labour (skill x commute-adjusted hours; the farm also loses output in a drought, every firm a little in smoke), electricity demand (up in a drought) and rationing, firm direct emissions |
| 5 | Nature | `nature.rs` | S7 | Emissions by source (firm, households, aviation, plants, sink), forest, commons, cumulative emissions, atmosphere |
| 6 | Goods | `goods.rs` | S3 | Household spending, firm sales, housing market, rent, evictions and moves |
| 7 | Income | `income.rs` | S4 | Wages, dividends by system, rent, interest, taxes on income |
| 8 | Credit | `credit.rs` | S5 | Lending, repayment, defaults, bank failure/bailout |
| 9 | Government | `government.rs` | S8 | Taxes, benefits, public spending, builds (pays `pending_build`), road and fire-station upkeep, `resilience_spending`, legitimacy |
| 10 | Investment | `investment.rs` | S6 | Capital investment, green technology |
| 11 | Class | `class.rs` | S9 | Derived class per household, mobility, founding a firm in the dormant slot |
| 12 | Lobbying | `lobbying.rs` | S9b | Net lobbying; `effective = nominal x (1 - net_lobby)` for carbon price, profit tax, wealth tax, caps |
| 13 | Wellbeing | `wellbeing.rs` | S10 | Per-person wellbeing (incl. commute, unhoused, local pollution, smoke from fires) |

(`TICK_ORDER` has 14 entries; the estate system is row 3a so the numbering of the other rows is unchanged.) Then, in `step()`: `sim_metrics::compute` (indicators), `check_outcome`, a money check, household income by type into the trace, `tick += 1`.

Note: `SPEC.md` section 11 lists a different, older ordering and "S" numbers do not match tick order (Investment S6 runs after Government S8). The table above is what runs. Effective policy computed at the end of a tick is used from the next tick (`deviations.md` #18).

## 5. Data ownership

| Owned by the sim (`State`) | Owned by the UI (store / scene / panels) |
|---|---|
| The map (12 x 12 tiles) and what stands on it; all buildings, roads, plants | Camera angle, zoom, lens, hover, selection |
| People, households, homes, firms, bank, government, nature | The **draft** (commands not yet submitted) and its previews |
| Policy (active), effective policy (after lobbying), pending queue, PC | Which phase the UI is in (briefing, plan, playback, results, end), playback month |
| Outcome, history series (in `Api`) | Layout, formatting, animation, chart scaling, onboarding copy and the wording of wire ids (event sentences in `panels/format.ts`). Model text, thresholds and labels are NOT here: they come from the UI catalog (section 9) |
| Random state (PCG32 `state`, `inc`) | Save file wrapper (`ecotown-save`, saveVersion 2): sim blob + last result + previous snapshot + lens/camera |
| Lever metadata (labels, ranges, arrows) via `content/levers.json` | Nothing about layout: since Town v2 the store derives the map from `snapshot.map` (`layoutOf`) |

The UI must never compute economy numbers, and there is no second implementation of the rules: when wasm is unavailable the web replays a recorded run of the real simulation (`sim-port/replay/`, `docs/how-to.md` section 22), badged "Recorded demo", with the controls that cannot change the outcome turned off.

**Entity registry (contract v2.1).** `content/entities.json` lists the kinds of actor, facility and stock (label, plural, roles, tiles and footprint, owner, scene model ids, inspector sections), validated at load by `crates/sim-api/src/entities.rs` and served in `Catalog.entities`. `web/src/store/entities.ts` is the web's only view of it: the inspector builds its sections from the entity's section list (`panels/inspect.ts`), the scene picks models and labels from it (`scene/entityModel.ts`, `scene/entities.ts`), and the Industry list uses its plurals. `docs/how-to.md` section 21 is the recipe.

**Pressure mode (web, Town v3).** Relaxed mode is the phase machine above (`submitTurn`). In Pressure mode the phases stay `review`/`plan` all game long and the quarter plays live: `store/clock.ts` (pure, injected time source, config in `clock-config.json`) emits `month` and `quarter` events; `store/live.ts` turns them into `game.liveBegin/liveMonth/liveFinish` (`beginTurn` + `advanceMonth` + `finishTurn`) and never waits for the player (the plan is submitted at the boundary per `onTimeout`). `store/scene-live.ts` feeds the scene the clock position inside the current month. Saves hold the last turn boundary, never a turn in progress. Details: `docs/web-notes-v3.md`.

**Finance, ledgers and energy (web, Town v5).** `snapshot.finance` and `snapshot.energy` feed the Finance tab and the energy panel (`panels/finance.ts`, `panels/energy.ts`: pure derivations); transactions come from `SimPort.ledger()` / `personLedger()` through `game.ledger()` (the store is the only caller), and `panels/ledger.ts` holds the viewer logic (rows, filters, reconciliation, roll-up, paging). `GameState.ledgerView` opens the full viewer (`LedgerModal`). The mock books its own invented money as double-entry entries (`sim-port/mock/ledger.ts`); An older wasm package is refused or answers `unsupported` for the calls it lacks; there is no upgrade shim (`sim-port/wasm/upgrade.ts` was removed in Town v5). Shopping trips are planned in `scene/people.ts` (`planShopping`). Details and rough edges: `docs/web-notes-v5.md`.

**Transparency (web, Town v5.2).** `snapshot.breakdowns`, `policies` and `industry` (contract v1.6) and `SimPort.assumptions()` (v1.5) are rendered, never recomputed: `panels/explain.ts` (popover model, units, sums-to check), `emissions.ts`, `lorenz.ts`, `taxes.ts`, `policies.ts`, `industry.ts`, `explorer.ts` (search, groups, anchor to topic), `backing.ts` (the numbers behind every percent). `HowButton` (a "how?" beside a metric) opens `BreakdownModal` (breakdown key in `panels/ui.ts`) or the `ExplorerModal` ("How it works", top bar); both are `Modal`s, so the Pressure clock pauses. `store/labels.ts` relabels the `tax_wage` lever "Income tax". Details: `docs/web-notes-v52.md`.

**Weather (web, Town v4).** Everything is derived from the snapshot (`snapshot.weather`, `nature.*Health`, forest tile `state`): `store/weather.ts` for the panels (health meters, stress gauge, forecast card, fire labels) and `store/scene-weather.ts` for the scene; there is no UI-owned weather state. The mock's weather (`sim-port/mock/weather.ts`) is invented and labelled so, except the stress curve. Details and rough edges: `docs/web-notes-v4.md`.

## 6. Determinism and the golden test

- All state is integer money (`Cents(i64)`) plus f64 quantities; no `HashMap`; `libm` for transcendentals; PCG32 seeded by `Rng::new(seed, 54)`. `serde_json` uses `float_roundtrip` so saves restore every f64 bit.
- `tests/golden/plan.json` (seed 7, scripted turns incl. a cancel, an export and an import) is replayed through the exact JSON strings the browser gets. `tests/golden/seed7.json` records an FNV-1a 64 hash of every reply plus headline numbers.
  - Native: `crates/sim-wasm/tests/golden.rs`. Wasm: `web/tests/wasm-golden.test.ts` (skipped if `web/src/sim-port/wasm/pkg` is not built).
  - Any intended simulation change makes both fail until `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` is run and the diff is understood.
- `tests/scenarios.rs` also has `same_seed_gives_identical_state`, invariants over several seeds, an insta snapshot (`golden_snapshot_seed_42`), and 20-seed baseline tests A-D.
- Seeds matter since Town v4: S11 weather draws from the RNG every tick once climate stress is above zero (`deviations.md` Town v4). Before onset the RNG is used only for commons enclosure and firm founding.

## 7. Save/load and schema versions

| Version | Where | Value | Meaning |
|---|---|---|---|
| `SCHEMA_VERSION` (state) | `sim-types/src/lib.rs` | 5 | Layout of `State`. Town v1 was 1, Town v2 and v3 were 2, Town v4 (weather in `State`) was 3, Town v7 (the estate, `Person.staff_of`, two tile kinds) was 4, the weather's own random stream (`Weather.rng`) is 5 |
| `SCHEMA_VERSION` (wire) | `sim-api/src/lib.rs` (sent as `Snapshot.schemaVersion`, written in the save blob) | 8 | Layout of the wire and of the saved wrapper blob (6 and 7: contract v2.0 and v2.3; 8: the state schema 5 change). Town v5 (ledger in the blob, finance and energy on the snapshot) was 4; Town v5.2 (wrapper log in the blob: per-tick emissions and legitimacy records, policy history) is 5; before v5 it was the same number as the state's |
| Sim blob | `Api::export_state` | `{ schemaVersion, paramsHash, rngState, rngInc, initialPopulation, clampedTotal, state, history, ledger, log }` | Import refuses other versions (`schema_mismatch`), a different params hash (`params_mismatch`), and malformed structure or failed invariants (`bad_state`). A failed import leaves the game untouched |
| `SAVE_VERSION` | `web/src/store/save.ts` | 2 | The browser save file (`format: "ecotown-save"`) around the sim blob. `parseSave` returns plain-language errors |

`paramsHash` is FNV-1a 64 over the canonical params JSON, so **any change to `content/` invalidates existing saves** (`params_mismatch`). The recorded demo carries the `schemaVersion` it was made with (a test pins it to 6). Town v7 left the wire version at 5 (contract v1.9 is additive) and raised the state version to 4; the platform cleanup (contract v2.0, sources as lists) raised the wire version to 6 without changing the saved blob.

## 8. Wrapper log, breakdowns and the explorer (Town v5.1/v5.2)

- **Wrapper log** (`sim-api/src/log.rs`, field `log` of the save blob): one `TickRecord` per monthly tick (emissions by source as the model's trace reported them, the state's own gross and net figures, the legitimacy update) and the policy history (initial values, queued, activated, cancelled, rejected, weakened events). It is written by `Api` around the engine calls and never read by the model. `persist::import` checks it against the saved state (`log.ticks.len() == state.tick`, turns in range, events in order).
- **Breakdowns** (`breakdowns.rs`, `emissions.rs`, `policies.rs`, `industry.rs`) build `snapshot.breakdowns`, `snapshot.policies` and `snapshot.industry` from the same state and the same model/metrics functions as the headline numbers (`sim_metrics::gpi_annual`, `gdp_month`, `debt_to_gdp`, `sim_model::government::legitimacy_step`, ...). The model emits an output-only `TraceRecord::Legitimacy` so the breakdown shows the terms the model actually applied; the wire drops it from the public trace.
- **Explorer** (`assumptions.rs`, `content/assumptions.json`): authored text, parameter paths resolved against the live `Params`, worked examples computed by a registry of small model functions (`production::household_energy_use`, `firm_direct_emissions`, `plant_emissions`, `nature::forest_sink`, `util::tax_on`, ...). Changing a formula in one of those functions changes the example with it.

- **Decision points** (`docs/features/decision-points.md`, stages 1-5): 24 decisions (firm hire, fire, wage, price, expected sales, green step, capex and loan request, payout, exit; household found-a-firm, spend, shop, lobby; bank headroom, lend, spread, dividend, rescue; government fund, cut, legitimacy; estate pilot, flight; nature enclosure) are pure `decide_*` functions in `sim-model` (`labour.rs`, `goods.rs`, `investment.rs`, `income.rs`, `credit.rs`, `class.rs`, `government.rs`, `estate.rs`, `nature.rs`) over views built from `State` and `Params`; `run` builds the view, decides and applies deltas. `sim-model::decisions::DECISIONS` registers them; `sim-api/src/decisions.rs` joins the registry with `content/decisions.json` into `catalog().decisions` and `quantities`, and generates the "Who decides" lines (decides, observes, parameters, rule) of every kind that owns decisions (Lobbyists and Weather have none and carry documented notes from `content/decisions.json`). `sim-model/src/causal.rs` derives the causal graph (nodes, reads, writes and derived edges, source and sink roles, components, `causes_of`) from the registry as pure functions; `sim-api` serves it as `catalog().causal` (contract 2.8). `tests/decision_builders.rs` is the builder gate.

## 9. The thin renderer and the UI catalog (Town v6)

`content/ui.json` holds what the web used to hard-code: UI bands (crisis, health, word bands, community cap, capital "tight" factor), lever group order and blurbs, authored text by group and key (metric hints, finance hints, crisis and inspector copy, weather text, ledger kinds, infra, class and owner labels) and the economic-system cards. `sim-api/src/catalog.rs` resolves it against the live `Params` (system cards read `params.systems.*`); `Api::catalog()`, `WasmSim.catalog()` and `SimPort.catalog()` serve it (contract v1.8, `snapshot-contract.md` section 18). `createGame` loads it once into `web/src/store/catalog.ts`; panels and the scene call `text()`, `bands()`, `wordOf()`. Tests load the real reply from `docs/fixtures/catalog.json` (`web/tests/setup.ts`). Also on the wire since v1.8: `game.ticksPerTurn`, `weather.coverageRadius` and `smokeRadius`, `Tile.fireRisk`, `CommandPreview.expectedEffective`, `breakdowns.emissions.runway`. The mock serves the same catalog (copied from the fixture by `sync-mock`) and builds its lever table from `content/levers.json`; `web/tests/thin-renderer.test.ts` compares the mock's levers, actions and snapshot key sets with `docs/fixtures`.
