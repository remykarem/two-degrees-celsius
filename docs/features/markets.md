# Feature: Markets view (supply and demand made visible)

Status: **APPROVED by the user, scope A only (the information-only Markets view), with every open-question default (Q1 to Q7) and the pedagogy section. Scope B (price response to excess demand) is NOT approved and not in scope.** Implementation in progress. Written from the code at commit `f29ca4a` (`crates/sim-model/src/{goods,labour,credit,production,...}.rs`, `content/params.json`). Where this doc and code disagree, code wins (`docs/README.md`). This doc changes no code. Scope A is information only (state hash byte-identical); scope B is an optional, balance-changing modelling decision, flagged as such.

## 1. Why
The user is new to economics and cannot see supply and demand or markets anywhere in the game. The model does have markets, but each is spread over a system module and surfaces only as a number (a price, a wage, a rent). A Markets view shows, per market, what people want, what is on offer, the gap, the price and which way it is moving, and who is left short. It must also say honestly how this model sets prices, because it is not the textbook way (section 2.1).

## 2. What the model does today (re-checked in code)

### 2.1 Goods, services and food (`goods.rs`, sectors Food = Farm, Goods = Factory, Services = Shop)
- **Demand (monthly, in money, then units).** Per household: food need = members x `food_need_units_pc` (25) x average food price; food budget = min(need cost, savings). Discretionary = min(propensity(class) x max(disposable - need cost, 0) + wealth effect for Capitalist/Rentier, savings - food budget); split `services_share` (0.45) to services, the rest to goods. Firms add a capital-goods budget (`capex`) to goods; the government adds its funded purchases (`gov_services_share` 0.5 to services, rest to goods). All of this is a **budget**, set before looking at prices of individual shops.
- **Shop split (`allocate`).** A sector budget is split over its firms by weight stock x (stock-weighted average price / own price)^`price_elasticity` (2.0), in two passes so a firm cannot sell more than its stock. `demand_units` (pass 0 share / price) is what buyers would have bought before the stock limit; `sold` is after it. The difference is unsold demand (shortage); stock minus sold is unsold inventory (glut).
- **Price is cost-plus, NOT excess-demand-driven.** After sales, each firm moves its price by `price_adjust` (0.05) of the gap to `(1 + markup) x unit cost` per month, where unit cost = (wage bill + carbon cost) / output. Demand never enters that formula. Excess demand (`Firm.excess_demand` = (demand_units - supply) / supply) is computed and stored, but it feeds only `expected_sales` (smoothed with `expected_smoothing` 0.3) and so hiring and production, never the price. The stale wording "prices follow unit cost and excess demand" in the `goods.rs` module comment and "firm prices adjust with demand" in `docs/features/gst-and-state-ownership.md` section 2 were wrong; both are corrected (with scope A, commit "Markets: output-only trace...").
- **What cost-plus implies (to say in the UI).** Prices rise when wages, carbon prices or energy costs rise, or when productivity falls; they do not rise because shelves are empty. A shortage shows up as unsold demand and as the firm hiring and producing more over the following months (quantity adjusts, price does not). A glut shows up as inventory and, via `inventory_cover`, less hiring. A carbon price therefore passes through to shelf prices even in a slump.

### 2.2 Labour (`labour.rs`)
- **Demand.** Each firm wants `needed = expected_sales x (1 + growth_aspiration) + inventory_adjust x (cover x expected - inventory)` units, turned into workers via the production function; `target` workers rounds up past `hire_slack`. **Vacancies** = max(target - workers, 0), split skilled/general. Hiring is capped at `max_hire_per_month` (1), firing at 1 after `fire_after_months` (3) of sustained surplus.
- **Supply.** People in the labour force without a job, not household staff, and not (after the pool) in the job guarantee. Cheapest reservation wage first, ties by skill.
- **Price (wage).** Each firm: `w *= 1 + step`, `step = clamp(trend + wage_k x (u_ref - u), -wage_cap, +wage_cap)`, `trend = inflation_anchor + productivity_passthrough x sector TFP growth`, floored at the (system-adjusted) minimum wage. `u` is smoothed unemployment; here, unlike goods, the price does respond to slack (`wage_k` 0.1, `u_ref` 0.09, `wage_cap` 0.01 per month).
- Stored today: `Firm.unfilled` (a bool), `Firm.over_target_months`, `workers`, `wage`; `Indicators.unemployment` and `unemployment_smooth`. Vacancy counts are **not** stored (they are a local in `run`).

### 2.3 Housing (`goods.rs::housing`)
- **Supply** = homes. **Demand** = households (each needs one home). Seekers (no home, not evicted this month), richest first, take the cheapest vacant home with rent <= `move_rent_share` (0.35) x last disposable income. Eviction after `evict_after_months` (3) of arrears (renters only).
- **Price (rent).** Private rent rises by `rent_up` (0.005) when occupancy >= `occupancy_trigger` (0.95) AND mean rent burden (private rent / renter disposable income) < `rent_burden_cap` (0.2); otherwise falls by `rent_down` (0.005); floor `min_rent_usd`. Public rent is a lever.
- On the snapshot already: `housing.homes`, `rentPrivate`, `rentPublic`, `occupancy`, `unhousedHouseholds`. Vacant homes = homes x (1 - occupancy) (derivable). Burden is a local in `housing()`.

### 2.4 Credit (`credit.rs`)
- **Demand** = firms' `loan_request` (set by investment, `investment.rs`). **Supply** = bank headroom: `max_loans = equity / kappa` (capital ratio) minus loans outstanding, further capped per firm by `max_firm_leverage` (0.6) x (capital + cash) - debt.
- **Price (rate).** Per firm: `base + spread_base (0.005) + spread_leverage (0.02) x leverage + credit_spread_add` (the last from the economic system); deposits pay `base - deposit_spread`. The rate is rationed by quantity (a limit), not auctioned. `Firm.loan_request` is zeroed after the decision, so the **unmet** request is not stored today.
- On the snapshot already: `bank.equity`, `loans`, `deposits`, `defaultRate`, `failed`; `FirmView.debt`; `Firm.loan_rate` exists in state but not on `FirmView`.

### 2.5 Energy and commons (`production.rs`, `nature.rs`)
- **Energy:** a quantity constraint with no price. Demand (firms plus households) against plant capacity; if demand exceeds supply, output is rationed in proportion (`energy.served`, 1 when enough). Already on the wire: `finance.energy` (`supply`, `demand`, `served`, `intensity`, `renewableShare`).
- **Commons:** open or community access, harvest against regrowth; no price. Health shares on the snapshot already (`commonsHealth`). Shown as a stock gauge, not a price; see open question Q5.

## 3. Scope A: an information-only Markets tab

### 3.1 What a card shows
One card per market, always the same anatomy (the "supply and demand strip", section 5):

| Market | Demand | Supply | Gap | Price and direction | Who is short |
|---|---|---|---|---|---|
| Food, Goods, Services | units households, firms and government would buy this month, before stock limits | units in stock at the start of the month (stock-weighted across firms) | demand - sold (unmet) and stock - sold (unsold) | average shelf price; arrow from the 3-month trend | unmet units; unsold inventory by firm |
| Labour | workers firms want (target headcount) | people in the labour force | vacancies and unemployed | average wage; arrow | unemployed people, firms with `unfilled` |
| Housing | households | homes | vacant homes; unhoused households | private rent; arrow | unhoused households, rent-burdened renters |
| Credit | loans requested | headroom the bank can lend | unmet requests | average firm loan rate; arrow | firms refused or cut back |
| Energy | units wanted | plant capacity | rationed units | none (no price) | share of demand not served |

Each card also has: a sparkline of the price (or wage, rent, rate) over the last N turns, a one-line plain-English verdict ("Shoppers wanted 310 units, shops had 280: 30 units went unmet. Prices did not rise because ..."), and a "how?" button.

### 3.2 Plain-English explainers (assumptions topics)
Add topics to `content/assumptions.json` (recipe `docs/how-to.md` section 15): `market_goods`, `market_labour`, `market_housing`, `market_credit`, `market_energy`, and one `market_price_setting` that states, in beginner words: "In this town firms set prices from their costs plus a markup. They do not raise prices when shelves empty. Empty shelves make firms hire and produce more instead. Wages and rents do respond to scarcity (unemployment, occupancy)." Worked examples use small `pub fn`s in the model (for example `goods::price_target(unit_cost, markup)`, `labour::wage_step(...)`) registered in `assumptions::registry()`, as section 15 step 2 requires; behaviour unchanged. `usedBy` carries the new anchors `markets:<id>`; `crates/sim-api/tests/assumptions.rs` `ANCHORS` gets them.

### 3.3 Where the numbers come from (computed in Rust only)
All figures are computed in `sim-metrics` (pure functions of `State`) and assembled in `sim-api` (`crates/sim-api/src/markets.rs`, new, patterned on `networth.rs`). The web does no arithmetic beyond formatting and chart scaling (`docs/town-v6.md`, AGENTS web rule 8).

**Already available (no sim change):**
- Goods: per firm `price`, `output`, `revenue`, `workers`, `wage` (`FirmView`); `Firm.excess_demand`, `sales_units`, `sales_hist`, `inventory`, `expected_sales` exist in `State` but only some are on the wire (`FirmView` has price, output, revenue, not inventory or excess demand: add them to the market output, not to `FirmView`).
- Labour: `indicators.unemployment`, `unemployment_smooth`, `FirmView.workers`, `wage`; `Firm.unfilled`.
- Housing: `housing.*` and per-home `rent`/`household`.
- Credit: `bank.*`, `FirmView.debt`.
- Energy: `finance.energy` complete.
- History: `HistoryPoint` per turn; a price series needs new fields (below).

**Needs new sim output (output-only, no behaviour change):**
1. **Demand before stock limits, per sector:** `alloc.demand_units` summed per sector, and the sector `budget`, as a `TraceRecord` from `goods::run` (recipe section 14 step 3). `Firm.excess_demand` is already stored but is a ratio per firm; the unit totals are needed for a "units wanted vs sold" tile that reconciles (section 3.4).
2. **Unmet demand:** derived: sum of demand_units - sum of sold. Needs item 1.
3. **Vacancies:** `skilled_vacancies + general_vacancies` per firm as a `TraceRecord` (or a stored `Firm.vacancies: u32`, which is state and would change the hash and save schema; prefer the trace). Today only `unfilled: bool` persists.
4. **Credit requested vs approved, per firm:** `loan_request` and `approve` as a trace from `credit::run`.
5. **Rent burden and vacant homes:** the burden local in `housing()` as a trace; vacant = homes - occupied.
6. **Price series:** add `pricesBySector`, `wageAvg`, `rentPrivate`, `loanRateAvg` (and the same for unmet/glut) to `HistoryPoint`. `HistoryPoint` is saved with the game, so this bumps `SCHEMA_VERSION` (recipe section 14 step 3); old saves load with empty series (decision Q3).

Trace records are output-only: they are not part of `State`, so `cargo run -p sim-cli --example statehash` must be byte-identical.

### 3.4 Reconciliation
Every card carries `reconciles: bool` and the identities below, like `networth` and the emissions tally:
- Per goods market: `demanded = sold + unmet`; `stock_start = sold + unsold`; `sold` equals the sum over firms of `sales_units`; `revenue = sum(sold x price)` within rounding (revenue is floored to cents per firm).
- Labour: `target_workers = employed_by_firms + vacancies`; `labour_force = employed + unemployed (+ job-guarantee)`.
- Housing: `homes = occupied + vacant`; `households = housed + unhoused`.
- Credit: `requested = approved + unmet`.
- Energy: `demand = delivered + rationed`.
The unit-bearing ones are exact only if the trace is taken from the same tick data; the test (section 7) pins that.

## 4. Scope B (optional, later): let goods prices respond to excess demand
**This changes model behaviour and the balance. It is not part of scope A and should be a separate decision.** Today prices are cost-plus only (section 2.1), so a goods shortage never raises a price and a glut never lowers one, which a new economist may find odd.

- **Proposal:** a new parameter `goods.price_demand_k` (default **0**, reproducing today's behaviour exactly). Target becomes `(1 + markup) x unit_cost x (1 + price_demand_k x clamp(excess_demand, -cap, +cap))`, still closed at `price_adjust` per month. `Firm.excess_demand` already exists. Add `goods.price_demand_cap` (default 0.2).
- **Sign tests** (`docs/causal-model.md` style): with k > 0, a positive demand shock raises the sector price relative to k = 0; a negative one lowers it; k = 0 leaves the state hash identical to baseline; price stays at or above `min_price`.
- **Baseline re-run:** with default 0 the baselines A-D in `docs/tuning-log.md` are unchanged. Any k > 0 needs a new baseline row and a re-tune.
- **Tuning risks:** a feedback loop price -> real income -> demand -> price (damped by `price_adjust` 0.05 but a small town with three firms and one per sector is lumpy: one shop's stockout is the whole sector); interaction with the carbon price pass-through (higher shelf prices from demand could mask or amplify it); Gini and legitimacy move, since inflation hurts the poor (propensity 1.0) more than capitalists (0.45); `wage_k` stays on unemployment, so real wages can fall in a boom. The wage and rent rules already respond to scarcity, so only goods is asymmetric.
- A UI toggle is not proposed; the card text reads the parameter from the wire so it says whichever is true.

## 5. UI design (beginner friendly)

### 5.1 The supply and demand strip (one per market)
A tiny horizontal diagram built from real numbers, readable without knowing the theory:

```
FOOD            Price $4.10  ^ slowly rising (cost-driven)
Wanted  [##########----]  310 units
On shelves [########------] 280 units      30 units went without
```

- Two bars on the same scale: **Wanted** (demand) and **Available** (supply), with the **gap** coloured and labelled in plain words ("30 units went without" / "45 units unsold"). Units are always shown ("units", "people", "homes", the energy unit from the wire), never a bare percentage.
- A small arrow beside the price with a word ("rising", "steady", "falling") and a cause word from the wire ("cost-driven", "scarcity-driven"). A mini line of the last turns' prices.
- A "Who is short" row of chips (for example "3 people looking for work", "1 home empty", "Farm sold out"), each a link to the existing inspector selection.
- A classic crossing-curves diagram is deliberately not drawn: the model has no demand curve for the town, and drawing one would imply behaviour that is not there. The strip shows the real quantities instead.
- Consistent with Finance and Society: same card chrome from `ui-kit`, same "how?" buttons and `usedBy` anchors, same wire-driven lists (`docs/web-notes-panels2.md`, `docs/town-v8.md` Who holds what).

### 5.2 Thin renderer rule
The web iterates `markets: MarketView[]` and draws a strip for each. Labels, units, ordering, category (`products`, `factors`, `capital`, `resources`), the cause words and the explainer text live in `content/ui.json` (`markets` list, like `net_worth_lines`) and come over `catalog()`. A new market is a content entry plus a Rust arm, no web change (recipe in section 8).

### 5.3 Navigation
The metrics panel has seven tabs (`METRIC_TABS`: Economy, Ecology, Society, People, Finance, Policies, Industry) and is crowded. Proposal [PROPOSED, default]: **do not add an eighth tab.** Put Markets as a sub-view of **Economy** (a "Markets" segment next to the GDP and prices charts), because that is where a learner looks for prices and shortages, and link to it from three places: the Finance circular-flow nodes (households, firms), the Industry firm cards ("sold out", "hiring"), and the "how?" of unemployment. If review prefers a tab, replace Policies or merge People into Society; that is a layout decision with no wire impact.

## 6. Wire and contract changes (additive)
- New top-level `Snapshot.markets: Vec<MarketView>` (or a `finance`-style call `markets()` if the payload is large; default: on the snapshot because it is small and per-month). Contract v2.3 (`docs/snapshot-contract.md` new section 23).
- `MarketView { id, unit, demand, supply, sold, unmet, unsold, price: Option<f64>, priceTrend, priceSeries, drivers: Vec<MarketDriver>, short: Vec<MarketShort { label, count, entity: Option<EntityRef> }>, howId, reconciles }`. `id`, `unit`, `label`, `category` and the cause words resolve from `content/ui.json` `markets` (validated at load like `net_worth_lines`: a missing or unknown id fails).
- ts-rs generated: `cd web && npm run gen:contract`; aliases in `web/src/contract/index.ts`; never edit `generated.ts`.
- `HistoryPoint` grows (item 6, section 3.3): additive serde with defaults; `SCHEMA_VERSION` bump if the saved log changes (recipe `docs/how-to.md` section 10, section 14 step 3).
- Regenerate: `cargo run -p sim-api --example fixtures`, `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice), `npm run gen:recording`, `npm run build:wasm` (same list as `docs/how-to.md` section 23 step 5).
- The mock/replay parity test compares key sets with `docs/fixtures`; the new keys appear there automatically.

## 7. Tests
- `crates/sim-api/tests/markets.rs`, patterned on `tests/networth.rs`: for several seeds and months, every `MarketView` has `reconciles == true` and the identities in section 3.4 hold within a tolerance (1e-6 x the larger side); live mode as well as turn mode.
- State hash byte-identical before and after (`cargo run -p sim-cli --example statehash`) and the engine suite `tests/scenarios.rs` unchanged.
- `assumptions.rs`: every new `howId` and `markets:*` anchor has a topic; every `params` path in the new topics exists.
- Content test: every market id in `ui.json` has a Rust arm and the reverse.
- Cross-check the price direction word against the actual price change over the month.
- Web: `web/tests/panels-markets.test.ts` (strips render for every wire market, no bare percentages, boundaries test passes); `catalog.ts` missing-key scan covers the new text keys.
- Scope B only: sign tests in section 4; baseline re-run with `price_demand_k = 0` identical, then with k > 0 against `docs/tuning-log.md`.

## 8. Delivery plan
1. Output: traces for demand-before-limits, vacancies, loan requested/approved, rent burden; `HistoryPoint` series. Statehash identical.
2. `sim-metrics` pure functions and `sim-api/src/markets.rs` with `content/ui.json` `markets` list and load-time validation; reconcile test.
3. Assumptions topics and "how?" breakdowns (recipes sections 14 and 15).
4. Contract v2.3 docs, ts-rs types, fixtures, golden, recording.
5. Web: Markets sub-view and the strip component; navigation links; screenshots (`web/playwright`).
6. Docs: `docs/snapshot-contract.md` section 23, `docs/how-to.md` new section "Add a market", `docs/glossary.md` (supply, demand, shortage, glut, cost-plus), and fix the stale "demand" wording noted in section 2.1.
7. Later, separate decision: scope B.

## 9. Risks
- Reconciliation drift: demand units are floats and revenue is floored per firm; the test tolerances must be agreed up front.
- A three-firm, one-firm-per-sector economy makes the strips lumpy (a sector with one shop shows all-or-nothing gaps); say so in the card, do not smooth it.
- Credit "demand" is a firm's request after investment logic, not a free desire to borrow; label it accordingly.
- Teaching risk: showing a gap beside a flat price can look like a bug. The explainer and the cause word are the mitigation; this is the reason section 4 exists.
- Save compatibility if `HistoryPoint` grows (decision Q3).
- Payload size per month if series are long; cap at the history length already kept.

## 10. Out of scope
New simulation behaviour (scope A); a town-wide demand curve or price-discovery; stock or asset markets; a market for the commons or carbon permits; per-person job search view; changing `price_adjust`, `wage_k` or any parameter; the daily-tick refactor.

## 11. Open questions (defaults in bold)
- Q1. Goods and services as one card or two? **Two cards, same category `products`**: they have different firms and budgets.
- Q2. Nav placement? **Sub-view of Economy, not an eighth tab** (section 5.3).
- Q3. Old saves without the new `HistoryPoint` fields? **Load with empty series and show the live month only.**
- Q4. Vacancies as a trace or as stored `Firm.vacancies`? **Trace (no hash change)**.
- Q5. Commons as a market card? **No, show it under Ecology; mention it in the energy explainer as the other "no price" resource.**
- Q6. Do scope B now? **No; ship scope A first, then decide with the user.**
- Q7. One `markets` snapshot field or a `markets()` call? **On the snapshot.**

## Pedagogy: markets are one way to coordinate an economy (user request)

The game is **pedagogical** (user reminder). The Markets view must not imply that markets are *the* economy. It should teach that **a market is one coordination mechanism** among several, each answering the same problem.

**The problem (economics' basic questions):** resources are scarce, so a society must decide *what* to produce, *how*, and *for whom*, and the people deciding each know only a small part of what is needed (the knowledge and incentive problem). A **coordination mechanism** (also called a resource-allocation mechanism) is how that gets decided without anyone knowing everything.

**Mechanisms to name in the UI** (one line each, with a real-world example):
- **Markets:** prices signal scarcity; buyers and sellers adjust (shops, labour, housing).
- **Planning/command:** an authority sets quotas and prices (public housing allocation, wartime rationing, the Soviet system).
- **Firms/hierarchy:** managers direct work inside an organisation instead of buying every step on a market (Coase).
- **Community rules/commons:** shared rules and reciprocity (the commons, co-operatives; Ostrom).
- **Mixed:** real economies combine all of these. Note that markets do not imply capitalism: ownership (who holds the firms) is a separate question.

**Where to show it (content-driven, no web logic):**
1. A short intro block at the top of the Markets view: "A market is one way an economy decides what to make and who gets it. This town uses several."
2. In the economic-system switch modal (Market/Regulated/Coop): which mechanisms each system leans on.
3. An explorer topic `coordination_mechanisms` in `content/assumptions.json`, linked from the intro and from each market card's "how?".
4. Tag each part of the model with its mechanism (goods and labour: market; public housing, regulation and the carbon price: planning/rules; the commons: community rules; firm hiring: hierarchy) so the player sees the mix.
Open question (default in brackets): does a planned-economy system get added later, where the Markets view shows shortages and queues instead of price moves? [Later, as its own feature.]
