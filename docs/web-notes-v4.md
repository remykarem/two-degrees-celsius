# Web notes: Town v4 (weather: forest fires and drought)

Design: `docs/town-v4.md` (section 6 UI and scene). Rules: `docs/causal-model.md` section 11. Contract: `docs/snapshot-contract.md` section 13 (v1.2). Scope: `web/` only. Works in Relaxed and Pressure; in Pressure every weather card is non-blocking (banners and sidebar cards, no dialogs).

## What was built
- **Contract types** (`contract/index.ts`): `Weather`, `RiskLevel`, `ForestState`, `Tile.state`, `NatureView.*Health`, `HistoryPoint` weather fields, `fire_station`, `resilience_spending`, the seven weather events, trace source `fire_sink_loss`, four new `MetricKey`s. `SchemaVersion` is `2 | 3` (the real sim is 3 since the weather State layout). `sim-port/wasm/index.ts` accepts schema 2 or 3 (it refused 3 before: the real sim fell back to the mock until this was fixed).
- **Mock weather** (`sim-port/mock/weather.ts`, numbers in `mock-only.json` `weather`): only the stress curve is the documented formula; everything else is invented and labelled MOCK. Own random stream (`weatherRng`) so the economy's draws are unchanged. Drought and fire lifecycles, spread, regrowing, fire station coverage (shorter and slower fires), resilience spending with diminishing returns, deterministic forecast, events with importance, `fire_sink_loss` trace record, health fields, history fields. Same lever and PC rules as before (cost entries come from `content/params.json`; the mock falls back to labelled numbers when content lacks them). A save from before weather is refused with `schema_mismatch` and a message naming weather.
- **sync-mock** now reads `params.weather` (onset, full, curve, coverage radius, regrow months, duration, station cost and upkeep) and the `resilience_spending` lever from `content/levers.json` when present, and omits them when absent. The mock's lever and action cost models equal the real ones (tested against the wasm package).
- **Store** (derived only, no UI-owned weather state): `store/weather.ts` (health meters and levels, stress gauge, forecast card text, fire labels "Forest fire near <building or (x,y)>", nearest landmark, `fireSelection`), `store/scene-weather.ts` (`WeatherLook`: drought severity, fires, regrowing tiles, stations, per-tile fire risk, cover and smoke tests). New lens id `fire`.
- **Panels**: health bars (Ecology tab, full; top bar, compact cluster with number, word and a different bar pattern per level), climate stress gauge (axis is "carbon budget left", onset and full marked, marker at the budget now), forecast card (Ecology tab always; above the tabs on other tabs when not calm; calm wording and "Adapt..." that opens the policy drawer on resilience spending and selects the fire-station build), crisis banners for an active fire (with "show on map", "why?", "adapt") and drought, build palette "Fire station" with a coverage hint, policy drawer "Resilience spending" in the nature group (arrows come from the lever metadata, so only where the causal model supports them), event wording, results "why?" for emissions includes "Sink lost to fire", metrics (forest, commons and farm health, climate stress, with sparklines), end-screen story facts (`weatherFacts`, templates only), inspector for burning, regrowing and fire-station tiles.
- **Scene**: models `forest_burning`, `forest_regrowing`, `fire_station`; `scene/weather.ts` `WeatherLayer` (three instanced pools: flames 48, smoke 96, haze over people near fires 40; coverage ring and disc on hover, selection or while placing a station); drought look (`weatherLook.ts` `droughtLook`: ground tint, haze fog, warmer and dimmer sun, all scaled by severity, none at 0); ground colours for burning and burnt tiles; lens "Fire and drought risk" (forest by ignition risk, ground by cover). Reduced motion: flames, smoke and haze are drawn static and the repaint loop does not run for them. Everything disposes with the scene.
- **Tests**: `mock-weather`, `panels-weather`, `scene-weather` (mock), `wasm-weather` (real package, skipped when not built), fixtures v1.2 block (runs when `docs/fixtures` carry weather, which they now do). Screenshots: `web/playwright/weather-shots.mjs` (mock, edits the exported save to set up situations) and `weather-real.mjs` (real wasm).

## Decisions
- The Ecology tab lists health (bars plus sparklines) and no longer lists the forest and commons stock rows (same numbers as the bars on the mock; the stocks are still in the inspector).
- Health words: healthy 60% and up, strained 35%, poor 15%, failing below. (70% was too strict: the real start values sit around 71% to 79% and drifted to "Strained" on turn 1.)
- Fire station count comes from the map tiles (`Buildings` has no field for it).
- Water shine: there is no water in the scene, so nothing is dimmed; the drought look is ground tint, haze and a warmer, dimmer sun.

## Rough
- The top bar wraps to a second row at 1440 px (three rows in Pressure mode); the health cluster is compact but wide.
- The coverage ring is a flat circle: it ignores buildings and looks large at default zoom. It uses a web copy of the coverage radius.
- Flames are cones, not animated sprites; they read at zoom 1.5 and up. Smoke columns share their colour ramp with nothing else.
- Haze over people is placed at idle positions, not on commuting people.
- The forecast card says "next quarter" but the wire does not say what horizon the levels cover.
- Mock numbers are plausible, not tuned: on the mock a no-action game reaches fires and droughts earlier and more often than on the real sim.
- Not tested in a real browser: reduced motion (code path only), WebGL context loss with fires on screen.

## Contract issues
1. The stress curve parameters (`onset`, `full`, `curve`), the fire-station `coverage_radius` and `smoke_radius` are not on the wire. The web copies them in `store/weather.ts` (`WEATHER_UI`) to draw the gauge marks, the ring and the haze; they only decide pixels, but they can drift from `content/params.json`.
2. `Buildings` (summary counts) has no fire-station count.
3. A `regrowing` tile has no months left, and a freshly burnt tile cannot be told from one that is almost grown back: the charred look is one model.
4. `fire_sink_loss` is a trace source only; `MonthFlows.emissions` has no equivalent, so nothing can show it per month.
5. `forecast` has no stated horizon and no numbers (only low, medium, high), so the card cannot say "20% chance".
6. Real `resilience_spending` arrows include `debt` up (from `content/levers.json`); the mock uses the same metadata when content has it.
7. `docs/known-issues.md` and `AGENTS.md` still say the web fixtures test pins schemaVersion 2; it now accepts 2 or 3.
