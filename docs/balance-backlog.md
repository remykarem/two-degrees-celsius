> **Status: HISTORICAL** (Phase 1-2, cohort model, numbers superseded by Town v2). Open items are carried into `docs/known-issues.md`; current baselines are in `docs/tuning-log.md` (Town v2).

# Balance and model backlog (parked)

Parked on purpose so the UI work is not blocked. Pick this up as "Milestone 1, Phase 2". Source: builder and reviewer reports after Phase 1 (commits `f85b96b`, `fb110f5`, `363b8a7`).

## State after Phase 1
- Turn layer (commands, pending queue, political capital, `submit_turn`, `preview`, `--plan`) is implemented.
- Demand-side redesign is implemented.
- `cargo test --workspace` passes (27 passed, 2 ignored in the scenarios suite); fmt and clippy clean. No invariant fired in baseline A.
- The economy is **not balanced** as a game. See below.

## Baseline A (no action, seed 42, 160 turns) at end of Phase 1

| Year | Unemployment | Gini | Debt/GDP | Legitimacy | Carbon budget left |
|---|---|---|---|---|---|
| 1 | 10% | 0.47 | 0.95 | 0.72 | 98% |
| 10 | 5% | 0.54 | 1.74 | 0.02 | 79% |
| 40 | 8% | 0.63 | 3.85 | 0 | -6% |

Budget exhausted in year 38 (target in section 9: years 22 to 35).

## Decisions made
- **Starting government budget is balanced** (decided). Doing nothing must not collapse the town on day one through debt alone. Expect higher starting unemployment; the player then chooses between spending, taxing and borrowing.

## Phase 2 status (update)
Items 1 to 7 below are RESOLVED or documented in `docs/deviations.md` #39 to #46, `docs/tuning-log.md` (Phase 2) and the baseline tests in `tests/scenarios.rs`. Remaining work: see 'Open problems' in `docs/deviations.md`.

## Open issues

| # | Issue | Kind | Notes |
|---|---|---|---|
| 1 | Default government runs a permanent deficit; legitimacy reaches 0 around year 10 even with low unemployment | Gameplay balance | Addressed by the balanced start. Re-check the legitimacy debt term afterwards. |
| 2 | Private demand cannot sustain the economy: with government spending G = 0, unemployment reaches 52% and Gini 0.8. G is effectively the only steering lever. | Gameplay design + plausibility | Give the player at least one more demand lever. Candidates: higher minimum wage, cooperatives (wage share), progressive tax with transfers, public investment, UBI. |
| 3 | Deflation: wages stuck at the minimum-wage floor while the price index falls to about 0.5 by year 40; real wage rises during unemployment | **Model plausibility (a modelling artifact)** | The one item that is an error, not a design choice. Review the wage and price rules in `crates/sim-model/src/labour.rs` and `goods.rs`. |
| 4 | Green investment arrives in a lump: unemployment swings 0% to 18% in the two years after a carbon price is announced | Gameplay feel | Smooth the response (spread over quarters) or accept it as a deliberate "transition shock". |
| 5 | Carbon-price sign test passes only when emissions are measured in years 2 to 4 after the price takes effect (it takes effect 2 turns after queueing) | Test design | Decide the intended response time and document it. |
| 6 | Government financing loop only partly broken: bank lending room now limits government borrowing, but baseline debt still rises | Model | Check after the balanced start. |
| 7 | Budget exhaustion year 38 vs target 22 to 35 | Calibration | `budget_b0` is not calibrated. Do this last. |

### Resolutions (Phase 2)
Item 1: RESOLVED (balanced start; legit debt term softened, #41, #43).
Item 2: RESOLVED by verification (benefit, UBI and wage tax already steer demand, #Demand levers); no new lever.
Item 3: RESOLVED (wage trend and anchor, #39); the start-up price index of 0.75 remains.
Item 4: RESOLVED (spread over months, #42).
Item 5: DOCUMENTED: a carbon price is announced and takes effect 2 turns after queueing; firms start to react to the announced price (discounted), green investment builds over 2 to 3 years, and emissions fall visibly from year 2. The test measures years 2 to 4 after the price takes effect.
Item 6: CHECKED: bank lending room limits government borrowing; unfunded payments only appear late in baseline A ($2.5 million over 40 years). Debt still rises with the structural deficit (#41).
Item 7: RESOLVED (b0 220000: A exhausted in year 24.9, #44).

## Still to do in Phase 2
1. Apply the balanced start; re-run baseline A.
2. Write **baseline B (sensible play)** and **baseline C (neglect)** turn plans. Section 9 requires: B survives 160 turns with the budget intact and legitimacy above 0.3; C fails by about year 30; at least one single-tool strategy (for example carbon tax only) fails.
3. Resolve issues 2 to 6.
4. Calibrate `budget_b0` (issue 7).
5. Un-ignore the two ignored tests: `default_market_run_lands_near_the_design_targets` (rewrite for section 9 targets) and `golden_snapshot_seed_42` (after calibration).
6. Review the 17 new deviations (#22 to #38 in `docs/deviations.md`) and decide which to promote into the registry.
7. Re-run the independent reviewer on the final code.

## Framing for Phase 2
These are mostly **game design questions**, not bugs: the code does what the design says, so the work is deciding how the game should feel and tuning to it. Test every change against baselines A, B and C, not against single numbers.

---

## Town v7 open balance decisions (CURRENT; the rest of this file is HISTORICAL)

Measured in `docs/tuning-log.md` "Town v7". Each needs the user's decision; nothing here was changed.

1. **Carbon budget (`nature.budget_b0`, 900 t).** The estate (about +12% gross emissions) moved A's exhaustion from turn 27.3 to 24.3 and B's final margin from 30% to 10%. `budget_b0` 1,050 gives A at turn 28.0 and B 23% left. Alternative: `estate.flight_prob_monthly` 0.2 (A 25.0, B 15%).
2. **Legitimacy (`government.legit_gini`, 0.012).** The estate takes the Gini from 0.72 to 0.88; B's minimum legitimacy fell from 0.61 to 0.24, A's to 0.11 to 0.16. `legit_gini` 0.008 puts B at 0.44.
3. **Baseline F** survives in 20 of 20 seeds (was 13): the "one tool must not be enough" target (known-issues WN-1) is now missed by F as well as being only just met by D and E. The cause is the Shop price (70 to 75) that the estate made necessary (V7-2, V7-3).
4. **Government borrowing capacity** at the start is $65k from raw content ($215k before; V7-4). Options: lower the estate's savings further, or raise a firm's debt in `initial_town.json`.
5. **Pilot cash margin.** $1,800 a month leaves a margin of about $1.2k; neglect (C) dismisses the pilot in 7 of 20 seeds. Raise the wage or the estate's income if the user wants no dismissal there.
