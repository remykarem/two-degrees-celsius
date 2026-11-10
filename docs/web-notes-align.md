> **Status: HISTORICAL** (alignment to contract v0.3, `schemaVersion` 1, cohorts). The current contract is `docs/snapshot-contract.md` section 11 and `web/src/contract/index.ts`.

# Web alignment notes (contract v0.3)

Work done after the independent review of the web app against the real Rust simulation. Contract section 10 wins over sections 3, 4 and 9 of `docs/snapshot-contract.md`. The real fixtures in `docs/fixtures/` (from `crates/sim-api`) are checked by `web/tests/fixtures.test.ts`.

## What changed

**Contract in the UI.** `Result<T>` on every fallible `SimPort` call (`init`, `preview`, `submitTurn`, `cancelPending`, `exportState`, `importState`); `Snapshot.buildings`; `LeverInfo.costModel`, new units (`usd_per_month`, `tonnes_per_month`), `zeroMeansNone`; `MonthFlows` arrays optional. The store never throws: a failed call sets `state.error` and the `ErrorScreen` offers "Close and keep playing" or "New game". `endTurn` makes one store update.

**Layout is derived.** `store/layout.json` now holds geometry only (grid, bank, town hall, forest and commons tiles, home slots, firm slots, build zones). `reconcileLayout(prev, snapshot, hints, claims)` builds homes from households, firm slots from firms, one tile per built building (extras dropped last-first, new ones go on the tile the player chose, else the kind's zone, else the first free tile), and reserves a tile per queued build. It returns the same object when nothing changed. Queued builds are no longer stored as "placed with an effectiveTurn": they are drawn from `snapshot.pending` plus `layout.pendingTiles`. "Remove one" appears only when the count is above zero.

**Costs.** No lever is locked by cost. Rows show "About N PC for a small change, M PC per <unit> of change" from `costModel`, then "Draft: costs X PC (includes Y% lobbying opposition)" from `preview()`. Build and switch costs use the cost model (exact for them). Units and labels: `$1.2k/month`, `$50/t`, `120 t/month`, `no cap`.

**Timing.** Items fire at the start of the next submit. "Coming next turn" = `effectiveTurn <= snapshot.turn`. `turnsUntil(effectiveTurn, turn) = effectiveTurn - turn + 1` drives the pending list, the inspector and the 3D "under construction" counters.

**Mock sync.** `npm run sync-mock` writes `src/sim-port/mock/data.generated.json` from `content/params.json`, `content/initial_town.json` and `content/systems/*.json`. `tests/mock-sync.test.ts` fails if the committed file differs from a fresh generation. The mock follows `crates/sim-engine/src/turn.rs`: cost = base x max(|change|/unit, min size) x (1 + opposition), size against the latest pending value, opposition only for raising carbon price / wealth tax / profit tax and for tightening the cap, bool/enum size 1, funds check = cash + lending headroom - queued builds, `transition_in_progress` for a pending or running switch, `lever_unavailable` from the current system, range checks, unknown cancel id is an error, halt at the first outcome, `game_over` error after it.

## Still mock-only (invented, labelled "Mock data" in the top bar)

GDP, GPI, unemployment, Gini, wellbeing, legitimacy and prices dynamics; lobbying pressure (drifts toward a level set by the system and union density); emissions growth, the household emissions share, plant effects; the bank's equity path and failure (which makes `mass_unemployment`, `bank_failure_cascade` and `legitimacy_zero` reachable); household and firm book values; lever ranges, arrows and the effect coefficients in `mock-only.json`; when a firm opens (turn 40); builds are built and paid at activation in one step. Wage share, profit share, debt/GDP and money are now computed from the mock state, not constants. Starting values for GDP, unemployment, emissions and bank balance are derived from `initial_town.json`.

## Review claims checked

- True: `PCFSoftShadowMap` warns in three r187 ("has been removed. Using PCFShadowMap instead"); fixed.
- True: the policy drawer's wrapper `.wrap` was invisibly wider than the drawer (713 px next to a 390 px drawer) and swallowed map clicks beside it. Not in the review; found while testing the zoom buttons; fixed with `pointer-events`.
- Plausible, not reproducible here: the blank frame after `setSize`. `resize()` now renders synchronously. A screenshot taken right after dismissing a banner shows content, but a headless screenshot waits for a frame, so this does not prove the old behaviour.
- True: "not enough political capital" was listed under lobbying. It is now its own "Could not afford" line, apart from "Blocked by the rules" and "Weakened by lobbying". Zero gaps are no longer listed (the real sim sends all four lobbied levers every turn).
- True: the top bar and the end screen were one quarter apart at game over (`snapshot.turn` has already moved on). Both use `shownTurn(snapshot)` now.
- True: system copy said the emissions cap needs Regulated. In the rules only the campaign cap and community commons are blocked in Market.
- Not a bug on its own, but removed: the "why?" links for GPI, unemployment, wellbeing and legitimacy (the data behind them, income by type or lobbying gap, does not explain them).

## Contract issues (section 10 followed as written; these are the problems)

1. **`nature.forest` and `nature.commons` are raw stocks in the real fixture (about 71,000 and 39,000), the UI shows them as 0..1 shares.** The mock sends shares. The wire should send shares or add `forestMax`, `commonsMax`. `tests/fixtures.test.ts` pins this as a known mismatch.
2. `ActionInfo.costModel` is absent in the real `actions.json`. It is optional in the TypeScript type; when absent the UI uses `{ base: baseCostPc, unit: 1, minSize: 1 }`, which is exact for place, remove and switch.
3. `availableIn` cannot say "commons_access is available in Market only as open". I added optional `LeverInfo.optionAvailableIn` (the mock sends `{ community: ['regulated','coop'] }`). The real wire does not, so the UI lets `community` be drafted in Market and relies on `preview()` answering `lever_unavailable`.
4. The real accepted `PendingItem.command` has no `tile`. The store matches accepted items to the drafts that produced them (same order, skipping rejected indexes) to keep the clicked tile.
5. `gameOver.turn` is the turn that ended the game (collapse: outcome tick / 3 + 1; survived: 161 per the contract text, while the last played turn is 160), and `snapshot.turn` has already advanced. Section 10.6 does not say which. The UI uses `min(gameOver.turn, lastTurn)`.
6. `HistoryPoint.emissions` is unspecified. The real wire sends net monthly tonnes; the mock now does too, and the charts label it "per month (net, end of turn)".
7. `Snapshot.mock?: boolean` is my addition (drives the "Mock data" badge). The real wire omits it.
8. Real `schemaVersion` is 1; the mock's is 3 (its own save format). The save file stores whichever the port reports.
9. Arrows: the real `levers.json` omits tax_wage, tax_profit, tax_capital, wealth_threshold, campaign_cap, emissions_cap, commons_access and bailout. The mock keeps arrows for those that `docs/causal-model.md` states (taxes and threshold -> debt, emissions cap, commons access, bailout -> legitimacy) and none for campaign_cap. Both follow "arrows describe raising the lever"; the UI flips them when the draft goes down and treats `0` as "no cap" (loosest) for `zeroMeansNone` levers.
10. `flows.purchases` is always empty in the real wire; the scene already skips it.
11. Section 10.5 says the wire must document when build money is charged. `docs/sim-api-notes.md` says at activation, so the plan tray says "on activation".

## Save file

`{ format: 'ecotown-save', saveVersion: 1, schemaVersion, simState, layout, lastResult, previous, uiPrefs: { lens, camera }, savedAt }`. `parseSave` checks every field and returns a plain-language message; a failed load changes nothing.

## Verification

`npm test`, `npm run typecheck`, `npm run lint`, `npm run build`; `node playwright/align-shots.mjs <outDir>` (dev server on 5177) checks the derived layout, real costs, banner dismiss, save/load round trip, keyboard placement, focus handling, the narrow-window notice and prints console errors. WebGL context loss and restore were exercised with `WEBGL_lose_context` (message appears, then clears; screenshots 9a and 9b). Not verified: screen-reader output (only the DOM roles and live regions were checked), phones, and the old blank frame itself.
