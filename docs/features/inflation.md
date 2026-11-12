# Feature: Inflation (measure it, then explain what drove it)

Status: **DRAFT, for review. Not approved, not implemented.** Docs only: this page changes no code. Written from the code at commit `3bd0871` (`crates/sim-model/src/{goods,labour,income,util}.rs`, `crates/sim-metrics/src/lib.rs`, `crates/sim-api/src/{finance,history,snapshot}.rs`, `content/params.json`). Where this doc and code disagree, code wins (`docs/README.md`). Builds on the Markets view (`docs/features/markets.md`, scope A, contract v2.3) and the trade lists (`docs/features/market-trades.md`, v2.4). Everything here is **output only**: `cargo run -p sim-cli --example statehash` must be byte-identical before and after.

## 1. Why
The user is new to economics. The game has prices but no answer to "are prices rising, by how much, who does it hurt, and why?". Inflation is also the cleanest place to teach the cost-plus pricing that `markets.md` section 2.1 warns can look odd. Rule of the game (memory: pedagogical, thin UI): the sim computes every number and sentence in Rust and `content/`; the web only renders.

## 2. What the model does today (re-checked in code)
- **Goods prices are cost-plus** (`goods.rs` ~404-416). Each month a firm moves `price += price_adjust x (target - price)`, `price_adjust` = 0.05, `target = (1 + markup) x unit_cost` (`price_target`, line 239), `unit_cost = (wage_bill + carbon_cost) / output`. Excess demand is computed but never enters the price. So there is **no demand-pull inflation** in this model today.
- **What really enters unit cost** (this corrects the shorthand "wages, carbon, energy, productivity, markup, interest"):
  - *Wages:* `wage_bill` = sum of `person_pay` (firm wage x skill multiplier, floored at the minimum wage). Yes.
  - *Carbon:* `Firm.carbon_cost` = carbon price x the firm's footprint, which already includes the emissions of the electricity it uses (`income.rs` ~58). Because energy has no price in this model (`markets.md` 2.5), **the energy bill reaches prices only through carbon**, and the stored value is one tick old. Yes, as "carbon".
  - *Productivity:* output per worker (`output` is in the denominator). Yes.
  - *Markup:* set once at init from the starting price and cost (`init.rs` ~410) and never changed afterwards (verified: only read). It is a constant factor today; it is a decomposition row only so that a future rule that changes it shows up (it contributes zero now).
  - *Interest / credit spread:* **not in unit cost.** Interest (`income.rs` ~70) reduces profit only. Today a higher interest rate cannot raise a price. The row is shown with value zero and a plain sentence saying so (honest, and it teaches that interest-driven cost-push needs a rule that does not exist yet).
- **Wage rule** (`labour.rs` ~150): `trend = inflation_anchor + productivity_passthrough x sector TFP growth`, `inflation_anchor` = 0.0016 a month (about 2 per 100 a year), and `step = clamp(trend + wage_k x (u_ref - u), +-wage_cap)`. Wages drift up by the anchor even if prices do not move, and respond to unemployment. The anchor is a hard-wired expectation, not measured inflation: **there is no wage indexation to the price level** (out of scope, section 9).
- **What is measured today:** `sim-metrics::compute` (lib.rs ~367) sets `price_index = sales_value / real_sales`, where `real_sales = sum(sales_units x initial price)` for active firms. It is an **implicit deflator** (current-quantity weights, like a GDP deflator; starts near 1.0; includes machines bought by firms and government purchases, not only what households buy). It feeds `gdp_real_annual`. It is on `Indicators.price_index`, on `Finance` (`priceIndex`) and shown **only** as one row "Price index" under Finance > "Prices and distribution" (`web/src/panels/finance.ts` ~126, `fmt: 'index'`, no sparkline) and as an input of the real GDP breakdown (`breakdowns.rs` ~286). **`HistoryPoint` does not keep it** (it keeps `gdp_real_annual`, no price series for the index), so no chart and no inflation rate exists.
- Markets scope A already gives per-market price series on `HistoryPoint`: six optional scalar fields (shelf prices, average wage, private rent, average loan rate). Food, goods and services shelf prices there are the raw material for the CPI; no new price history is needed for them.

## 3. Measures (all computed in Rust)

### 3.1 Two inflation numbers, named honestly
1. **Producer-side deflator inflation** = change in the existing `price_index`. Cheap, already computed, but it moves when the sales mix changes and includes capital goods and government purchases. Label: "Prices of everything the town's firms sell". Kept for the real GDP link.
2. **Cost of living (CPI)** = a basket price index of what households buy. This is the headline. Label: "Cost of living".

Teach the difference in one explainer line: a deflator follows what is produced; a CPI follows what a household buys.

### 3.2 The basket and its weights
Components: **food, goods, services, rent**. (Wages are not in a CPI. Energy has no price here. Credit is not a household purchase; mortgage interest is out of scope.)
- **Weights are the town's own spending**, never typed in `content/`: for the window, `w_s = household spending on s / total household spending on the basket`. Goods, food and services spending come from the household purchase flows already in the ledger (`FlowKind::Purchase` by `Purpose`, `docs/how-to.md` section 12a; the exact accessor is to be confirmed in step 1, it must be the same figure the Finance ledger shows); rent from private plus public rent paid by households. Computed in `sim-metrics` as a pure function of the ledger window, no `f64` money in state.
- **Fixed-base Laspeyres vs chained.** Fixed base (weights frozen at the start) is simple and its parts add up trivially, but this town changes a lot in 10 years (rent burden, food share, a carbon price) so frozen weights drift from real spending, the classic substitution bias. Fully chained every month is accurate but noisy with three firms (one per sector) and cannot be explained in one sentence. **Recommendation (default, Q1): chained Laspeyres with weights reset once a game year (every 4 turns) from the previous year's spending; base 100 at the first month.** Month-to-month the index is a Laspeyres of that year's weights, linked at each reset. For the first year the weights are the start-of-game weights from the opening month's own spending (the opening month is already real: it is a normal simulated month, see 3.4).
- A share is shown as money, never a percentage: "Of every $100 households spend, $31 goes on food" (`per_hundred`, as markets does).

### 3.3 Rates and honest early months
- `cpi[t]` index, base 100 at the first simulated month.
- **Month on month** from month 2. **12 months** from month 13 (`cpi[t] / cpi[t-12] - 1`).
- Months 2 to 12: no fake annual rate. The card shows "Over the last N months: prices rose $X per $100" with N the months so far, labelled "since the start" (not annualised; annualising 2 months is noise). Month 1: "no change to measure yet".
- **The first months are a catch-up, not a shock.** Initial prices are not exactly at the cost-plus target, so a gap closes at 5 percent a month. The decomposition (section 4) shows this as its own row, "Prices settling to where costs put them", so the player is not told that the starting adjustment is caused by wages. Open question Q3: show or hide it (default: show, it is true and teaches the adjustment lag).
- Wording: never a bare percentage (rule from markets). Text is "Prices rose $4 per $100 over the last 12 months" (rate x 100, formatted in Rust like `per_hundred`); a fall is "fell $2 per $100"; near zero (|rate| under `content` `flat_band`) "were about flat".
- Per-sector price changes: the same window for each of food, goods, services, rent, and for wages ("Wages rose $6 per $100"), as a list `{id, label, change, text}`, in the same word form.

### 3.4 Real vs nominal, and who it hurts
- **Real wage** = nominal wage / CPI (index to base 100): per sector's average wage (already a `HistoryPoint` series) and the town average. Teaches that a pay rise below inflation is a cut.
- **Household CPI** (the poor feel different inflation): each household's own shares (its own spending over the window, same ledger, same chained rule at household level but with a single window of weights, 12 months, no reset, to stay cheap). Real income = disposable income / that household's CPI. Shown in the Society income view as a small list "Prices rose $X per $100 for the poorest fifth, $Y for the richest fifth" (quintiles by the same income rank the Gini uses). Honest limit: the model has one price per sector, so the difference comes only from basket shares (poor households spend more on food and rent), not from different shops.
- Explain gainers and losers from the sim's own numbers: savers (cash loses value; deposits pay `base - deposit_spread`, so real return can be negative), debtors (fixed nominal debt gains), wage earners (real wage), renters, the estate (rich, largely asset-held). Only statements the numbers back are shown; the topic text says "in this town" and cites the figure.

### 3.5 History
`HistoryPoint` gains optional scalar fields (additive, serde defaults, same decision as markets Q3 for old saves: load with empty series and start at the load point): `cpi`, `cpiFood`, `cpiGoods`, `cpiServices`, `cpiRent`, `priceIndex` (deflator). Stored as indices so any window can be differenced; no stored rates. `SCHEMA_VERSION` bump if the saved log changes (recipe `how-to.md` section 14 step 3 and section 4 step 2).

## 4. Decomposition: what drove it (the cost pass-through identity)
Goal: for any window, sector price change = sum of named causes, summing **exactly**, from the cost-plus rule itself.

### 4.1 The identity
Per firm (a sector has one firm today; with several, sum is the unit-weighted average, section 4.4), the target is
`T = (1 + m) x ( wbar / q + kappa x eps )`
with `wbar` = wage bill per worker (pay incl. skill mix and the minimum-wage floor), `q` = output per worker (productivity), `kappa` = carbon price, `eps` = tonnes of footprint per unit made (carbon intensity; it includes the electricity the firm uses, so this is where the "energy" cause lives), `m` = markup (constant today). Interest is not a factor (section 2).
Output-only trace per firm per month (new `TraceRecord`, as `markets.md` 3.3 item 1): `wage_bill`, `workers`, `output`, `carbon_cost`, `footprint`, `carbon_price`, `markup`, `price_before`, `price_after`, `floored` (min price hit) and `unit_cost_zero` (price frozen). It is exactly what `goods.rs` already holds locally; nothing reads it, so the hash cannot move.

### 4.2 Step one: split each month's target change exactly (log-mean, LMDI)
`T` is a sum of two product terms, `A = (1+m) wbar/q` and `B = (1+m) kappa eps`. For a product term `X = f1 x f2 x ...` moving from `X0` to `X1`, the factor `i` gets `L(X1, X0) x ln(f1_i / f0_i)` with `L(a, b) = (a - b)/(ln a - ln b)` (logarithmic mean; `L(a, a) = a`). This sums to `X1 - X0` **exactly** (no interaction leftover, unlike one-at-a-time changes). Causes per month, in dollars of target: **wages** (`wbar`), **productivity** (`q`, sign reversed: more output per worker lowers price), **carbon price** (`kappa`), **energy and carbon intensity** (`eps`), **markup** (`m`), and **interest** (always 0). Log and exp through `libm`, as for all transcendentals.

### 4.3 Step two: carry the causes through the 5 percent catch-up (linear filter)
Actual price is not the target: `D[k+1] = D[k] + a (g0 + tau[k] - D[k])`, where `D = P - P0`, `a` = `price_adjust`, `g0 = T0 - P0` (the starting gap) and `tau[k] = T[k] - T0 = sum of causes up to k`. This recursion is **linear**, so run it once per cause with that cause's cumulative `tau_i` as the input, once more for the constant `g0` (the "settling" row), and the parts add up to `D` by linearity. A window `[t0, t1]` is the difference of cumulative parts at `t1` and `t0`.
The sector percentage change is `D / P(t0)`; the CPI change is `sum_s w_s x (sector change)` with the Laspeyres weights of section 3.2, so **each cause's contribution to the CPI is `sum_s w_s x part_s / P_s(t0)`** and the causes sum to the CPI change. Rent and wages are added as their own causes ("rents rose", with the occupancy/burden rule, `markets.md` 2.3) because rent is a quarter or so of the basket; its own cause is "rent rule", not cost pass-through.

### 4.4 What is exact and what is approximate (say so on the card and in the test)
- Exact: the LMDI split, the filter linearity, the weighted sum to the CPI.
- Approximate, shown as one row **"Rounding and price floors"** (never hidden): (a) prices are rounded to cents each month (`Cents::from_f64`), (b) `min_price_usd` floor and the frozen months when `unit_cost = 0`, (c) the carbon cost is one tick old (the trace records what the rule used, so this is exact, not an error), (d) chained weight resets (a re-weighting changes the index's composition, not its parts; the reset month is split at the old weights). The residual is `actual change - sum of parts`; the test bounds it (section 7).
- With more than one firm per sector (a dormant slot can wake) the sector price is the unit-weighted average and the parts are the same weighted average of per-firm parts.

### 4.5 Output
`Inflation.drivers: Vec<{id, label, value, text}>` ordered as `content/ui.json` says, e.g. "Wages +$2.1 per $100 (higher pay is passed into prices)", "Carbon price +$0.9", "Productivity -$0.4 (more output per worker lowered prices)", "Interest $0 (this town's firms do not add interest to prices)", "Prices settling +$0.3", "Rounding and floors $0.0". Plus a headline sentence chosen in Rust from the largest driver.

## 5. Pedagogy
Topics in `content/assumptions.json` (recipe `how-to.md` section 15), each linked from the card's "how?":
- `inflation_what` (what inflation is: the same dollar buys less; CPI is the price of a typical household basket; the rate and the index; why the base is 100).
- `inflation_gainers_losers` (savers, debtors, wage earners, renters, the poor and the rich; uses the household list).
- `inflation_demand_pull_cost_push`: **demand-pull** (too much spending chasing too few goods) vs **cost-push** (costs rise and are passed on). In this model only cost-push exists: firms price by cost plus a markup and never raise prices when shelves empty (`market_price_setting`). So a boom in demand shows up as hiring and unmet demand, **not** inflation, which is not how most real economies behave and is called out as a model limit. What **scope B** (`markets.md` section 4: `goods.price_demand_k`, default 0) would add: a shortage would raise prices, creating demand-pull, with the feedback risks listed there. The topic says scope B is not built and not approved.
- `inflation_decomposition` (how to read the drivers; the rounding row; why interest is zero today).
- `real_wage` (nominal vs real; links the wage rule: wages follow an expected inflation of 2 per 100 a year plus productivity plus unemployment slack, not actual prices, so real wages can fall).
Worked examples are small `pub fn`s in the model registered in `assumptions::registry()` (for example `inflation::log_mean`, `inflation::cumulative_part`), behaviour unchanged. New anchors `inflation:*` go in `ANCHORS` in `crates/sim-api/tests/assumptions.rs`; the wage rule topic gets a "real wages" link.
**Placement (thin renderer; text, labels, ids from `content/ui.json`):**
- Markets view (Economy > Markets): an **Overall prices** card above the six market strips: headline sentence, a CPI sparkline from `HistoryPoint`, the per-sector list, the drivers list, a "how?". Same card chrome as a market strip; not a tenth market (it has no supply or demand).
- Economy overview: a one-line "Cost of living" tile (sentence from Rust) that opens the card, and the existing "Price index" in Finance gets a "how?" and a note that it is a deflator, plus a link to the CPI.
- Wage rule explainer (`income_wages_prices` and the labour card's how?): link to `real_wage` and the real-wage figure.
- Finance and Society income views: a **Real wage** line (average wage divided by CPI) next to the nominal wage and median income, and the quintile list from 3.4.
Frontend rule: no arithmetic beyond formatting; `catalog.ts` missing-key scan covers every new text key.

## 6. Wire (additive, contract v2.5 proposed)
- New `Snapshot.inflation: InflationView { cpi, cpiChange: Window[], sectors: Vec<SectorPrice>, realWage, drivers: Vec<Driver>, byHousehold: Vec<{group, text}>, headline, notes: Vec<String>, reconciles }`. Lists, not named fields (like `markets`, `emissions`): the web iterates `sectors`, `drivers`, `windows` (`{id: "month" | "year" | "since_start", months, text, value}`). All labels, ids, ordering, cause words, flat band and the card's words in `content/ui.json` `inflation` (validated at load like `markets`: unknown id fails; a closed id set in `inflation.rs`).
- `HistoryPoint` fields from 3.5; `Indicators` unchanged (keep `price_index`; the CPI is not a headline indicator so no `MetricKey` is needed unless review wants one: then recipe section 4).
- ts-rs generated types: `cd web && npm run gen:contract`; aliases in `web/src/contract/index.ts`; never edit `generated.ts`. Docs: `docs/snapshot-contract.md` new section 25.
- Regenerate as in `how-to.md` section 24 step 5 (fixtures, golden twice, recording, wasm). The replay mock gets the keys automatically; the card renders from the recorded snapshot.
- Code: `crates/sim-metrics` (pure CPI, log-mean and filter functions), `crates/sim-api/src/inflation.rs` (assemble, patterned on `markets.rs`/`networth.rs`; no model logic), a `TraceRecord::PriceStep` from `goods.rs`.

## 7. Tests
- `crates/sim-api/tests/inflation.rs` (patterned on `tests/markets.rs`, several seeds, all 40 turns and live months): `reconciles == true`; the drivers sum to the CPI change with residual under a stated bound (default: one cent per firm per month in price, scaled by the weights); the sector parts sum to each sector change; weights are non-negative and sum to 1; the 12-month window absent before month 13 and "since start" present; `price_index` equals `sales_value / real_sales`.
- **Sign tests** (`causal-model.md` style): a carbon price rise (lever) raises the CPI and the carbon driver is positive while wages, productivity, interest are unchanged within a tolerance; raising the minimum wage raises the wage driver; a TFP rise lowers the productivity driver; interest rate shock changes only profit, interest driver exactly 0; with the carbon price at zero the carbon driver is exactly 0.
- Unit tests: `log_mean(a, a) = a`, LMDI parts sum to `X1 - X0` for random inputs; filter linearity (sum of per-cause runs equals the run on the sum); chained reset keeps the index continuous.
- State hash byte-identical (`cargo run -p sim-cli --example statehash`), engine suite `tests/scenarios.rs` unchanged; save round trip with and without the new `HistoryPoint` fields.
- `assumptions.rs` anchors and paths; content test: every `inflation` id in `ui.json` has a Rust arm and the reverse; no bare percentages (the wire text test used for markets).
- Web: `web/tests/panels-inflation.test.ts` (card renders from the wire list alone, a made-up driver appears, no bare percentages, boundaries test passes).

## 8. Delivery plan
1. Confirm the household purchase accessor for weights (3.2); add the `PriceStep` trace; statehash identical.
2. `sim-metrics` pure functions with unit tests (CPI, log-mean, filter), then `sim-api/src/inflation.rs`, `HistoryPoint` fields (schema bump), content list and load validation; the reconcile and sign tests.
3. Assumptions topics and the worked-example functions.
4. Contract v2.5 docs, ts-rs types, fixtures, golden, recording.
5. Web: Overall prices card, Economy tile, real-wage lines, links; screenshots (`web/playwright`).
6. Docs: `snapshot-contract.md` section 25, `glossary.md` (inflation, CPI, deflator, real wage, cost-push, demand-pull), `known-issues.md` for what is approximate.

## 9. Risks and limits
- **Weights move with prices:** Laspeyres weights go stale and a price rise on a good changes its own share; the yearly reset limits it, and the card says "typical basket".
- **Lumpy:** three firms, one per sector: a sector price is one firm's price. Rent moves in 0.5 percent steps by rule. Month-on-month is noisy; the headline is the 12-month number once it exists. Say so, do not smooth.
- **Early months** show settling, not a shock (3.3).
- The CPI excludes things people pay (tax, electricity as a bill) and everything not priced here; say so.
- Teaching risk: "interest 0" and "demand not inflationary" can look like bugs. The explainers state they are the model's current rules, with scope B named.
- Rounding row can look like an error; its definition is on the card.
- Save size: six more scalars per history point (small); no per-trade data.

## 10. Out of scope
Scope B (demand-responsive goods prices; `markets.md` section 4); wage indexation to measured inflation or any change to `inflation_anchor`; interest passed into prices; a central-bank or interest-rate policy response; inflation expectations; imports and exchange rates; changing any parameter; the daily-tick refactor. This feature changes no behaviour.

## 11. Open questions (defaults in bold)
- Q1. Fixed-base or chained? **Chained Laspeyres, weights reset every game year.**
- Q2. Is the CPI a headline `MetricKey` (chart in the metrics panel)? **No; Markets card and Economy tile only, add a metric later if wanted.**
- Q3. Show the "settling" row in the first year? **Yes, labelled plainly.**
- Q4. Household CPI by quintile in Society? **Yes, five lines, built from basket shares only.**
- Q5. Include rent in the CPI? **Yes (it is the largest cost for the poor), as its own cause.**
- Q6. Keep Finance "Price index" row? **Yes, relabel "Prices of what firms sell" and link to the CPI.**
- Q7. Residual bound for the reconcile test? **One cent per firm per month, scaled by weights; revisit after the first run.**
- Q8. Old saves? **Load with empty series; windows start at the load point.**
