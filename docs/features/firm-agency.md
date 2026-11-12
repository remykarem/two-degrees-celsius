# Feature: Firm agency (letting the player act as firms)

Status: **DRAFT, for review. Not approved, not implemented.** Docs only: this page changes no code. Written from the code at commit `399443b` (`crates/sim-model/src/{labour,production,goods,investment,credit,income,class,lobbying}.rs`, `crates/sim-types/src/state.rs`, `content/params.json`, `content/initial_town.json`, `content/systems/*.json`, the `agents` catalogue in `content/assumptions.json`). Where this doc and code disagree, code wins (`docs/README.md`). Claims that were not checked in code are marked **(unverified)**. Numbers quoted from `content/` are shipped values at this commit; the code holds none of them.

## 1. Why

Today the player is a town **planner**: levers (carbon price, taxes, minimum wage, subsidies, job guarantee), buildings (plants, roads, public housing) and the economic system (Market, Regulated, Coop), paid for in political capital (PC) and delivered after a lead time. Firms are automatic: three of them (Factory, Farm, Shop) plus one dormant slot follow fixed rules (`docs/town-v5-3.md`, the `firms` entry of the `agents` catalogue). The user is new to economics (memory: the game is pedagogical) and wants to explore **acting as firms**, to see what a firm can actually do and to find what real firms do that the model lacks. Two themes decide what is worth building: **ecology** (the 900 t carbon budget) and **inequality** (Gini, class, who owns what). Rule of the codebase: the web is a thin renderer (`docs/town-v6.md`): every decision rule, number and sentence below would live in Rust and `content/`, and reach the web over the wire.

## 2. Inventory: every decision a firm makes today (from code)

Tick order (`crates/sim-model/src/lib.rs`, `TICK_ORDER`): policy, **labour**, weather, estate, **production**, nature, **goods**, **income**, **credit**, government, **investment**, **class**, **lobbying**, wellbeing. "Who decides" is always the rule in the module named; no firm has a choice stored anywhere. A firm's per-firm knobs in `State` are only data (`Firm` in `crates/sim-types/src/state.rs`): `markup`, `capital_per_sale`, `intensity_base`, `energy_per_unit`, `tfp` are set at init and (apart from `tfp`) never change.

| # | Decision | Who decides, which rule | Parameters (`content/params.json` unless noted) | What the firm observes | Code |
|---|---|---|---|---|---|
| 1 | **Price** | The firm moves `price += price_adjust x (target - price)` each month, `target = (1 + markup) x unit_cost`, `unit_cost = (wage bill + carbon cost) / output` (`price_target`). Demand never enters. `markup` is per firm, fixed at init (verified: only read) | `goods.price_adjust` 0.05, `goods.min_price_usd`; `Firm.markup` from `initial_town.json` | Own wage bill, own carbon cost, own output | `goods.rs` ~404-416 |
| 2 | **Hiring** | Rule: `target workers = ceil(l_need - hire_slack)`, from `expected_sales x (1 + growth_aspiration)` plus an inventory-cover term; vacancies split skilled/general; hires the cheapest-reservation unemployed, at most `max_hire_per_month` | `labour.growth_aspiration`, `inventory_cover`, `services_inventory_cover`, `inventory_adjust`, `hire_slack`, `skilled_share`, `max_hire_per_month` (1), `min_target_workers` | Smoothed own sales, own inventory, the unemployed pool (not their wishes) | `labour.rs` |
| 3 | **Firing** | Rule: after `fire_after_months` (3) of overstaffing by at least `fire_slack`, fire up to `max_fire_per_month`, least skilled first | `labour.fire_after_months`, `fire_slack`, `max_fire_per_month` | Own headcount against target | `labour.rs` |
| 4 | **Wage** | Rule: `w *= 1 + clamp(trend + wage_k x (u_ref - u), +-wage_cap)`, floored at the (system-adjusted) minimum wage; `trend = inflation_anchor + productivity_passthrough x sector TFP growth` (`wage_step`). Each worker is then paid base wage x skill multiple, scaled down if cash is short (`income.rs`) | `labour.wage_k` 0.1, `u_ref` 0.09, `wage_cap` 0.01, `inflation_anchor`, `productivity_passthrough`; `policy.min_wage`; `production.tfp_growth_*` | Town-wide smoothed unemployment only; not its own vacancies, quits or competitors' wages | `labour.rs` ~150 |
| 5 | **Output** | Not a decision: Cobb-Douglas `tfp x K^alpha x L^beta` of the workers it has; then cut by energy rationing (`served`) and the emissions cap | `production.alpha_k`, `beta_n`, `energy.rationed_below`; lever `emissions_cap` | Workers, capital, plant capacity | `production.rs` |
| 6 | **Capacity / investment** | Rule: replacement plus an accelerator toward `expected_sales x capital_per_sale`, gated by return versus `loan_rate + hurdle`; capped at `(depreciation + max_invest_share) x K`; not at all while distressed. Capital goods are bought from the Goods sector (so the Factory sells to its neighbours) | `invest.depreciation`, `accelerator`, `max_invest_share`, `retain_share` 0.7, `return_scale`, `capital_price_usd`, `smoothing`; system `invest_hurdle` | Smoothed profit, own cash, loan rate, expected sales | `investment.rs` |
| 7 | **Technology: green** | The only technology choice. A firm buys a step of `green_tech` when (carbon bill over `green_payback_years` > cost) or the emissions cap binds, and while `green_tech < green_max` (0.8). It pays labour (so the spend becomes wages), the green subsidy is the treasury's share. Reduces both direct intensity and electricity per unit (`firm_intensity`, `firm_energy_per_unit`) | `invest.green_cost_share` 0.1, `green_payback_years` 5, `green_spend_share`, `green_step` 0.05, `green_max` 0.8, `cap_binding_share`; lever `green_subsidy`; system `green_cash_share` | Own footprint, the carbon price (an announced pending price counts at `turn.announce_discount`), the cap | `investment.rs` |
| 8 | **Energy source** | **None.** All firms draw from the grid; its carbon content is the fossil share of capacity x `energy.factor_fossil`. Only the player (building plants) changes it. A firm's own `energy_per_unit` and `intensity_base` are fixed at init | `energy.*`; `Firm.energy_per_unit`, `intensity_base` | Rationing (`served`) | `production.rs` |
| 9 | **Payout (dividends)** | Rule: `profit (after tax) x payout_ratio`, never below the cash reserve, plus `excess_payout_rate` of idle cash over a line. Who gets it depends on `Firm.owner`: a household, the workers (Coop), the state, or the bank. `payout_ratio` is a **system** value (Market 0.7, Regulated 0.6, Coop 0.5), not a firm value | `systems/*.json` `payout_ratio`; `income.cash_reserve_months`, `excess_cash_months`, `excess_payout_rate` | Own profit, cash, payroll and debt service | `income.rs` ~83-130 |
| 10 | **Reinvest vs payout** | Implicit and fixed: `retain_share` of free cash goes to capital before borrowing; the rest of the cash is payout under (9). No choice between the two | as above | Own free cash | `investment.rs` ~120 |
| 11 | **Borrowing** | Firm side: `loan_request = planned investment - retained cash`. Bank side: approves `min(request, bank headroom, leverage cap)`; rate `= base + spread_base + spread_leverage x leverage + system spread`. Rationed by quantity, not auctioned. Default after `default_after_months` of missed payments | `credit.spread_base`, `spread_leverage`, `max_firm_leverage` 0.6, `default_after_months`; bank `kappa` | Own leverage, the loan rate | `investment.rs`, `credit.rs` |
| 12 | **Exit / closure** | **Involuntary only**: a firm closes by default (distress months at the bank step); workers lose jobs, debt is written off, residual cash goes to the owner (or the state for Coop and State firms). No firm ever chooses to close or sell up | `credit.default_after_months` | Missed payments | `credit.rs` ~49-135 |
| 13 | **Entry / startup** | Not by a firm but by a **household**: a Worker or Manager household with savings >= startup cost + buffer may found the firm in the **dormant slot** with probability `mobility_rate x max(0, sector return - base rate)`, only where the sector's mean excess demand >= `startup_min_excess_demand` (0.2). The new firm copies its sector's incumbents (price, wage, technology, markup). The draw (`r.next_f64()`) is taken every month so other draws do not shift (schema 5 note). The dormant slot is a **second Services (Shop) slot** at tile (7, 7) (verified in `initial_town.json`); `MAX_FIRMS` is 4 | `class.startup_cost_usd` 30000, `startup_buffer_usd`, `mobility_rate` 0.03, `startup_min_excess_demand` 0.2, `new_firm_sales_share`, `startup_capital_share` | Sector return, excess demand (via the household) | `class.rs` ~140-240 |
| 14 | **Lobbying** | **Not a firm decision.** Capital-class households (Capitalist, Rentier) spend `min(spend_rate, policy.campaign_cap) x capital income` into a pooled fund paid to the richest Manager household; `effective = nominal x (1 - net_lobby)` for four levers (carbon price, wealth tax, profit tax, emissions cap, the last one divided). Labour's counter-pressure is automatic union density x wage share. No firm decides or sees any of this | `lobby.spend_rate`, `k`, `max_net`, `union_*`, `labour_u`; lever `campaign_cap` | Aggregate | `lobbying.rs` |
| 15 | **Tax** | Fixed: profit tax at the effective rate, carbon cost on footprint (`carbon_cost(effective carbon price, footprint)`). No avoidance, no choice of where to report | levers `tax_profit`, `carbon_price` | -- | `income.rs` ~58-83 |
| 16 | **Products, suppliers, buyers, advertising, location, mergers** | **None.** One product per sector; buyers choose shops by price and stock only (`allocate`, `goods.price_elasticity` 2.0, in two stock-limited passes); no supplier choice (energy and labour are pooled); the firm's tile is fixed (startup picks a site); no merger rule | -- | -- | `goods.rs` |

### 2.1 Facts that matter for the design (all verified in code)

1. **A firm's own price does not change what the sector spends.** Goods and services budgets are set per household in money, before any price is looked at (`goods.rs` ~255-275); `allocate` only **splits** that fixed budget over the sector's firms by `stock x (average price / own price)^elasticity`. With one firm in the sector (today's Factory, Farm and Shop), raising the price leaves its revenue **unchanged** (it sells fewer units for the same money) until its stock runs short. Food is a partial exception (the food budget is `need units x average food price`, capped by savings). So a controlled price in a one-firm sector has **no demand penalty**; "price up" is a free lunch there. This is why a price lever on a monopolist teaches the wrong thing (section 5, option B).
2. **Output follows labour, not price.** A firm cannot decide how much to make; it decides workers (by rule), and production follows. Overproduction shows up as inventory, which lowers hiring through the cover term.
3. **There are three levers of firm cost the player could move without touching the rules' structure:** `markup` (price), the wage step offset, and `green_tech` spending. These already exist as per-firm or per-step values.
4. **RNG.** Firms draw nothing. Only `class.rs` (the startup roll and the shared class stream), `nature.rs` and `weather.rs` draw; weather has its own stream since schema 5. Directives that replace deterministic rules add no draws.
5. **Profit tax and carbon cost are effective (lobbied) values**, so a firm-side action cannot reach the nominal lever, only the gap to the effective one, and only through households.
6. **Firms have no memory of decisions**, and `Firm.unfilled` is a bool; vacancy counts are a trace (`TraceRecord::Vacancies`), not state (`docs/features/markets.md` 2.2).

## 3. Gap analysis: real-world firm actions the model lacks

Ranking: **Teach** = pedagogical value for a newcomer (H high, M medium, L low); **Eco** and **Ineq** = how directly it touches the carbon-budget and inequality themes (H/M/L); **Cost** = implementation cost: S (a directive on an existing rule, content plus a small rule change), M (a new state field and rule, wire and UI), L (a new subsystem or entity), XL (new actors or a time-step change). Costs are estimates **(unverified)** from the how-to recipes (`docs/how-to.md` sections 1, 2, 7, 16, 21), not measured.

| Real-world action | Today | Teach | Eco | Ineq | Cost | Taxonomy roles and entities touched |
|---|---|---|---|---|---|---|
| **Choice of technology** (green vs brown) | One rule buys `green_tech` when carbon pays back (7) | H | H | M | **S** | `emitter`, `energy_consumer`, `payer`; Firm; levers `green_subsidy`, `carbon_price` |
| **Energy source** (own solar, buy renewable power, switch grid share) | None; grid pooled (8) | H | H | L | M (a per-firm renewable share or a firm-owned plant) | `energy_producer`, `energy_consumer`, `owned`, `buildable`; Plant (needs an owner) |
| **Automation replacing labour** | `tfp` drifts by sector growth; no capital-for-labour swap | H | L | H | M (a per-firm automation level cutting `beta_n`-labour per unit, raising capital intensity) | `employer`, `income_source` (wages); Firm, Person |
| **Wage setting beyond the rule** (efficiency wages, paying above or below the step) | Rule only (4) | H | L | H | **S** | `employer`, `payer`; Firm, Household (class) |
| **Price setting beyond cost-plus** (markup, loss leader, market power) | Cost-plus, fixed markup (1) | H | L | M | **S** to change the rule, but **needs competition to teach** (fact 2.1.1) | `payee`, `income_source` |
| **Product mix and quality** | One product per sector | M | M | L | L (a product dimension) | New concept; Firm, goods sectors |
| **Expansion and relocation** (new site, move out of smoke or fire zones) | Capacity grows in place; tile fixed after init | M | M | M | M | `located`, `hazard_exposed`, `buildable`; Firm tile |
| **Advertising** | None | L | L | L | M | `payer`; goods demand rule |
| **Supplier and buyer choice, bargaining** | Pooled labour and energy; buyers pick shops by price and stock | M | L | M | L | `payer`/`payee`, `employer`; goods and labour allocation |
| **Finance: reinvest vs payout vs buybacks** | `payout_ratio` per system; `retain_share` fixed (9, 10) | H | M | **H** | **S** (per-firm payout and retain override) | `payer`, `owned`, `income_source` (dividends); Firm, Household (Capitalist), Bank |
| **Borrowing choice** (borrow to green-invest or to expand; refuse debt) | Automatic request (11) | M | M | M | S to M | `payer`; Bank |
| **Political action and lobbying** (firms, not households, fund it; per firm stance) | Household-pooled, automatic (14) | H | **H** | **H** | M (a firm-level lobby spend that feeds `net_lobby`) | `payer`, `lever_target`; Lobbyists aggregate |
| **Tax avoidance and offshoring** | None (15) | H | M | **H** | M to L (a share of profit moved to an untaxed account, plus an audit rule; needs a money sink, so money conservation checks change) | `payer`, `owned`; Government, Account |
| **Voluntary exit and closure** (plant closure, relocate abroad) | Default only (12) | M | M | H | M (a "wind down" directive: stop hiring, fire, pay out, set `active = false`) | `employer`, `owned`; Firm, Person (jobs) |
| **Entry** (startups as a strategy, the player founding a firm) | A household roll fills the dormant slot (13) | H | L | M | M to L (see option B) | `owned`, `located`, `buildable` |
| **Mergers and acquisitions** | None | L | L | M | L | `owned`; Firm, Bank (`Owner::Bank` exists) |
| **Unions and strikes** (the worker side) | One aggregate `union_density` with an automatic drift (14); no strike | H | L | **H** | M (a strike removes a firm's output for a month; a per-firm bargaining state) | `employer`, `income_source`; Person, Household; `Class::Worker` |
| **Ownership form** (private, co-op, state) | Town-wide system switch (Market, Regulated, Coop); `Firm.owner` is per firm | H | M | **H** | M (a per-firm conversion command, e.g. Workers buy the firm) | `owned`; `Owner::{Household, Workers, State, Bank}` already per firm, so the state supports it **(verified: `Owner` is per firm; the system switch is town-wide)** |

Reading the table:
- The **cheapest, best-teaching** actions are the ones that already exist as a per-firm or per-step value: green technology, wage step, markup, payout. They map straight onto the profit-versus-emissions and profit-versus-wages dilemmas.
- **Lobbying, tax avoidance, unions and ownership** are the actions that connect firms to the inequality theme and to **collective action**. Lobbying in the model is a household aggregate today, so a firm that lobbies is also a missing piece of "who captures the policy".
- **Missing but expensive:** products and quality, advertising, mergers. Park them.
- A **separate honest gap**: there is no price signal for energy (`markets.md` 2.5), so energy choice would only matter through carbon and rationing, not cost.

## 4. Design options

Each section covers: what the player sees, commands and PC and lead time, how uncontrolled firms behave, determinism and save impact, Markets view and trades lists, scene and UI sketch, wire changes, tests, risk to balance (baselines A to F), pedagogical payoff, costs.

### Option A: firm directives layered on the planner (recommended start)

**Idea.** The player stays the planner and gains a **Firms** drawer: per firm, a few strategy **sliders and switches** (a directive). The automatic rules in section 2 stay as the **default policy**; a directive only overrides one term. A firm with no directive behaves exactly as today.

**Directives (first set, each wraps an existing rule term):**
| Directive | Override | Hook in code |
|---|---|---|
| Price stance (-, 0, +) | multiplies `markup` in `price_target` | `goods.rs` ~410, `f.markup` |
| Wage policy (below, rule, above) | adds an offset to `wage_step` (still clamped by `wage_cap`) | `labour.rs::wage_step` |
| Payout vs reinvest | a per-firm `payout_ratio` and `retain_share` | `income.rs` ~12, `investment.rs` ~120 |
| Green technology push (off, rule, on) | forces `worthwhile` and raises `green_spend_share` for that firm | `investment.rs` ~38-50 |
| Hiring posture (freeze, rule, expand) | scales `growth_aspiration` or `hire_slack` | `labour.rs` ~40 |

**What the player sees.** The firm inspector (entity registry `content/entities.json`, an inspector section) gets a "Directives" block listing each directive, the **rule's current choice** next to it ("the automatic rule would pay out 70% of profit"), and the **projected effect** from a Rust preview (like `CommandPreview.expectedEffective`, `docs/town-v6.md` finding 4). Markets and Finance views show the effect afterwards.

**Commands, PC and lead time.** A new `Command::SetFirmDirective { firm, directive }` in `sim-types/src/enums.rs`, wire `command.rs`, queued like levers with a lead time and a PC cost from `content/params.json` `turn.levers`-style entries **(unverified: whether levers' PC cost model can be reused per firm; recipe `docs/how-to.md` section 1 says the cost lookup is by key and panics when missing)**. Suggested default: directives cost PC (the player is leaning on firms the way a planner would, "moral suasion") and have a short lead time (1 turn). Open question Q1.

**Uncontrolled firms.** Identical to today (directive = `Rule`, the default). **By construction the baselines A to F do not move** while no directive is set.

**Determinism and save.** Adds a field to `Firm` (state), so `SCHEMA_VERSION` 5 -> 6 and the wire version bumps (additive, contract v2.6 proposed **(version number unverified; check `docs/snapshot-contract.md` for the next free number)**). The state hash (`cargo run -p sim-cli --example statehash`) changes because a field is added, even though the trajectory does not; to prove no behaviour change, a test compares the wire output of a no-directive run with the golden before the change apart from the new fields (the same method as `docs/town-v6.md` section 5). Old saves load with `Rule` defaults (`#[serde(default)]`, the pattern used for schema bumps **(unverified)**). No new RNG draws.

**Markets view and trades.** No new market. The Markets cards (`docs/features/markets.md`, scope A) already show price, wage and who is short; they would show the **effect** of a directive (a price stance up in the one-firm Shop market moves the price series and unsold units, not demand, per fact 2.1.1, which the card text must say). Trades lists (`market-trades.md`) need no change. The Finance and ledger views show changed dividends and wages.

**Scene and UI sketch.**
```
Click Factory -> Inspector
  [Directives]                              PC 3 per change, takes effect in 1 turn
  Technology   [ Rule ][ Push green ]       rule: buys a step when carbon pays back in 5 y (it would not yet)
  Payout       [ 40% ---o----- 90% ]        rule: 70%   ->  dividends +$X, capital -$Y
  Wage         [ below | rule | above ]     rule: +0.2% a month
  Price        [ - | rule | + ]             in a one-firm market, price is not limited by demand (why?)
```
A firm badge on the scene (a small flag) marks a firm under a directive.

**Wire changes.** `FirmView.directive` (the choice and the rule's default for comparison), `CommandPreview` for directives, `catalog` entries (labels, hints in `content/ui.json`), `levers.json`-style metadata `content/directives.json` (new; ranges, labels, lead time, PC cost). TS via `npm run gen:contract` (`docs/how-to.md` section 19). The web only renders (no formula in `web/src`).

**Tests.** (1) default directive reproduces the golden exactly (apart from the new field); (2) sign tests per directive in `tests/scenarios.rs` (green push lowers footprint and raises the firm's cost in the short run; payout up lowers capital growth; wage above raises household wage income and lowers profit; price up in a one-firm sector leaves revenue unchanged and raises inventory); (3) assumptions topics and the randomness table untouched (no new draws); (4) the registry and contract tests the compiler flags.

**Risk to balance.** Zero by construction at defaults (baselines A to F). Playing with directives will change balance, which is the point; the risk is **dominant strategies**: price up in a monopolist sector (fact 2.1.1) and payout up (starves investment, but raises Capitalist income, raising Gini) must be bounded by clamps in content (`directive.price.max` etc.). Re-run `cargo run --release -p sim-cli --example baselines` with scripts that set directives **(a new baseline G, "firms play brown" and H "firms play green", is proposed, unverified)**.

**Pedagogical payoff.** The **profit vs emissions dilemma** (green push costs the firm now and pays back with the carbon price), **who gets the profit** (payout vs reinvest and the Gini), **wages vs profit** (the labour share), and the first honest view of what the automatic rules were deciding for the firms.

**Cost.** S to M: one state field, one command, five hooks, one directive content file, an inspector section, preview. About a feature the size of `inflation.md` **(unverified estimate)**.

### Option B: play as one chosen firm

**Idea.** The player picks a firm (Factory, Farm or Shop) and runs it with its own decisions (all of option A plus entry-like moves, hiring and a wider menu), while the planner levers go to the AI or are fixed by a scenario.

**Core problem: market structure.** With one firm per sector (verified: `initial_town.json` has Factory, Farm, one Shop; the fourth slot is a dormant **Shop**), a controlled firm is a **monopolist whose price has no demand penalty** (fact 2.1.1). Market lessons (competition, price war, race to the bottom, entry deterrence) need two or more firms per sector. The model supports this partially: `allocate` already splits a budget over `n` firms by stock and price elasticity (verified), and `class.rs` can wake the dormant slot (when sector excess demand >= 0.2 and a household has savings).

**A multi-firm sector (needed for real competition).** Options: (a) start the Services sector with both Shops active (turns the dormant slot on; `MAX_FIRMS` stays 4; startup then has no slot); (b) raise `MAX_FIRMS` (touches every per-firm array, the map reservation, `persist.rs`, the wire `firms[]` and the web scene, `docs/how-to.md` section 7), to give a second Factory and Farm; (c) keep the dormant slot as an entry event the player can trigger or deter. A competitor changes the initial town, so **all baselines A to F move and must be re-tuned** (`docs/tuning-log.md`; a small town is noisy and one firm closing swings unemployment, `docs/how-to.md` section 7).

**What the player sees.** A firm dashboard: its income statement, cash, loans, inventory, workers and wage, price against competitors, market share (Markets card for its sector with "you" highlighted), carbon footprint and cost, and the town's response (unemployment, wages, legitimacy of the rest). The planner levers become **exogenous**: a scripted or AI planner moves them, which is a large new behaviour (a "government AI" that does not exist today **(unverified)**; the government only runs fixed rules in `government.rs`).

**Commands, PC and lead time.** Firm actions replace PC with **cash and time** (a firm has no political capital): a firm command costs money (e.g. green investment is paid labour) and takes effect with the pipeline's lead time. The PC mechanism would be reused only for lobbying (spend that raises `net_lobby`). Open question Q2.

**AI firms when not controlled.** The automatic rules. The **player's** firm uses the same rules until overridden, so the existing Rule is the AI.

**Determinism and save.** Needs a "controlled firm" field in the turn layer (not the model) and directives (as in A); schema bump as in A, plus any added firm slots (state size). RNG unchanged if no entry draw is added; an extra competitor adds no draws but changes the class and startup branches, so hashes of the default game change.

**Markets view and trades lists.** The Markets cards become the **main screen**: price vs competitors, market share, unmet demand, a "who sold to whom" trade list (`market-trades.md`, already lists trades) showing where the player's firm wins or loses customers.

**Scene and UI.** The scene stays the town; a firm "cockpit" replaces the planner's top bar; other firms are labelled "AI". A "you are a firm" mode flag in the snapshot.

**Wire changes.** `game.mode`, `controlledFirm`, `FirmView.directive` and a firm-scoped command set; new catalogue entries; if `MAX_FIRMS` changes, `firms[]` and TS contract regenerate.

**Tests.** As A, plus competition tests: with two Shops, a price cut shifts share by the elasticity; a price rise loses sales; startup into a vacant slot; no regression in monopoly sectors.

**Risk to balance.** **High**: a changed initial town (two Shops) and a scripted planner break the comparability of baselines A to F and the 40-turn win condition (the carbon budget is a town goal, not a firm goal). A firm's goal (profit) is also not the game's goal (survive under 1.5 C); a **win condition for the firm player** would need design (profit, survival, reputation, share of the town's wellbeing).

**Pedagogical payoff.** Highest for **market competition**, **price wars and the race to the bottom** (wages and green spending are cut to win share), **entry and market power**, and the **free-rider problem** (a green firm that pays a carbon price its rival also avoids). It teaches the firm's side of every planner lever. Without a competitor the payoff is poor (fact 2.1.1).

**Cost.** L to XL: a multi-firm sector (re-tuned town), a government or planner AI, a firm win condition and UI, plus option A's directives as a base.

### Option C: role switch (firm, worker, household, government)

**Idea.** The player picks a role and a command set: firm (A or B), worker, household, or government (today's planner).

**Per role.**
- **Household or worker:** decisions are spending vs saving, where to work, whether to move or found a firm, lobbying and union membership. Today households decide by automatic rules (`goods.rs` budgets, `housing`, `class.rs`); the player would override budgets and job search. The commands and views are new (a personal ledger exists: `docs/town-v5.md`, per-entity ledgers).
- **Government:** today's game.

**What the player sees, commands, PC.** Each role has its own resource (PC for government, cash for firm, income and time for households) and its own win condition. Switching roles mid-game (hot seat) is cheap in UI terms but forces every role to have an AI, since the other roles must be automatic.

**AI when uncontrolled.** For each role the current automatic rules, which exist for households and firms and government, but **the government's rules are not a planner AI** (it taxes and spends by lever settings; it never sets levers, **verified: no code sets policy levers except commands and lobbying's effective gap**).

**Determinism and save.** Adds a role field to the turn layer and per-role directive state (households need their own directive fields: a larger schema change than A). RNG unchanged unless household behaviours change.

**Markets and trades.** Each role sees a different slice: a worker sees labour and housing, a household sees goods and rent, a firm sees goods and labour (demand side) and credit.

**Wire changes.** Large: role-scoped commands and views for every role. 

**Tests, risk.** As A for each role; balance risk moderate (no change to the AI defaults) but the **surface is three games**. **Pedagogical payoff**: broad (collective action and bargaining: unions, lobbying, tenants), but diluted. **Cost: XL.** Not recommended before A and B.

## 5. Recommendation and staged delivery

**Recommendation.** Build **A first, in stages**, then decide on B with evidence from A. C is out of scope for now. Reasons: A has zero balance risk at defaults, reuses the lever pipeline (lead time, PC, preview), teaches the strongest dilemma (profit vs emissions, profit vs wages, payout vs reinvest), and every directive is a **wrapper over a rule term that already exists** (section 2). B teaches more about competition but requires a re-tuned town and a firm win condition, and one monopolist firm teaches the wrong lesson about price (fact 2.1.1).

**Stage 1 (small).** Two directives only, on all three firms: **green technology push** and **payout vs reinvest**. Reason: they are the two best-teaching, lowest-risk wrappers (ecology and inequality), use existing hooks (`investment.rs`, `income.rs`), and need no competition. Delivers: `Command::SetFirmDirective`, a `Firm.directive` field (schema 6), `content/directives.json`, the inspector block, a preview, an assumptions topic "What firms decide", the same `agents` catalogue update (`docs/how-to.md` section 16), tests (no change at defaults; sign tests), baselines G and H. Roughly one feature the size of `docs/features/inflation.md` **(unverified)**.

**Stage 2.** Wage policy and hiring posture; show worker-side effects in the household inspector.

**Stage 3.** Price stance, **only together with a second Shop** active at start (or a clear UI note that a monopolist has no demand limit); tie to `markets.md` scope B (demand-responsive prices) so that price has a cost. Needs a user decision since scope B is not approved.

**Stage 4.** Firm-level lobbying and a union side (strike). Moves lobbying from the household aggregate to firms. Larger, a feature of its own.

**Stage 5 (decide after 1 to 4).** Option B on top of the directives, with a firm win condition.

### 5.1 Open questions (with defaults)

| # | Question | Default |
|---|---|---|
| Q1 | Do directives cost PC (the planner "leans on" firms) or something else? | PC, with a lead time of 1 turn; revisit if it feels odd |
| Q2 | In option B, what does the firm player win by? | Not decided here; default to a firm score (profit, survival, share of workers' wellbeing) shown next to the town's carbon budget |
| Q3 | Should uncontrolled firms keep today's rules exactly? | Yes (zero balance risk) |
| Q4 | Should the first stage ship with price stance? | No: fact 2.1.1; wait for a second Shop or scope B |
| Q5 | Do directives apply to the dormant slot's firm once founded? | Yes, but only to its own `Firm` once `active` |
| Q6 | Does a household-founded firm (Owner::Household) differ from a Coop firm? | Directives apply to both; the payout directive overrides the system's `payout_ratio` for that firm only |
| Q7 | Where do directive numbers live? | `content/directives.json` (new, loaded by `sim-params`), not `params.json` and not the web |

### 5.2 Out of scope

Product mix and quality, advertising, mergers, offshoring models that need a new money sink, a planner AI, daily ticks (`docs/features/daily-ticks.md`), a full multiplayer hot seat, and GST or state ownership (`docs/features/gst-and-state-ownership.md`, which touches the same `Firm.owner`).

## 6. What was not checked

- Whether the PC cost and lead-time tables can be keyed by firm and directive without a refactor of `turn.rs` and `plan.rs` **(unverified; read only `docs/how-to.md` section 1)**.
- Exact next contract version and whether `#[serde(default)]` is how earlier bumps loaded old saves.
- Whether the state hash covers every `Firm` field **(unverified)**.
- Implementation cost figures (estimates).
- No code was run; nothing was measured with the CLI for this draft.
