# Web notes: Town v3 (Pressure mode)

Design: `docs/town-v3.md`. Contract: `docs/snapshot-contract.md` section 12. Scope: `web/` only. Relaxed stays the default and is unchanged.

## What was built
- **Contract and port**: `TurnStart`, `MonthResult`, `TurnPhase`, `Snapshot.turnPhase` (optional on the TS side so an older package still type-checks); `SimPort.beginTurn/advanceMonth/finishTurn` plus `liveTurn?: boolean`. The wasm wrapper calls `begin_turn/advance_month/finish_turn`; when the package lacks them `liveTurn` is false, the calls return `unsupported`, the New game screen disables Pressure with a notice and `?mode=pressure` falls back to Relaxed with a notice.
- **Mock**: `submitTurn` IS `beginTurn` + 3 x `advanceMonth` + `finishTurn` (one code path); the open turn lives outside `MockState`. Errors `turn_state`, `exportState` refused while open, `importState`/`init` close it. `tests/live-turn.test.ts` checks result, frames and exported state against `submitTurn` for three seeds. `tests/wasm-live.test.ts` does the same on the real package (skipped when not built).
- **`store/clock.ts`** (pure): `createClock(config, now)`; `tick()` advances by real time (one step capped at 1 s so a sleeping laptop does not skip quarters). Phases `planning | locked | paused`, pause reasons `manual | hidden | modal`, speeds, slow pace, optional pause budget, timeout policy in the `quarter` event (`submit | discard | skip`; a locked plan is always submitted). `secondsPerQuarter(cfg)` = 0.143 x 91.25 = 13.05 s. `gameDate(turn, calendar)` gives date and hour. `configFromSearch` reads `?mode=&pace=&timeout=&budget=&pause=0`. All defaults in `store/clock-config.json`.
- **Game loop**: `store/live.ts` + `Game.live*` methods in `store/game.ts`. A quarter: `beginTurn(plan)`, then `advanceMonth` at the start of each month (frame plays through that month), `finishTurn` at the boundary, immediately the next `beginTurn`. `GameState` gained `mode`, `liveAvailable`, `live` (slice), `savedClock`. Edits are refused while the plan is locked. Rejected commands go to a non-blocking card. `turn_state` errors resync (`liveRecover`); other errors are reported and after 3 in a row the loop stops (no deadlock); game over mid-quarter closes the turn and stops.
- **Saves**: `SaveFile` gained optional `mode`, `clock {speed, slow}`, `drafts`. In Pressure mode a turn is open while the quarter plays and the sim only exports at a boundary, so the store captures the boundary state each time a quarter finishes and Save writes that (with the current drafts); the toast says so. Loading yields the start of the quarter that was playing, with the drafts re-previewed.
- **Panels**: `StartScreen` (mode choice, Relaxed preselected; shown on a plain visit; EndScreen "New game" reopens it), `ClockStrip` in the top bar (date and hour, progress ring with month ticks, "Playing / Planning" labels, countdown cue in the last 3 s, pause, 1x/2x/4x, slow pace, pause budget), `LockButton` in the plan tray ("Lock plan · applies Q2 2031", then "Unlock to edit"), `QuarterCard` (non-blocking summary, rejected-commands card, notices), plan tray says "Applies when the next quarter begins". Keys: `P` pause, `L` lock/unlock. Any open modal (MutationObserver on `[aria-modal]`) pauses the clock; so does a hidden tab. Reduced motion removes the pulse. Relaxed UI is unchanged (briefing, coach, End turn only in Relaxed).
- **Scene**: `store/scene-live.ts` feeds `monthT` (0..1 inside the playing month, frozen on pause, stretched by speed). The scene plays `state.live.frame` when `frameSeq` changes; Relaxed playback code is untouched.

## Decisions and rough edges
- A month's frame is requested at the START of the month and plays through it, so the sim is up to one month ahead of the date shown.
- The first quarter starts at once (empty plan) when the game starts; the player plans Q2 while Q1 plays.
- `skip` keeps the drafts for the next boundary; `discard` clears them; both submit nothing.
- Preview during a live quarter sees PC after this quarter's charges and before the boundary regeneration (the sim's own rule), so a draft can only get cheaper at the boundary.
- Save/load: the saved drafts were meant for the quarter after the one now playing; on load the player is back at the boundary and can adjust them.
- The ring is small and its month ticks are subtle; fine at 1440 px, not tuned below 1024 px. The end-of-game flow reuses EndScreen. The Pressure first-turn coach is skipped.
- The summary card shows the last 5 metric deltas and 3 events, not the full Results screen.

## Verification
Playwright on the mock and the real wasm (rebuilt with the TurnSplit commits): start Pressure, quarters pass, lock, pause, 4x, hidden tab (emulated), modal pause, auto-submit with a rejected road reported, Save in Pressure, Relaxed regression. No console errors.

## Contract issues
1. `turnPhase` is optional in the TS `Snapshot` so old packages type-check; contract section 12 makes it required. Fine once all packages are rebuilt.
2. Nothing in the wire says what `preview()` charges against mid-turn (PC before or after the boundary regeneration); the web assumes the sim's current PC.
3. No read-only "snapshot" call exists in `SimPort`, so after a `turn_state` error the web can only resync by trying `finishTurn`/`advanceMonth`. A `turnPhase` query would be cleaner.
