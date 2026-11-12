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
| B7 | Tiny population: one firm closing swings unemployment 10-30 points; mitigated by `unemployment_smooth` and hiring/firing bands (hire 1, fire 1 a month, 3-month wait). The same lumpiness makes win or lose a cliff: a plan that adds public housing, a third plant or resilience of 2,000 a month collapses in 15 to 20 of 20 seeds (government borrowing, wage-price squeeze, a services firm defaults) while the shipped B wins in 20 of 20 | open (mitigated) | deviations.md T3, T10; tuning-log.md |
| B8 | Minimum wage sign test no longer asserts a higher wage share (ambiguous in a 3-firm town); asserts the floor binds and unemployment does not fall. A floor of 1,700+ gives 43-89% unemployment | open (test weakened on purpose) | deviations.md T22 |
| B9 | The Shop's second-firm founding rule needed `startup_min_excess_demand` 0.2; founding is rare and did not occur in the reported 20-seed runs | open | tuning-log.md |
| B10 | Government debt/GDP drift, extraction not punished by the economy itself, wage-price spirals when tax/benefit changes are expansionary | unverified (pre-v2 numbers; Phase 2 "Open problems") | deviations.md |
| B11 | Wealth tax inert at default threshold | resolved for v2: threshold is 150,000 (about twice the average household) | deviations.md T18 |
| B12 | Phase 2 items (balanced start, deflation, lumpy green investment, budget calibration, extra demand lever) | resolved (and numbers redone for v2) | balance-backlog.md |
| B13 | Budget `B0` 4,400 t was not recalibrated after the 1/50 scaling; A exhausts it at turn 123 (target 90 to 140) | resolved: B0 is now 600 t for the Paris 2030 horizon (A exhausts at turn 18 to 19) | tuning-log.md |
| B14 | **Resolved by 'Winnable' (plan B rewritten with adaptation; no parameter changed).** Baseline B now survives all 40 turns in 20 of 20 seeds (about 30% of the budget left). The old B collapsed at turns 22 to 26 because it predated weather and used no adaptation tool (fire stations); see tuning-log 'Winnable: plan B with adaptation'. History: with the 600 t, 160-turn game B could not win by carbon alone (lags), and in the first Horizon pass the town collapsed from climate damage to the farm | resolved | tuning-log.md Winnable |
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
| C12 | `content/levers.json` (real metadata) and `web/src/sim-port/mock/mock-only.json` (hand-authored mock lever presentation) are two copies of lever labels, ranges and arrows. Adding a lever means editing both; a test checks only part of this **Closed:** `mock-only.json` and the mock are deleted (platform cleanup phase 2). | closed (phase 2: mock deleted) | how-to.md; web/tests/mock-sync.test.ts |

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
| H-3 | `web/src/sim-port/mock/mock-only.json` still says `onset` 0.5 and `web/src/store/weather.ts` `WEATHER_UI` keeps the old onset/full/curve as the fallback for a package without the new wire fields; both are overridden by the real numbers **Closed in part:** the mock file is gone. | closed (phase 2: mock deleted) |  |
| H-4 | There is no web URL option to force weather stress; use `sim-cli --set weather.force_stress=0.7` | open | how-to |
| W-8 | (Was contract issue 1 in web-notes-v4) The stress curve is now on the wire (`weather.onset`, `full`, `curve`); `coverage_radius` and `smoke_radius` still are not | partly resolved | sim-api-notes |

## Winnable (plan B with adaptation)

| # | Issue | Status | Source |
|---|---|---|---|
| WN-1 | **Baseline F (renewables only: two plants, both fossil plants closed) survives in 13 of 20 seeds (65%)**, so "a single tool must not be enough" holds for D (carbon tax: 0 of 20 survive) and E (adaptation only: 0 of 20) but not for F. The grid is about half of the emissions; closing it leaves about 640 t (71% of the budget). The 7 failures are mass unemployment after fires and droughts hit an unprotected farm. Not forced by tuning: raising `climate_damage` is not monotonic (F 13, 11, 6, 9 of 20 at 0.3, 0.4, 0.5, 0.6) and stronger weather tips B over a cliff too (`a_f` 0.5 with `fire_base` 0.14: B and F both 0 of 20). A decision for the user | open (decision) | tests/scenarios.rs F test |
| WN-2 | B wins near a cliff: more public housing, a third renewable plant, resilience spending of 2,000 a month or a public-spending raise each turn B into a collapse in 15 to 20 of 20 seeds, and `weather.a_f` 0.5 together with `fire_base` 0.14 does the same. Spending is paid by government borrowing from the bank; a player who spends freely loses. Intended as a lesson but it is also a fragile balance | open (documented) | tuning-log Winnable |
| WN-3 | Community-managed commons needs the Regulated or Coop system (a 45 PC switch); plan B stays in Market and uses the harvest cap instead, so that lever is not exercised by a Market-only plan | open (by design) | content/levers.json |

## Town v5 (ledgers, finance, energy)

| # | Issue | Status | Source |
|---|---|---|---|
| L-1 | **`wealth_consumption` is not a separate money path.** The wealth effect only raises a capitalist's or rentier's discretionary budget in S3 and is spent as ordinary food, goods and services purchases, so the ledger kind exists but is never emitted and the wealth effect cannot be shown on its own without a model change | open | sim-api-notes Town v5 |
| L-2 | **Firm revenue in the model includes self-purchases.** A firm that buys capital goods from itself (the Factory is the only goods firm) records the sale in `firm.revenue` but moves no money (`Deltas::transfer` skips from == to). The ledger's revenue and `investment` are money received and paid, up to ~10% below the model's `revenue` | open (documented) | goods.rs, finance.rs |
| L-3 | `household.income.wages` in the model (and so the Finance "wages") includes lobbying fees, public-works labour, green-subsidy labour and the other labour-like transfers that the model books as `FlowKind::Wage`; the ledger separates them by kind and detail | open (documented) | apply.rs `record_flow` |
| L-4 | Job-guarantee pay reaches households as one government transfer, not through a person's wage patch, so it is in the household ledger but not in any person's earnings | open | ledger.rs |
| L-5 | The save blob is about 1.1 MB after 28 turns (state 21 KB, history 18 KB, ledger 1.07 MB: 2,300 detail entries and 5,600 roll-up rows); about 1.5 MB by turn 40. Browser `localStorage` allows roughly 5 MB, so one save fits but a second slot may not. Lowering `ledger.detail_turns` shrinks it (about 36 KB per detail turn) | open | `Api::export_state` |
| L-6 | Rolled-up periods keep the kind but not the detail label ("wage tax" becomes "tax"); roll-up rows are per month, not per transaction | open (by design) | ledger.rs |
| L-7 | Energy magnitudes: household use (0.17 to 0.19 per person-month) and firm use (about 1 per firm-month) are plausible as MWh; a 5.0-per-plant capacity is about 7 kW, a rooftop array, so "MWh" holds for demand but plants are toy-sized. Not rescaled (a balance change) | open (decision) | sim-api-notes |
| L-8 | The ledger starts at turn 0 after the 48 warm-up ticks; warm-up transactions are discarded, so opening balances are the settled start, not a round number | open (by design) | init |
| L-9 | Web tests pinning `schemaVersion` 3, the old `energy` shape, `wasm-golden` key sets and fixture counts fail until the web side moves to contract v1.4 (the golden now has a `ledger` object) | open (web agent) | this work |
| L-10 | **Wage share has two definitions.** `indicators.wageShare` is household wages over monthly GDP (the Economy metric, and `breakdowns.wageShare`); `finance.wageShare` is the firm wage bill over wage bill plus profit (Finance tab). They differ by design but are both labelled "wage share" | open | breakdowns.rs, finance.rs |
| L-11 | **The headline emissions metric (`nature.grossEmissionsMonthly`, labelled "Emissions (per month)") is gross; the history `emissions` series and the budget are net.** The tally is fixed on the wire (`breakdowns.emissions`); the UI should show net (or label gross) next to the lines | open (web) | docs/sim-api-notes.md |
| L-12 | A firm that closes in the credit step has its end-of-tick `emissions` zeroed after the nature step already counted it in that month's gross (from the code; not observed in a run). Sums over `snapshot.firms[].emissions` can therefore be lower than the month's tally; `breakdowns.emissions` uses the per-tick trace and is exact | open (by design) | credit.rs, nature.rs |
| L-13 | The legitimacy breakdown shows the terms the model applied in the latest month (recorded from the tick). Before the first month the terms are what month 1 would apply, not part of the sum. System-change resistance appears in `parts`, not in `terms` (the contract lists five term ids) | open | breakdowns.rs |
| L-14 | The wrapper log adds one record per tick to the save blob (about 120 by turn 40); its size was not measured | open | log.rs |


## Town v6 (thin renderer)

| # | Issue | Status | Source |
|---|---|---|---|
| V6-1 | **Fixed (platform cleanup).** The wire has no `hired` or `fired` events (cohort-era); the web handling, `mergeEvents` and their tests were removed | fixed | `web/tests/thin-renderer.test.ts` |
| V6-2 | The mock's breakdowns, policies and industry lack some optional detail the real wire has (`parts`, `sumsToResult`, `monthLines`, tax-row notes, `cancelRefund`, `active`, `upkeepEachMonthly`). The parity test lists them (`KNOWN_MOCK_GAPS`) so a new difference fails **Closed:** the mock is deleted. | closed (phase 2: mock deleted) | `web/tests/thin-renderer.test.ts` |
| V6-3 | Text that is still in the web: onboarding and coach copy (allowed), event and block-reason sentences (wording of wire ids, allowed), the row labels and section titles of the Finance tab other than flows, `panels/backing.ts` and `panels/explain.ts` unit words, `why.ts` kind labels, the scene's class-marker legend. Model facts among them are candidates for `content/ui.json` | open | own finding |
| V6-4 | `Tile.fireRisk` ignores fires that burn out before the next ignition roll, so right after a fire it slightly overstates spread risk. It is the odds as they stand, not a prediction | open (documented) | `crates/sim-model/src/weather.rs` |
| V6-5 | The mock's per-tile `fireRisk` and weather dynamics are invented (labelled in `mock-only.json`); only the numbers in `content/params.json` `weather` are real **Closed:** the mock is deleted; the recorded demo is real sim output. | closed (phase 2: mock deleted) | `web/src/sim-port/mock/weather.ts` |
| V6-6 | Any params change invalidates saves (`params_mismatch`); the new `government.bank_cascade_unemployment` did so once | by design | architecture.md section 7 |

## Town v7 (the estate)

| # | Issue | Status | Source |
|---|---|---|---|
| V7-1 | **Sensible play (B) wins with a thin carbon margin and low legitimacy.** The estate adds about 170 t over the game (household 0.5 t, the pilot's 0.12 t, the jet's expected 0.8 t a month). B still survives 20 of 20 seeds but ends with about 10% of the budget left (was 30%; 7% to 12% across seeds) and its minimum legitimacy is 0.24 (was 0.61): the Gini is now about 0.88 against the 0.45 threshold of `government.legit_gini`. A, no action, exhausts the budget at turn 24.3 (was 27.3). Not retuned (the design says `nature.budget_b0`, `estate.flight_prob_monthly` and `government.legit_gini` are the user's call); what each would change is measured in `docs/tuning-log.md` "Town v7" | open, decision needed | `docs/tuning-log.md`, `docs/balance-backlog.md` |
| V7-2 | **Baseline F (renewables only) now survives in 20 of 20 seeds (was 13 of 20).** The Shop price was raised from 70 to 75 so that the marginal Shop does not default (without it every baseline collapsed on mass unemployment). A steadier Shop removes most of F's collapses. This weakens "one tool must not be enough" (WN-1). The F test pins only `survival >= 0.4` and that B is at least as good | open | `tests/scenarios.rs` |
| V7-3 | **The economy is a knife edge around the Shop.** A Shop price of 71 collapses in 20 of 20 seeds, 72 and above does not (see the table in `docs/tuning-log.md` "Town v7"). The same fragility as WN-2, now visible because the estate removed the slack | open | `docs/tuning-log.md` |
| V7-4 | **The government's early borrowing is tighter.** The bank's lending headroom for the first turn is $65k from raw content (about $82k after settling), not $215k, because the estate household's $15k deposit and the Farm's change of owner reduce bank equity. A $40k plant still fits; two at once may not | open | `docs/town-v7.md` section 13 |
| V7-5 | The estate pays its pilot from savings plus dividends; its cash margin is thin (about $0.6k a month at a $2,400 wage, about $1.2k at $1,800). Neglect (C) starves the Farm and dismisses the pilot in 7 of 20 seeds (turns 20 to 31), which is a visible wealth to jobs link and intended; a carbon price at 100 to 200 or a wealth tax of 0.5% (B) no longer does | by design | `docs/town-v7.md` section 13 |
| V7-6 | **Fixed.** The wire now sends `estate.finance.wealthTaxOwed` (the monthly wealth tax due on the estate's net worth now, at the rate in force); the web's estimate (`wealthTaxShortfall`) reads it and no longer copies the rule. Note it is the amount owed for the coming month, compared with the month just paid | fixed | `crates/sim-api/src/snapshot.rs` `estate_view` |
| V7-7 | The web scene and panels do not render the estate yet: the map shows `estate_grounds` and `runway` tiles with no model, there is no jet, tag, band or inspector line, and the scene's palette has 8 household colours and marker shapes for 9 households (the ninth wraps to the first) | open (web) | `docs/town-v7.md` sections 6 and 7 |
| V7-8 | The mock's estate (flights, dismissal) is a labelled copy of the rules on the mock's own invented money; the mock does not charge the carbon price on a flight or the wealth tax to the estate (`estate.finance.*.wealthTax` and `carbonCharge` are 0) **Closed:** the mock is deleted. | closed (phase 2: mock deleted) | `web/src/sim-port/mock/estate.ts` |
| V7-9 | `Estate.last_flight` and the counters are reset after the settling months, but `unpaid_months` and the pilot's status are not: a pilot let go during settling would stay let go | by design | `crates/sim-engine/src/lib.rs` |
| V7-10 | **Fixed.** A domestic wage now names the employing household as the payer in the person ledger (`Earning.employer`, also in the rolled-up summary rows); the job guarantee still shows the Government | fixed | `crates/sim-api/src/ledger.rs` `person_ledger` |

## Platform cleanup (phase 1)

| # | Issue | Status | Source |
|---|---|---|---|
| PC-1 | **V6-2 is mostly fixed.** The mock is now typed against the generated contract, so it had to supply the fields the hand-written contract never declared (`monthLines`, `turnNumber`, `turnMonths`, `cancelRefund`, `active`, `upkeepEachMonthly`, tax-row `unit` and notes, history `grossEmissions` and `cumulativeEmissions`, assumption `function`). They are placeholders (`monthLines: []`, notes empty), not model output. The mock is still a second implementation; phase 2 replaces it **Closed:** the mock is deleted; the recorded demo is real output. | closed (phase 2: mock deleted) | `web/src/sim-port/mock/transparency.ts` |
| PC-2 | The generator's type list in `crates/sim-api/src/contract_ts.rs` is hand-maintained (a test fails if it is wrong, so it cannot drift silently). Generated names are the Rust names; `index.ts` aliases the old web names. `ts(optional)` gives `field?: T` and rejects `null`, which matches `skip_serializing_if`. Types that are loosened on purpose: `FrameInput` (planners accept a frame whose flow lists are missing; the real wire omits only `staffCommutes`), `Snapshot` (adds the mock-only `mock?`), `SchemaVersion` (a plain number). Doc comments from Rust are copied into the generated file | by design | `docs/how-to.md` section 19 |
| PC-3 | **Vocabulary renames skipped as not low-risk or not display-only** (taxonomy section 6): "Industry" tab to "Firms" (the tab id is a type and is used by tests and saves of UI state), "bank buffer" for the capital ratio (the real term is "capital ratio"; renaming would hide it), "fossil/renewable plant" to "power plant" in `content/levers.json` action labels, `infra_label` and scene text (touches fixtures, golden and many tests; the glossary and the build palette already say "power plant"), "carbon charge" for the ledger's "carbon tax" (the label is a Rust `Purpose::resolve` string: ledger fixtures and golden), "owner class" for the Capitalist class, one `power_plant` kind with a `fuel` attribute (needs the registry, phase 2), "Estate" naming in the wire (`Estate` is already the bundle) | open | `docs/taxonomy.md` section 6 |
| PC-4 | Not converted to lists: `TraceRecord::Income` kinds and `TraceRecord::Emissions` sources (their own closed enums; the web labels them through the catalog), government revenue and spending records (`govRevenue`, `govSpending`), `Indicators`. Only the fixed names the taxonomy listed for emissions and household income were converted. Looked at again in phase 2 and left: converting government revenue and spending is a breaking wire change (`SCHEMA_VERSION` 7, finance row ids and ledger links, the recording, fixtures, golden, saves) and the trace kinds touch the saved log, so neither is a cheap follow-up | open | `docs/taxonomy.md` section 5.3 |
| PC-5 | Contract v2.0 refuses saves made under schema 5 (a plain `schema_mismatch`), although the saved blob did not change shape. A player with an old save must start over | by design | `docs/snapshot-contract.md` section 20 |
| PC-6 | `web/playwright/platform-shots.mjs` is a manual script (needs a dev server and chromium); it was run at 1440 wide against the wasm build for Ecology, Finance and Society, and the emissions "how?" table. The other scripts written for the mock were repointed to `?sim=replay` or the real sim but not re-run; `weather-shots.mjs` edits a save and so only works where saves work | manual | `web/playwright/` |
| PC-7 | The recorded demo is one fixed run (seed 7, 12 turns, a scripted set of policies and buildings). It cannot answer "what if": policies, building, previews, the live clock (Pressure mode), the transaction ledger links, saving and loading are off or return `demo_unavailable`, and the seed asked for is ignored. The recording is about 1.2 MB of TypeScript (about 215 KB gzipped, its own chunk, loaded only when wasm is unavailable) and must be regenerated after any sim, content or wire change (`npm run gen:recording`; a Rust test fails when stale) | by design | `docs/how-to.md` section 22 |
| PC-8 | Without the wasm package `npm test` leaves out the files that need the real simulation (32 of 41; they import `tests/port.ts` or the package). Only the replay, boundary, contract, fixture and pure-logic files run. CI runs that subset first, then builds wasm and runs all | by design | `web/vite.config.ts` |
| PC-9 | **Entity registry limits (phase 2).** Registry-driven: labels, plurals, scene models, inspector sections, build-kind labels, the emitter, sink, producer and mitigator role checks in the lenses and the fire-cover ring. Still named in the web: the selection routing in `store/map.ts` (`selectionAt`, `selectionTile`: tile kind to person, home, firm, bank or government selection), the `SceneryKind` switch in `layoutOf`, `scene/flows.ts` (forest sinks, firm tiles), ledger account kinds, the commons collapse threshold 0.12 in `scene/entities.ts`, and the section headings and row labels inside `panels/inspect.ts` (web wording). A new inspector section needs one builder in `SECTIONS`. `Purpose` categories (taxonomy step 5) were not done. The two plant kinds stay two tile kinds, linked by `group: power_plant` and `attributes.fuel` | open | `docs/taxonomy.md` section 12 |

## Town v8 (net worth breakdown)

| # | Issue | Status | Source |
|---|---|---|---|
| NW-1 | Unpaid rent is not a liability line: the model keeps `arrears_months` (a count), not an amount owed, so net worth cannot subtract it. The statement says so in words | by design (no new simulation behaviour in v8) | `docs/town-v8.md` section 2 |
| NW-2 | Holder groups give each household one group by priority (estate, bank owner, firm owner, other). The bank's owner also owns the Factory, so that household shows under "The bank's owner", not "Firm owners". Workers' cooperatives count their members' households as firm owners | by design | `crates/sim-api/src/networth.rs::holder_of` |
| NW-3 | The reconcile flags compare floats with a tolerance of 1e-9 of the largest magnitude (the Rust lines are summed in a different order from `household_wealth`, which is left untouched so the state hash does not move). A bank with negative equity shows as a negative asset line | open, cosmetic | `crates/sim-api/tests/networth.rs` |
| NW-4 | The recorded demo shows the statement because the recording was regenerated; the person card there divides by people like the live game. Not tested in a browser beyond the 1440 wide screenshots listed in the v8 final report | info | `web/playwright/v8-shots.mjs` |

## Markets view (contract v2.3)

| # | Issue | Status | Source |
|---|---|---|---|
| MK-1 | Goods and services prices ignore excess demand (cost-plus): a shortage never raises a price and a glut never lowers one. The cards say so in words. Making prices respond (feature scope B) would change the balance and is not approved | by design | `docs/features/markets.md` section 4 |
| MK-2 | `markets` is empty until the first month has been played, so the Markets view shows a note on a fresh game and on the first screen of a loaded game before a month is played | by design | `crates/sim-api/src/markets.rs` `build` |
| MK-3 | The doc's revenue identity (`revenue = sum(sold x price)`) is not on the cards: the firm price has moved by the end of the tick and revenue is floored to cents per firm, so it cannot be recomputed exactly. The reconcile flag checks traced sales against each shop's stored `sales_units` and shelves against end inventory instead | open, cosmetic | `docs/features/markets.md` section 3.4 |
| MK-4 | A goods "wanted" figure is `sold + max(wanted - sold, 0)` per shop. When a second-pass reroute lets one shop sell more than its first-pass share, the shown demand is the bought amount, not the first-pass wish | by design | `crates/sim-api/src/markets.rs` `goods_raw` |
| MK-5 | A card is one month (the latest tick). The sparkline is one point per finished turn (plus the open month in live mode). No unmet or unsold series is kept | open | `crates/sim-api/src/history.rs` |
| MK-6 | The Markets view is read-only: nothing on a card changes the simulation. There is no centrally planned system, so shortages and queues under planning are not shown (a later feature) | by design | `docs/features/markets.md` pedagogy section |
| MK-7 | Playwright checks (`web/playwright/markets-shots.mjs`) were run at 1440 wide on the real sim and the recorded demo; no mobile or narrow layout was checked, and the panel is narrow (about 290 px), so cards are tall | info | `web/playwright/markets-shots.mjs` |
