# Town v5 design: daily ticks (draft for review)

Status: **DRAFT, nothing implemented.** Planned after the "Horizon: 40 turns, ~900 t, weather visibility" change (see `docs/tuning-log.md`) and a playtest. Numbers are placeholders. **[DECIDED]** = from the user; **[PROPOSED]** = my choices.

## 1. Goal [DECIDED]
All the displayed numbers should change continuously with the game clock instead of jumping once a month or a quarter. The user chose a **truly finer simulation** (not just visual smoothing): **daily ticks**, by dividing the monthly deltas proportionately.

## 2. The idea, and where "just divide the deltas" is enough and where it is not
Dividing a monthly delta by 30 is the right basic move (it is Euler time-stepping with `dt = 1/30` month). It is exact enough for **flows** (production, emissions, income and spending accruing, nature stocks, interest). It is **not** enough for four kinds of rules the model has today:

| Kind of rule | Example | Right treatment |
|---|---|---|
| **Compounding or rates per month** | wage up 2% a month, depreciation, regrowth | use `(1+r)^dt`, or `r x dt` for small rates |
| **Probabilities per month** | fire ignition, drought onset, firm founding | `p_day = 1 - (1 - p_month)^(1/30)` |
| **Counters in months** | 3 months of rent arrears, `fire_after_months`, drought `months_left` | count in days (x30) or in `dt` units |
| **Discrete decisions** | hire/fire, set prices, invest, move house, lobby, class change | **do not slice**: run on a cadence |

Also **money is integer cents** and conservation checks are exact, so slicing needs a remainder-carrying helper (`slice(total, day)`: day 30 gets the remainder), never plain division.

## 3. Proposed design: continuous processes daily, decisions on a staggered monthly cadence [PROPOSED]
- **Time:** 1 month = **30 ticks (days)**; a turn (quarter) = **90 ticks**. Calendar simplification: 12 x 30 = 360-day years in the sim; the UI date display may still use real dates.
- **Daily (scaled by `dt`):** production and output, all emissions, nature stocks and the sink, weather (hazard draws per day), income accrual (wages, dividends, benefits, interest, taxes), spending and rent accrual, loan interest, inventories, wellbeing, commute time effects.
- **Decisions run once every 30 days, staggered:** each firm, household or person has a decision day = `id mod 30`, so layoffs, price changes, moves, investments and class changes are spread across the month. That is what makes aggregate numbers such as unemployment, prices and Gini move almost continuously instead of in monthly steps, while each rule keeps the calibrated monthly logic the Phase 2 tuning was based on.
- **Player commands** still apply at the turn boundary (`begin_turn`); lead times stay in turns. Political capital regenerates per turn.
- **Safety net:** a param `time.ticks_per_month` (default 30). With **1** the new engine must reproduce today's behaviour **bit for bit** (the current golden files must still pass). That is the regression test for the whole refactor.

## 4. What changes in the code
- **`sim-types` / `sim-params`:** a `Clock { day_in_month, ticks_per_month, dt }` passed to every system; `time` block in params. `run.ticks_per_turn` becomes derived (`3 x ticks_per_month`).
- **`sim-model`:** every system classified into the table above and updated (an audit table of all rules goes into `docs/deviations.md`). Systems stay pure and return deltas; money deltas go through the slicer.
- **`sim-engine`:** the turn runs `ticks_per_turn` daily ticks; `advance_month` becomes `advance_days(n)` (with `advance_month = advance_days(30)` kept for Relaxed mode and tests). Events and trace are collected per day.
- **`sim-metrics`:** indicators computed per day. Flow indicators (GDP, income per person, emissions) are **annualised from a trailing 30-day window** so the numbers do not look jittery; stocks (savings, forest, budget) are read directly.
- **`sim-checks`:** the exact money-conservation and sector-balance checks run **every day**; add a check that daily slices sum to the monthly total.
- **Weather:** probabilities per day; `months_left` counters in days.
- **RNG draw order changes**, so goldens and fixtures regenerate. Behaviour will shift (an Euler step is not the old monthly map), so **balance and `budget_b0` need re-calibration** and the baselines A to D are re-run.

## 5. Wire and web
- **Contract v2.0 (additive where possible):** `advanceDays(n) -> DaysResult` returning the new day number, a light snapshot (indicators, date, PC, weather, turnPhase) and the events of those days; a full snapshot at month ends and turn ends. `MonthFrame` (the animation frame) stays one per month, **aggregated** from the 30 daily ticks, so the scene's animations keep working. `turnPhase` gains `dayInMonth` and `daysDone`. `advanceMonth` stays as a convenience.
- **Pressure mode:** at the default pace (1 s = 7 game days) the clock asks for 7 daily ticks a second; the web calls `advanceDays(k)` per animation frame and **every metric** updates from each result. Pause, speeds and the quarter boundary behave as in v3.
- **Relaxed mode:** `submitTurn` runs 90 ticks at once, as it runs 3 today; the end-of-turn playback is built from the 3 aggregated month frames.
- **UI:** all metrics tick (GDP, GPI, unemployment, income per person, median income, wealth share, emissions, budget, forest/commons/farm health, stress, money, debt); a day counter in the clock strip; sparklines sample monthly points (not daily); a subtle "live" cue on changing numbers. Respect reduced motion by throttling updates.
- **Performance estimate:** the sim costs about 0.5 ms per 3-month turn today, so about 15 ms per 90-day turn (30x). The JSON snapshot per tick is the cost to watch: hence the light per-day result.

## 6. Tests
- **Bit-for-bit regression:** `ticks_per_month = 1` equals today's output (all existing goldens).
- **Convergence:** with `ticks_per_month` 30 vs 15 vs 60, aggregate quarterly outcomes agree within tolerance (a sign that the Euler step is stable, not a precise proof).
- **Conservation every day**; slicing sums to monthly totals; staggered decisions touch each agent exactly once a month; weather rates match monthly probabilities in expectation over many seeds; determinism; native-vs-wasm bit-identical.
- Web: clock and live loop with day ticks (fake timers), metrics update per day, Relaxed regression.

## 7. Risks
- **Biggest:** behaviour shifts, so a re-calibration of baselines, `budget_b0` and weather. The `ticks_per_month = 1` switch lets us compare old and new directly.
- **Audit size:** every rule must be classified; a missed one gives a quiet bug (for example a monthly constant applied 30 times). The audit table and the convergence test are the defence.
- **Rounding:** integer cents in slices; remainder carry required.
- **Data volume:** 30x more events; mitigated by aggregation and light day results.
- **Feel:** some numbers will still move in small integer steps (one person hired or fired changes unemployment by 10 points with 10 workers). That is the small-population effect of Town v2, not a time-step problem.

## 8. Delivery plan
1. You review this doc. I update `docs/causal-model.md` (time-step section), `docs/snapshot-contract.md` (v2.0), `docs/architecture.md`, `docs/how-to.md`.
2. **Rust agent (staged, one at a time):** (a) clock plumbing and the `ticks_per_month = 1` regression; (b) audit and convert systems; (c) indicators, checks, weather; (d) re-calibration and baselines.
3. **Wire and web agents:** `advanceDays`, light results, live metrics, day counter, Pressure loop (can start on the mock once the contract exists).
4. You play; then an independent review.
5. Start only after the horizon and weather change is committed (same parameter and golden files).

## 9. Open questions
1. Is **30-day months, 360-day years** acceptable in the sim (UI keeps real calendar dates)? I assume yes.
2. Should player commands also be able to apply **mid-quarter** (on any day)? I assume no; the turn boundary stays the unit of play.
3. Daily tick makes the **end-of-turn playback** in Relaxed mode a 3-month replay as today. Is that fine, or should Relaxed mode also play a day-by-day replay?
