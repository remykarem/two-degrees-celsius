# Town v5 design: information release (approved)

Status: **APPROVED (user: make v5 "more info-friendly" and expose more metrics; daily ticks are deferred to `docs/town-v10.md`).** No economic rules change in v5: it makes the existing model **visible and traceable**. Numbers and labels are defaults for the user to adjust.

## 1. What the user asked for
1. A **Finance tab**.
2. For **every entity (government, bank, household, person) the past transactions**, so any number can be traced back to its original value.
3. A unit of measurement for the **power grid** (it is not shown anywhere).
4. Clarity on whether **people spend at shops**.

## 2. Answers about the current model (checked in the code)
- **Power grid units.** Electricity is in abstract "energy units" per month (`energy.plant_capacity = 5.0` per plant; households use `0.1 + 0.15 x consumption/2000` per person; firms use `energy_per_unit` per output). Fossil plants emit `factor_fossil = 1.0` t CO2 per unit. **The UI shows none of this and no unit.** The magnitudes fit **MWh per month** (a person uses about 0.1 to 0.25 MWh a month; 1 t CO2 per MWh is coal-like), so v5 labels the unit **MWh/month** [PROPOSED] and surfaces supply, demand, rationing and intensity. No model change is needed; the label lives in one config field (`energy.unit_label`) so it can be changed.
- **People and shops.** Yes in the model: households split their monthly budget between food (Farm), goods (Factory) and **services (Shop)** (`goods.rs`, one `Purchase` event per household and firm). It is **not visible**: the scene draws coins flowing to firms, but figures never walk to a shop. v5 adds shopping trips (cosmetic, driven by real purchase flows) and, more importantly, shows where each household's money went in its ledger.

## 3. Finance tab (new panel, all values from the sim)
Sections, each with a monthly value, a trailing-quarter total and a sparkline from history:
- **Government:** revenue by source (wage tax, profit tax, capital tax, wealth tax, carbon revenue), spending by category (benefits, UBI, public purchases, job guarantee, road/plant upkeep, subsidies, interest), balance, debt, debt/GDP, legitimacy.
- **Bank:** equity, loans, deposits, capital ratio vs the limit, lending headroom, default rate, interest income, bailouts.
- **Households (sector and per household):** income by type, taxes, rent, loan payments, spending, saving rate, debt, net worth.
- **Firms (sector and per firm):** revenue, wage bill, profit, investment, debt, cash.
- **Money:** supply, created and destroyed this turn, with the sector-balance identity the sim already checks.
- **Circular flow:** a simple flow diagram (households, firms, government, bank) for the last quarter.
- **Prices and distribution:** price index, wage share, profit share, Gini, median vs average income.

## 4. Transaction ledgers (the traceability feature)
**Every money movement becomes a ledger entry**, and each account's ledger **reconciles**: `opening balance + sum of entries = closing balance`, checked per account on every tick.

- **Accounts with a ledger:** each household, each firm, the government, the bank. **Persons have no purse in the model** (money is household-level), so a person's ledger lists **wages earned** (each credited to their household), **employment events** (hired, fired, commute changes) and their share of nothing else; it says so. [PROPOSED]
- **Entry fields:** turn, month, tick, from, to, amount ($), kind, memo (for example "wage, Factory, Adult 2", "rent, home 3", "interest, loan"), and the account's running balance after the entry.
- **Kinds:** wage, dividend, rent, benefit, tax (by type), purchase (by sector), lobby, interest, loan_new, loan_repay, loan_writeoff, build, upkeep, subsidy, bailout, wealth_consumption, neutral (mobility, liquidation, payout).
- **Coverage rule:** all balance changes must be expressed as ledger entries (transfers, and bank money creation and destruction recorded against an `external` account such as "bank credit creation"). The Rust agent audits every delta that touches a balance. Anything it cannot attribute is reported as a bug.
- **Storage [PROPOSED]:** the ledger lives in the **API wrapper** (like history), not in the model state, so the simulation and its golden hashes are unaffected. Full-detail entries for the last **8 turns** (config `ledger.detail_turns`); older periods are kept as **monthly aggregates per (account, counterparty, kind)** for the whole game, plus **opening balances at turn 0** and closing balances per turn, so a value can be traced all the way back to the start with exact totals even when old detail is rolled up. Saved with the game.
- **UI:** a **Transactions** section in the inspector for household, firm, government and bank (and a person's earnings and events), filter by kind and period, with "open counterparty" links, a running balance column, and a reconciliation badge (opening + entries = closing). Metrics link to the ledger where they come from (for example debt, savings, tax revenue).

## 5. Energy panel
Supply (per plant, kind, MWh/month), demand (firms, households), served share, rationing events, grid intensity (t CO2 per MWh), renewable share, emissions by plant. Unit label from config. Ecology tab or its own tab. Hover text explains the unit.

## 6. Wire (contract v1.4, additive; section 14 of `docs/snapshot-contract.md`)
`snapshot.finance`, `snapshot.energy`, `SimPort.ledger(account, query)` returning a `LedgerPage` (opening, entries, rolled-up summary rows, closing, `reconciles`), the existing `purchase` flows, and new per-kind totals in `HistoryPoint`. Schema version unchanged unless the saved wrapper blob changes (it does: the ledger is saved; bump `schemaVersion` to 4 and refuse older saves cleanly).

## 7. Delivery plan
1. **Rust agent:** ledger capture in the engine (per-tick transaction list alongside events), audit of every balance-changing delta, ledger store with roll-up in `sim-api`, finance aggregates (monthly and trailing quarter), energy metrics, wire v1.4, reconciliation and coverage tests, fixtures, golden, wasm.
2. **Web agent (parallel, on the mock first):** Finance tab, ledger viewers in inspectors, energy panel, shopping trips in the scene, contract types, mock ledger with the same rules.
3. You play and tell me which numbers you still cannot trace; independent review.

## 8. Tests
- Per-account reconciliation every tick, native and wasm; total of all ledgers equals total money movement; roll-up preserves totals exactly; save/load round trip; ledger memory bounded; every `Delta` variant that changes a balance has a ledger kind (a test enumerates them).
- Web: ledger viewer logic, roll-up display, filters, finance derivations, energy formatting, shopping-trip planner.

## 9. Risks
- **Coverage gaps:** a balance change with no ledger entry would break reconciliation; the strict per-tick check turns that into a test failure, not a silent gap.
- **Data volume:** entries per turn could reach a few thousand; the detail window and roll-up bound it.
- **No behaviour change:** state golden hashes should be unchanged; wire hashes change.
- **Unit label is cosmetic:** if the user wants realistic physical units everywhere, plants and demand would need re-scaling and a balance pass.

## 10. v5.1 addendum: the "How the model works" explorer (user request, queued after v5)

**Request:** show the key modelling assumptions in the UI, for example how emissions are calculated, or how much power a household usually needs.

**Principle:** the explanations must be **generated from the real parameters and real model functions**, never hand-copied into the UI, so they cannot go stale when `content/params.json` changes. They must also be **honest about what is placeholder** (the numbers are tuned for gameplay, not calibrated to real data) and what the model leaves out.

### Catalog (a new content file, authored text with parameter references)
`content/assumptions.json`: one entry per topic with:
- `id`, `title`, `group` (Emissions and energy, People and households, Production and prices, Nature and climate, Weather, Government and policy, Finance and credit, Class and politics, Scoring and win/lose);
- `summary` (plain language, two or three sentences);
- `formula` (human-readable, with named symbols);
- `params`: the parameter paths used (for example `energy.hh_energy_base_pc`), resolved to **live values with units** by the API;
- `examples`: worked examples **computed by the model's own functions** (for example "a household of 3 spending $2,000 a month uses X MWh a month"; "a fossil plant running at full output emits Y t CO2 a month"; "a firm making N units with green tech 20% emits Z t");
- `status`: `placeholder` | `tuned` | `rough_real_world` (with a one-line note, for example "1 t CO2 per MWh is coal-like");
- `leaves_out`: what the simplification ignores (closed economy, 20 people, no trade, no births, and so on);
- `code`: the Rust file for developers (shown in a collapsed "for developers" line).

### Initial topics (minimum set)
1. Emissions: firm process emissions, energy emissions attributed to plants, household footprint, the sink, net emissions, grid intensity.
2. Power: household demand per person, firm demand per unit of output, plant capacity, rationing rule, the unit (MWh per month).
3. Carbon budget: `budget_b0`, how remaining share and the temperature display are derived, why a 40-turn game uses a small budget.
4. Production and wages: output function, effective hours and commutes, wage and price rules.
5. Household spending and housing: budget split between food, goods and services; rent; evictions and moves.
6. Money and credit: how bank lending creates money, capital ratio, defaults.
7. Taxes, benefits, political capital and lobbying: the effective-policy gap.
8. Class: how class is derived from ownership and income.
9. Nature: forest and commons growth, harvest, collapse threshold, climate damage to the farm.
10. Weather: stress curve, drought and fire rules, forecast.
11. Scoring: GPI, Gini, win and lose conditions.
12. "What this model leaves out" (a standing section).

### Wire (contract v1.5, additive; section 15 of `docs/snapshot-contract.md`)
`SimPort.assumptions(): Result<AssumptionTopic[]>`; static for a given params file, so the web caches it. Values are resolved in Rust from `Params`; examples call the model functions through a small registry in `sim-api` so there is **one implementation** of each formula.

### UI
- A **"How it works"** button (top bar) opening a searchable explorer: groups on the left, topic cards on the right (summary, formula, live parameter values with units, worked examples, status badge, what it leaves out).
- **Contextual links** "How is this calculated?" from: each metric, the energy panel, health meters, the stress gauge, the inspector (for example a household's energy use or footprint), the ledger kinds, and policy levers.
- In Pressure mode opening the explorer **pauses the clock** (it is reading time); a note says so.
- Accessible (keyboard, headings, search with live results), printable-friendly, light/dark.

### Tests
A test fails if a catalog entry references a parameter path that does not exist, if an example's function is missing, or if a documented group has no entry; examples are recomputed in tests so they match the live model; a `params` section with player-visible effect and no catalog entry is flagged (warning list, not a failure). Web: explorer filtering, contextual-link resolution (every link target exists), formatting of units.

### Delivery
After the v5 Rust and web agents finish (same wire and panels): a Rust agent for the catalog, the API call and the example registry (plus moving any inline formula that an example needs into a callable model function **without changing behaviour**), then a web agent for the explorer and links. The catalog text is authored by an agent from the code and docs, then reviewed by the user.

## 11. v5.2 transparency pass (user feedback after playing v5)

Merged with the v5.1 explorer (section 10): the "How is this calculated?" links and the explorer are the delivery vehicle. Principle: **every number is explained, decomposed and traceable**; a percentage never appears alone.

### 11.1 Findings checked in the code (facts, not decisions)
- **Taxes in the model:** *wage tax* (`tax_wage`, on wages; **relabel "Income tax"** in the UI, keep the key), *profit tax* (`tax_profit`, corporate tax on firm profit before dividends), *capital tax* (`tax_capital`, on households' dividends, rent and interest), *wealth tax* (annual rate above a threshold) and *carbon price*. **There is no GST/VAT or sales tax.** Adding one is a model change (a new lever, a new revenue line, ledger kind, effects on prices and demand): **not in this pass**; the Taxes explainer says so and lists it as a future option.
- **State shareholding:** the `Owner::State` variant exists, but no current economic system uses it (systems are Market, Regulated, Coop), so **state share is 0% everywhere**. The new Industry tab shows the ownership split per firm with that fact stated. Adding a state-ownership system is a future design.
- **Legitimacy:** `legitimacy += w_wellbeing x (avg wellbeing - 0.5) - w_unemployment x max(0, smoothed unemployment - threshold) - w_gini x max(0, Gini - threshold) - w_debt x max(0, debt/GDP - 1)`, minus a bailout penalty, clamped to 0..1 (`government.rs`). Currently only the total is shown.
- **Cumulative emissions** are already computed (`nature.cumulative`, on the wire as `cumulativeTonnes`) but not shown as a series; the emissions "by source" lines the user summed do not tally with the headline: **to be investigated** (suspects: net vs gross, the sink being negative, the informational `fire_sink_loss` line that must not be summed, firm vs plant energy attribution, monthly vs last-turn windows).

### 11.2 Requests and what they become
| Area | Request | Delivery |
|---|---|---|
| Economy | how Public debt/GDP, Wage share, GDP, GPI are calculated | explorer topics with live inputs, plus a **breakdown popover** on each metric: formula, each input with its live value, the result, and the components (GDP: firm sales + investment + government services; GPI: GDP x (1 - Gini) - social cost of carbon x emissions - depletion cost, each term in $). Wage share = wages / GDP and profit share = profits / GDP, with the numbers. Debt/GDP = government debt / annual GDP, both shown |
| Ecology | accumulated emissions; make the by-source numbers tally | **Emissions panel**: this month (gross by source: firms, households, plants; minus sink; = net), **this turn**, **accumulated (cumulative)** with a chart against the budget line, remaining budget; a **reconciliation line** (sources - sink = net = headline) that must hold; fix any mismatch the investigation finds (in the wire or the UI; model changes only with the user's approval) |
| Society | how Gini is calculated | explorer topic + a **Gini breakdown**: wealth by household (net worth, people), the ordering, the Lorenz curve, the formula and the result; also income Gini if cheap |
| Finance | rename "Wage tax" to "Income tax"; explain profit and capital tax; where is GST | label change (UI and wire `label` only), a **Taxes table**: each tax (income, profit, capital, wealth, carbon), what it is applied to, current rate (nominal and effective), revenue this month and quarter; "GST/VAT: not modelled" row with the reason |
| Finance | how legitimacy is calculated | **legitimacy breakdown**: each term (wellbeing, unemployment, inequality, debt, bailout) with its input, threshold, weight and the signed contribution this month, trend, and what drains or raises it |
| Policies tab (new) | list of policies initially in place and those proposed etc.; list of pending items | **"Policies" tab**: all levers grouped, each row: initial value (turn 0), current active value, effective (after lobbying), pending/queued value with its effective turn, drafted value (not submitted), last change (turn) and who blocked or weakened it; a **timeline** of changes (queued, activated, cancelled, rejected, weakened); the pending list as its own section with cancel and refund. Replaces nothing: the drawer stays for editing |
| Industry tab (new) | list all goods and services, public goods vs public infrastructure, % state shareholding | **"Industry" tab** (name: Industry; "Goods and services" in the subtitle): sections **Private firms** (Factory goods, Farm food, Shop services: output, price, workers, wage, revenue, profit, emissions, owner and **ownership split** with state share, with a clear "no state shareholding in the current systems" note), **Public infrastructure** (roads, public housing, power plants, fire stations: count, upkeep, build cost) and **Public goods and services** (government goods purchases, government services, job guarantee, benefits: spending per month and what it buys) |
| Overall | no bare percentages | every % metric shows its **backing numbers** next to it: Legitimacy (terms), Capital ratio (equity, loans, limit, headroom in $), Unemployment ("N of M", already), Debt/GDP (debt $ and GDP $), Wage and profit shares ($), Health meters (stock and max), Budget (tonnes used and remaining), Served share (supply and demand in MWh) |

### 11.3 Wire (contract v1.6, additive; section 16 of `docs/snapshot-contract.md`)
`snapshot.breakdowns` with named, typed objects: `gdp`, `gpi`, `wageShare`, `debtToGdp`, `gini` (with Lorenz points), `legitimacy` (terms), `capitalRatio`, `emissions` (month, turn, cumulative, by source, sink, net, `reconciles`), `taxes` (rows), `policies` (rows, timeline), `industry` (firms with ownership split, infrastructure, public goods). Every breakdown has `inputs[]` ({label, value, unit, source}) and `formula` text so the UI never recomputes model math. All values are produced in Rust from the same state and functions as the headline numbers; a Rust test asserts each breakdown's parts sum to its headline within a stated tolerance.

### 11.4 Delivery
1. **Rust agent:** investigate and explain the emissions tally gap; breakdown builders in `sim-api` (reusing model/metrics functions, no behaviour change; State hashes unchanged); policy history (wrapper state, saved); industry data; the explorer catalog and `assumptions()` (section 10); wire v1.5 and v1.6; fixtures; golden.
2. **Web agent (parallel, mock first):** breakdown popovers, Emissions panel with accumulated chart and reconciliation, Taxes and legitimacy views, Policies tab, Industry tab, "How it works" explorer and contextual links, relabel "Income tax", "no bare percentages" sweep across all panels.
3. Independent review, then the user's playtest.
