# Deviations from docs/causal-model.md

Status: milestone 1 is a WIP. The model is unstable and not calibrated (see tuning-log.md).

| # | Deviation | Reason / symptom it was meant to fix |
|---|---|---|
| 1 | Government goods purchases = `public_spending` x monthly GDP x unemployment stabiliser x debt brake (not in doc; doc has only benefits, upkeep, builds, subsidy, UBI). | Taxes were saved or repaid, draining demand; the circular flow did not close and unemployment hit 40-60%. Balanced-budget variants were procyclical and spiralled into depression. Brake added because gov debt reached 4x GDP. |
| 2 | 24-month warm-up before tick 0 (`run.warmup_ticks`). | The hand-written initial town was not a stationary state; the first year was a recession. |
| 3 | Capital price 1500/unit, depreciation 0.8%/month (doc 0.4%). | Return on capital was 40-60% with low prices, and wealth Gini at start was 0.6; 1500 gives Gini 0.40. Higher depreciation keeps investment (a demand source) large. |
| 4 | Investment: scaled by (return - threshold)/return_scale, capped at a share of capital value, profit and plan exponentially smoothed. | Raw rule gave 12-month boom-bust cycles with unemployment swinging 0-40%. |
| 5 | Wage rises (vacancies, tight labour) only if the firm made a profit last month; extra small rise when unemployment < 5%. | Wages rose while profits were negative, causing squeeze and layoffs. Doc has only +2% (here +1%) and -0.5%. |
| 6 | Hire cap 10% and fire cap 6% of workforce per month; target workers include an inventory adjustment. | Violent hire/fire oscillation. |
| 7 | Skilled/general job tiers: 16% of jobs go to premium-wage cohorts (managers). | Cheapest-first hiring left managers permanently unemployed. |
| 8 | Inventory holding cost is physical spoilage (2%/month), not money. | A money cost needs a recipient; spoilage keeps conservation simple. |
| 9 | No inter-firm input costs. | Closed economy, one firm per sector; nothing to buy from. |
| 10 | Carbon price applies to firm emissions only, not households. | Doc formula ambiguous (P_c x sum E). |
| 11 | Households do not take new loans; existing household debt amortises. | Doc gives no household borrowing rule. |
| 12 | Bank holds only loans; equity = loans - deposits (checked exactly). Initial loans are set above deposits. Deposit interest paid to households only. Bailout swaps government bonds for equity; failure without bailout is a deposit haircut. | Simplest stock-flow-consistent reading of S5. |
| 13 | Investment spending, subsidies and startup capital are paid to households as wages (construction work). | Closed economy: money must go somewhere. |
| 14 | Founding probability is `mobility_rate x weight x (return - rate) / founders_per_firm` per month, with 5 founders moving to the capitalist cohort. | Discretises the doc's fractional weight transfer. |
| 15 | Class function works per cohort: employed if >= half the labour force; manager needs skill > 0.7 and wage > 2500. | Cohorts are groups, not individuals. |
| 16 | Gini is computed between cohorts only (within-cohort inequality ignored); owner-occupied homes added as wealth. | Cohort agents. |
| 17 | Goods allocation across firms in a sector by stock and price^-2, two passes. | Needed once dormant slots become firms. |
| 18 | Effective (post-lobbying) policy takes effect the tick after it is computed. | S9b runs at the end of the tick. |
| 19 | Campaign cap and community commons cannot be set under Market. | Doc table lists them as available only in other systems. |
| 20 | Tax rates default to wage 28%, capital 40%, profit 25% (doc has no values). | Chosen to balance the budget with deficit 1; not validated. |
| 21 | Initial disposable income, government purchases and investment plans are seeded in initial_town.json. | First-month spending needs a previous month. |

## Open problems
- Unemployment is not self-stabilising: it settles wherever government plus investment spending put it, with 12-month cycles in some configurations.
- Targets not met; see tuning-log.md. The `tests/scenarios.rs` target tests probably fail.
