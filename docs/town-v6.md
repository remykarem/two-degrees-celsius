# Town v6: DRY cleanup, the web is a thin renderer

Status: **APPROVED (user: "go ahead, call it v6, move UI bands to content"). IMPLEMENTED.** No new features and no simulation change: the state hash (`cargo run -p sim-cli --example statehash`) is byte-identical before and after. The contract grows additively to v1.8 (`docs/snapshot-contract.md` section 18). The previous v6 draft (daily ticks) is now `docs/town-v8.md`.

## 1. Principle [DECIDED]
The web UI is a thin renderer. Simulation rules, assumptions, model descriptions, thresholds and explanatory text live on the Rust and content side (`content/*.json`, `sim-api` catalogs) and reach the web over the wire. The web keeps layout, formatting, animation, chart scaling, onboarding copy and the wording of wire ids (event sentences, block-reason text).

## 2. What an audit found, and what was done
An independent read-only audit listed ten problems. Each was checked against the code first. "Real" means confirmed.

| # | Finding | Verdict | Done |
|---|---|---|---|
| 1 | `WEATHER_UI`/`stressAt` hard-code onset 0.5, curve 1.5 (content: 0.75, 1.0), radii 3 and 2 | Real | `weather.coverageRadius`, `smokeRadius` on the wire; `WEATHER_UI`, `stressAt` and the fallbacks deleted; the scene and panels read the snapshot |
| 2 | `story.ts` says "more than half of its carbon budget" with 0.5 and 0.25 | Real | Milestones are `weather.onset` and `weather.full` from the snapshot; text in `ui.json` with placeholders |
| 3 | `panels/content/systems.ts` retypes `content/systems/*.json` numbers into prose | Real | `catalog().systems`: one card per system, numbers read from `Params.systems`, prose in `ui.json`; the file is deleted |
| 4 | `lobbyNote` used `current + (drafted - current) x (1 - opposition)`; the sim uses `nominal x (1 - net_lobby)` (a cap is divided) | Real | `CommandPreview.expectedEffective` from `sim_model::lobbying::expected_effective`; the web makes no claim without it |
| 5 | Model text hard-coded in the web | Real (partly moved) | Metric hints, finance flow labels and hints, GST reason, energy unit help (the invented 0.1 to 0.25 range is gone), inspector notes, crisis copy, weather text, ledger kinds and notes, infra, class and owner labels (triplicated, now one table) are in `content/ui.json`. Kept in the web: coach and onboarding copy, event and block-reason wording; tests check every wire `BlockReason` and `SimEvent` kind is worded. The rest is listed in `known-issues.md` V6-3 |
| 6 | Mock lever table re-typed; labels and arrows differ | Real | `sync-mock` builds the lever table and action labels from `content/levers.json`; only invented effect sizes remain (`leverEffects`). A parity test compares the mock's levers, actions and snapshot, finance and energy key sets with `docs/fixtures` |
| 7 | The literal 3 (months per turn) in `metrics.ts`, `finance.ts`, `ClockStrip.svelte`, `live.ts` | Real | `game.ticksPerTurn` on the wire; the runway is computed on the Rust side (`breakdowns.emissions.runway`); the Pressure clock takes the calendar from the snapshot |
| 8 | `LEVER_GROUPS` in `policy.ts` duplicates groups | Real | `LeverInfo.group` plus the order and blurbs in `catalog().leverGroups` |
| 9 | Household smoke is an invented 0.1 x gross; the fire lens is a hand-built heuristic | Real | Smoke reads `breakdowns.emissions.month.households`. `Tile.fireRisk` (the model's ignition and spread odds per standing forest tile) feeds the lens, scaled to the riskiest tile |
| 10 | UI-invented bands | Partly real | Moved to `content/ui.json` `bands` and served: crisis 0.2 and 0.15, health 0.6, 0.35, 0.15, stress words, severity words, community cap 0.5, capital "tight" factor 1.25, the budget meter's low and high. The audit said `TopBar.svelte` held crisis thresholds; it held only the budget meter's 0.25 and 0.5 |

Also: the engine's bank-cascade threshold (a literal 0.2 in `check_outcome`) moved to `params.government.bank_cascade_unemployment`, and the mock reads it. `estimatePc` in `panels/costs.ts` stays as a labelled fallback, with a test that it equals the sim's `preview().pc` when opposition is 0. `docs/architecture.md` no longer mentions the removed `sim-port/wasm/upgrade.ts` as live.

## 3. Design
- `content/ui.json` (new): `bands`, `lever_groups`, `text` (group, key, string), `system_cards`, `system_param_labels`.
- `crates/sim-api/src/catalog.rs`: parses it, resolves the system cards against `Params`. `Api::catalog()`, `WasmSim.catalog()`, `SimPort.catalog()`.
- `web/src/store/catalog.ts`: the loaded catalog (`createGame` fills it), `text()`, `bands()`, `wordOf()`, `fill()`. A missing key shows as `?group.key`; a test scans the source for every `text('group', 'key')` and fails on a missing one. `tests/setup.ts` loads the real reply from `docs/fixtures/catalog.json`.
- Additive wire fields: `game.ticksPerTurn`, `weather.coverageRadius`, `weather.smokeRadius`, `Tile.fireRisk`, `CommandPreview.expectedEffective`, `breakdowns.emissions.runway`.

## 4. Rules kept
Money, RNG and no-`HashMap` rules unchanged; the new Rust functions read state and draw nothing. No bare percentages in new text (placeholders carry numbers). Module boundaries unchanged (`tests/boundaries.test.ts`).

## 5. Verification (this session)
See the final report of the work and `docs/known-issues.md` "Town v6". State hash identical; Rust and web suites, lint, typecheck and build run; the real simulation (wasm) was screenshotted for Ecology, the system modal, policies, metrics, the fire lens and the end screen (`web/playwright/v6-shots.mjs`).
