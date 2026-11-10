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
