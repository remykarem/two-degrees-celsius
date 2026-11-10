# How-to: recipes for common changes

Each recipe lists the files to touch (found by following a real feature through the code), the tests that will tell you what you missed, and the commands. Paths are from the repo root. **Always finish with the checklist in `AGENTS.md`.** Line numbers drift; search for the names given.

Two rules of thumb: (1) the Rust compiler and `web/tests/*` tell you most of what you forgot, because every closed key set is matched exhaustively or pinned by a test; (2) after any change to `content/` or the sim, three generated files go stale (see section 0).

## 0. What regenerates when

| You changed | Regenerate | Command |
|---|---|---|
| Any number in `content/params.json`, `initial_town.json`, `systems/*.json` | mock data | `cd web && npm run sync-mock` |
| Same, if wire output changes (almost always) | fixtures | `cargo run -p sim-api --example fixtures` |
| Anything that changes sim output | golden | `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` |
| Anything that changes seed-42 market output | insta snapshot | `INSTA_UPDATE=always cargo test -p sim-engine --test scenarios golden_snapshot_seed_42` (unverified: not run) |
| `content/levers.json` | fixtures (`levers.json`, `actions.json`) and golden | as above |
| Rust sim (for the web wasm test) | wasm package | `cd web && npm run build:wasm` |
| `content/` at all | invalidates old saves (`paramsHash`) | nothing to run; say so in the commit message |

Then run all tests. Baseline tests A-D in `tests/scenarios.rs` (20 seeds each) can fail after a parameter change; that is information about balance, not necessarily a bug. Do not tune to make them pass (see `AGENTS.md`).

## 1. Add a policy lever

Real example to follow: `job_guarantee` (the newest lever).

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
- Mock: `web/src/sim-port/mock/mock-only.json` (hand-authored lever presentation), `web/scripts/sync-mock.mjs` `POLICY_KEYS` (maps `initial_town.json` policy keys to lever keys; unknown keys throw), and the mock rule in `mock/rules.ts` / `mock/dynamics.ts` if the mock should react to it.

Tests/commands: add a sign or behaviour test in `tests/scenarios.rs` (see `job_guarantee_employs_people_and_lowers_unemployment`); `web/tests/contract.test.ts`, `mock-sync.test.ts`, `fixtures.test.ts` will flag missing pieces. Run section 0 regeneration, then the full checklist. Update `docs/causal-model.md` section 10 (the rule) and `docs/glossary.md` if the lever adds a concept.

## 2. Add a buildable kind

Real example: **road** (`place:road`, `remove:road`). Needs tile rules, costs, lever table, wire, palette, scene mesh.

Rust
- `crates/sim-types/src/enums.rs`: `InfraKind` and `TileKind` variants; `place_<kind>` / `remove_<kind>` names in the lever-name list and `lever_key()` mapping (`InfraKind::Road => "place_road"`).
- Costs: `content/params.json` `government.<kind>_cost_usd` and upkeep (`road_cost_usd`, `road_upkeep_usd`), mirrored by fields in `crates/sim-params/src/lib.rs`; `turn.levers.place_<kind>` and `remove_<kind>` (PC cost, lead time).
- Tile rules: `crates/sim-engine/src/turn.rs::tile_problem` (what tiles are buildable; today forest, commons and off-map are `TileInvalid`, anything else non-empty is `TileOccupied`), `build_cost`.
- Effect on activation and construction: `crates/sim-engine/src/commands.rs` (pushes to `gov.pending_build`), `crates/sim-model/src/government.rs` (build and upkeep, near the `InfraKind::Road` arm). Gameplay effect: `crates/sim-model/src/space.rs` (commute routes use roads).
- `crates/sim-api`: `keys.rs` (`InfraKind`, `TileKindKey`, `ALL`), `snapshot.rs::buildings` (summary count), `meta.rs` (`place:<kind>` money cost; needs `content/levers.json` action entries with `tileRule`).

Web
- `web/src/contract/index.ts`: `InfraKind`, `TileKind`, `ActionKey` types.
- Palette: `store/palette.ts` builds entries from `actions()` automatically; label/hint text in `panels/BuildPalette.svelte`; `store/game.ts` (the kind guard near line 185) and `store/map.ts`; inspector text in `panels/inspect.ts`.
- Scene: `scene/models.ts` (mesh/model id), `scene/entities.ts` (`Building` type, planning), `scene/entityLayer.ts`; roads have their own `scene/roads.ts` and ghost handling in `scene/index.ts`.
- Mock: `mock/data.ts` (`INFRA_KINDS`, `moneyFor`), `mock/rules.ts` (`REMOVABLE`, validity), `mock/state.ts`, `mock/space.ts`, `mock/snapshot.ts`; costs via `sync-mock.mjs`.

Tests: roads tests in `tests/scenarios.rs` (`roads_are_built_on_their_tile_cost_money_and_need_upkeep`, `a_road_shortens_a_commute_...`), `tests/golden/plan.json` (add the kind to the golden plan only if you want it covered), `web/tests/mock.test.ts`, `scene-roads.test.ts`. Regenerate per section 0. Update `docs/snapshot-contract.md` section 11 if the wire changed.

## 3. Add an event type

Real example: `electricity_rationed` (raised in `sim-model/src/production.rs`).

1. `crates/sim-types/src/enums.rs`: add a variant to `Event`.
2. Emit it from a system: `d.events.push(Event::...)` in the relevant `crates/sim-model/src/<system>.rs`.
3. `crates/sim-api/src/events.rs`: add the `SimEventKind` variant (wire shape, snake_case tag, camelCase fields), the `from_sim` arm, and its **importance** (3 game-shaping, 2 notable, 1 routine) in `importance()`. The match is exhaustive, so the compiler points here.
4. `crates/sim-api/tests/wire.rs`: add it to the list of every kind (near `K::ElectricityRationed`).
5. `web/src/contract/index.ts`: add to the `SimEvent` union.
6. `web/src/panels/format.ts::formatEvent`: the text (the contract carries no display text). Tests: `web/tests/panels-v2.test.ts` (see the `electricity_rationed` case). Optionally `scene/flows.ts` (event-driven effects/labels) and the mock (`mock/people.ts` etc.) if the mock should emit it.
7. Regenerate fixtures and golden; update `docs/snapshot-contract.md` section 11 events list.

## 4. Add a metric / indicator

Real example: `median_income` / `top_wealth_share` (Town v2).

1. `crates/sim-types/src/state.rs`: add to `Indicators` (e.g. `median_income_pp`). `crates/sim-metrics/src/lib.rs::compute`: compute it (pure; `libm` for math). Unit test in that crate.
2. `crates/sim-api/src/snapshot.rs`: add to the wire `Indicators` struct and the conversion (near line 431). History: `crates/sim-api/src/history.rs` (`HistoryPoint` field and `point()`), if it should be charted.
3. If a CLI line or CSV column helps: `crates/sim-cli/src/main.rs`, `crates/sim-engine/src/series.rs`.
4. `web/src/contract/index.ts`: add to `Indicators`, `HistoryPoint`, and `MetricKey` if it is a panel metric.
5. Panel: `web/src/panels/metrics.ts` (add an entry to the `METRICS` list with tab, label, hint) and `web/src/panels/format.ts` (`METRIC_LABEL`, a `Record<MetricKey,...>`, so the compiler forces it). Results/end story text: `panels/results.ts`, `panels/story.ts`.
6. Mock: `mock/snapshot.ts` (indicators), `mock/index.ts` (history).
7. If a lever's arrow should point at it: `content/levers.json` `arrows[].metric`.
Tests: `web/tests/panels-metrics.test.ts`, `fixtures.test.ts` (pins key sets via `satisfies Record<keyof T,...>`, so adding a field fails until listed), `mock.test.ts`, `crates/sim-api/tests/wire.rs`. Regenerate fixtures and golden.

## 5. Add a new tick system (a new rule module)

1. `crates/sim-model/src/<name>.rs` with `pub fn run(s:&State, p:&Params, r:&mut Rng) -> Deltas` (pure; no mutation; `libm`). Declare it in `crates/sim-model/src/lib.rs` and add it to `TICK_ORDER` (the array length is part of the type: update it). Order matters: the engine applies each system's deltas (and runs the checks) before the next system runs, so a later system sees the state the earlier ones produced this tick.
2. `crates/sim-types/src/enums.rs`: add a `SystemId` variant and its `name()` (used in violation messages).
3. New state: fields in `sim-types/src/state.rs` (and `Deltas` / `Delta` variants in `deltas.rs` plus an arm in `crates/sim-engine/src/apply.rs`), initial values in `sim-engine/src/init.rs`, numbers in `content/params.json` with a struct in `sim-params` (`deny_unknown_fields`).
4. Invariants: extend `crates/sim-checks/src/lib.rs` if the system adds money or structure that must hold. Money must go through `Delta` money operations so `check_money` stays exact.
5. State layout changed => bump `SCHEMA_VERSION` (`sim-types/src/lib.rs`), update `persist.rs` structural checks, `web/src/store/save.ts` only if the save wrapper changed.
6. Tests in `tests/scenarios.rs` (behaviour, sign test, invariants over seeds). Document the rule in `docs/causal-model.md` section 10 and list deviations in `docs/deviations.md` (Town v2 table). Regenerate per section 0.

## 6. Add a new switchable economic system (like Market/Regulated/Coop)

Real example: `coop` (`content/systems/coop.json`, `content/scenarios/worker_coops.json`).
- `content/systems/<name>.json` (fields as in `market.json`); `crates/sim-params/src/lib.rs` (load it into `Params.systems`), `sim-types/src/enums.rs` (`EconomicSystem`), `crates/sim-api/src/content.rs` (embed the file), `keys.rs` (`SystemKey`), `content/levers.json` (`availableIn`, the `switch_system` action), `content/params.json` (`turn.levers.switch_system` cost).
- Web: `contract/index.ts` (`SystemKey`), `panels/content/systems.ts` (copy), `panels/systemSwitch.ts`, `panels/format.ts`, `panels/story.ts`, `mock/rules.ts`, `mock/mock-only.json`, `web/scripts/sync-mock.mjs` (`SYSTEM_KEYS`).
- Tests: scenario tests (`coop_scenario_has_lower_inequality_than_market`), `web/tests/mock-sync.test.ts`, `panels-logic.test.ts`.

## 7. Add a firm or a sector

Real example: the **Shop** (Services) added in Town v2 (commits `4fb53fc` to `e9b4bc7`, `aa5939c`; deviation T21).
- Firm only (same sector): `content/initial_town.json` `firms` and `params.initial.firms`; `MAX_FIRMS` in `crates/sim-types/src/lib.rs` is 4 (slots Factory, Farm, Shop, one dormant); raising it touches every per-firm array, the map reservation (`initial_town.json` `map`), `persist.rs` checks, wire `firms[]`, and the web scene (`scene/entities.ts` places firms by tile).
- New sector: `Sector` in `sim-types/src/enums.rs`; production and goods rules (`sim-model/src/production.rs`, `goods.rs` spending shares such as `goods.services_share`); `class.rs` founding rules; `SectorKey` in `sim-api/src/keys.rs`; web `Sector` in `contract/index.ts`, firm model/colour in `scene/entities.ts` and `scene/models.ts`, labels in `panels/inspect.ts`; mock `rules.ts`/`dynamics.ts`.
- Retune start values by running the CLI, not by guessing; record what you tried in `docs/tuning-log.md`. A small town is noisy: one firm closing swings unemployment (`deviations.md` T3, `tuning-log.md`). Check baselines A-D.

## 8. Change a parameter safely

- Where: `content/params.json` (rules and costs), `content/initial_town.json` (the start: people, households, tiles, firms, policy), `content/systems/*.json`, `content/levers.json` (UI metadata only: labels, ranges, arrows).
- Try first without editing: `cargo run -p sim-cli -- --seed 42 --set labour.wage_up=1.01` (prefix `town.`, `market.`, `regulated.` or `coop.` for the other files).
- Edit, then: `cd web && npm run sync-mock`; `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`; `cargo test --workspace`; `cd web && npm run build:wasm && npm test`. Some tests hard-code values (for example `web/tests/mock-sync.test.ts` pins `GEN.pc`, `GEN.carbon.budget`, `policyStart`; `tests/scenarios.rs` checks start Gini and baselines). Update them only if the change is intended.
- Judge on 20 seeds and baselines A-D, not one run. Record the change and result in `docs/tuning-log.md`. Do not tune just to hit a target (`AGENTS.md`).

## 9. Add a lens

Real example: the `commute` lens (commit `015d3bc`, scene notes in `docs/web-notes-scene2.md`).
1. `web/src/store/types.ts`: add to the `Lens` union and `LENSES` list.
2. `web/src/scene/lenses.ts`: add a ramp to `RAMP`, cases in `entityTints`, `tileTints`, `personTints` and `legendFor` (pure colour functions: snapshot in, hex out; no Three.js).
3. **`web/src/store/save.ts`: the `LENS` allow-list in `parseSave`.** It currently lacks `'commute'` (see `docs/known-issues.md`); add new lenses here or a save made with that lens will not load.
4. `web/src/panels/ViewBar.svelte` reads `LENSES`; check the legend renders.
Tests: `web/tests/scene-lenses.test.ts`, `scene-lenses2.test.ts`. Commands: `npm test`, `npm run typecheck`, `npm run lint`.

## 10. Change the contract (the wire)

1. Decide if it is breaking (a removed or reshaped field, a changed meaning, a changed `State`): then bump `SCHEMA_VERSION` in `crates/sim-types/src/lib.rs` (the api re-exports it) and `SAVE_VERSION` only if the browser save wrapper changed. Old saves are refused with `schema_mismatch`; the mock's constant follows (`mock/snapshot.ts`).
2. Rust: shapes in `crates/sim-api/src/{snapshot,turn,events,command,keys,meta,history}.rs`. Keep camelCase fields and snake_case tags. No logic: convert and validate only.
3. TypeScript: `web/src/contract/index.ts` (types only; imports nothing). Then the mock (`web/src/sim-port/mock/*`) and `sim-port/wasm/index.ts` if the call surface changed (`SimPort` in `port.ts`, and `crates/sim-wasm/src/lib.rs` for a new method).
4. Fixtures: `cargo run -p sim-api --example fixtures`; `web/tests/fixtures.test.ts` pins key sets against the TS types, so it fails until the type and fixture agree. Golden: `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`.
5. Docs: add a section to `docs/snapshot-contract.md` (next number after 11, saying which sections it supersedes) and a "Town v2" style entry in `docs/sim-api-notes.md` for details; update `docs/known-issues.md` if you leave a wart.
6. Run the whole checklist, including `npm run build:wasm` before `npm test` (otherwise the wasm golden test is skipped, not run).

## 11. Add a scene mesh

Real example: the Shop and tent models in commit `aa5939c`.
1. `web/src/scene/models.ts`: define the model (a list of boxes/primitives merged by `geometry.ts::buildModel`), give it a `ModelId`.
2. `web/src/scene/entities.ts`: decide when an entity uses it (`planEntities`: pure function of snapshot + layout, so unit-testable without WebGL).
3. `web/src/scene/entityLayer.ts` draws descriptions; colours come from `palette.ts`; lens tints from `lenses.ts`.
4. Test the planning, not the pixels: `web/tests/scene-entities.test.ts`. For a visual check run `npm run dev` and `node playwright/scene-shots2.mjs <outDir>` (manual; needs `npx playwright install chromium`).
The scene may import only three, store and contract. Anything the scene needs from panels goes through `store/scene-bus.ts`.

## 12. Add a panel

Real example: `PeoplePanel.svelte` (commit `7ae59a0`).
1. `web/src/panels/<Name>.svelte`; pure logic in a sibling `.ts` file (so it can be tested without a DOM), e.g. `people.ts`. Read state via the store (`game.state`); act via `game.*`. Do not import `scene`, `three` or `sim-port`.
2. Export it from `web/src/panels/index.ts`; mount it in `web/src/app/App.svelte`.
3. Generic widgets come from `web/src/ui-kit`; tokens in `ui-kit/tokens.css` (light and dark).
4. Tests: logic in `web/tests/panels-logic.test.ts` / `panels-v2.test.ts`; components in `panels-components.test.ts`. Keyboard/a11y: check with `web/playwright/panels-a11y.mjs` (manual).
5. New store action needed? Add it to `store/game.ts` (+ `Game` interface) and test in `web/tests/game.test.ts`. New cross-component state belongs in the store, never in a module-level variable shared by scene and panels.

## 13. Try Pressure mode (Town v3)
1. `npm run dev` in `web/`, open `http://localhost:5173/?mode=pressure` (add `&pace=0.02` to make a quarter about 2 s; `pace` is real seconds per game day, default 0.143 = about 13 s per quarter). A plain visit shows the New game screen with Relaxed preselected; `?seed=` or `?mode=` links start straight away.
2. Other overrides: `timeout=submit|discard|skip`, `budget=<seconds of pause>`, `pause=0`. Defaults live in `web/src/store/clock-config.json`.
3. Pressure needs the live-turn calls (`begin_turn`, `advance_month`, `finish_turn`) in the wasm package; rebuild with `npm run build:wasm` if the New game screen says it is unavailable. The mock always has them (`?sim=mock`).
4. Tests: `web/tests/clock.test.ts` (state machine, fake time), `live.test.ts` (game loop), `live-turn.test.ts` (mock equals `submitTurn`), `panels-v3.test.ts`.
