> **Status:** the part above the heading "# Town v2 (contract v1.0)" is HISTORICAL (v0.3 era: cohorts, derived building counts, `schemaVersion` 1, empty purchases). The Town v2 section is CURRENT. Index: `docs/README.md`.

# sim-api notes

`crates/sim-api` is the wire layer of `docs/snapshot-contract.md` (v0.3 wins). It depends on `sim-types`, `sim-params`, `sim-metrics` (via the engine), `sim-checks`, `sim-rng` and `sim-engine`; no model crate depends on it. It has no logic of its own beyond conversion, validation of wire input, frame building and save/load, so a thin `sim-wasm` can wrap `Api` one-to-one (`Api::init`, `levers`, `actions`, `preview`, `submit_turn`, `cancel_pending`, `export_state`, `import_state`, `snapshot`, plus `api::parse_commands` for the JSON command list). All fallible calls return `Result<_, ApiError>` with `{ code, message }`.

Layout: `keys` (closed key sets), `command`, `meta` (LeverInfo/ActionInfo), `snapshot`, `events`, `turn` (previews, frames, results), `frames` (per-tick flows), `history`, `persist` (save blobs), `fixtures`, `content` (the JSON embedded with `include_str!`, so the library does no I/O).

## Wire conventions
Money is dollars (`Cents::dollars()`), rates are fractions, people are floats, emissions are tonnes. Unions are `{ "type": ... }` snake_case; fields are camelCase. `SimEvent` flattens `importance` next to the payload. `GameOutcome` is `"budget_exhausted" | "survived" | { "collapse": reason }`.

## Building counts
`buildings.publicHousing = round(housing.public_cap / 1)`, `fossilPlant = round(energy.fossil_cap / government.plant_capacity)`, `renewablePlant = round(energy.renewable_cap / government.plant_capacity)`. `plant_capacity` is 20 Units (`params.government.plant_capacity`). One public housing building adds exactly 1 Unit of `public_cap` (hard-coded in `sim-model/src/government.rs`, `Units(k)`; there is no parameter, so the constant `PUBLIC_HOUSING_PER_BUILDING = 1.0` lives in `sim-api/src/snapshot.rs`). `private_cap` is not a building. Initial town: public housing 0, fossil 100 Units = 5 plants, renewable 0; 6 cohorts, 4 firm slots (2 active). Verified by tests.

## Money timing for builds
`commands.rs::apply_commands` only pushes `(kind, count)` onto `gov.pending_build` when a pending command **activates** (start of the submit whose turn is `effectiveTurn`). The next government tick (S8, first tick of that same turn) adds the capacity and pays `count x cost` (plus upkeep) to the labour-supplying cohorts as wages, financed from cash and taxes first and then bank borrowing (`Delta::Gdp(GovServices)`; `pending_build` is then cleared). At **queueing** time the only money check is `available_funds` = gov cash + bank lending headroom - already queued builds; nothing moves. Therefore `CommandPreview.money` and `PendingItem.moneyCost` are the amount charged on activation, and are also what must be affordable at queueing. The test `builds_are_paid_at_activation_not_at_queueing` runs a twin game: government and bank are identical until the activation turn, and the debt then differs by about the bill (1.69M of 2.0M for one plant; the rest came from cash and taxes; **Town v1 figures, a plant now costs 40,000**). Cancelling refunds 50% of the PC only. `remove` commands cost no money. Plants take effect (capacity) in the activation turn, not after construction time: the lead time is the whole delay.

## Frames and flows (what is real, what is empty)
Three `MonthFrame`s are built from the `on_tick` callback of `submit_turn_with`.
- `events`: all sim events of the tick. Events raised before the first tick (rejections, activations, `system_switch_started`) are added to month 0, after wire-level `policy_blocked{invalid_command}` events.
- `flows.commutes` (real): for each cohort and active firm with employed people, the end-of-tick `employment[h][f]`. This lists every commuter each month, not only new hires; `Event::Commuted` (new hires only) stays in `events`.
- `flows.emissions` (real): the tick's `TraceRecord::Emissions` (per active firm, households, sink). The sink is **negative** tonnes (carbon absorbed), as in the sim.
- `flows.rent` (real): `income.rent_paid` per household when positive (no counterparty).
- `flows.wages` (derived): wage x employment x wage premium for each cohort and active firm (the formula in `income.rs`), scaled down so a cohort's total never exceeds the wages it received that tick. Exact unless a firm's cash was short.
- `flows.purchases`: **always empty**. The sim records a household's total spending with no sector or counterparty; any split would be invented.
`TurnResult.trace` is all trace records of the three ticks (emissions, income, policy gaps).

## Event importance (my ranking, for the results screen)
3: `bank_failed, collapse, budget_exhausted, commons_collapsed, commons_enclosed, bank_bailout, firm_closed, system_switch_completed`. 2: `loan_defaulted, firm_founded, forest_shrank, policy_blocked, command_activated, rent_shortfall, spending_cut, class_changed, system_switch_started`. 1: `commuted, hired, fired`.

## History
One `HistoryPoint` per completed turn, kept in the `Api` (not in `State`, so model `schema_version` is unchanged) and saved in the blob. `emissions` = net monthly tonnes at the end of the turn (undefined in the contract). `debtToGdp = gov.debt / gdp_annual`, 0 if GDP is not positive. `budgetShare` = remaining budget / B0 clamped to 0..1.

## Game over
`Api` halts at the first outcome. `stop_on_loss` stays false in the engine, so the 3 ticks of the turn in which the outcome fires are all played; `gameOver` is derived from `state.outcome` (turn = outcome tick / 3 + 1) or, after the last turn, `survived` with turn 161. Later `submit_turn` and `cancel_pending` return `game_over`.

## Save format
`export_state` = JSON `{ schemaVersion, paramsHash, rngState, rngInc, initialPopulation, clampedTotal, state, history }`. `paramsHash` is FNV-1a 64 over the canonical JSON of the params. Import checks `schemaVersion` (`schema_mismatch`), params hash (`params_mismatch`), shapes and ids, the `sim-checks` state invariants, and re-validates pending commands; any failure (`bad_state`) leaves the game untouched. `serde_json` is built with `float_roundtrip` (needed for bit-exact f64; Cargo unifies features across the workspace, and the full suite passes unchanged). The test `export_import_round_trip_is_deterministic` checks turns 7-12 are identical after a reload.

## Validation of input
Out-of-range numbers, NaN/infinity, wrong value types, community cap outside 0..=1 and build counts outside 1..=100 give `blocked: invalid_command` in `preview` and a `rejected` entry in `submit_turn` (the engine never sees them, so they cannot panic it, and they do not shift the costs of later commands). Ranges are in `content/levers.json`. Everything else comes from the engine's `validate()`: `lever_unavailable` (campaign cap and community commons in Market; enclosure is not expressible on the wire), `transition_in_progress` (a switch pending or running, including earlier in the same batch), `same_system`, `nothing_to_remove`, `insufficient_*`.

## Metadata source
`content/levers.json` holds label, wire unit, min, max, step, `zeroMeansNone`, `lobbied`, `arrows`, `availableIn`, `options`, the 7 action labels/availability and `game.startYear` (2031). `baseCostPc`, `leadTime` and `costModel` come from `params.turn.levers`; `ActionInfo.moneyCost` from `params.government` (Town v1 values were 14,000 housing and 2,000,000 plant; **corrected: Town v2 `content/params.json` `government` has `public_housing_cost_usd` 40,000, `plant_cost_usd` 40,000, `road_cost_usd` 2,500**). `bank_capital_ratio` has min 0.02 because the engine divides by it.

## Arrows
Only effects stated in `docs/causal-model.md` (S5/S7/S8, sign tests in section 5) are included, for the direction "lever raised": carbon_price: emissions down; wealth_tax: gini down; base_rate: money down (credit falls); min_wage: wage_share up; benefit, ubi: debt up (spending items, deficits are borrowed); green_subsidy: emissions down; commons_harvest: commons down; bank_capital_ratio: money down (lending limit equity/kappa); public_spending: gdp up, debt up; job_guarantee: unemployment down.
**Omitted (no documented sign, or direction ambiguous):** tax_wage, tax_profit, tax_capital, wealth_threshold, campaign_cap, emissions_cap (0 means none, so "raise" flips meaning), commons_access, bailout. Also omitted for the lever above whose second effect is uncertain: min_wage unemployment (doc says "up or flat"), carbon_price gdp.

## Contract issues
1. `web/src/contract/index.ts` still has v0.1 shapes (no `buildings`, `costModel`, `zeroMeansNone`, `usd_per_month`/`tonnes_per_month`); the Rust output follows v0.3. Wire `schemaVersion` is `sim_types::SCHEMA_VERSION` (1).
2. `HistoryPoint.emissions` is unspecified: net monthly tonnes used.
3. The community commons cap is not visible in the snapshot (`policy.commons_access.nominal` is only the mode string, `nature.commonsAccess` too).
4. Sink emissions are negative in `flows.emissions` and `trace`.
5. `commutes` flows list all employed commuters each month; `commuted` events only new hires.
6. `availableIn` cannot say that `commons_access` is available in Market only as `open`; `preview` answers `lever_unavailable` for `community`.
7. `gameOver.turn` for collapses is derived from the outcome tick; the 1-2 remaining ticks of that turn still run.
8. Rejected `index` refers to the submitted list (including wire-invalid commands); accepted `PendingItem.command` has no `tile` (the sim has none).
9. Costs are measured against the latest pending value of the same lever, so a batch's second command depends on the first (preview does the same).
10. (Phase 1: every plan collapsed on legitimacy around turns 38-41.) After the Phase 2 balancing, no action ends in `budget_exhausted` around turn 100, and baseline B survives 160 turns. The tests assert `history.length == turns played` and halting.

# Town v2 (contract v1.0)

The wire layer was ported to the Town v2 simulation (individual people, households, homes, a 12 x 12 map, roads, plant emissions, a Shop). Contract section 11 of `docs/snapshot-contract.md` is implemented; the sections above are historical where they differ (cohort households, derived building counts, `count`-based place/remove, empty purchases are all gone). `schemaVersion` is 2 (`sim_types::SCHEMA_VERSION`).

## Wire shapes (all camelCase, money in dollars)
- `Snapshot` gains `map { width, height, tiles[] }` (row-major, `tiles[y*width+x]`; tile = `{x, y, kind, ref?, buildable, emissionsMonthly?}`), `people[]`, `households[]` (new shape), `homes[]`, `plants[]`. `ref` is the home, firm or plant id. `buildable` is "the tile is empty"; the exact answer including queued builds is `validTiles(kind)`. Plant tiles carry `emissionsMonthly`.
- `people[]`: `id, label, household, ageBand, skill, job {firm} | null, wage, commuteTiles, commuteMinutes, wellbeing` plus `inLabourForce, jobGuarantee, effectiveHours` (extras). `job` is null for a job-guarantee worker (`jobGuarantee: true`).
- `households[]`: `id, label, members[], home | null, class, savings, debt, netWorth, income{...}, politicalPower, arrearsMonths` plus `wellbeing, footprintTonnes, foodMet`. `homes[]`: `id, tile, owner (household id | "public"), household | null, rent, value`.
- `buildings` is now a summary counted from the map: `publicHousing, fossilPlant, renewablePlant, road`. The old capacity-derivation (`PUBLIC_HOUSING_PER_BUILDING`, `plant_capacity`) is gone.
- `firms[]` gain `tile`; names are Factory, Farm, Shop (a second Shop for the dormant slot is "Shop 2"); `sector` adds `services`.
- `housing`: `homes, rentPrivate, rentPublic, occupancy, unhousedHouseholds` (capacities removed). `energy`: `fossilCap, renewableCap, fossilShare, demand, supply, delivered, served, gridIntensity`.
- `indicators` and every `HistoryPoint` gain `avgIncomePerPerson, medianIncomePerPerson, topHouseholdWealthShare`. `MetricKey` gains `avg_income, median_income, top_wealth_share`.
- Commands: `{type:"place", kind:"public_housing"|"fossil_plant"|"renewable_plant"|"road", tile}` (tile required: a missing tile is `bad_request`), `{type:"remove", tile}`. Pending items keep their tile. Tile integers are accepted as i32 on the wire; off-map values (negative, > 255 or beyond the grid) become `tile_invalid` without reaching the engine.
- `BlockReason` adds `tile_occupied`, `tile_invalid`. `CommandPreview` adds `tileValid` and `tileReason` (`tile_occupied | tile_invalid | nothing_to_remove` when not valid; always valid for non-tile commands). `Api::valid_tiles(kind)` / `WasmSim.valid_tiles(kind)` returns `{x,y}[]` in row-major order: empty tiles with nothing queued on them (scenery, occupied tiles and queued tiles are excluded). It is the same for all four kinds today.
- Events: `commuted{person,firm,tiles,minutes}`, `person_hired{person,firm}`, `person_fired{person,firm}`, `purchase{household,firm,amount}`, `household_moved{household,to}` (`to` = home id), `household_evicted{household}`, plus `built{kind,tile}`, `removed{tile}`, `electricity_rationed{served}` (the sim raises them; the contract did not list them). The v0.x `hired` / `fired` are gone. Importance: 3 evicted (and the old game-shaping set); 2 moved, fired, built, removed, rationed; 1 commuted, hired, purchase.
- Emissions: source `plant` carries `id` (plant id) and `tile`, in `flows.emissions` and `trace`.

## Frames (what is real, what is empty)
All five flow lists are real in Town v2; none is always empty any more.
- `commutes`: one per person with a firm job, each month: `{person, household, firm, from, to, tiles, minutes}`. `from` is the home tile (`null` for the unhoused household), `to` the firm tile. The sim does not expose the route, so there is **no tile path** (contract said "if the sim exposes it"). The scene can draw an L-shaped or straight line; `tiles` and `minutes` are the sim's cheapest-route cost.
- `purchases`: `{household, firm, sector, amount}` from the tick's `purchase` events (a household buys from each firm it spends at).
- `wages`: `{person, household, firm, amount}` = the wage each employed person was paid this tick. Job-guarantee workers have no firm and no row.
- `rent`: `{household, home, amount}` rent paid this tick.
- `emissions`: the tick's trace: firm (direct), households, plant (with tile), sink (negative).

## Save and load
`export_state` has `schemaVersion: 2`. Any other version (including every v1 save) is refused with code `schema_mismatch` and the message "save has schemaVersion ..., this build reads 2". Import then checks the params hash, and a structural pass over the whole save before anything indexes it: counts of people/households/firms and the map size against the content, ids equal positions, and every person, household, home, plant, firm, owner, map cell, pending command and queued build refers to something that exists on the map. Then `sim-checks` (`check_state`, which includes the people/homes/tiles invariants). A failed import leaves the game untouched.

## Metadata
`content/levers.json` gained `place:road` and `remove:road` actions and a `tileRule` text on every place/remove action. `ActionInfo` gains `needsTile` and `tileRule`; `place:road` money is `params.government.road_cost_usd`. Costs, lead times and cost models still come only from `params.turn.levers` (`place_road`, `remove_road` were already there). The 19 lever entries and arrows are unchanged: no road lever exists and `docs/causal-model.md` states no new lever-to-metric sign.

## Fixtures and golden
`docs/fixtures/` is regenerated by `cargo run -p sim-api --example fixtures`: `snapshot-turn1.json`, `turn-result.json` (carbon price 50, a road at (4,6), a renewable plant at (10,5), and a campaign cap the Market refuses), `levers.json`, `actions.json` and new `tiles.json` (`validTiles` per kind at the start). `tests/golden/plan.json` was rewritten for tile commands (including a removal, an occupied tile, a forest tile and an off-map tile) and `tests/golden/seed7.json` regenerated; the golden file layout is unchanged.

## Contract issues
1. No commute path: `from`/`to` tiles only (the sim exposes distance and time, not the route).
2. `household_moved.to` is a home id (contract left it open). `built`, `removed`, `electricity_rationed` events and `plants[]` are additions.
3. `buildable` on a tile is static (empty); it ignores queued builds. `validTiles` is the authority.
4. `place_public_housing` etc. are all valid on any empty tile; the sim has no per-kind terrain rules, so `validTiles` is identical for the four kinds.
5. `remove` is priced by the kind on the tile (`remove_<kind>`); the wire `remove` has no kind. A wire-invalid remove previews with the `remove_road` lead time.
6. Wage flows are per person (`person` added to the old `{household, firm, amount}`); commutes lost `people` (they are per person now).
7. Game over, history, and the halt-at-first-outcome behaviour are as before. Balance is untouched: the scripted plan in `tests/game.rs` halts on a bank-failure collapse around turn 27, the mild plan on legitimacy around turn 150, and no action on the carbon budget around turn 124 (seed 42).
