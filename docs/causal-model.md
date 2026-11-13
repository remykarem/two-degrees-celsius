# Why the model is the way it is

The original design doc of the economy (equations per system, loop table, targets) was written before the code and is now **removed: the code, the registry and the tests are the rules**. This page keeps only what lives nowhere else: the reasons for the main modelling choices, what is deliberately left out, and where each rule now lives.

## Why these choices

- **Demand-led economy, every payment is someone's income.** The first build let the rich save most of their income with nothing recycling it, and the economy slid to wherever government and investment spending landed; parameter search could not fix it. So investment buys goods from the Factory, savings recycle through the bank's lending, firms plan output and hiring from expected sales (smoothed, to stop 12-month boom-bust cycles), and consumption has a wealth term so wealth cannot be hoarded without limit.
- **Cost-plus prices, wages follow employment.** Prices close a share of the gap to `(1 + markup) x unit cost`; inventories absorb short-run gaps. A shortage never raises a price (known issue MK-1, by design; the "scarcity" loop is deliberately absent).
- **The government has no hidden reaction rule.** Spending is a player policy; only automatic stabilisers (benefits, income-linked taxes) react. A rule reacting to its own revenue or debt caused spirals. Debt bites through the bank's lending limit and legitimacy. A job guarantee exists as a lever the player chooses and pays for, not a hidden stabiliser.
- **Money is created and destroyed only by credit** (a loan creates a deposit, repayment destroys it), so conservation is one testable identity per tick.
- **A turn is a quarter of three monthly ticks.** Policy is queued with a lead time and costs political capital; firms discount a still-pending policy. The player steers every turn, so the economy must be steerable, not self-stabilising: sensible play works, neglect fails, nothing is `NaN`.
- **Households are the unit of money; people the unit of work.** Cohorts (weighted representative groups, the first design) were withdrawn: they were coarse and hid who is employed, housed or commuting. 22 people in 9 households is small enough to inspect and large enough for class and inequality.
- **Class is derived, never assigned** (income mix, ownership, skill, employment), so changing the economic system changes class by changing who receives dividends. Mobility moves a household through founding a firm in a dormant slot or losing equity; there is no inheritance.
- **One town, closed economy.** No trade, so the town cannot offshore its emissions (a limitation to tell the player) and money conservation stays easy to test.
- **Carbon budget and temperature are linear in cumulative emissions** (the temperature reaches 1.5 C exactly when the budget is gone). Overshoot hurts before game over through the weather and farm output.
- **Randomness has its own streams.** Weather draws from its own stream so nothing the player does shifts when a fire or drought strikes, only the odds (through emissions).
- **Lobbying scales a nominal policy down**: the player sets 50, gets 31, and the UI shows the gap and who caused it.
- **Systems are config.** An economic system (`content/systems/*.json`) overrides parameters and who receives dividends; it is not separate code.

## Deliberately left out

Trade and migration; stock markets; several firms per sector; births, deaths and inheritance; technology spillovers; household credit (the `household_credit` flag exists and is off); taxes on goods (GST is only a draft, `docs/features/gst-and-state-ownership.md`); price feedback from shortage (MK-1); a price level that feeds back (inflation is output only, `docs/features/inflation.md`); fuel, crew and hangar costs of the estate's jet. Random events exist only as weather (fires, drought) and the estate's flights. Each can be added as a new system without changing the others.

## Where each rule lives

| Question | Go to |
|---|---|
| The rule of a system | `crates/sim-model/src/<system>.rs` (one pure module per system; `TICK_ORDER` in `lib.rs`); values in `content/params.json` |
| Who decides what, with which inputs and effects | `content/decisions.json` and the Agents explorer; `docs/features/decision-points.md` |
| Cause and effect, sign, lag, loops | `catalog().causal` (generated); `docs/features/causality-graph.md` |
| Economic systems, levers, UI text | `content/systems/*.json`, `content/levers.json`, `content/ui.json` |
| The cohort-to-household change, the map, roads, plants | `docs/town-v2.md`; weather `docs/town-v4.md`; estate `docs/town-v7.md` |
| Where the code departs from a design | `docs/deviations.md` |
| Targets, baselines A to F, tuning history | `docs/tuning-log.md` |

## Tests that replace the old sign list

The causal sign tests are automated: `crates/sim-model/src/sign_tests/` and `tests/mechanism_signs` (the perturbation sign test checks every declared effect's direction), plus the scenario baselines in `tests/scenarios.rs`. The loops are generated, not listed; the findings against the original hand-written nine (three not modelled, one of the wrong kind) are in `docs/features/causality-graph.md` section 14.
