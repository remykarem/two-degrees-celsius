# Town v8 design: GST and state ownership (draft for review)

Status: **DRAFT, nothing implemented.** The user answered **yes to both** (model a GST/VAT; model state shareholding) **and yes to a separate release**. Starts after the v5.1/v5.2 transparency pass (`docs/town-v5.md` sections 10 and 11) and a playtest. Numbers are placeholders. **[PROPOSED]** items are my defaults for the user to change.

## 1. Why these two belong together
Both are **fiscal and ownership levers on the same circular flow**: GST raises revenue at the point of purchase; state ownership routes firm profit to the treasury. Together they let the player build a very different economy (consumption-tax-funded state with public firms) and finally make the "economic system" choice (Market, Regulated, Coop) include a **State** option. Both need new ledger kinds, finance lines, breakdowns and explorer topics, so the v5 transparency layer is the foundation.

## 2. Facts about today's model (checked)
- Taxes: income (`tax_wage`), profit (`tax_profit`, applied to firm profit before dividends), capital (`tax_capital`, on dividends, rent, interest), wealth, carbon. **No sales tax.**
- `Owner` is a single value per firm: `Household(id) | Workers | State | Bank`. `Owner::State` already routes dividends to the government (`Purpose::StateFirmDividend`), but no system sets it, and **a firm cannot be partly owned** (no shares).
- `EconomicSystem` is `Market | Regulated | Coop`; system files are `content/systems/{market,regulated,coop}.json` with a `dividend_rule` (`Owner` or `Employees`).
- Households buy food, goods and services from firms each month with their budget (`goods.rs`); firm prices adjust with demand; purchases already generate ledger entries and `Purchase` events.

## 3. GST / VAT

### 3.1 Design
- **A new lever `gst_rate`** (share, 0..0.30; default 0 in Market, so v6 behaviour is unchanged until the player raises it). Lead time 1 turn, PC cost per the lever table, lobbying applies like other taxes (capital-owning classes oppose a rise less than a profit tax, but firms lobby against it: opposition factor from `net_lobby` scaled by a param) [PROPOSED].
- **Applied at the point of sale** on **goods and services** by default; **food is zero-rated** by default [PROPOSED] (so the regressive effect on the poor is a real choice: a second lever `gst_food_rate`, default 0, lets the player tax food, and an `gst_exempt` list in params for later).
- **Tax-inclusive pricing [PROPOSED]:** the shelf price the household pays is `price x (1 + gst)`; the firm receives `price` and the government receives the tax. The household's budget is unchanged, so a rise in GST reduces the **real** quantity bought (the budget buys fewer units); no separate price adjustment rule is needed.
- **Who pays [PROPOSED]:** households and the government (public purchases are GST-exempt to avoid circularity). Firms buying capital goods are **exempt** (input credit), so GST does not tax investment.
- **Collection:** a monthly transfer household -> government at the moment of each purchase, with ledger kind `tax`, detail `gst`, purpose `Gst` (a new `Purpose`); conservation and reconciliation stay exact (integer cents, remainder carried by the existing apportion helper).
- **Effects:** revenue line "GST" in the government finance section; regressive incidence (poorer households spend a larger share of income); lower real consumption -> lower sales and employment (existing demand loop); a small **price-level effect** is not modelled (stated in the explorer). A **GST breakdown** shows base (taxable spending by sector), rate, revenue, and incidence by household (tax paid as a share of income).
- **Interactions:** replaces the "GST: not modelled" row in the Taxes table; legitimacy is unchanged except through its existing terms (wellbeing, unemployment, inequality, debt); political power and lobbying apply as for other tax levers.

### 3.2 Tests and checks
Sign tests (higher GST lowers real consumption and raises revenue up to a point; GST on food hits low-income households harder than on services); money conservation and per-account ledger reconciliation with GST on; the Laffer-like shape is not asserted, only monotone revenue at low rates; State hashes **unchanged when `gst_rate = 0`** (regression: the existing golden must still pass).

## 4. State ownership

### 4.1 Ownership shares (the real change)
- Replace `owner: Owner` on a firm with **`ownership: Vec<Stake>`** where `Stake { holder: Holder, share: Rate }` and shares sum to 1.0 (a check in `sim-checks`). `Holder = Household(id) | State | Bank | Workers`. A single-owner firm is a one-element vector, so **all current runs are unchanged** (golden hashes must pass after a migration that maps today's `owner` to one 100% stake).
- **Dividends, wealth and class** use shares: dividends are apportioned by share (existing `apportion` helper); `cohort_wealth`/net worth credits each holder its share of equity; class derivation counts owning households by share; lobbying power and the wealth tax follow net worth. The state's holding is a **public asset** shown in the government finance section ("state equity in firms") and in the Industry tab (the ownership split, already specced).

### 4.2 A fourth economic system: `State` [PROPOSED]
`content/systems/state.json`: `dividend_rule` `Stakes` (by share), key sectors **nationalised on transition** (default: the Factory and the Shop-chain's energy side: see 4.4; the Farm and the Shop stay private) [PROPOSED], `payout_ratio` higher for state firms, a **productivity penalty and slower investment** in state firms to represent inefficiency (the design promise "alternatives have real costs"; params `state_firm_tfp_penalty`, `state_firm_invest_hurdle`), no capitalist lobbying power over state firms, a political-capital cost and long transition (24 months, output penalty, resistance from owners) like the other systems, and **compensation** for expropriated owners at book value paid by the government (borrowing from the bank within its limit), logged as ledger entries (kind `neutral`, detail `nationalisation`).

### 4.3 Partial state ownership as a player action [PROPOSED]
Two new commands, usable in any system: **`BuyStake { firm, share }`** (the government buys a share from the current private holders pro rata at book value, funded from cash and borrowing; lead time 2 turns; PC cost scaled by share and by capitalist opposition) and **`SellStake { firm, share }`** (privatisation: sells to the largest private holder or the bank at book value; revenue to the treasury). This gives a **smooth dial** from 0 to 100% state ownership, the "% state shareholder" the user asked for. UI: a control on the firm in the Industry tab and the inspector, with a cost preview.

### 4.4 Public energy [PROPOSED, optional]
Plants today are government-built and unowned. Making plants state-owned firms with revenue (selling electricity) is a larger change and is **left out of v8**; the plant ownership question is noted in the explorer.

### 4.5 Effects and tests
Dividends from state stakes flow to the treasury (already the `StateFirmDividend` path); government revenue and the deficit change; state firms behave per `state.json`. Tests: shares sum to 1; apportionment exact (cents); ledger reconciliation with partial stakes, buy/sell and nationalisation; class and wealth derivations by share; the old single-owner scenarios reproduce today's golden; sign tests (more state share raises treasury dividends and lowers the capitalist class's income and lobbying power; nationalisation lowers private investment in that firm); baselines A to F re-run and reported (this changes nothing when no state ownership is used).

## 5. Wire, UI and docs (additive; contract v2.1)
- Snapshot: `firms[].ownership` (list of stakes with holder and share), `government.stateEquity`, `finance.government.revenue.gst`, `tax` row `gst` with `modelled: true`, `economicSystem` gains `state`, levers `gst_rate` and `gst_food_rate`, commands `buy_stake` and `sell_stake`, ledger kinds `gst`, `nationalisation`, `stake_purchase`, `stake_sale`, a GST breakdown and an ownership breakdown. Save schema bumps (firm state changes).
- UI: GST in the policy drawer (with the incidence preview in words, arrows only) and in the Taxes table; **ownership control** and split bar per firm in the Industry tab; system switcher gains **State**; stake purchases in the plan tray; explorer topics for GST, ownership, nationalisation, "what state firms leave out".
- Docs: `docs/causal-model.md` section 12 (GST rules, ownership shares, State system), `docs/snapshot-contract.md` section 17, `docs/architecture.md`, `docs/how-to.md`, `docs/known-issues.md`.

## 6. Delivery plan
1. You review this doc.
2. **Rust agent, staged:** (a) ownership stakes with a migration and the regression that existing goldens pass; (b) GST lever and collection; (c) State system, buy/sell stake commands, nationalisation compensation; (d) breakdowns, explorer topics, wire, fixtures, golden, baselines.
3. **Web agent (parallel on the mock):** GST UI, ownership UI, State system, stake commands, breakdowns.
4. Balance pass and independent review.

## 7. Risks
- **The ownership refactor touches class, wealth, dividends, lobbying and checks**; the single-stake migration and the unchanged-golden test are the defence.
- **Balance:** GST and state ownership give the player powerful fiscal tools (a consumption-tax-funded state could fix the structural deficit), which may make baseline B or others too easy or break known failures; baselines are re-run and the "one tool is not enough" property re-checked.
- **Small population:** with 3 firms, nationalising one is a 33% change of the economy; the optional dormant slot and class mobility must still work with stakes.
- **Realism:** state firms' inefficiency is a stated assumption, not calibrated; the explorer says so.

## 8. Open questions (defaults taken if you just approve)
1. Food zero-rated by default, with a separate lever to tax it? (default yes)
2. Which firm(s) does a transition to the State system nationalise? (default: Factory; Farm and Shop stay private)
3. Compensation at book value, paid by the government? (default yes)
4. Should the **bank** be nationalisable too? (default no)
5. Is the **State system** a fourth option in the system switcher, in addition to the partial buy/sell-stake dial? (default yes to both)
