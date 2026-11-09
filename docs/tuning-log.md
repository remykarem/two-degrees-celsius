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
