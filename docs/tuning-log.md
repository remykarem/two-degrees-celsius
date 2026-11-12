> **Status:** everything before the "Town v2 (Rust model builder)" section is HISTORICAL (cohort model, 1,000 people). The Town v2 section and its baselines are CURRENT. Index: `docs/README.md`.

# Tuning log (milestone 1, WIP)

Seed 42 unless noted; seeds 42 and 7 gave near-identical results (the RNG is used rarely).
Search scripts lived in /tmp/cal (random search over ~16 parameters, then hill-climb); scoring penalised
|mean unemployment - 9%|, peaks > 25%, |GDP CAGR - 2%|, |Gini end - 0.55|, legitimacy < 0.3, debt/GDP > 0.9, unemployment std.

## Manual parameter changes
| Param | From | To | Why |
|---|---|---|---|
| firm wage (town) | 2000 | 1650 | factory payroll exceeded revenue at tick 1 |
| household_amort | 0.008 | 0.003 | landlord could not service debt |
| firm_term_months | 60 | 120 | debt service drained firm cash |
| wage_up | 1.02 | 1.01 | wage-profit squeeze cycles |
| max_fire_share / max_hire_share | 0.25 / none | 0.06 / 0.10 | oscillation |
| inventory_adjust | 0.25 | 0.10 | oscillation |
| capital_price_usd | 1000 | 1500 (4000 tried) | return on capital / initial Gini |
| depreciation | 0.004 | 0.008 | keep investment large |
| max_invest_share | 0.25 of revenue | 0.012 of capital value/month | investment booms |
| return_scale | 0.15 | 0.30 | smoother intensity |
| wealth_consumption_rate | 0.002 | 0.004 | rich-saving demand leak |
| home values C, D, E | 9M, 8.75M | 12M, 14M | initial Gini 0.62 -> 0.40 |
| tax rates (wage/capital/profit) | 20/20/20 | 28/40/25 | fund government purchases |
| public_spending | 0.15 of revenue | 0.20 of GDP | see deviations.md #1 |
| initial_income_usd | guessed | fixed point of tick 1 | initial consistency |

## Observed runs (480 ticks)
- Stage 2 only (no government spending, no investment): economy collapsed to ~99% unemployment; conservation held.
- Everything on, first version: boom-bust, unemployment 0-53% mean 21%, gov debt 127M, Gini 0.86, collapse (legitimacy 0).
- tfp growth 0 and no investment: unemployment ~55% for 10 years.
- Public spending sweep 0.16-0.26 of GDP (taxes 22/30/25): unemployment mean 28% -> 11%, gov debt/GDP ~4, Gini ~0.81, legitimacy 0.
- Balanced-budget purchases: unemployment ~60%, depression spiral (rule abandoned).
- Random search 1 (old structure): best u mean 3-5%, u max 20-22%, CAGR -0.7% to +0.6%, Gini end 0.40-0.47, debt/GDP 0.7-1.2.
- Random search 2 (with smoothing, warm-up 24, 4x400 samples), best of each:

| Run | u mean | u max | u std | Gini start -> end | GDP CAGR | gov debt/GDP | legit end |
|---|---|---|---|---|---|---|---|
| 11 | 6.1% | 21.5% | 5.8% | 0.45 -> 0.54 | 0.8% | 1.12 | 0.00 |
| 12 | 5.0% | 20.0% | 6.1% | 0.46 -> 0.55 | 0.3% | 1.05 | 0.14 |
| 13 | 0.2% | 13.2% | 2.0% | 0.46 -> 0.55 | -1.1% | 0.83 | 1.00 |
| 14 | 0.3% | 12.7% | 2.2% | 0.46 -> 0.48 | -0.8% | 0.74 | 0.40 |

Run 11 parameters: public_spending 0.273, debt_brake_start 2.39, worker c 0.933, manager c 0.681, tax wage/capital/profit 0.314/0.295/0.256,
wealth_consumption_rate 0.0098, max_invest_share 0.0137, return_scale 0.307, tfp goods/food 0.00126/0.00056, wage_tight_up 1.0015,
growth_aspiration 0.001, stabilizer 3.6, payout 0.647, min_wage 1142, repay_share 0.487.

## Not done
Carbon budget B0 (currently 400000 t) was not calibrated to year 28. No search config met all targets. The
chosen parameters in content/ are NOT the search winners.

## Stage C (redesign) setup changes, second builder
Not a calibration. Done only to get a consistent starting state under the new rules.
- `initial_income_usd` for the six cohorts set to the tick-1 disposable income of the new model (122106, 5203, 233719, 345114, 345114, 85060).
- `public_spending_usd` 330,000 (the old tick-1 value) gave 20% unemployment after warm-up; 450,000 gives about 10%. Chosen by two trial runs, not searched.
- `run.warmup_ticks` 48 (was 0 in the WIP commit): the first year without warm-up is a 40% unemployment shock because the hand-written town is not a fixed point.
- New parameters with first-guess values: `labour.wage_k` 0.1, `u_ref` 0.07, `wage_cap` 0.01, `expected_smoothing` 0.3, `goods.price_adjust` 0.05, `invest.accelerator` 0.05, `max_invest_share` 0.02, `goods.wealth_consumption_rate` 0.002.

## Phase 2 (balancing), seed 42
Method: one-parameter and small-grid runs of the CLI (`--set`), judged on the yearly table (unemployment, Gini, debt/GDP, legitimacy, budget left), not on a score. Baselines A, B, C, D are the scripts in `content/scenarios/`.

Findings that drove the changes:
- Government deficit about 140k a month at the old start (revenue 480k, committed payments 118k, G 450k, interest 47k on 15 million of debt after warm-up). Cutting G or raising taxes barely moved it: output and revenue fell instead (the deficit equals the private sector's net saving). Net saving by cohort: managers 42k a month, landlord 9k, workers 7k each, firms 9k, bank 4k.
- So the levers that worked were on the saving side: manager propensity, wealth effect, payout.

| Param | From | To | Why |
|---|---|---|---|
| goods.propensity manager / worker / capitalist / rentier | 0.7 / 0.95 / 0.4 / 0.45 | 0.9 / 1.0 / 0.45 / 0.5 | balanced start |
| goods.wealth_consumption_rate | 0.002 | 0.01 | balanced start |
| market payout_ratio | 0.6 | 0.7 | balanced start |
| policy tax_wage / public_spending_usd | 0.28 / 450000 | 0.30 / 400000 | balanced start, about 8% unemployment |
| labour.u_ref | 0.07 | 0.09 | natural rate of this economy; wages no longer fall to the floor in warm-up |
| labour.inflation_anchor, productivity_passthrough (new) | n/a | 0.0016, 1.0 | wage/price drift (backlog 3) |
| income.excess_cash_months, excess_payout_rate (new) | n/a | 6, 0.1 | idle firm cash |
| invest.green_spend_share (new) | n/a | 0.1 | smooth green investment (backlog 4) |
| government.legit_debt | 0.02 | 0.008 | structural deficits |
| nature.budget_b0 | 400000 | 220000 | baseline A exhausted in year 24.9 |

Tried and rejected: initial firm wage 1250 to 1400 (unemployment 18% to 25%, wages pinned at the floor anyway); higher u_ref 0.10 to 0.12 (unemployment follows u_ref up: 12% to 14%); a lower initial government debt (bank equity would be negative); raising taxes in baseline B (legitimacy collapsed).

Baselines at the end of Phase 2 (years 1, 6, 11, ... in the CLI table):
- A: exhausted year 24.9 (turn 100); unemployment mean 11%, Gini 0.415 to 0.46 (0.51 after exhaustion), debt/GDP 0.4 to 1.1, legitimacy 1.0.
- B: survives, budget 50% left, unemployment 5% to 25% (92% of ticks below 15%, the peak is the carbon-price shock in years 3 to 6), Gini 0.40 to 0.45, debt/GDP 1.35, legitimacy at least 0.7.
- C: budget exhausted year 23.7 (turn 95), Gini 0.57, price index doubles (wage-price spiral after tax cuts).
- D (carbon tax 100 to 400): budget exhausted year 33.5 (turn 134); emissions cut by 20% to 25% only, unemployment up to 21%.

## Town v2 (Rust model builder)
Seed 42 unless a range is given. Method: one-parameter changes and small grids through the release CLI (`--set`), judged on the yearly table and on 8 to 20 seeds, never on a search score. Baseline plans are in `content/scenarios/`.

Start: the town is hand-built, not a fixed point; `run.warmup_ticks` stays 48. The numbers below are the **settled** start (turn 1, after warm-up): Gini 0.72, top household 61% of wealth, average income per person 1,727 dollars a month, median 1,204, one of eight households unhoused, 2 of 10 labour-force people without work. Before warm-up (the file as written): Gini 0.742, top household 60.5%. Destitute household net worth about -1,300 dollars.

| Parameter | From | To | Why |
|---|---|---|---|
| all extensive quantities | Phase 2 | divided by 50 | 1,000 to 20 people (deviation T1). `B0` 4,400 t was **not** recalibrated: baseline A exhausts it at turn 123 in the median of 20 seeds, inside the 90 to 140 target. |
| `goods.wealth_consumption_rate` | 0.01 | 0.005 | 0.004 gave a recession (rich households saved ~3,600 dollars a month, a tenth of GDP, nothing recycled it); 0.01 drained the Owner family's savings in warm-up and pulled the top share from 61% to 56%. 0.005 is a compromise, not a fixed point. |
| `labour.fire_after_months` | 1 | 3 | One layoff is 25 to 33% of a 3-person firm. |
| `labour` hire/fire (new) | caps as shares | 1 hire and 1 fire a month, slacks 0.1 and 1.0, min 1 worker | flip-flopping (Goods 4,3,2,1 workers in 10 months, 60% unemployment) |
| `metrics.unemployment_smoothing` (new) | n/a | 0.05 | see T3 |
| `goods.services_share` (new), Shop price and output | n/a | 0.45; price 70, output 140 (first guess 60 and 200) | Shop revenue was 8k against a 9k wage bill: it defaulted in year 2 and the town collapsed to 87% unemployment |
| Goods start output | 160 | 130 | the Factory over-produced and piled up inventory, then cut staff in a spiral |
| `class.startup_*` (new) | n/a | cost 30,000, buffer 5,000, capital share 0.8, `mobility_rate` 0.03, `startup_min_excess_demand` 0.2 | with 0.05 a transient excess demand let a second Shop in; both Shops failed (3 to 7 of 20 seeds collapsed in A and B). At 0.2 founding is rare (it did not occur in the 20-seed runs reported below). |
| `government.legit_gini` | 0.02 | 0.012 | at 0.02 and a Gini of 0.72 legitimacy reached 0 in year 12 in baseline A, before the budget mattered |
| `energy.plant_capacity`, plant capacities | n/a | 5 per plant, two fossil plants at the start (supply 10 against a demand of 8 to 9.5) | capacity close to demand so that removing a plant rations output |
| `goods.rent_*`, `rent_burden_cap` | rent +1% a month while full | +-0.5% a month, burden cap 0.2 | rent must stop rising while one household is always unhoused |
| `labour.skill_wage_slope` etc. (new) | cohort premiums | 2.3, ref 0.5, floor 0.6 | T11 |

Tried and rejected: wealth consumption 0.004 (recession); Goods output 160 and Shop price 60 (Shop insolvency cascade); `startup_min_excess_demand` 0.05 (destructive founding); tier rounding that counted skilled workers only as skilled (over-hired the general tier, a bug, fixed); minimum wage 1,700 to 2,000 (unemployment 43% to 89%); G 10,000 to 16,000 (gov debt grows faster, unemployment mean 11% to 13%, no better).

### Baselines at the end of Town v2, 20 seeds (1 to 20), 160 turns
- **A (no action):** budget exhausted in 20 of 20 runs, turn 122.7 to 126.7 (median 122.7); legitimacy reaches 0 near the same time in most runs; unemployment mean 12% (the labour force is 10 people). Gini does **not** rise: seed 42 goes 0.72 to 0.63 and the top share from 61% to 41% by year 40 (managers and workers save while the rich spend their savings). The design said "inequality rises"; reported, not forced.
- **B (sensible play, `baseline_b_sensible.json`):** survives 19 of 20 (95%), median budget left 46%, median minimum legitimacy 0.44, Gini about 0.55 at the end. One seed collapses on mass unemployment after a firm defaults.
- **C (neglect and extraction):** all 20 runs fail by legitimacy zero at turn 31 to 33 (year 8); Gini rises to about 0.8. It fails earlier than A, which is the design; the cause is the legitimacy rule, not the carbon budget (the budget is exhausted at turn 132 to 135, later than A, because the town collapses first). Fossil plants added in C change nothing: supply already exceeds demand, so a plant only matters when demand outgrows 10 units.
- **D (carbon tax only):** all 20 runs end in legitimacy zero around turn 154; median budget left 5%. The carbon tax works on firms (green technology cuts their footprint) but the plants stay fossil and inequality keeps eating legitimacy. "Fails" here is marginal (6 turns before the end).

## Carbon budget horizon (Paris 2030)
Request: the budget took too long to deplete (no-action baseline A: 50% left at turn 64, exhausted at turn 124). The Paris horizon is 2030, about 4 years from now, so the budget should run out in about that time. Decision for the user: A should have 50% left at about turn 8 to 10 and exhaust at about turn 17 to 20; the calendar starts in 2026 so turn 17 is about the end of 2030.

Measured (seed 42, no actions, net emissions about 10.3 t at turn 1 rising to 11.4 t a month by turn 20, so about 33 t a turn): cumulative 288 t at turn 9, 591 t at turn 18. Linear, so `nature.budget_b0` = 600 gives 50% at turn 9 to 10 and exhaustion at turn 18 to 19.

| Parameter | From | To | Why |
|---|---|---|---|
| `nature.budget_b0` (`content/params.json`) | 4400 | 600 | see above. Temperature `T = t0 + (t_limit - t0) x cumulative / b0` still works: 1.30 at the start, 1.5 at exhaustion by construction |
| `game.startYear` (`content/levers.json`, read by the wire `meta`; `web/scripts/sync-mock.mjs` now reads it too instead of a hard-coded 2031) | 2031 | 2026 | turn 17 (Q1 2030) to turn 20 (Q4 2030) |

Nothing else changed: `run.turns` stays 160 (it is already a param). The remaining-budget share, the production climate-damage ratio and temperature all scale by `cumulative / b0`, so they bite 7 times faster per turn; the wellbeing and legitimacy terms were checked and do not collapse early.

### Baselines with the 600 t budget, 20 seeds (turns are 1 quarter)
Seeds differ by at most one tick (B2).
- **A (no action):** 50% left at turn 9.7 (tick 29), exhausted at turn 18.7 (19.0 in 2 seeds), outcome `BudgetExhausted` in 20 of 20. No early collapse: seed 42 at the end of the budget has legitimacy 0.57 (never below 0.3 before it), wellbeing 0.70, unemployment 0 to 10%. The loss is the budget alone.
- **B (sensible play):** cannot win in this horizon. Seed 42 remaining budget: 54% at turn 9, 19% at turn 17, 0% at turn 25.3 (all 20 seeds 25.3 to 26.0). Emissions fall from 10.3 t to 9.0 t a month at turn 9 and 6.6 t at turn 17, 3.4 t at turn 25: the carbon price takes 2 turns, each plant 4 turns, so the cut arrives after most of the 600 t is emitted. B buys about 6.5 turns over A, and the town stays healthier (legitimacy 0.65 to 0.67, wellbeing 0.78 to 0.81, unemployment 0 at turn 17), not a win. With a very large budget the same script survives 160 turns (cumulative 554 t at turn 20, 610 at 25, 783 at 40, 1,284 at 80, 2,388 at 160).
- **C (neglect and extraction):** 50% at turn 9, budget exhausted at turn 17.3 (tick 52), outcome `BudgetExhausted` in every seed (earlier than A by about 1.5 turns). The old "fails by legitimacy zero at year 8" is no longer the failure mode.
- **D (carbon tax only):** 50% at turn 10, budget exhausted at turn 19 (tick 57), fails in 20 of 20. Emissions fall only to 10.3 t by turn 17.

### Options to keep a sensible game winnable (not implemented, a decision for the user)
1. **Shorter game** (`run.turns` = 20, the end of 2030; already a param, no model change). Seed 42 B ends turn 20 with about 8% of the budget left (554 of 600 t): a win, but a thin one that depends on the B script being good, and the UI, fixtures and about ten tests assume 160. Not done: the margin is not clear enough to ship.
2. **Larger budget.** B needs `b0` above about 800 t to survive 40 turns and about 2,400 t for 160 turns. A 1,000 t budget gives A about 30 turns and B a win at the 40-turn mark only if the game is also shortened.
3. **Faster emissions response** (plant build time 4 turns, carbon-price lag 2 turns, a larger green subsidy effect): a model change, would need new tuning of B, C, D and 20 seeds.
4. Changing the starting emissions (smaller fossil plants) is also a model change.
Recommendation: option 1 together with a modest raise of `b0` (for example 700 to 800 t, so that B wins at turn 20 with 20 to 30% left and A still loses by about turn 22), or accepting that the 4-year Paris game is a "buy time" scenario where success is the best score, not survival. Pick one before the weather model.

## Town v4: weather (forest fires and drought)
Goal: weather that works, is tunable (every number in `content/params.json` under `weather`) and is gentle by default. It is **not** tuned for baseline B to win: the game length and `budget_b0` may still change.

Start values (first guess, kept): `onset` 0.50, `full` 0.10, `curve` 1.5; `drought_base` 0.01 a month (x(1 + 4s)); `fire_base` 0.012 a month (x(1 + 3s) x dryness x standing share); `spread_p` 0.10 per neighbour a month; fire lasts 3 months; `burn_frac` 0.30; `a_c` 0.8, `a_f` 0.3, `a_e` 0.1, community buffer 0.5; fire station 8,000 dollars, upkeep 30, radius 3, spread cut 0.6, duration cut 0.34; resilience scale 2,000 dollars a month, cuts 0.5 (ignition) and 0.4 (drought). Nothing was iterated to hit an outcome; the one change made while building was structural (weather off during the warm-up, deviation W14).

### Baselines with weather, 20 seeds (1 to 20), counted until the budget is exhausted (the game ends there)
Command: `cargo test -p sim-engine --test weather weather_baselines -- --nocapture` (it also re-runs each script with `fire_base` and `drought_base` at 0 for the "off" column).

| Script | 50% left at turn | Exhausted at turn (weather on / off) | Fires started (mean) | Droughts started (mean) | Burnt tiles (mean) | Runs with a fire / a drought | Farm health low point (on / off) |
|---|---|---|---|---|---|---|---|
| A no action | 9.7 | 18.7 / 18.7 | 0.75 | 1.00 | 1.10 | 12 / 13 of 20 | 0.72 / 0.74 |
| B sensible play | 10.0 | 25.4 / 25.4 | 1.10 | 1.05 | 1.60 | 15 / 14 | 0.72 / 0.74 |
| C neglect | 9.0 | 17.7 / 17.7 | 1.25 | 0.85 | 1.65 | 17 / 12 | 0.56 / 0.60 |
| D carbon tax only | 9.7 | 19.2 / 19.3 | 0.95 | 1.00 | 1.20 | 14 / 14 | 0.72 / 0.74 |

Reading: with a 600 t budget onset is at turn 10 and the game is over at turn 19 (A), so a typical run sees about one fire and one drought, and the weather moves the exhaustion turn by at most about 0.1 turn and the farm health low point by 0.02 to 0.04 (0.04 in C, where the commons is also over-harvested). A to D keep their order and outcomes; the A-D baseline tests pass unchanged. B lasts longer (turn 25), so it meets more weather (1.1 fires) at higher stress. The weather is felt as events and as farm and wellbeing hits, not as a change to the budget race. After the budget is exhausted stress stays at 1 (about 0.05 droughts and 0.07 ignitions a month), which only matters if the game is lengthened to play on.

Sign checks done in `tests/weather.rs`: higher stress raises ignition (measured over 20,000 draws against the formula); burning lowers the sink and raises net emissions once; stations cut burnt area (paired seeds); the community commons loses less regrowth in a drought; resilience spending lowers ignition and drought damage with diminishing returns.

Easy knobs if the weather should bite more: `fire_base`, `drought_base`, `a_f` (the farm), `smoke_wellbeing`; less: the reverse, or raise `onset`'s distance from `full`.

## Horizon: 40 turns, ~900 t, weather visibility pass
Request (user): a 40-turn game (10 years, 2026 to 2036), a carbon budget of about 900 t, and weather that the player SEES: it starts when 75% of the budget is left (first asked at 50%, then changed to 75%) and intensifies gradually. The user had played to 50% left and seen no fire or drought (the old defaults give about 2.5 events a game, some seeds none; they may also have been on the mock or a stale wasm package, see README). Method: a throwaway probe test over seeds 1 to 20, baselines A to D, no behaviour tuning; the probe was deleted.

### Changed
| Parameter | From | To | Why |
|---|---|---|---|
| `run.turns` | 160 | 40 | the user's decision; tests and the web mock read it (the mock's `lastTurn` comes from `sync-mock`) |
| `nature.budget_b0` | 600 | 900 | A: 75% left at turn 7.3, exhausted at turn 27.5 (target 26 to 30) in 20 of 20 seeds |
| `weather.onset` | 0.50 | 0.75 | user decision (stress starts at 75% of the budget left) |
| `weather.full` | 0.10 | 0.10 (unchanged) | stress is 1 at 10% left, about turn 24 in A |
| `weather.curve` | 1.5 | 1.0 | a linear ramp, so the first turns after onset already have some stress; still gradual |
| `weather.drought_base`, `fire_base` | 0.01, 0.012 | 0.12, 0.09 | per month at zero stress; needed so the first event comes soon |
| `weather.drought_stress_gain`, `fire_stress_gain` | 4.0, 3.0 | 1.0, 1.0 | frequency doubles from onset to full stress; severity, duration and spread still scale with stress |
| `weather.force_stress` (new) | n/a | `null` | development override: a number 0..1 fixes the stress from tick 1 |

### Result, 20 seeds, events counted until the budget is exhausted (fires started + droughts started)
| | 75% left at turn | exhausted at turn | outcome | events a game (mean, range) | seeds with an event within 3 turns of onset |
|---|---|---|---|---|---|
| A no action | 7.3 | 27.4 | budget 19, bank failure at turn 27 in 1 | 16.1 (11 to 20) | 19 of 20 |
| B sensible | 7.3 | not exhausted | collapse in 20 of 20 (bank failure or mass unemployment, turns 22 to 30) | 25.7 over the game to the collapse (21 to 32) | 19 of 20 |
| C neglect | 7.0 | 25.8 | budget 20 | 17.0 | 19 of 20 |
| D carbon tax only | 7.3 | 28.4 | budget 20 | 17.1 | 19 of 20 |

Events per 3 turns after onset in A: about 1.9, 1.9, 2.4, 2.2, 2.6, 2.9, 2.3. The player sees an event within about 1 turn in most seeds (first delay: median 3 months); the worst seed waited 13 months. Weather off (the same code with both base odds at 0): A exhausts at 27.7, C at 25.8, D at 29.0.
Compromise, stated plainly: the brief asked for both "an event within about 3 turns in 90% of seeds" and "roughly 3 to 6 events by exhaustion". They conflict. Stress is near zero just after onset, so a 90% chance of an event in 9 months needs about 0.2 events a month from the start, and 50 months later that is 12 to 20 events. Visibility won (19 of 20 seeds, 95%); the count is 16 in A. The pattern is steady rather than sharply rising: the frequency rises from about 1.9 to 2.9 per 3 turns; what rises most is severity, duration and spread (they scale with stress). If this is too busy, lower the two bases and raise the two gains: visibility drops (0.08/0.06 with gains 1.5 gave 18 of 20 and 10 events at the old 50% onset).

### Baseline B: honest answer, it cannot win in 40 turns at 900 t
- By carbon alone B has a margin: it never exhausts 900 t; about 18 to 20% of the budget is left when the game stops, and it would need more than about 800 t.
- The town collapses first. Cause (measured by switching parts off): `production.climate_damage` x (cumulative / b0)^2 cuts farm output; at about 15% damage (turn 28 with b0 900) the farm fails, unemployment goes to 70% and the bank fails. With `climate_damage` 0 the same B script survives 40 turns in 20 of 20 seeds with 13% of the budget left. With `climate_damage` 0.15 it still collapses.
- Before the weather bump (old weather, 900 t): collapse at turn 28 to 30 in 20 of 20 seeds, budget left 19.7%, 3.4 events. With the weather switched off: turn 30 (17 of 20), left 20.1%. With the new weather: turn 22 to 26, because fires and droughts cut farm health (lowest farm health in B 0.47, mean 0.52, against 0.70 without weather).
- Budget needed for B to survive 40 turns: 1,500 to 1,600 t (18 of 20 seeds; 1,200 t still collapses at turn 31). At 1,500 t A is not exhausted in 40 turns in 12 of 20 seeds, so no budget satisfies both "A fails clearly before 40" and "B wins". The shipped 900 t favours A. Options for the user: lower `production.climate_damage` (a model change, needs a new tuning round), make the farm less fragile, or accept B as a "buy time" strategy.
- C and D fail (by the budget); A fails.
Sanity of what scales with cumulative / b0: temperature is 1.30 at the start and 1.5 at exhaustion by construction (final mean 1.57 in A at turn 28); legitimacy in A stays above 0.45 until the end (lowest 0.53 mean-seed minimum); wellbeing is above 0.45 in every seed; the farm health low point in A is 0.41 to 0.54 (0.69 without weather).

### Developer override
`cargo run -p sim-cli -- --seed 3 --turns 6 --set weather.force_stress=0.7` (verified: stress 0.70 from tick 1, droughts and fires begin at once; a value outside 0..1 is refused). `--set` works because every param is overridable. A web URL option is not added: it would need a web change (`WasmSim.init` already accepts a params JSON string, but the web never passes one).

## Winnable: plan B with adaptation
Request: make baseline B (sensible play) able to win the 40-turn, 900 t game, preferring (a) a better plan, then (b) lower `production.climate_damage`, (c) a farm-fragility mitigation, (d) a larger budget. No parameter was changed: (a) worked. Weather parameters are untouched, so visibility is as before: no-action game 16.1 events (11 to 20), an event within 3 turns of onset in 19 of 20 seeds (`a_no_action_game_shows_weather...`).

### What was wrong with B
The old plan B predated weather and adaptation: carbon price ramp, three renewables, roads, public housing, a wealth tax, the commons harvest cap. It collapsed in 20 of 20 seeds (turns 22 to 26). Cause, measured: farm output falls (climate damage and drought), food gets dearer, wages follow (about +38% in 4.5 years at full employment against +13% for goods prices), the Services firm's profit goes negative and it defaults, unemployment jumps and the bank fails.

### Findings from the experiments (20 seeds, survived of 20)
- Fire stations carry the result. Renewables, fossil plants closed and roads with stations, and nothing else: 20 of 20; the same with no stations: 15 of 20.
- Government borrowing kills it. Public housing (40,000 dollars) added to B: 1 of 20; a third renewable plant: 1 of 20 (the earlier plan only worked because those two builds were rejected as `InsufficientFunds`); `public_spending` 10,000: 5 of 20; resilience spending 2,000 a month: 5 to 6 of 20, 1,000: 19, 500: 20.
- Carbon price ramp, green subsidy, benefit and a wealth tax are neutral here (19 to 20 of 20 each way). A bank capital ratio of 0.15 lowers it (14).
- With the old climate damage the shipped B wins; with `climate_damage` 0.2 and 0.1 it wins 20 and 18 of 20 in an earlier variant; the response is not monotonic because outcomes are decided by one firm defaulting. So (b) was not needed and no value was searched.

### Plan B now (`content/scenarios/baseline_b_sensible.json`, no command rejected, PC never binding)
Turn 1: carbon price 50, green subsidy 0.4, resilience spending 500, fire station (2,3), renewable plant (11,3). Turn 2: renewable (11,5), benefit 800. Turn 3: fire station near the farm (7,8). Turn 4: commons harvest cap 0.6. Turn 6: third fire station (3,6) among the forest. Turn 7: roads (4,6), (9,6). Turn 9: remove the first fossil plant, carbon price 100; turn 11: remove the second. Turn 16: carbon price 150, resilience 750. Turn 17: wealth threshold 100,000 and tax 0.005. Turn 24: carbon price 200. Not used: public housing (net loss), a third plant (borrowing), community commons (needs a 45 PC system switch), UBI, job guarantee, minimum wage (not helpful or not needed; not all were tested individually).

### Results, seeds 1 to 20
| Script | Survived | Outcome | Budget left (median) | Farm health low point | Weather events a game |
|---|---|---|---|---|---|
| A no action | 0 of 20 | budget exhausted (1 bank failure), turn 27.0 to 28.0 | 0 | 0.53 mean | 16.1 |
| B sensible with adaptation | 20 of 20 | survived 40 turns | 30% | 0.57 to 0.69 | 22.6 |
| C neglect | 0 of 20 | budget exhausted, first loss tick 75 to 81 | 0 | 0.31 mean | 17.0 |
| D carbon tax only | 0 of 20 | budget exhausted | 0 | 0.52 mean | 17.1 |
| E adaptation only (3 stations, resilience 500 then 750) | 0 of 20 | budget exhausted (1 bank failure) | 0 | 0.62 mean | 15.3 |
| F renewables only (2 plants, both fossil closed) | 13 of 20 | 7 mass unemployment | 29% | 0.48 mean | 23.7 |

Honest remainder: F does not fail in most seeds, so "a single tool must not be enough" is met for D and E only (known-issues WN-1). B also sits near a cliff (WN-2). Not claimed: B was not tried by a human, and the Regulated and Coop systems were not tested for a winning plan.

## Town v7: the estate and its pilot

Request: implement `docs/town-v7.md` (approved) and measure baselines A to F before and after; tune content only if the estate breaks stability or makes sensible play unwinnable. Harness: `cargo test -p sim-engine --test scenarios baseline -- --nocapture --test-threads=1` (20 seeds each), `cargo run --release -p sim-cli --example estate_probe` (pilot dismissals, flights, estate savings, Gini for A to F) and `cargo run --release -p sim-cli --example tune_probe -- section.field=value ...` (A exhaustion turn, B survival, budget left and legitimacy minimum under overrides). State hashes changed on purpose (`cargo run -p sim-cli --example statehash` before and after); the wasm golden, fixtures, insta snapshot and mock data were re-blessed.

### What broke first, and what was changed
With the design's numbers as written (Farm to the estate, pilot $2,400, estate savings $20,000, Shop $70) **every baseline collapsed on mass unemployment in 20 of 20 seeds, in the settling months** (loss at tick 0). Cause, found by switching parts off: the Shop (profit about zero already in Town v6, WN-2) defaulted. Adding the pilot lowers unemployment to about 0% (wage drift up), so the Shop's wage bill rises; with the Farm's dividends moved to the estate the Shop's demand is a little lower. Neither change alone collapsed it (no pilot: survived; Farm kept by the Owner family: survived), both together did.

| Change | From | To | Result (20 seeds) |
|---|---|---|---|
| Shop `price_usd` (content/initial_town.json) | 70 | 75 | 70 and 71: collapse in 20 of 20 (A, B, F alike). 72: A 20 of 20 budget, B 19 survived and 1 bank failure, F 20 survived. 73 and 75: A to F all as the design intends. 75 chosen for margin from the cliff at 71/72 |
| `estate.pilot_wage_usd` | 2,400 | 1,800 | At 2,400 (estate savings 20,000): pilot dismissed in B at turn 13 in 20 of 20 seeds (carbon price 100 reaches the Farm), in C at turns 19 to 33 in 20 of 20; none in A, D, E, F. At 1,800: none in A, B, D, E, F; C 7 to 8 of 20 (turns 20 to 31). At 1,500: C 2 of 20. 1,800 keeps the design target "no dismissal in A to F unless the player raises the wealth tax" and keeps C's neglect to jobs link |
| Estate household `savings_usd` | 20,000 | 15,000 | Bank headroom at the start 15k to 65k (it was 215k); the mock (which starts from raw content) could not afford any plant at 15k |
| Worker C `initial_income_usd` | 1,700 | 3,800 | the pilot's wage is part of the household's income |

### Baselines, seeds 1 to 20 (before = Town v6, after = Town v7 as shipped)
| Script | Before | After |
|---|---|---|
| A no action | half the budget gone at turn 14.0, exhausted at **27.3** (min 27.0, max 28.0), 19 budget and 1 bank failure; legitimacy minimum over the game 0.18 to 0.48 | half gone at turn 12.0, exhausted at **24.3** (min 23.7, max 24.7), 20 of 20 budget; legitimacy minimum over the game 0.11 to 0.16 |
| B sensible with adaptation | 20 of 20 survived, 30% budget left, legitimacy minimum 0.61 | 20 of 20 survived, **10%** left (7% to 12%), legitimacy minimum **0.24** (0.24 lowest) |
| C neglect | budget exhausted, first loss tick 75 to 81 | first loss tick 66 to 67 |
| D carbon tax only | 20 of 20 budget exhausted | 20 of 20 budget exhausted |
| E adaptation only | 19 budget, 1 bank failure | 20 of 20 budget exhausted |
| F renewables only | 13 of 20 survived (7 mass unemployment) | **20 of 20 survived** |

Start (after settling, seed 42): Gini 0.72 to 0.88, top wealth share 0.61 to 0.79, unemployment (raw content) 10.0% to 9.1%. Jet in A: 47 flights and 95 t per game (expected 96), flights in every seed.

### Not tuned: the user's decisions (design section 9, point 3)
Measured with `tune_probe` (A exhaustion turn, B survived, B budget left, B legitimacy minimum):
| Override | A exhausted | B survived | B budget left | B legitimacy min |
|---|---|---|---|---|
| none (shipped) | 24.3 | 20 of 20 | 10% | 0.24 |
| `government.legit_gini` 0.012 to 0.008 | 24.3 | 20 of 20 | 10% | 0.44 |
| `nature.budget_b0` 900 to 1,050 | 28.0 | 20 of 20 | 23% | 0.24 |
| `estate.flight_prob_monthly` 0.4 to 0.2 | 25.0 | 20 of 20 | 15% | 0.24 |

So: the budget race is a `budget_b0` or flight-chance choice (1,050 puts A back at about turn 28 and B at 23% left); legitimacy is a `legit_gini` choice. Nothing is unwinnable, so nothing was changed (AGENTS.md: do not tune to hit a number). Logged in `docs/balance-backlog.md` "Town v7" and `docs/known-issues.md` V7-1 to V7-4.

## Independent review F3: the weather's own random stream

Request: give weather its own stream (state schema 5), so player actions that change whether the class, estate or enclosure draw cannot shift fire and drought timing; take those other draws every month. The state hash changed on purpose (fixtures, golden, recording and the insta snapshot re-blessed). Harness: `cargo run --release -p sim-cli --example baselines` (A to F, seeds 1 to 20). No content was retuned.

| Script | Before (survived; budget run-out turn median; budget left median) | After |
|---|---|---|
| A none | 0 of 20 (budget); 24.33; -0.57 | 0 of 20 (budget); 24.00; -0.58 |
| B sensible | 20 of 20; no run-out; 0.10 | 20 of 20; no run-out; 0.10 |
| C neglect | 0 of 20 (budget); 22.33; -0.62 | 0 of 20 (budget); 22.33; -0.68 |
| D carbon tax | 0 of 20 (budget); 24.67; -0.52 | 0 of 20 (budget); 24.67; -0.52 |
| E adaptation | 0 of 20 (budget); 24.00; -0.58 | 0 of 20 (budget); 24.00; -0.58 |
| F renewables | 20 of 20; no run-out; 0.09 | 20 of 20; no run-out; 0.09 |

Result: stability and winnability are unchanged within seed noise (win rates identical, run-out turns within one third of a turn). F still survives in every seed (known-issues WN-1, unchanged).
