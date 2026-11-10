# Town v3 design: the pressure clock (draft for review)

Status: **APPROVED by the user ("go ahead, I trust your judgment"); implementation starting.** Weather is a separate, later release: see `docs/town-v4.md`. Every number is a placeholder and configurable. Items marked **[DECIDED]** came from the user; **[PROPOSED]** are my defaults for you to confirm or change.

## 1. Goal

Give the player pressure, in a fun way: the world keeps moving while they decide. Today the game waits for End turn. In **Pressure mode** time passes on its own, the player queues moves against a visible clock, and the quarter plays out live. Relaxed mode stays exactly as it is.

## 2. Decisions

| # | Decision | Status |
|---|---|---|
| 1 | **Relaxed is the default.** Pressure is an opt-in mode chosen when starting a game | [DECIDED] |
| 2 | The pace is **easily configurable**. **Target: a no-action game reaches 50% of the carbon budget remaining in about 15 real minutes.** Default **1 real second = 1 game week** (see section 2a) | [DECIDED] the target; [PROPOSED] the exact default |
| 3 | Pressure mode is **purely a front-end driver**: the simulation never sees wall-clock time | [PROPOSED] |
| 4 | When the clock runs out mid-planning, **the drafts are submitted** (config: `submit` / `discard` / `skip`) | [PROPOSED] |
| 5 | Weather is **not** in this release (v4) | [DECIDED] |

### 2a. The pace, with numbers (measured)
The user asked for "1 s = 3 h" and for the no-action game to reach 50% budget remaining within about 15 minutes. These conflict: at 1 s = 3 h a quarter takes **12 minutes** (2,190 game hours / 3 per second = 730 s). Measured on the current sim (seed 42, no actions, `cargo run -p sim-cli -- --seed 42`): **50% of the budget remains at turn 64**, 10% at turn 112, and the budget is gone at turn 124. So 15 minutes for 64 turns means about **14 s per quarter, about 6.5 game days per second**. The default is therefore **1 real second = 7 game days (a quarter is about 13 s)**, config `realSecondsPerGameDay = 0.143`; a no-action game reaches 50% in about 14 minutes and ends in about 27. The 50% point will move as the economy is rebalanced and weather arrives (v4), so the config also exposes the equivalent `secondsPerQuarter`. 13 s a quarter is frantic for planning: drafts persist across quarters, an empty quarter is fine, pause and 0.5x speed exist, and Relaxed mode is unchanged. This is deliberately tunable after playtesting.

**Why not 1 s = 1 game hour:** a turn is a quarter. At 1 s = 1 h a quarter lasts 36 minutes and a 40-year game about 97 hours. Any pace is reachable through the config.

## 3. How Pressure mode plays

The quarter plays **live** while the player plans the **next** quarter, so the player is never "waiting for their turn":

```
Quarter N plays live (3 months over ~13 s):          Player plans quarter N+1:
  month 1 ........ month 2 ........ month 3          queue policies, builds
  frames animate as each month completes            [Lock plan] when done
                                                     |
                                      boundary: plan locks (auto if not done) -> quarter N+1 begins
```

- **Clock:** a top-bar game clock (date and hour) ticks continuously; a progress ring shows time to the quarter boundary (about 13 s by default); month markers show when each month's results land.
- **Execute (the player's "I'm done"):** locks the plan. It applies when the next quarter begins; commands still follow the normal rules (political capital, lead times, costs). Locking early does not start the next quarter early, but the player can **fast-forward** (2x / 4x) to get there sooner.
- **Timeout [PROPOSED]:** if the boundary arrives with unlocked drafts they are submitted as they stand. Drafts that became unaffordable are rejected as usual and reported.
- **Pause:** a pause button; the clock also pauses when the browser tab is hidden and while a blocking modal (system switch confirmation) is open. Config `allowPause` (default true) and `pauseBudgetSeconds` (default unlimited) so a harder variant can ration "thinking time".
- **Results:** in Pressure mode the quarter summary is a **non-blocking card** (it must not stop the clock). Crisis banners behave as today.
- **Scene:** playback is no longer a skippable 6 to 10 s sequence. Each month's frame is played as it lands, spread over that month; ambient flows (commutes, purchases) loop between months.
- **Political capital** still regenerates once per turn, at the quarter boundary.

## 4. What changes in the code

### Simulation: split the turn (the only sim change)
`submit_turn_with` (`crates/sim-engine/src/lib.rs`) already does: queue commands, activate due items, run `ticks_per_turn` (3) ticks, compute policy gaps, regenerate PC, advance the turn. Split it into three calls that, run in order, are **bit-identical** to `submit_turn`:

| Call | Does |
|---|---|
| `begin_turn(sim, cmds)` | charge and queue commands, activate due pending items |
| `advance_month(sim)` | run one tick, return that month's events, trace and frame |
| `finish_turn(sim)` | policy gaps, PC regeneration, turn += 1, history point |

`submit_turn` becomes `begin_turn` + 3 x `advance_month` + `finish_turn`, so Relaxed mode and every existing test are unchanged. A **golden/equality test** asserts the split path equals `submit_turn` for the same seed and plan (native and wasm).

### Wire (`sim-api`, `sim-wasm`): additive contract v1.1
- New calls: `beginTurn(commands) -> TurnStart`, `advanceMonth() -> MonthResult`, `finishTurn() -> TurnResult`. `submitTurn` stays.
- `TurnStart` carries accepted/rejected/activated and PC before; `MonthResult` carries that month's `MonthFrame` (events, flows, trace) and a snapshot; `TurnResult` is as today.
- Errors: calling them out of order returns `{ok:false, error:{code:'turn_state'}}`.
- **Save/load:** a save is only valid at a turn boundary (no mid-turn state). In Pressure mode "Save" saves the last boundary state and the current drafts; progress inside a quarter is not saved. `schemaVersion` stays 2 (additive API only).
- Update `docs/snapshot-contract.md` (new section), fixtures, and regenerate the golden file.

### Web
- **`store/clock.ts` (pure, no DOM):** the pace and phase state machine (`planning` while the quarter plays live, `locked`, `paused`), driven by an **injected time source** so it is testable with fake timers. Config in one file, `web/src/store/clock-config.json`: `mode` default `relaxed`, `realSecondsPerGameDay` 0.143, `speeds` [1,2,4], `onTimeout` `submit`, `allowPause` true, `pauseBudgetSeconds` null, `pauseWhenHidden` true. URL overrides for development (`?pace=0.5&mode=pressure`).
- **Game loop:** in Pressure mode the app calls `advanceMonth()` at month boundaries and `beginTurn`/`finishTurn` at quarter boundaries; in Relaxed mode nothing changes (`submitTurn`).
- **Panels:** clock and progress ring, Lock/Execute button, pause and speed controls, a mode choice in the New game screen (with a short explanation), a countdown cue when the quarter is about to end, a non-blocking quarter summary.
- **Scene:** per-month frame playback instead of the 3-month burst.
- Respect `prefers-reduced-motion`; a **slower pace option (0.5x)** and the pause button are the accessibility answer to timers.

## 5. Tests
- Rust: `begin_turn` + `advance_month` x3 + `finish_turn` equals `submit_turn` (state, events, trace, history) for several seeds and plans, native and wasm; out-of-order calls return errors; export/import only at boundaries.
- Web: clock state machine with fake timers (boundary, lock, timeout policies, pause, hidden tab, speed changes); the mock implements the three calls with the same rules; Relaxed mode behaviour unchanged (existing tests stay green).
- A scripted Pressure-mode playthrough in Playwright with an accelerated clock.

## 6. Delivery plan
1. **You review this doc.** I update `docs/snapshot-contract.md`, `docs/architecture.md`, `docs/how-to.md` and `docs/known-issues.md`.
2. **Rust agent:** the turn split, wire calls, fixtures, golden equality test.
3. **Web agent (can start in parallel on the mock):** clock module, game loop, panels, scene per-month playback.
4. **Independent review**, then you play it.

## 7. Risks
- **Fairness and accessibility:** timers exclude some players. Mitigations above (Relaxed default, pause, slow pace, no penalty beyond the clock).
- **Lead times are in turns:** a locked plan applies at the next boundary, so very short reaction windows can feel unresponsive. That is intended pressure, not a bug, but the UI must say "takes effect next quarter".
- **Mid-quarter saves** are not supported; if players expect them, we revisit.
- **Feel needs playtesting:** the 13 s quarter is a guess, chosen to hit the 15-minute target.

## 8. Defaults taken (user delegated: "I trust your judgment")
1. **Execute locks the plan until the quarter ends**; fast-forward (2x/4x) is the explicit way to get there sooner.
2. **No harder preset** now; only the config knobs.
3. **Timeout = submit** the drafts; no PC penalty.

## 9. (Former) open questions, now answered as above
1. Is **Execute = lock the plan until the quarter ends** right, or should executing **start the next quarter immediately**? (Immediate start would let a fast player skip ahead; I prefer fast-forward as the explicit control.)
2. Should Pressure mode have a **harder preset** (shorter quarters, limited pause budget) now, or only the config knobs?
3. Should the result of a hard timeout (**skip**) cost political capital, or is `submit` your final answer?
