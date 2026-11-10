# Known issues and technical debt

Consolidated and deduplicated from `balance-backlog.md`, `deviations.md`, `tuning-log.md`, `integration-notes.md`, `sim-api-notes.md`, `web-notes*.md` and a verification pass when this file was written. Status: **open**, **resolved**, **doc-fixed** (a stale statement was corrected), **unverified** (carried over from older docs, not re-checked against the Town v2 code). Balance items are design questions: do not "fix" them by tuning to a number (see `AGENTS.md`).

Verified clean at the time of writing: `docs/fixtures/*` regenerate with no diff, `tests/golden/seed7.json` passes natively and in wasm, `data.generated.json` is current. Nothing is currently stale.

## Balance and model

| # | Issue | Status | Source |
|---|---|---|---|
| B1 | **Inequality does not rise in the no-action run (baseline A).** Seed 42 goes Gini 0.72 -> 0.63 and top share 61% -> 41% by year 40 (managers and workers save, the rich spend savings). The design said inequality should rise. Reported, not forced | open | tuning-log.md (Town v2) |
| B2 | **(Resolved by Town v4 weather: S11 draws from the RNG once stress is above zero, so seeds differ in fires and droughts; the economy itself still barely depends on the seed before turn 10.)** Old text: Seeds barely matter. RNG is used only for commons enclosure and firm founding, both rare. 20 seeds of baseline A exhaust the budget within one tick of each other (turn 18.7 to 19.0 with the 600 t budget). The 20-seed distribution tests are real but narrow. No shocks or random separations exist | open | deviations.md T23 |
| B3 | (Numbers before the 600 t budget; D now exhausts it at turn 19.) Baseline D (carbon tax only) "fails" only marginally (legitimacy zero around turn 154, 6 turns early; median budget left 5%). Plants stay fossil | open | tuning-log.md |
| B4 | (Superseded: with the 600 t budget C now ends by the budget at turn 17.) Baseline C fails by legitimacy zero at turn 31-33, not by the carbon budget (budget exhausted at 132-135). Fails earlier than A as designed, but for a different reason than the carbon story | open | tuning-log.md |
| B5 | Fossil plants added in C change nothing: supply (10) already exceeds demand (8 to 9.5), so a plant matters only when demand outgrows capacity. Removing a plant rations output | open | tuning-log.md, deviations.md item 46 (pre-v2 form) |
| B6 | Start is not a fixed point: 48 warm-up ticks run; `goods.wealth_consumption_rate` 0.005 is "a compromise, not a fixed point" | open | tuning-log.md |
| B7 | Tiny population: one firm closing swings unemployment 10-30 points; mitigated by `unemployment_smooth` and hiring/firing bands (hire 1, fire 1 a month, 3-month wait). Baseline B has 1 of 20 seeds collapsing on mass unemployment after a firm default | open (mitigated) | deviations.md T3, T10; tuning-log.md |
| B8 | Minimum wage sign test no longer asserts a higher wage share (ambiguous in a 3-firm town); asserts the floor binds and unemployment does not fall. A floor of 1,700+ gives 43-89% unemployment | open (test weakened on purpose) | deviations.md T22 |
| B9 | The Shop's second-firm founding rule needed `startup_min_excess_demand` 0.2; founding is rare and did not occur in the reported 20-seed runs | open | tuning-log.md |
| B10 | Government debt/GDP drift, extraction not punished by the economy itself, wage-price spirals when tax/benefit changes are expansionary | unverified (pre-v2 numbers; Phase 2 "Open problems") | deviations.md |
| B11 | Wealth tax inert at default threshold | resolved for v2: threshold is 150,000 (about twice the average household) | deviations.md T18 |
| B12 | Phase 2 items (balanced start, deflation, lumpy green investment, budget calibration, extra demand lever) | resolved (and numbers redone for v2) | balance-backlog.md |
| B13 | Budget `B0` 4,400 t was not recalibrated after the 1/50 scaling; A exhausts it at turn 123 (target 90 to 140) | resolved: B0 is now 600 t for the Paris 2030 horizon (A exhausts at turn 18 to 19) | tuning-log.md |
| B14 | **(Updated by the Horizon pass: 40 turns, 900 t.) Baseline B keeps a carbon margin (never exhausts the budget; 18 to 20% left when the game stops) but the town collapses at turn 22 to 26 (bank failure or mass unemployment), because climate damage to farm output (`production.climate_damage` x (cumulative / b0)^2) fails the farm; with the weather off it collapses at turn 30; with `climate_damage` 0 it survives. B survives only with a budget of 1,500 t or more, which lets A survive too. A decision for the user: rebalance the farm and climate damage, or accept B as buying time. See tuning-log "Horizon".** Old text: **Sensible play (baseline B) cannot win the 600 t, 160-turn game**: the budget is gone at turn 25 whatever is built, because lags (carbon price 2 turns, plant 4 turns) delay the cut. Options (shorter `run.turns`, bigger `B0`, faster response) are in the tuning log | open (decision pending) | tuning-log.md Carbon budget horizon |
| B15 | `tests/golden` plan plays 26 turns but the game now ends near turn 23 (with the 900 t budget and 40 turns it ends near turn 27, so most of the plan plays), so the native/wasm golden compares only the turns that played (later turns are identical game-over errors) | open (documented) | crates/sim-wasm/tests/golden.rs |

## Contract and wire

| # | Issue | Status | Source |
|---|---|---|---|
| C1 | `flows.commutes` has no tile path (`from`, `to`, `tiles`, `minutes`); `from` is null for the unhoused household. The scene draws its own route; the unhoused "camp tile" is a scene invention | open | sim-api-notes.md, web-notes-scene2.md |
| C2 | `Tile.buildable` is static (empty tile only); only `validTiles(kind)` / `preview().tileValid` know about queued builds. `validTiles` is identical for all four kinds (no per-kind terrain rules) | open | sim-api-notes.md |
| C3 | `Command.remove` carries only a tile, so its price depends on what stands there; a wire-invalid remove previews with `remove_road`'s lead time | open | sim-api-notes.md |
| C4 | `ActionInfo.moneyCost` (static table in `meta.rs`) and `preview().money` (`turn.rs::build_cost`) read the same params through two code paths; nothing forces them to agree. The UI now shows `preview()` and labels the table value "Est." | open (mitigated in UI) | web-notes-panels2.md |
| C5 | Contract under-specified for Town v2: real types carry extras (`inLabourForce`, `jobGuarantee`, `effectiveHours`, `plants[]`, `needsTile`, `tileRule`, ...) that section 11 of the contract does not list | open (code is the truth) | web-notes-v2.md |
| C6 | `HistoryPoint.emissions` unspecified (net monthly tonnes used); average/median income per person have no unit in the contract (per month); no evictions/moves count in history | open | integration-notes.md, web-notes-panels2.md |
| C7 | `gameOver.turn`: collapse = outcome tick / 3 + 1, rest of that turn still plays; survived = 161 (last played turn is 160). UI uses `min(gameOver.turn, lastTurn)` | open (documented oddity) | integration-notes.md |
| C8 | Community commons cap is not visible in the snapshot; `availableIn` cannot say "commons_access `open` only in Market" (`optionAvailableIn` covers it); some levers have no arrows (no documented sign) | open | sim-api-notes.md |
| C9 | `household_moved.to` is a home id; `built`, `removed`, `electricity_rationed` events and `plants[]` are additions beyond contract section 11; `tileReason` typed `string`; sink emissions are negative | open (documented) | sim-api-notes.md |
| C10 | `LeverInfo` has no unit label for plain `usd` levers (cannot say "per month" or "per hour"); `TraceRecord` has no period | open | web-notes-panels.md |
| C11 | `flows.purchases` always empty; `nature.forest` raw stocks; schema 1 vs 3 mismatch; accepted `PendingItem` lacking `tile`; `costModel` absent | resolved in Town v2 / integration | web-notes-align.md, integration-notes.md |
| C12 | `content/levers.json` (real metadata) and `web/src/sim-port/mock/mock-only.json` (hand-authored mock lever presentation) are two copies of lever labels, ranges and arrows. Adding a lever means editing both; a test checks only part of this | open | how-to.md; web/tests/mock-sync.test.ts |

## Code and tooling

| # | Issue | Status | Source |
|---|---|---|---|
| T1 | **`web/src/store/save.ts` `parseSave` allows lens values `none, emissions, class, wealth, employment, flows` but not `commute`** (`Lens` in `store/types.ts` has it). By reading the code, a save made with the Commute lens is refused ("damaged view settings"). Not reproduced in a browser | open (found while writing docs) | own finding |
| T2 | CI (`deploy.yml`) does not run `cargo fmt --check` or `cargo clippy`; both pass locally now | open | own finding |
| T3 | `.idea/` files are tracked in git | open | git ls-files |
| T4 | The Vite build warns that a chunk exceeds 500 kB (Three.js and the app in one bundle) | open | own finding (`npm run build`) |
| T5 | Playwright scripts (`web/playwright/*.mjs`) are manual, not in `npm test` or CI, and need a dev server and a browser download | open | web notes |
| T6 | `wasm-opt` is not installed by default; the wasm is 1,376 KiB raw / 347 KiB gzipped without it | open (cosmetic) | integration-notes.md |
| T7 | The wasm golden web test is silently skipped when `pkg/` is not built, so `npm test` can pass without exercising it | open | web/tests/wasm-golden.test.ts |
| T8 | The scenario insta snapshot refresh command in these docs (`INSTA_UPDATE=always`) was not run; `cargo-insta` is not installed | unverified | own finding |

## Docs

| # | Issue | Status | Source |
|---|---|---|---|
| D1 | `docs/sim-api-notes.md` (v0.3 half) quoted Town v1 costs: housing 14,000, plant 2,000,000; real values are 40,000 and 40,000 (`content/params.json` `government`) | doc-fixed (inline note) | web-notes-panels2.md |
| D2 | `docs/integration-notes.md` and `docs/web-notes-align.md` say real `schemaVersion` is 1 and the mock's 3; both are now 2 | doc-fixed (banners) | integration-notes.md |
| D3 | `docs/deviations.md` T2 says `sim-api` and `sim-wasm` are commented out of the workspace; they were restored (`Cargo.toml` lists them) | doc-fixed (inline note) | deviations.md |
| D4 | SPEC.md body still describes cohorts, UI-owned map and an older tick order; sections 17 and `docs/town-v2.md` supersede them | doc-fixed (banner), body intentionally unchanged | SPEC.md |
| D5 | `docs/ui-design.md` "Decisions" still say the UI owns the map; the amendment at the bottom withdraws it | doc-fixed (banner) | ui-design.md |
| D6 | `docs/sim-api-notes.md` Town v2 section says `Api::valid_tiles` result is "the same for all four kinds today" and that `buildings.road` exists; both match the code now | verified | sim-api-notes.md |

## UI and scene (rough spots)

| # | Issue | Status | Source |
|---|---|---|---|
| U1 | Inspector is a card over the map; the household card is long and scrolls at 1024x768. People tab at 280 px is dense; no sort/filter, no wellbeing breakdown or commute chart | open | web-notes-panels2.md |
| U2 | Removing occupied public housing is only reachable from the tile or vacant-home card, not the household card | open | web-notes-v2.md, panels2 |
| U3 | End story evictions only cover the final turn (no per-game count on the wire) | open | web-notes-panels2.md |
| U4 | Wealth-tax threshold shows as money; the lever range (to $5M) is mostly beyond this town | open | web-notes-panels2.md |
| U5 | A11y: focus is not moved into the inspector on select; inspector close button has no Escape handler. Dark theme tokens exist but were not visually reviewed | open | web-notes-panels*.md |
| U6 | Briefing/lobbying text: notes now only appear when nominal and effective differ | resolved | web-notes-v2.md, commit 77720a6 |
| U7 | Scene: each building is its own draw call (about 38); roads have square corners; pending roads read as pale grey squares at distance; commute pace is "share of the month" and people vanish at the doorstep; dirt paths are decorative, not the sim's route; closure and construction pops are approximated (the store holds the final snapshot during playback); marker shapes small at default zoom | open | web-notes-scene*.md |
| U8 | `personHover` on the scene bus is not read by panels; `EntityKind` has no `person` | open | web-notes-scene2.md |
| U9 | Frame time about 33-40 ms during playback (software GL; hardware not measured) | open (unmeasured on hardware) | web-notes-scene2.md |
| U10 | Not verified in a browser: touch input; the wasm `valid_tiles` path at the time of v2 foundation (later covered by golden) | open | web-notes-v2.md |

## Resolved milestones (for orientation)

Cohorts replaced by people and households; the map moved into the sim; roads; plants that emit; the Shop; ten-system doc reconciled; wire and wasm ported to schema 2; scene and panels v2; GitHub Pages deploy.

## Town v4 (weather)

| # | Issue | Status | Source |
|---|---|---|---|
| W-1 | (Resolved by the Horizon pass: weather starts at 75% of the budget left, about 16 events in a no-action game, an event within 3 turns of onset in 19 of 20 seeds. It now moves baseline B's collapse about 5 turns earlier. Old text: weather was mild, about one fire and one drought per game.) Stress stays at 1 once the budget is gone | resolved (by design) | tuning-log.md Town v4 |
| W-2 | Forest stock is a scalar; fire tiles are a view, and `regrowing` is a display state that does not slow regrowth beyond the logistic rule | open (deviations W3) | deviations.md |
| W-3 | `fire_sink_loss` in `flows.emissions` is informational; a client that sums all emission lines would double count the sink loss | open (documented in sim-api-notes) | contract section 13 |
| W-4 | No `Buildings.fireStation` count and no wire field for fire-station coverage; clients derive both from tiles | open | sim-api-notes.md |
| W-5 | `home_damage_prob` is implemented but off; no eviction risk, and a damaged home keeps its tenant | open (optional in the design) | deviations W13 |
| W-6 | The native-vs-wasm golden hashes every reply, so the weather block, forecast and tile states are covered every turn; whether a fire or drought actually occurs in the seed 7 plan was not checked | open | tests/golden |
| W-7 | Web tests pinning 19 levers / schema 2 / fixture key sets fail until the web side is updated (`wasm-golden.test.ts` fails today on the lever count) | open (web agent) | this commit |

## Horizon (40 turns, 900 t)

| # | Issue | Status | Source |
|---|---|---|---|
| H-1 | Hard-coded 160-turn text remains in docs: `AGENTS.md` (fixed), `docs/glossary.md`, `docs/ui-design.md` ("Turn 1/160" mock-up), `docs/balance-backlog.md`, `docs/sim-api-notes.md` item 10, `docs/town-v3.md`, and protected `SPEC.md` / `docs/causal-model.md`. The web reads `game.lastTurn` (the top bar shows Turn n/40); the start screen copy was changed to "10 years" | open (docs only) | grep |
| H-2 | The brief's "an event within 3 turns in 90% of seeds" and "3 to 6 events by exhaustion" conflict; the shipped weather has 16 events in a no-action game | open (decision) | tuning-log Horizon |
| H-3 | `web/src/sim-port/mock/mock-only.json` still says `onset` 0.5 and `web/src/store/weather.ts` `WEATHER_UI` keeps the old onset/full/curve as the fallback for a package without the new wire fields; both are overridden by the real numbers | open (harmless) | |
| H-4 | There is no web URL option to force weather stress; use `sim-cli --set weather.force_stress=0.7` | open | how-to |
| W-8 | (Was contract issue 1 in web-notes-v4) The stress curve is now on the wire (`weather.onset`, `full`, `curve`); `coverage_radius` and `smoke_radius` still are not | partly resolved | sim-api-notes |
