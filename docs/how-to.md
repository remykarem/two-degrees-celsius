# How-to: recipes for common changes

Each recipe lists the files to touch (found by following a real feature through the code), the tests that will tell you what you missed, and the commands. Paths are from the repo root. **Always finish with the checklist in `AGENTS.md`.** Line numbers drift; search for the names given.

Two rules of thumb: (1) the Rust compiler and `web/tests/*` tell you most of what you forgot, because every closed key set is matched exhaustively or pinned by a test; (2) after any change to `content/` or the sim, three generated files go stale (see section 0).

## 0. What regenerates when

| You changed | Regenerate | Command |
|---|---|---|
| Any number in `content/params.json`, `initial_town.json`, `systems/*.json` | fixtures, golden, the recorded demo | `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`; `cd web && npm run gen:recording` |
| Same, if wire output changes (almost always) | fixtures | `cargo run -p sim-api --example fixtures` |
| Anything that changes sim output | golden | `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` |
| Anything that changes seed-42 market output | insta snapshot | `INSTA_UPDATE=always cargo test -p sim-engine --test scenarios golden_snapshot_seed_42` (unverified: not run) |
| `content/levers.json` | fixtures (`levers.json`, `actions.json`) and golden | as above |
| Rust sim (for the web wasm test) | wasm package | `cd web && npm run build:wasm` |
| Any wire type in `crates/sim-api` (field, type, rename) | generated TS contract | `cd web && npm run gen:contract` (`cargo test --workspace` fails until you do) |
| `content/decisions.json` (quantities, decisions, mechanisms, parameter labels, role and live-binding words) | fixtures (`catalog.json`, `agents.json`), golden (`catalog`, `agents` hashes), the recorded demo | `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice); `cd web && npm run gen:recording` |
| `content/decisions.json` `causal.loops` (loop words, teaching names, DP-13 list) or any signed effect that adds or removes a cycle | the same, and the pinned counts in `crates/sim-model/src/causal_loops.rs` and `crates/sim-api/tests/decisions.rs` | as above; read the loop count change, do not just update it |
| `content/ui.json` (labels, `emission_sources`, `income_sources`) | fixtures (`catalog.json`), golden, the recorded demo | `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice); `cd web && npm run gen:recording` |
| `content/` at all | invalidates old saves (`paramsHash`) | nothing to run; say so in the commit message |

Then run all tests. Baseline tests A-D in `tests/scenarios.rs` (20 seeds each) can fail after a parameter change; that is information about balance, not necessarily a bug. Do not tune to make them pass (see `AGENTS.md`).

## 1. Add a policy lever

Real example to follow: `job_guarantee`; `resilience_spending` (Town v4) is the newest lever and the first that the government pays out monthly (see `government.rs`, the `upkeep` sum). Count of levers: 20 (`LeverKey::ALL`, `LEVER_KEYS` has 31 keys with the place/remove actions).

Rust
- `crates/sim-types/src/enums.rs`: add a `PolicyChange::<Name>(type)` variant, its `key()` string, `value()` arm, and add the key to the lever-name list (the array ending `"job_guarantee", "switch_system", "place_..."`). `crates/sim-types/src/state.rs`: add the field to `Policy` (and to the effective policy if lobbied).
- `crates/sim-params/src/lib.rs`: add the initial-policy field (`job_guarantee: f64` near line 548); `content/initial_town.json` `policy` gets the start value; `content/params.json` `turn.levers.<key>` gets `base_cost`, `lead_time`, `unit`, `min_size` (a missing entry panics on lookup).
- `crates/sim-engine`: `init.rs` (copy initial policy into `State`), `apply.rs::set_policy` (clamp and store), `turn.rs::active_value` (the value PC cost is measured against), `plan.rs::lever` (name for JSON plans).
- The rule that uses it: a system in `crates/sim-model/src/` (job guarantee: `labour.rs`). Read the value via params/state; add the numbers to `params.json`.
- `crates/sim-api`: `keys.rs` (`LeverKey` variant, the `ALL` list, `as_str`, `from_policy`), `command.rs` (wire value -> `PolicyChange`), `snapshot.rs` (the `policy` map near `m.insert(LeverKey::JobGuarantee, ...)`).
- `content/levers.json`: label, unit, min, max, step, `lobbied`, `arrows`, `availableIn` (and `options`/`zeroMeansNone` if needed).

Web
- `web/src/contract/index.ts`: add to the `LeverKey` union (and `LeverInfo` unit if new).
- `web/src/panels/policy.ts`: put the key in a drawer group.

Tests/commands: add a sign or behaviour test in `tests/scenarios.rs` (see `job_guarantee_employs_people_and_lowers_unemployment`); `web/tests/contract.test.ts`, `fixtures.test.ts` will flag missing pieces. Run section 0 regeneration, then the full checklist. Update `docs/causal-model.md` section 10 (the rule) and `docs/glossary.md` if the lever adds a concept.

## 2. Add a buildable kind

Real examples: **road** (`place:road`, `remove:road`) and the **fire station** (`place:fire_station`, `remove:fire_station`, Town v4: its cost and upkeep are in `weather.fire_station_*`, its effect is read by `sim-model/src/weather.rs::covered`; the buildable list is now public housing, fossil plant, renewable plant, road, fire station). Needs tile rules, costs, lever table, wire, palette, scene mesh.

Rust
- `crates/sim-types/src/enums.rs`: `InfraKind` and `TileKind` variants; `place_<kind>` / `remove_<kind>` names in the lever-name list and `lever_key()` mapping (`InfraKind::Road => "place_road"`).
- Costs: `content/params.json` `government.<kind>_cost_usd` and upkeep (`road_cost_usd`, `road_upkeep_usd`), mirrored by fields in `crates/sim-params/src/lib.rs`; `turn.levers.place_<kind>` and `remove_<kind>` (PC cost, lead time).
- Tile rules: `crates/sim-engine/src/turn.rs::tile_problem` (what tiles are buildable; today forest, commons and off-map are `TileInvalid`, anything else non-empty is `TileOccupied`), `build_cost`.
- Effect on activation and construction: `crates/sim-engine/src/commands.rs` (pushes to `gov.pending_build`), `crates/sim-model/src/government.rs` (build and upkeep, near the `InfraKind::Road` arm). Gameplay effect: `crates/sim-model/src/space.rs` (commute routes use roads).
- `crates/sim-api`: `keys.rs` (`InfraKind`, `TileKindKey`, `ALL`), `snapshot.rs::buildings` (summary count), `meta.rs` (`place:<kind>` money cost; needs `content/levers.json` action entries with `tileRule`).

Registry and content (since the entity registry, contract v2.1)
- `content/entities.json`: one entry for the kind (labels, roles, tiles, `infraKind`, scene model id, inspector sections). `crates/sim-api/src/entities.rs` (`ALL_TILE_KINDS`) and `industry.rs` (infra key): `cargo test -p sim-api --test entities` fails until every tile and infra kind is claimed once.
- `content/ui.json` for any label or text, `content/assumptions.json` for the "why" explainer (section 15).

Web (mostly nothing: labels, palette entries and inspector sections come from the registry and `actions()`)
- Types are generated: run `cd web && npm run gen:contract` (section 19); do not edit `contract/generated.ts` by hand.
- Optional: a mesh in `scene/models.ts` (section 11; until then it draws as a plain block). Roads have their own `scene/roads.ts` and ghost handling in `scene/index.ts`.
- Only if the kind needs a new tile-to-selection route: `store/map.ts` (known-issues PC-9).

Tests: roads tests in `tests/scenarios.rs` (`roads_are_built_on_their_tile_cost_money_and_need_upkeep`, `a_road_shortens_a_commute_...`), `tests/golden/plan.json` (add the kind to the golden plan only if you want it covered), `scene-roads.test.ts`. Regenerate per section 0. Update `docs/snapshot-contract.md` section 11 if the wire changed.

### 2a. Worked plan: a new public facility (for example a sanitation facility)

Overall idea: build in layers from the inside out and check each before the next. Keep the **rule** bespoke and everything else data-driven.

1. **Decide the effect first, in one sentence.** "Part of the whole system" means it must move at least one existing quantity (wellbeing, a nature stock, money, emissions, a hazard). Name that quantity and the direction (add a sign test, `docs/causal-model.md` section 5).
2. **Content and types:** `content/params.json` (cost, upkeep, lead time, effect size, `turn.levers.place_<kind>` and `remove_<kind>`), `content/entities.json`, `content/levers.json`, `content/ui.json`; `enums.rs`, `sim-params`, `sim-api/src/{keys,entities,industry,meta}.rs`, `sim-engine/src/turn.rs` (section 2 and 21 above). Run `cargo test --workspace`; the registry test tells you what you missed.
3. **The rule:** one pure function in the right `crates/sim-model/src/<system>.rs`, reading the facility count or coverage from `State` (pattern: `weather.rs::covered` for the fire station). Numbers in `params.json`, a unit test and a sign test. If it emits, add an emission source (section 20); if it costs money each month, add it to the government upkeep sum; if it adds a new kind of money movement, section 12a.
4. **Explain it:** an assumptions topic (section 15) and, if it feeds a headline metric, a breakdown (section 14), so the player can see why.
5. **Regenerate in this order:** `cd web && npm run gen:contract`; `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`; `npm run build:wasm`; `npm run gen:recording` (section 0). The state hash changes the moment a rule is added; that is expected.
6. **Play it, then run the baselines** (A to F, 20 seeds): a new rule shifts the economy; log changes in `docs/tuning-log.md` and `docs/balance-backlog.md`.
7. **Web:** usually nothing. Add a mesh (section 11) when you want it to look right, and check the inspector, the palette and the Industry or Society views in the real sim.

## 3. Add an event type

Real example: `electricity_rationed` (raised in `sim-model/src/production.rs`).

1. `crates/sim-types/src/enums.rs`: add a variant to `Event` (the Town v4 weather events, `FireStarted` etc., are recent examples).
2. Emit it from a system: `d.events.push(Event::...)` in the relevant `crates/sim-model/src/<system>.rs`.
3. `crates/sim-api/src/events.rs`: add the `SimEventKind` variant (wire shape, snake_case tag, camelCase fields), the `from_sim` arm, and its **importance** (3 game-shaping, 2 notable, 1 routine) in `importance()`. The match is exhaustive, so the compiler points here.
4. `crates/sim-api/tests/wire.rs`: add it to the list of every kind (near `K::ElectricityRationed`).
5. `web/src/contract/index.ts`: add to the `SimEvent` union.
6. `web/src/panels/format.ts::formatEvent`: the text (the contract carries no display text). Tests: `web/tests/panels-v2.test.ts` (see the `electricity_rationed` case). Optionally `scene/flows.ts` (event-driven effects/labels).
7. Regenerate fixtures and golden; update `docs/snapshot-contract.md` section 11 events list.

## 4. Add a metric / indicator

Real example: `median_income` / `top_wealth_share` (Town v2).

1. `crates/sim-types/src/state.rs`: add to `Indicators` (e.g. `median_income_pp`). `crates/sim-metrics/src/lib.rs::compute`: compute it (pure; `libm` for math). Unit test in that crate.
2. `crates/sim-api/src/snapshot.rs`: add to the wire `Indicators` struct and the conversion (near line 431). History: `crates/sim-api/src/history.rs` (`HistoryPoint` field and `point()`), if it should be charted.
3. If a CLI line or CSV column helps: `crates/sim-cli/src/main.rs`, `crates/sim-engine/src/series.rs`.
4. `web/src/contract/index.ts`: add to `Indicators`, `HistoryPoint`, and `MetricKey` if it is a panel metric.
5. Panel: `web/src/panels/metrics.ts` (add an entry to the `METRICS` list with tab, label, hint; give a percent metric a `detail` with its backing numbers, map it in `METRIC_BREAKDOWN` if the wire has a breakdown for it; a test fails if its key has no explorer topic, so add the key to a topic's `usedBy` in `content/assumptions.json`) and `web/src/panels/format.ts` (`METRIC_LABEL`, a `Record<MetricKey,...>`, so the compiler forces it). Results/end story text: `panels/results.ts`, `panels/story.ts`.
7. If a lever's arrow should point at it: `content/levers.json` `arrows[].metric`.
Tests: `web/tests/panels-metrics.test.ts`, `fixtures.test.ts` (pins key sets via `satisfies Record<keyof T,...>`, so adding a field fails until listed), `crates/sim-api/tests/wire.rs`. Regenerate fixtures and golden.

## 5. Add a new tick system (a new rule module)

1. `crates/sim-model/src/<name>.rs` with `pub fn run(s:&State, p:&Params, r:&mut Rng) -> Deltas` (pure; no mutation; `libm`). Declare it in `crates/sim-model/src/lib.rs` and add it to `TICK_ORDER` (the array length is part of the type: update it). Order matters (`TICK_ORDER` has 14 systems since Town v7; S11 weather runs third, before production and nature, and is the example of a system that draws from the RNG): the engine applies each system's deltas (and runs the checks) before the next system runs, so a later system sees the state the earlier ones produced this tick.
2. `crates/sim-types/src/enums.rs`: add a `SystemId` variant and its `name()` (used in violation messages).
3. New state: fields in `sim-types/src/state.rs` (and `Deltas` / `Delta` variants in `deltas.rs` plus an arm in `crates/sim-engine/src/apply.rs`), initial values in `sim-engine/src/init.rs`, numbers in `content/params.json` with a struct in `sim-params` (`deny_unknown_fields`).
4. Invariants: extend `crates/sim-checks/src/lib.rs` if the system adds money or structure that must hold. Money must go through `Delta` money operations so `check_money` stays exact.
5. State layout changed => bump `SCHEMA_VERSION` (`sim-types/src/lib.rs`), update `persist.rs` structural checks, `web/src/store/save.ts` only if the save wrapper changed.
6. Tests in `tests/scenarios.rs` (behaviour, sign test, invariants over seeds). Document the rule in `docs/causal-model.md` section 10 and list deviations in `docs/deviations.md` (Town v2 table). Regenerate per section 0.

## 6. Add a new switchable economic system (like Market/Regulated/Coop)

Real example: `coop` (`content/systems/coop.json`, `content/scenarios/worker_coops.json`).
- `content/systems/<name>.json` (fields as in `market.json`); `crates/sim-params/src/lib.rs` (load it into `Params.systems`), `sim-types/src/enums.rs` (`EconomicSystem`), `crates/sim-api/src/content.rs` (embed the file), `keys.rs` (`SystemKey`), `content/levers.json` (`availableIn`, the `switch_system` action), `content/params.json` (`turn.levers.switch_system` cost).
- Web: `contract/index.ts` (`SystemKey`), `panels/content/systems.ts` (copy), `panels/systemSwitch.ts`, `panels/format.ts`, `panels/story.ts`.
- Tests: scenario tests (`coop_scenario_has_lower_inequality_than_market`), `panels-logic.test.ts`.

## 7. Add a firm or a sector

Real example: the **Shop** (Services) added in Town v2 (commits `4fb53fc` to `e9b4bc7`, `aa5939c`; deviation T21).
- Firm only (same sector): `content/initial_town.json` `firms` and `params.initial.firms`; `MAX_FIRMS` in `crates/sim-types/src/lib.rs` is 4 (slots Factory, Farm, Shop, one dormant); raising it touches every per-firm array, the map reservation (`initial_town.json` `map`), `persist.rs` checks, wire `firms[]`, and the web scene (`scene/entities.ts` places firms by tile).
- New sector: `Sector` in `sim-types/src/enums.rs`; production and goods rules (`sim-model/src/production.rs`, `goods.rs` spending shares such as `goods.services_share`); `class.rs` founding rules; `SectorKey` in `sim-api/src/keys.rs`; web `Sector` in `contract/index.ts`, firm model/colour in `scene/entities.ts` and `scene/models.ts`, labels in `panels/inspect.ts`; mock `rules.ts`/`dynamics.ts`.
- Retune start values by running the CLI, not by guessing; record what you tried in `docs/tuning-log.md`. A small town is noisy: one firm closing swings unemployment (`deviations.md` T3, `tuning-log.md`). Check baselines A-D.

## 8. Change a parameter safely

- Where: `content/params.json` (rules and costs), `content/initial_town.json` (the start: people, households, tiles, firms, policy), `content/systems/*.json`, `content/levers.json` (UI metadata only: labels, ranges, arrows).
- Try first without editing: `cargo run -p sim-cli -- --seed 42 --set labour.wage_up=1.01` (prefix `town.`, `market.`, `regulated.` or `coop.` for the other files).
- Edit, then: `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`; `cargo test --workspace`; `cd web && npm run gen:recording && npm run build:wasm && npm test`. Some tests hard-code values (for example `tests/scenarios.rs` checks start Gini and baselines). Update them only if the change is intended.
- To see weather at once without playing 14 turns: `cargo run -p sim-cli -- --seed 3 --set weather.force_stress=0.7` (`weather.force_stress` is `null` in `params.json`; the web has no URL option for it).
- After any Rust change run `cd web && npm run build:wasm`, or the browser keeps an old package. Also `cd web && npm run gen:recording` after any change to the sim or `content/` (the recorded demo is a run of the sim; `cargo test --workspace` fails when it is stale).
- Judge on 20 seeds and baselines A-D, not one run. Record the change and result in `docs/tuning-log.md`. Do not tune just to hit a target (`AGENTS.md`).

## 9. Add a lens

Real example: the `commute` lens (commit `015d3bc`, scene notes in `docs/web-notes-scene2.md`).
1. `web/src/store/types.ts`: add to the `Lens` union and `LENSES` list.
2. `web/src/scene/lenses.ts`: add a ramp to `RAMP`, cases in `entityTints`, `tileTints`, `personTints` and `legendFor` (pure colour functions: snapshot in, hex out; no Three.js).
3. **`web/src/store/save.ts`: the `LENS` allow-list in `parseSave`.** It currently lacks `'commute'` (see `docs/known-issues.md`); add new lenses here or a save made with that lens will not load.
4. `web/src/panels/ViewBar.svelte` reads `LENSES`; check the legend renders.
The `fire` lens (Town v4, "Fire and drought risk") is a worked example that also reads derived weather data: colours in `scene/weatherLook.ts`, per-tile risk in `store/scene-weather.ts`, tests in `web/tests/scene-weather.test.ts`.
Tests: `web/tests/scene-lenses.test.ts`, `scene-lenses2.test.ts`. Commands: `npm test`, `npm run typecheck`, `npm run lint`.

## 10. Change the contract (the wire)

1. Decide if it is breaking (a removed or reshaped field, a changed meaning, a changed `State`): then bump `SCHEMA_VERSION` in `crates/sim-types/src/lib.rs` (the api re-exports it) and `SAVE_VERSION` only if the browser save wrapper changed. Old saves are refused with `schema_mismatch`; the mock's constant follows (`mock/snapshot.ts`).
2. Rust: shapes in `crates/sim-api/src/{snapshot,turn,events,command,keys,meta,history}.rs`. Keep camelCase fields and snake_case tags. No logic: convert and validate only. Put `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]` on every wire type (see section 19 for the attributes that matter).
3. TypeScript: do not edit `web/src/contract/generated.ts`; derive `TS` on the new Rust type, list it in `crates/sim-api/src/contract_ts.rs` and run `cd web && npm run gen:contract` (section 19). Add an alias in `web/src/contract/index.ts` only if the web wants another name. Then the mock (`web/src/sim-port/mock/*`) and `sim-port/wasm/index.ts` if the call surface changed (`SimPort` in `port.ts`, and `crates/sim-wasm/src/lib.rs` for a new method).
4. Fixtures: `cargo run -p sim-api --example fixtures`; `web/tests/fixtures.test.ts` pins key sets against the TS types, so it fails until the type and fixture agree. Golden: `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`.
5. Docs: add a section to `docs/snapshot-contract.md` (next number after 11, saying which sections it supersedes) and a "Town v2" style entry in `docs/sim-api-notes.md` for details; update `docs/known-issues.md` if you leave a wart.
6. Run the whole checklist, including `npm run build:wasm` before `npm test` (otherwise the wasm golden test is skipped, not run).

## 11. Add a scene mesh

Real example: the Shop and tent models in commit `aa5939c`.
1. `web/src/scene/models.ts`: define the model (a list of boxes/primitives merged by `geometry.ts::buildModel`), give it a `ModelId`.
2. `web/src/scene/entities.ts`: decide when an entity uses it (`planEntities`: pure function of snapshot + layout, so unit-testable without WebGL).
3. `web/src/scene/entityLayer.ts` draws descriptions; colours come from `palette.ts`; lens tints from `lenses.ts`.
4. Test the planning, not the pixels: `web/tests/scene-entities.test.ts`. For a visual check run `npm run dev` and `node playwright/scene-shots2.mjs <outDir>` (manual; needs `npx playwright install chromium`).
Town v4 weather meshes: `forest_burning`, `forest_regrowing`, `fire_station` in `models.ts`; the model for a forest tile's state is `forestModelFor` in `scene/weatherLook.ts`; flames, smoke columns, haze over people and the coverage ring live in `scene/weather.ts` (`WeatherLayer`, capped instanced pools). Data for them is derived in `store/scene-weather.ts`.
Instanced scenery that depends on a stock (commons corn): keep the stock-to-look mapping pure in a module like `scene/corn.ts` (named visual constants, seeded by tile coordinates, unit-tested) and draw it with one `InstancedMesh` for all tiles in a layer like `scene/cornLayer.ts`; see `docs/web-notes-corn.md`.
The scene may import only three, store and contract. Anything the scene needs from panels goes through `store/scene-bus.ts`.

## 12. Add a panel

Real example: `PeoplePanel.svelte` (commit `7ae59a0`).
1. `web/src/panels/<Name>.svelte`; pure logic in a sibling `.ts` file (so it can be tested without a DOM), e.g. `people.ts`. Read state via the store (`game.state`); act via `game.*`. Do not import `scene`, `three` or `sim-port`.
2. Export it from `web/src/panels/index.ts`; mount it in `web/src/app/App.svelte`.
3. Generic widgets come from `web/src/ui-kit`; tokens in `ui-kit/tokens.css` (light and dark).
4. Tests: logic in `web/tests/panels-logic.test.ts` / `panels-v2.test.ts`; components in `panels-components.test.ts`. Keyboard/a11y: check with `web/playwright/panels-a11y.mjs` (manual).
5. New store action needed? Add it to `store/game.ts` (+ `Game` interface) and test in `web/tests/game.test.ts`. New cross-component state belongs in the store, never in a module-level variable shared by scene and panels.

## 12a. Add a new kind of money movement (Town v5)
Every balance change must be a ledger transaction, and `step()` fails a run that moves money without one, so the recipe is mostly "label it".
1. A deposit moving between existing accounts (the usual case): in the system, call `d.transfer_as(from, to, amount, FlowKind::X, Purpose::Y)` instead of `d.transfer(...)`. `FlowKind` keeps deciding the household income bookkeeping; `Purpose` only decides the ledger label.
2. Add a `Purpose` variant in `sim-types/src/ledger.rs` and its `(LedgerKind, detail)` in `Purpose::resolve` (reuse a `LedgerKind` of the contract's closed set unless you also change section 14, the web `LedgerKind` and `LedgerKindKey` in `sim-api/src/ledger.rs`). A plain `d.transfer` already works: it is labelled from its `FlowKind` ("wages", "tax", ...).
3. If the money must be split for the ledger (one payment, two purposes), do not change the model's amounts: pay the same total and split it with `split_by_weights` as `government.rs` does, so the State does not change.
4. Finance categories: if it is government revenue or spending, or a household income type, add its `(kind, detail)` to the match in `sim-api/src/finance.rs` (`Sums::add`); otherwise it lands in `other`. `crates/sim-api/tests/ledger.rs::finance_totals_equal_the_sums_of_ledger_entries` and `finance_households_and_firms_agree_with_the_state` show what must still hold.
5. A genuinely new kind of balance change (a new `MoneyOp` variant, or a new account type): add the arm in `apply_money` (record a `Transaction` with a `Posting`), add the `Posting` effect in `sim-types/src/ledger.rs::effects_of`, and the match arms in `tests/ledger.rs` (`money_op_postings`, `delta_changes_a_balance`) fail to compile until they are classified. Then `cargo test -p sim-engine --test ledger`.
6. Check: `cargo test --workspace` (the reconciliation runs inside every simulation test), then regenerate fixtures and the golden (`cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`, twice if the live and whole-turn goldens race) and prove the State is unchanged with `cargo run -p sim-cli --example statehash > after.txt` against the same run before your change.

## 12b. Add a ledger link (Town v5)
A figure that comes from a ledger links to it with `game.openLedger(account, kind?)` (and `game.select(...)` so the inspector follows). For a Finance row set `ledger: { account, kind }` in `panels/finance.ts`; for a metric add it to `METRIC_LEDGER` in `panels/metrics.ts`; for an event add a case to `eventLedger` in `panels/results.ts`. A market figure links to its trades the same way: `game.openTrades(marketId, entity?)` opens the "See all trades" window (section 25). The viewer itself (`LedgerTable.svelte`) needs no change; its logic is in `panels/ledger.ts` and tested in `web/tests/ledger-viewer.test.ts`. Shot script: `node playwright/v5-shots.mjs <outDir> [url] [WxH]`.

## 13. Try Pressure mode (Town v3)
1. `npm run dev` in `web/`, open `http://localhost:5173/?mode=pressure` (add `&pace=0.02` to make a quarter about 2 s; `pace` is real seconds per game day, default 0.143 = about 13 s per quarter). A plain visit shows the New game screen with Relaxed preselected; `?seed=` or `?mode=` links start straight away.
2. Other overrides: `timeout=submit|discard|skip`, `budget=<seconds of pause>`, `pause=0`. Defaults live in `web/src/store/clock-config.json`.
3. Pressure needs the live-turn calls (`begin_turn`, `advance_month`, `finish_turn`) in the wasm package; rebuild with `npm run build:wasm` if the New game screen says it is unavailable. The recorded demo has no live clock, so Pressure is unavailable there.
4. Tests: `web/tests/clock.test.ts` (state machine, fake time), `live.test.ts` (game loop), `live-turn.test.ts` (live turn equals `submitTurn`), `panels-v3.test.ts`.

## 14. Add a breakdown (Town v5.2)
1. Make the headline a small callable function (in `sim-metrics` or the model crate) and have both the indicator and the breakdown call it; never recompute the formula in `sim-api`.
2. Add a builder in `crates/sim-api/src/breakdowns.rs` using `make(id, title, formula, inputs, result, unit, parts)`; `result` must be the headline value itself and `parts` (if any) must sum to it (tolerance `parts_tolerance`: 1e-9 of the largest magnitude, at least 1e-9). Add the field to `Breakdowns` (camelCase on the wire).
3. If it needs what the model did inside a tick (not recoverable from the end state), emit an output-only `TraceRecord` from the system, keep it in `log::TickRecord`, and bump `SCHEMA_VERSION` (the log is saved).
4. Test in `crates/sim-api/tests/transparency.rs` (`check_snapshot`): result equals the headline, parts sum, over several seeds and in live mode. Update `web/src/contract/index.ts`, `docs/snapshot-contract.md`, regenerate fixtures and golden.

## 15. Add an assumptions topic (Town v5.1)
1. Add an entry to `content/assumptions.json` (fields as in `assumptions::TopicDef`; `group` and `status` are closed sets). `params` are dotted paths into `content/params.json`; units may use `{energy}` for the energy label. Do not copy numbers into the text; they come from `params` and `examples`.
2. For a worked example, put the formula in a small `pub fn` in the model (not inline in `run`), call it from `run`, and register it in `assumptions::registry()` under a name; the catalog example names it and gives its inputs. Behaviour must not change: check `cargo run -p sim-cli --example statehash` is identical.
3. `usedBy` lists the metric keys, lever keys and UI anchors (`panel:*`, `finance:*`, `people:*`, `industry:*`, `energy:*`, `place:*`) the topic explains. `crates/sim-api/tests/assumptions.rs` fails if a metric, lever or anchor in its lists has no topic, if a path or function does not exist, or if a registry function has no example. A new metric or lever must be added to those lists and to a topic.
4. Regenerate fixtures and the golden (`assumptions` hash).

## 16. Add or change an agent rule or a random draw (Town v5.3)

1. Change the rule in `crates/sim-model/src` as usual (numbers in `content/`).
2. If the rule is a registered decision (`sim_model::decisions::DECISIONS`: firm, household and bank decisions) its Agents lines are generated, so change the spec and `content/decisions.json` instead (section 27). Otherwise update the matching entry in the `agents` section of `content/assumptions.json`: `kinds[].rule`/`decides`/`observes`/`params`, or a `randomness[]` entry (distribution `bernoulli`, `uniform` or `uniform_index`; `params` are dotted paths into `params.json`, or `systems.<name>.` for the system files).
3. A new `r.chance(`/`r.next_f64(`/`r.below(` call fails `randomness_table_matches_the_draw_sites_in_the_model` until the table lists it. Add the entry (and, for a draw that is really two, adjust the test's count rule).
4. `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`, `cd web && npm run gen:recording && npm run build:wasm`, then the usual tests. State hashes must be unchanged unless the rule itself changed.

## 17. Add or change text, a threshold or a band shown in the UI (Town v6)

The web is a thin renderer: it must not hold model facts, explanatory text or thresholds. Where it goes:
1. Authored text (a hint, a note, a label of a wire id, a crisis sentence): `content/ui.json` `text.<group>.<key>`. Use `{name}` for a number the web fills in, never a literal number. In the web call `text('group', 'key')` (`web/src/store/catalog.ts`); `tests/thin-renderer.test.ts` fails when a key the code names is missing.
2. A threshold or a word band (crisis levels, health bands, stress and severity words, the community cap, the capital "tight" factor): `content/ui.json` `bands`; read it with `bands()`.
3. A number the model owns (a radius, a rate, a count of ticks): `content/params.json`, then put it on the wire (a field of the snapshot, see "Change the contract") instead of copying it into the web.
4. A derived figure (a risk, an expected effective value, a runway): compute it in `sim-model` or `sim-api`, serve it, and let the web only scale or format it.
5. Economic-system card text: `content/ui.json` `system_cards` (prose) and `system_param_labels`; the numbers are read from `params.systems.*` by `crates/sim-api/src/catalog.rs`.
6. Then: `cargo run -p sim-api --example fixtures` (refreshes `docs/fixtures/catalog.json`), `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`, `cd web && npm run gen:recording && npm run build:wasm`, tests.

What stays in the web: layout, formatting, animation, chart scaling, onboarding/coach copy and the wording of wire ids (event sentences, block-reason text).

## 18. Work with the estate (Town v7)

- **Content:** `content/initial_town.json` `estate` (household and home index, grounds, runway tiles in take-off order, `hangar_tile`, jet name and value), the estate household (the last in `households`) and its home, `staff_of` on the pilot (a person of Worker C's household), and the 7 road and 11 forest tiles in `map`. Rules numbers: `content/params.json` `estate`. Words: `content/ui.json` `tags`, `wealth_bands`, `text.estate`. Explorer: the `private_jet` topic and the `estate` kind and `estate_flight` draw in `content/assumptions.json`.
- **A town without an estate** still loads and runs: remove the `estate` block, the estate household, its home and the pilot; no flight draw is taken (`tests/estate.rs`).
- **Measure the estate:** `cargo run --release -p sim-cli --example estate_probe` (dismissals, flights, Gini for baselines A to F, 20 seeds); `cargo run --release -p sim-cli --example tune_probe -- estate.flight_prob_monthly=0.2` (A exhaustion, B survival and margins under overrides of any `section.field`). The pilot is dismissed when the estate's pay is short for `estate.pilot_unpaid_months_max` months: lower `pilot_wage_usd` or raise the Farm's profit to avoid it.
- **Wire:** `docs/snapshot-contract.md` section 19. (Flights use their own seeded stream.)
- **Bank identity:** the estate's savings are deposits, so they reduce bank equity (`equity = loans - deposits`) and the government's lending headroom (`equity / capital ratio - loans`). Check `init` still passes and the first-turn headroom is not below the cost of one plant.

## 19. Regenerate the TS contract (platform cleanup)

`web/src/contract/generated.ts` is generated from the Rust wire types; never hand-edit it. `web/src/contract/index.ts` re-exports it plus aliases.

- **Regenerate:** `cd web && npm run gen:contract` (runs `cargo run -p sim-api --features ts --example contract_ts`). Needs network the first time (ts-rs 12). The `ts` feature is off for the wasm build, so the browser package does not carry ts-rs.
- **Check:** `npm run check:contract`, or any `cargo test --workspace` (sim-api enables its own `ts` feature for its tests): `crates/sim-api/tests/contract_ts.rs` fails when the committed file differs from what the Rust types generate, when a type derives `TS` but is not listed in `crates/sim-api/src/contract_ts.rs`, or when the generated text names a type it does not define. The web fixture tests (`web/tests/fixtures.test.ts`) and `npm run typecheck` are the second gate.
- **Add or change a wire type:** put `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]` above its `#[derive(...)]`, add one line to the list in `contract_ts.rs` (`("module", <crate::module::Name as TS>::decl(&cfg))`), regenerate, then fix the web until `npm run typecheck` passes. Derive `TS` only in `sim-api`, never in `sim-types` (so money stays dollars as `f64` -> `number`; `Cents(i64)` never reaches the generator).
- **Attributes that matter:** `#[cfg_attr(feature = "ts", ts(optional))]` on an `Option` field that is omitted when `None` (`skip_serializing_if`) or defaulted when sent by the caller (gives `field?: T`, not `T | null`); `ts(type = "...")` to narrow a type (`month: 0 | 1 | 2`, `importance: 1 | 2 | 3`, `policy: Record<LeverKey, PolicyEntry>`, `SystemCard.key`); `ts(rename = "...")` when two modules use the same Rust name (`EnergyPlantKind`); `ts(skip)` for a field that is never serialized.
- **Names:** the TS name is the Rust name (`TileView`, `MapView`, `WeatherView`, `Finance`, `EstateView`). `index.ts` keeps the names the web already used (`Tile` = `TileView`, `TileRef` = the wire `{x, y}`, `GameMap`, `Weather`, `Estate`, `FinanceView`, `PolicyValue` = the displayed scalar, `Snapshot` = generated plus the mock-only `mock?`). Prefer the Rust name in new code.
- Do not write branded ids: ids are plain `number` on the wire.

## 20. Add an emission or income source (contract v2.0)

1. Model side: a new emitter adds an `EmissionSource` arm (sim-types, nature tally, trace) as before; the wire only needs the total.
2. `crates/sim-api/src/sources.rs`: add the id to `EMISSION_IDS` (or `INCOME_IDS`), and a category variant if none fits (`EmissionCategory`, `IncomeCategory`).
3. `crates/sim-api/src/emissions.rs` (`Acc` field and `Acc::tonnes` arm, tallied in `acc()`), or `snapshot.rs` / `finance.rs` for income (the `match id` closures). A missing arm reads as 0, so the test in step 5 is what catches it.
4. `content/ui.json`: one entry in `emission_sources` (`id`, `label`, `category`, `showWhenZero`) or `income_sources` (`id`, `label`, `category`, `partOf`, `views`). Load-time validation refuses a missing or unknown id.
5. Regenerate: `npm run gen:contract`, fixtures, golden, `npm run gen:recording`. Tests: `crates/sim-api/tests/transparency.rs` (sources sum to gross), `web/tests/sources.test.ts`. The web needs no change: panels iterate the list and read labels from it.

## 21. Add a new entity kind (entity registry, contract v2.1)

`content/entities.json` is the one list of kinds of actor, facility and stock (labels, roles, tiles, scene models, inspector sections). Panels, the scene and the build palette read it through `web/src/store/entities.ts`; nothing in `web/src` names a kind to label it.

1. **An entity that needs no new simulation rule** (a new facility variant, a labelled kind): add one object to `content/entities.json` (fields: docs/snapshot-contract.md section 21). Fixtures and the golden hash change (`catalog.json`): `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`. The inspector, the scene plan, the Industry list and labels pick it up with **no web change** (`web/tests/entities.test.ts` proves it with a made-up landfill). Re-run `cargo test -p sim-api --test entities`.
2. **An entity on the map** also needs a Rust tile kind: `TileKind` (`sim-types/src/enums.rs`), `TileKindKey` and `from_sim` (`sim-api/src/keys.rs`), `ALL_TILE_KINDS` in `sim-api/src/entities.rs`; if the player builds it, an `InfraKind` / `InfraKindKey` and its params cost and lead time. The registry test fails until every tile kind and infra kind is claimed once.
3. **If it emits**: an emission id (`docs/how-to.md` section 20) and the `emitter` role plus the `emissions` inspector section.
4. **If it needs a rule** (the landfill's methane, a new decision): the system in `sim-model`, numbers in `content/params.json`, assumptions entry; that part stays bespoke.
5. **A new look**: add a model to `web/src/scene/models.ts` and name it in `sceneModel`. Until then it draws as a plain block.
6. **A new kind of inspector section**: add the id to `InspectorSection` (Rust) and one builder to `SECTIONS` in `web/src/panels/inspect.ts` (TypeScript checks the table is complete). Existing sections need no code.
7. Regenerate: `npm run gen:contract` if a wire type changed, fixtures, golden, `npm run build:wasm`.

File count, landfill example (a buildable emitter on the map with a rule): `enums.rs`, `keys.rs`, `entities.rs` (tile list), `industry.rs` (infra key), the model system, `params.json`, `entities.json`, `ui.json` (emission source label), plus generated files. The web needs 0 files (1 optional mesh). A scenery-only entity with no rule is 1 content file plus the Rust tile kind lists. Before the registry the taxonomy counted about 15 files in 3 languages.

## 22. The recorded demo (replaces the hand-written mock)

`npm run dev` and the deployed app always use the real simulation (wasm). When the package is missing or fails to load, `selectSimPort()` (`web/src/sim-port/index.ts`) loads a **recorded demo**: `sim-port/replay/` implements the same `SimPort` over a run of the real simulation that a cargo example wrote into a typed fixture. Nothing in the web simulates anything.

- **What it is:** `content/demo_script.json` (seed 7 and the commands of 12 turns) is played by `sim_api::recording::record()`; `web/src/sim-port/replay/recording.generated.ts` holds the replies (`Recording`, a generated contract type: initial snapshot, levers, actions, catalog, explorer data, and each turn's `TurnResult`). The replay port serves them in order; `init` starts over; the seed asked for is ignored (the recording's seed is shown).
- **Honest controls:** the port sets `demo` (`DemoInfo`); the store exposes `GameState.demo`. The top bar says "Recorded demo" (tooltip: what it is), the policy drawer, building, save/load, ledger links, live clock and previews are off or explain themselves, and End turn reads "Next recorded turn" and stops at the end. A call that would diverge (`preview`, `submitTurn` with commands, `ledger`, `exportState`, ...) returns an error result with code `demo_unavailable`.
- **Regenerate** after any change to the simulation, `content/` or the wire (it contains snapshots): `cd web && npm run gen:recording` (`cargo run -p sim-api --example recording`). Tests: `crates/sim-api/tests/recording.rs` fails in `cargo test --workspace` when the committed file is stale (like the contract check; `npm run check:recording` runs it alone); `npm run typecheck` fails when the file does not match the generated contract (it is annotated `: Recording`); `web/tests/replay.test.ts` covers the port and the store in demo mode.
- **To record a different run:** edit `content/demo_script.json` (any number of turns; the file grows about 100 KB per turn) and regenerate.
- **Force it:** `?sim=replay` or `VITE_SIM=replay`. **Without the wasm package:** `npm run dev` shows the demo with a notice; `npm test` leaves out the files that need the real simulation (they import `tests/port.ts` or the wasm package) and says so; CI (`deploy.yml`) first runs the tests without the package, then builds it and runs everything.
- **Tests that need the real simulation** import `createSimPort` from `web/tests/port.ts` (a fresh wasm sim, already started). There is no second implementation to keep in step: `sync-mock`, `mock/` and the parity tests are gone.

## 24. Add a market to the Markets view (contract v2.3)

The web needs no change: it draws every entry of `snapshot.markets` with the wire's words (`web/tests/panels-markets.test.ts` proves a made-up market on the wire appears). A new market is a content entry, one Rust arm, an explorer topic, and a regenerate.

1. `crates/sim-api/src/markets.rs`: add the id to `MARKET_IDS`; list the chip ids it can produce in `chip_ids` and the driver ids in `driver_ids`; write a `<name>_raw` function that returns a `Raw` (`demand`, `supply`, `traded`, `unmet`, `unsold`, the price if there is one, the chips, the drivers and an `ok` cross-check against the end state) and add an arm to `raw_for`. Use `sim-metrics` functions for figures that are pure functions of the state. If you need what happened inside a tick, emit an output-only `TraceRecord` from the system (it must not change the state hash: check `cargo run -p sim-cli --example statehash` before and after), keep it in `log::MarketTrace`, and bump `SCHEMA_VERSION` (the log is saved). Add the price to `HistoryPoint` if it should have a line, and to `history_price`.
2. `content/ui.json` `markets.list`: one entry (`id`, `label`, `category`, `mechanisms`, `blurb`, `unit`, `unitOne`, `amount`, price words and `priceCause`, `priceNote`, the three quantity labels, `gapFloor`, `gapUnmet`, `gapUnsold`, `howId`, the four `verdict` sentences, one `short` sentence per chip id, one `drivers` label per driver id). Placeholders in verdicts: `{demand} {supply} {traded} {unmet} {unsold}` and `{demand_unit}` etc. (the unit word that agrees with that number); in chips: `{count} {firm} {unit}`. Write shares as counts out of a hundred in Rust (`per_hundred`), never as percentages. Load-time validation refuses a missing or unknown id, category, mechanism or cause.
3. `content/assumptions.json`: a topic that lists the `howId` in `usedBy` (and `markets:intro` stays on `coordination_mechanisms`); add the `howId` to `ANCHORS` in `crates/sim-api/tests/assumptions.rs`. Put a worked example in a small `pub fn` in the model and register it in `assumptions::registry()` if the rule has a formula.
4. Add the market's trade list too (section 25), then add the market to the loop in `crates/sim-api/tests/markets.rs` (it asserts every card reconciles over seeds, turns and live months).
5. Regenerate: `cd web && npm run gen:contract` (only if a type changed), `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run gen:recording`, `npm run build:wasm`. Say in the commit that the wire JSON was re-blessed.
6. Screenshot: `node web/playwright/markets-shots.mjs <outDir> [url]` (needs `npm run dev -- --port 5190`).

## 25. Add or change a market's trade list (contract v2.4)

Every market in `content/ui.json` `markets.list` needs a `trades` block (load-time validation refuses a missing one), and the web needs no change (`web/tests/market-trades.test.ts` proves a made-up market renders from the wire alone).

1. `crates/sim-api/src/markets.rs`: list the column ids in `trade_column_ids` (in the order the builder writes cells) and the note ids in `trade_note_ids`.
2. `content/ui.json`: `markets.list[].trades` with `intro`, `empty`, `columns[]` (`id`, `label`, plain-English `help`, `numeric`; same ids and order), `notes` (one sentence per note id), `uncounted` (said under the table when some rows are listed but not counted) and `totals` (`{count} {units} {money} {span}`). No percentages.
3. `crates/sim-api/src/trades.rs`: add a `Tag` for the market, push its `Trade` rows in `TradeStore::record_tick` (from the state after the tick and the tick's events, or a new output-only `TraceRecord` from the system: never put per-trade data in `State`; check `cargo run -p sim-cli --example statehash` before and after), and add the cell arms in `cells_for`. Say in `build` whether the card compares counted units or counted rows.
4. Add the market id to `LISTED` in `crates/sim-api/tests/market_trades.rs` (it reconciles every list to its card over seeds, turns and live months).
5. Regenerate as in section 24 step 5 (the golden `catalog` and `export` hashes change; say so in the commit).
6. Shot script: `node web/playwright/trades-shots.mjs <outDir> [url]`.

## 23. Add an asset or liability to the net worth statement (Town v8)

1. Model side: if the new thing is part of `household_wealth` (sim-metrics), add a field to `WealthLines` and fill it in `household_wealth_lines`; `household_wealth` itself stays as it is (the state hash depends on its float order) and the reconcile test fails if the two disagree. A thing that is not in net worth is not a line.
2. `crates/sim-api/src/networth.rs`: add the id to `LINE_IDS` and an arm to `value_of` (and `count_of` if it stands for homes or firms).
3. `content/ui.json` `net_worth_lines`: one entry (`id`, `label`, `category` asset or liability, `howId`). Load-time validation refuses a missing or unknown id.
4. `content/assumptions.json`: put the `howId` in the `usedBy` of a topic (`net_worth` or `net_worth_firms`) and add it to `ANCHORS` in `crates/sim-api/tests/assumptions.rs`.
5. Regenerate: `cd web && npm run gen:contract` (only if a type changed), `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run gen:recording`, `npm run build:wasm`. Tests: `cargo test -p sim-api --test networth`, `web/tests/panels-networth.test.ts`.
6. The web needs no change: the inspector, the Gini view, the estate card and "Who holds what" iterate the wire lists. A new holder group is a `HOLDER_IDS` entry, a `net_worth_holders` entry and the grouping in `networth::holder_of`.
7. Screenshots: `node web/playwright/v8-shots.mjs <outDir> [url]` (needs `npm run dev -- --port 5190`; `?sim=replay` for the demo).

## 26. Add or change a natural stock in the sustainable-yield view (contract v2.6)

The web needs no change: `YieldPanel` draws every entry of `snapshot.natureYield` (`web/tests/panels-yields.test.ts` proves a made-up stock and row appear).

1. Model: put the pure figures (regrowth at a stock, best possible regrowth, the cap on what the farm can take or grow) in `sim-metrics` and have both the rule (`nature.rs`, `production.rs`) and the view call them; never recompute in `sim-api`. The cap is `food_cap` (production applies it to the farm's output before electricity rationing; it reads last month's end stocks, so no `State` field is needed). Numbers go in `content/params.json` `nature` (`farm_cap_share_*`, calibrated so the cap does not bind in the starting town).
2. `crates/sim-api/src/yields.rs`: add the id to `STOCK_IDS`, its rows to `row_ids`, a `*_figures(state, params)` function, and handle it in `build`. History gets optional stock, regrowth, taken fields in `history.rs`.
3. `content/ui.json` `nature_yield`: words (ratio, flag, months, access labels with their mechanism id, `slowMonths`) and one `stocks[]` entry (`id`, `title`, `blurb`, units, `formula`, a label for every row, four verdict sentences, `howId`). `text.ecology` holds the panel's intro and chart words. Load-time validation refuses a missing or unknown stock or row.
4. `content/assumptions.json`: a topic listing the `howId` in `usedBy` (`max_sustainable_yield`, `overshoot_and_collapse`, `tragedy_of_the_commons`, `commons_access_rules` exist); add the anchor to `ANCHORS` in `crates/sim-api/tests/assumptions.rs`; register a worked example in `assumptions::registry()` if the rule has a formula.
5. Tests: `crates/sim-api/tests/yields.rs` (rows reconcile, flag agrees with the figures, a heavy harvest goes over the line and food runs short); the model sign tests are `farm_cap_*` in `tests/scenarios.rs`.
6. Regenerate (section 0): `cd web && npm run gen:contract`, `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run gen:recording`, `npm run build:wasm`. Check the state hash with `cargo run -p sim-cli --example statehash` before and after: it must be identical while the cap does not bind.
7. Screenshot: `node web/playwright/yield-shots.mjs <outDir> [url] [harvest] [turns]` (needs `npm run dev -- --port 5190`; `harvest` 3 is the overconsumption run; `?sim=replay` for the demo).

## 27. Add a decision (decision points, contract v2.7)

A decision is a pure function over a read-only view; the registry says what it reads, writes and uses (`docs/features/decision-points.md` section 10). Worked example: `crates/sim-model/src/labour.rs`.
1. **Quantities.** Add any new `QuantityId` variant in `crates/sim-types/src/decisions.rs` (one line, with its wire name) and its entry (label, unit, owner, one-sentence summary) in `content/decisions.json` `quantities`. A missing or extra entry fails `catalog()`.
2. **The function.** In the system module write a `*View` struct (plain `Copy` data: only quantities, built from `State` and `Params`, never `State` itself), a `*_view(s, p, ...)` builder, a `*Choice` struct and `decide_x(&View, &Params-struct) -> Choice`. No `Rng`, no `State`, no `static`. Cut and paste the old expression; do not reorder float arithmetic. `run` then does: build view, decide, push deltas. Mechanics (matching, picking names) stay in `run`.
3. **The spec.** Next to it: `pub const FIRM_X: DecisionSpec = DecisionSpec { id, system, inputs, outputs, effects, params, rule }` (`effects` is required, see 3b). Add the `DecisionId` variant (and its arm in `DecisionId::owner()`) and the spec to `DECISIONS` in `crates/sim-model/src/decisions.rs`. `params` are dotted paths into `content/params.json`: list every parameter the function or its view builder reads.
3b. **Effects (stage 2, contract 2.11).** `effects: &[Fx::up(Q::A, Q::B), Fx::down(..), Fx::varies(.., "why"), Fx::none(.., "why"), ..]` with `use sim_types::Effect as Fx;`: **one entry per input -> output pair** (`cargo test -p sim-model causal` fails on a missing, duplicate or foreign pair). Choose the sign from the code and economics *before* running the test (never back-fill it from the measurement): `up` = more of the input gives more of the output, other things equal; `down`; `varies(reason)` for a hump, threshold or category whose direction depends on the state; `none(reason)` when the function reads the input but it has no direct effect on this output. `.lag(n)` only for months the decision itself adds (the tick order's wait is derived by `causal::edge_lag_months`, never written). Reasons are sentences (over 20 characters).
4. **Words.** `content/decisions.json`: the `decisions` entry (title, one-line `decides`, rule id/label/text; the rule id must equal the spec's `rule`) and a `params` entry (label, unit) for each new path. For a new owner kind, add an `owners` row naming its Agents kind id.
4e. **Live value (contract 2.9).** Every `QuantityId` needs an arm in `live::binding` (`crates/sim-api/src/live.rs`; the compiler forces it): bind it to a snapshot field (`path`, an `Aggregate`, a `read` closure over the typed `Snapshot`, optionally a lever and a history `previous` for a trend) or give an `Unbound` reason. `tests/live.rs` resolves every bound path on a real snapshot and checks the value; reason and aggregate words are in `content/decisions.json` `causal.live`.
4d. **Stage 5: notes and edges.** A decision's Agents lines (decides, observes, params, rule) are generated; never write them in `content/assumptions.json` (`catalog()` fails if you do). If its owner is a new Agents kind, add the kind to `assumptions.json` with empty `decides`, `observes`, `params` and `rule`. Anything about the kind that is not a decision (accounting, matching, "nothing decided here") goes in `content/decisions.json` `kind_notes`. The causal edges (`catalog().causal`) are derived from the spec: check them with `cargo test -p sim-model causal` and `cargo test -p sim-api --test decisions`, and read the new node's `role` (a quantity nothing writes is a "Starting point", one nothing else reads an "End point": say in the as-built note if that surprises you).
5. **Tests.** Add a `check_view_only` test in `crates/sim-model/src/decision_tests.rs` (copy one; the helper is there): a base view list, one perturbation per view field, an exhaustive destructure of the view. It fails if a view field is not a listed input, a listed input has no field, or a field never changes the choice.
4b. **Conventions from stage 3.** A decision over a list (the shop choice) takes a view that borrows slices (`ShopView<'a>`); the storage lives in a small `ShopInputs` the builder returns, and the view is made with `.view(budget)`. A class-dependent parameter (the spending propensity) is selected by the view builder and enters as an input quantity, with every possible path listed in the spec's `params`. A factor that does not apply is 0.0 or 1.0 exactly (`wealth_spending_rate` is 0 for the classes that do not spend wealth). A quantity that means two things in two decisions gets two ids (`base_rate` is the cost of borrowing for a founder; `policy_rate` is the player's rate for the bank). Several decisions may share one view type (bank headroom and dividend).
4c. **Conventions from stage 4.** Policy execution is not a decision: the government pays benefits, UBI, job-guarantee wages, upkeep and builds as the levers say, so only the funding, the pro-rata cut and the legitimacy step are decisions, and `run` passes the committed total to the builders. A draw that was `r.chance(p)` becomes `r.next_f64()` in `run` (the same single draw) and the decision compares it with the chance (`random_draw < chance`). A decision may reuse another decision's function for an input (`government_fund` takes the bank's lending room from `decide_headroom`). A choice that is a branch of what `run` applies is an enum (`FundChoice`, `PilotChoice`); `run` still moves the money and writes the patches. A setting read through `cfg(s, p)` (the economic system's enclosure chance, transition resistance) is an input quantity; the gate perturbs only `params.json`, so those system leaves are declared and listed in `DORMANT`.
5a. **Sign test (stage 2).** Add a probe in `crates/sim-model/src/sign_tests/decisions.rs`: a list of base views (several, so that some are not clamped), a `fn(&mut View)` per input that raises it, a `fn(&View) -> f64` per output that calls the real `decide_*` (booleans as 0/1), then `r.check(&probe)`. `cargo test -p sim-model sign_tests` fails when a declared sign disagrees with the measured one (with the counts it saw), when a signed effect never moves (a dead edge) or a `none` effect moves, and when an effect has no probe. A disagreement is a finding: decide whether the declaration or the code is wrong and write the reason in the commit; do not just flip the sign. Optional words: a hand-written because in `content/decisions.json` `causal.effects.because_overrides` (spec, from, to, because); without one the sentence says "following the rule ...".
5b. **Builder gate.** Add the decision to `choices()` in `tests/decision_builders.rs` (fingerprint the view and the choice) and its `reads()` entry (the State and params leaves the builder reads, array indices as `[]`). Run once with an empty entry and `DISCOVER=1 cargo test -p sim-engine --test decision_builders -- --nocapture` to see what it reads, and check each line is something you meant. Use `variants()` if a branch is not reached by the plain run (stage 3 added a bank with negative equity, rescued and failing; stage 4 an indebted government, an estate whose pilot is due to be let go, and a pilot it can afford to rehire). Arguments `run` passes in (a month's demand, the bank's running headroom, the market budget) are fed as constants at the top of the test; leaves the gate cannot bump (an enum such as `class`, or a length such as the head count) and parameters whose bump leaves the valid range are declared and listed in `DORMANT` with the reason.
6. **Prove nothing changed.** `cargo run -p sim-cli --example statehash > before.txt` before step 2, again after, `cmp` (must be identical); `cargo run --release -p sim-cli --example baselines` the same way.
7. **Regenerate** (section 0 order): `cd web && npm run gen:contract` (only if wire types changed), `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run build:wasm`, `npm run gen:recording`. The Agents explorer (decision line, causal edges, notes) picks the new decision up with no web change; the contract version moves only if a wire type changed.

## 28. Add a mechanism (causality graph stage 1, contract v2.10)

A mechanism is plain code (physics or accounting) that no decision makes but that links the quantities decisions read and write: production, emissions, weather, wellbeing, rent, taxes, matching. Declaring one **changes no behaviour**: it is documentation the graph, the Agents text and a test are generated from (`docs/features/causality-graph.md` section 12). Worked example: `FIRE_SMOKE` in `crates/sim-model/src/production.rs`.
1. **Quantities.** Reuse an existing `QuantityId` where one fits. A new one needs a variant in `crates/sim-types/src/decisions.rs`, an entry in `content/decisions.json` `quantities` (label, unit that reads after a number, owner kind, one-sentence summary) **and a `live::binding` arm** in `crates/sim-api/src/live.rs` (the compiler forces it): bind it to a snapshot field so it cannot disagree with its panel (`nature.grossEmissionsMonthly`, `weather.stress`, `housing.rentPrivate`), or give it an `Unbound` reason (`not_published` is the honest one for state the snapshot does not show). `tests/live.rs` resolves every bound path on a real snapshot (a flag path may hold an object or null: use `aggregate: Flag`).
2. **The spec.** In the module of the code it describes: `pub const X: MechanismSpec = MechanismSpec { id: M::X, system: SystemId::..., inputs, outputs, effects, params, rule: "rule_id" }` (`use sim_types::MechanismId as M;`). `inputs` and `outputs` are the quantities the code reads and writes (a stock that carries over lists itself on both sides); `params` are dotted paths into `content/params.json` that **this code reads** (read the function, and see the gate below). Add the `MechanismId` variant and its arm in `MechanismId::owner()` (the owner kind is derived, never stored; `Weather` owns weather), and the spec to `MECHANISMS` in `crates/sim-model/src/decisions.rs`. Do not touch the code or its float order.
3. **Words.** `content/decisions.json`: a `mechanisms` entry (`id`, `title`, one-line `explains`, `rule` with the same id as the spec and a label and text), and a `params` entry (label, unit) for each new path; an unused `params` entry fails `cargo test -p sim-api --test decisions`. The Agents explorer lists the mechanism under its owner's kind generically (no web change).
4. **Check the edges.** `cargo test -p sim-model causal` (no dangling ids, unique mechanism ids, one owner, every spec input and output has its reads, writes and drives edges, the named DP-13 chains connect) and `cargo test -p sim-api --test decisions`. If a quantity you wrote used to be a "Starting point" it is now "In the middle": say so in the as-built note.
4b. **Effects and the sign test (stage 2, contract 2.11).** Every input -> output pair needs an `Fx::up/down/varies/none(..)` in `effects` (see section 27 step 3b; sign from the code and economics, not back-filled). Then measure it, in this order of preference: (1) a pure function the code already calls: a probe in `crates/sim-model/src/sign_tests/mechanisms.rs` (`Basis::Perturbed`, the default); (2) a mechanism inline in a `run`: a probe in one of `tests/mechanism_signs/group_*.rs` that raises one field of a real mid-run `State` (`common::states()`), runs the real system (`common::after(SystemId::X, &s, &p)`) and reads one field of the result, then mark the effect `.system()`; (3) neither feasible: `.unverified("reason")` and list it in `docs/known-issues.md` DP-18. Do not extract a pure function from a `run` unless the float order stays identical and `statehash` and `baselines` cmp clean. Run `cargo test -p sim-model sign_tests` and `cargo test -p sim-engine --test mechanism_signs`. Rules (clamped samples are the flat ones and are ignored; a signed effect that never moves is a dead edge): `crates/sim-model/src/sign_probe.rs`.
5. **Parameter gate.** `cargo test -p sim-engine --test mechanism_params -- --nocapture`: it perturbs every `params.json` leaf and runs the real system on varied states; it fails if a declared param is never read by the mechanism's system, and if the system reads a param nothing of that system declares (print the list with `--nocapture`; allow one in `UNDECLARED` only with a reason). A param reached only by a random draw or a branch the samples miss goes in `DORMANT` with the reason, or add a state variant to `states()`. It does not test the quantity lists.
6. **Prove nothing changed.** `cargo run -p sim-cli --example statehash` and `cargo run --release -p sim-cli --example baselines` before and after, `cmp` byte-identical.
7. **Regenerate** (section 0): `cd web && npm run gen:contract` (only if a wire type changed), `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run build:wasm`, `npm run gen:recording`.

## 29. Loops of the causal graph (causality graph stage 3, contract v2.12)

Loops are **computed, never written**: `crates/sim-model/src/causal_loops.rs` finds the strongly connected components and every simple cycle of at most `MAX_LOOP_LEN` (8) quantities in the signed quantity graph and classifies each from the signs (`docs/features/causality-graph.md` section 14). You do not add a loop; you add or change an effect (sections 27 and 28) and the loops follow.
1. **After changing a spec or effect**, run `cargo test -p sim-model causal_loops` (every loop is a real cycle, the R/B parity, the component partition, determinism, the known examples, the nine rows of `docs/causal-model.md` section 4) and `cargo test -p sim-api --test decisions loops`. The counts are pinned (`counts_are_pinned`, 203 loops at the time of writing): a change is a change to the model's feedback structure, so read the new loops (print `loops()`), explain them in the as-built note, then update the pins.
2. **A teaching name** for a loop: `content/decisions.json` `causal.loops.names` (`members` are the quantity ids of the loop in any order, `name`, `story`). Loading fails if the members are not a generated loop, so a renamed or removed effect cannot leave a stale name. The loop's id is `loop:` plus the sorted member names joined with `+`.
3. **A new quantity that plain code also writes** (DP-13): add its id to `causal.loops.gap_quantities` so any loop through it is flagged `incomplete`.
4. **Words** (kind labels, the sentence template, the incomplete reasons) are in the same block; the sentence is generated from the cycle and its signs, so do not hand-write one.
5. Regenerate (section 0): `npm run gen:contract` only if a wire type changed, `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run build:wasm`, `npm run gen:recording`. `statehash` and `baselines` must `cmp` identical: loops read declarations, not the simulation.
6. **Web**: nothing. The Agents explorer renders `catalog().causal.loops` generically (`loopLines`, `loopSummary`); `web/tests/agents.test.ts` proves a loop added only to the catalogue appears.
