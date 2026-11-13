# Feature: explicit decision points (refactor, queryable, causal-graph ready)

Status: **DONE (all five stages implemented). Stages 1 (labour), 2 (firm decisions, section 12), 3 (household and bank, section 13), 4 (government, estate and nature, section 14) and 5 (generated Agents text for every kind, the causal graph on the wire, section 15).** See sections 10 to 16. Written from `crates/sim-model/src/*.rs` at HEAD (`7107499`). Where this page and code disagree, code wins. Claims not checked in code are marked **(unverified)**. First step toward `docs/features/firm-agency.md`.

## 1. Why

Each system is a long pure `run(&State, &Params, &mut Rng) -> Deltas`. The `run`s are good (pure, deterministic). The **decisions** inside them are not visible: hire, fire and wage sit in one 230-line `labour::run`; pricing is 6 lines inside `goods::run`. Consequences: (a) the "Who decides" text in `content/assumptions.json` (`agents` block: `kinds[].decides / observes / rule / params`) is hand-written and can drift from code; (b) a player directive (firm agency) has no seam to replace; (c) there is no machine-readable "what reads what", so a **causality view** (a -> b -> e, d -> e) would have to be hand-drawn. Goal: make each decision an explicit pure function with declared inputs, outputs and params, registered in one place. **No behaviour change**: state hash byte-identical.

Precedent already in the repo: `price_target` and `wage_step` are small `pub fn`s called from `run` and exposed through `sim-api::assumptions::registry()` (`docs/how-to.md` section 15). This feature generalises that pattern.

## 2. Inventory (checked against code)

"Fn?" = already a separate function. Only the rule core of a few (price target, wage step, shop split, lobbying effect, legitimacy step) is.

| # | Owner | Decision | File:line | Reads (inputs) | Effect (output) | Params | Fn? |
|---|---|---|---|---|---|---|---|
| 1 | Firm | Price | goods.rs:410-412 (`price_target` :239) | markup, wage bill, carbon cost, output, price | price | `goods.price_adjust`, `min_price_usd`; `Firm.markup` | rule yes, step no |
| 2 | Firm | Hire target and vacancies | labour.rs:40-63 | expected sales, inventory, workers, skills, capital | target workers, vacancies | `labour.growth_aspiration`, `inventory_cover`, `inventory_adjust`, `hire_slack`, `skilled_share` | no |
| 3 | Firm | Hire (who, how many) | labour.rs:65-113 | vacancies, unemployed, reservation wages | job assignments | `labour.max_hire_per_month` | no |
| 4 | Firm | Fire | labour.rs:116-146 | headcount vs target, `over_target_months` | job losses | `labour.fire_after_months`, `fire_slack`, `max_fire_per_month` | no |
| 5 | Firm | Wage | labour.rs:150-162 (`wage_step` :15) | unemployment, sector TFP growth, min wage | wage | `labour.wage_k`, `u_ref`, `wage_cap`, `inflation_anchor` | step yes |
| 6 | Firm | Green technology step | investment.rs:36-80 | footprint, carbon price (announced), revenue, cap, subsidy | `green_tech`, spend | `invest.green_payback_years`, `green_cost_share`, `green_max` | no |
| 7 | Firm | Capex plan and loan request | investment.rs:85-100 | profit EMA, capital, expected sales, loan rate | `plan_invest`, `loan_request` | `invest.accelerator`, `max_invest_share`, `return_scale` | no |
| 8 | Firm | Dividend payout | income.rs:85-100 | profit, cash, payroll, debt service | dividends | `payout_ratio` (system), `income.cash_reserve_months` | no |
| 9 | Household | Spending split (food, goods, services) | goods.rs `decide_spend` (was :255-273) | savings, last disposable, class, wealth | three budgets | `goods.propensity.*`, `food_need_units_pc`, `services_share` | no |
| 10 | Household | Shop choice (also used by firms buying capital goods and the government) | goods.rs `decide_shop` (was private `allocate` :23) | stock, price per firm, the market's budget | units sold, demand units per firm | `goods.price_elasticity`, `min_price_usd` | yes (private) |
| 11 | Household | Found a firm | class.rs:138-165 | savings, sector return, base rate, RNG | new firm | `class.mobility_rate`, `startup_cost_usd` | no |
| 12 | Household (capital class) | Lobby spend | lobbying.rs `decide_lobby` | capital income, savings, `campaign_cap` | gift to the lobbyist household | `lobby.spend_rate` (the draft said `lobbying.spend_rate`; the section is `lobby`) | no (`expected_effective` yes) |
| 13 | Bank | Lend to firms: headroom, approval, rate by leverage, payout, rescue (five decisions) | credit.rs `decide_headroom`, `decide_lend`, `decide_spread`, `decide_bank_dividend`, `decide_rescue` | requests, equity, loans, capital ratio, firm assets and debt | approved loans, loan rate, dividend, bailout or haircut | `credit.max_firm_leverage`, `spread_*`, `bank_*`, `bailout_target_ratio`, `haircut_margin` | no |
| 13b | Firm | Expected sales (the demand -> hiring link) | goods.rs `decide_expect` | demand units, expected sales | expected sales | `labour.expected_smoothing` | no (missing from the draft inventory) |
| 14 | Government | Fund spending, then ration it (3 decisions in the build: `government_fund`, `government_cut`; the legitimacy step is row 15) | government.rs `decide_fund`, `decide_cut` (was inline :203-225) | committed payments, next month's purchases (the G lever x `cash_target_months`), cash, taxes, the bank's lending room, public debt | borrow the gap within room, or repay `repay_share` of a surplus; pay every committed payment at one share | `gov.repay_share`, `gov.cash_target_months` | no (now yes) |
| 15 | Government | Legitimacy | government.rs `decide_legitimacy` (`legitimacy_step` is a wrapper) | wellbeing, smoothed unemployment, Gini, public debt over GDP, bailout, transition resistance | legitimacy | `gov.legit_*` (8, names verified) | step yes |
| 16 | Estate | Pilot dismissal and rehire; flight | estate.rs `decide_pilot`, `decide_flight` (was :26-67) | months of short pay, pilot employed and free, estate savings; RNG draw | keep, let go or hire; flight | `estate.pilot_*`, `estate.flight_prob_monthly` | no (now yes) |
| 17 | Nature (non-agent) | Commons enclosure | nature.rs `decide_enclosure` (was :148-160) | the system's chance (`cfg`), RNG draw, whether the commons are open | whether an owner fences it (who: matching, in `run`) | none (system `enclosure_prob`) | no (now yes) |

Not decisions (rules of physics or accounting; stay plain code): output (production.rs `capacity`, `cobb_douglas`), emissions, weather draws (weather.rs, own RNG stream), wellbeing, rent and eviction (goods.rs `housing`, :82). Government spending allocation by purpose (government.rs) is policy execution of player levers; checked in stage 4: no hidden choice (DP-10).

## 3. Target shape

```
system run(s,p,rng):  for each owner -> view = View::from(s,p,owner)   // read-only, plain data
                                     -> choice = decide_x(&view, &p.x)  // pure, no State, no Rng
                                     -> push Choice into Deltas          // same deltas as today
```

**Quantity vocabulary** (`QuantityId`, a closed enum, snake_case on the wire): every state variable or flow that decisions read or write: `price`, `wage`, `workers`, `vacancies`, `inventory`, `expected_sales`, `unmet_demand`, `unit_cost`, `profit`, `cash`, `loan_request`, `green_tech`, `footprint`, `emissions`, `carbon_price`, `unemployment`, `savings`, `consumption_budget`, `legitimacy`, ... Each carries a label, unit and owner kind in `content/` (text lives in content, per AGENTS.md rule 9).

```rust
// One enum, not two: the owner kind is a method on DecisionId (an exhaustive match), never a stored field, so the two cannot disagree.
// (A nested `Decision::Firm(FirmDecision::Price)` is an equivalent option if firms gain many more decisions.)
pub struct DecisionSpec {
    pub id: DecisionId,            // enum: FirmPrice, FirmHire, FirmFire, FirmWage, ...; the owner is derived: DecisionId::owner() -> OwnerKind
    pub system: SystemId,          // where it runs
    pub inputs: &'static [QuantityId],
    pub outputs: &'static [QuantityId],
    pub params: &'static [&'static str],   // paths into content/params.json
    pub rule: &'static str,        // default rule id, e.g. "cost_plus"
}
```

- **Graph for free:** nodes = quantities + decisions; edges `input -> decision -> output`. A quantity written by decision A and read by B links A to B. Later metadata (sign, strength, lag) are extra fields on the spec or on an edge table in `content/`; not designed here.
- **Override seam (later, out of scope):** `decide_*` is called through `fn decide(view, params, directive: Option<&Directive>)`; a player directive replaces the default rule's output, clamped. A directive adds no RNG draw (firm-agency fact 4).
- **Hidden state is forbidden:** `View` is built only from `State` and `Params`, so a decision cannot depend on anything the spec does not list.

## 4. Options compared

| | A. `macro_rules!` / attribute proc macro | B. Plain fns + hand-kept const registry + tests (recommended) | C. `trait Decision<View, Choice>` | D. Build-time generation (`build.rs`) |
|---|---|---|---|---|
| Spec next to the fn | yes (one place) | spec is a `const` right above the fn | on the impl | spec in JSON, code generated |
| Drift between spec and code | none | possible; caught by tests below | possible | none for ids, not for bodies |
| `unsafe` / `HashMap` / `f32` rules | `macro_rules!` clean; proc macro needs a new crate (`proc-macro = true`), and its output must still pass `forbid(unsafe_code)` (sim-wasm already needed `deny` for macro output, AGENTS.md repo map) | clean | clean | clean, but generated code is hidden |
| Compile time | `macro_rules!` negligible; proc macro adds syn/quote to the build (cold only) | none | small (monomorphisation, or dyn) | extra build step; breaks wasm/CI simplicity |
| Readability | fair to poor (magic, errors point into macro) | best: ordinary Rust, greppable | good but ceremony per decision | poor |
| clippy `-D warnings` | macro-generated lints are harder to silence | none | `dyn` and unused-generic warnings possible | generated-code lints |
| Verifies inputs really are read | no (declares, does not check) | no, but test `view-only reads` (section 6) | no | no |
| Determinism, state hash | neutral | neutral | neutral | neutral |

Why B: the data the graph needs is a *declaration*, and no option can prove a function body reads exactly the listed inputs; so we need tests regardless. B gets the declaration, queryability and tests with the least machinery, is ordinary reviewable Rust, adds no dependency, and can be wrapped by a `macro_rules!` later if the boilerplate hurts (a small `decision!{}` macro that expands to the const plus fn is an additive step, not a redesign). Traits (C) buy polymorphism we do not need: owners differ in `View`, so no shared call site exists. D hides logic from readers and conflicts with "all numbers in `content/`".

**Worked example (B), pricing and hiring**

```rust
// goods.rs
pub struct PriceView { pub price: f64, pub markup: f64, pub unit_cost: f64 }
pub const FIRM_PRICE: DecisionSpec = DecisionSpec {
    id: DecisionId::FirmPrice, owner: OwnerKind::Firm, system: SystemId::Goods,
    inputs: &[Q::Price, Q::Markup, Q::UnitCost], outputs: &[Q::Price],
    params: &["goods.price_adjust"], rule: "cost_plus",
};
pub fn decide_price(v: &PriceView, gp: &GoodsParams) -> f64 {   // the old lines 410-412
    if v.unit_cost > 0.0 { v.price + gp.price_adjust * (price_target(v.markup, v.unit_cost) - v.price) }
    else { v.price }          // keep the existing fallback exactly
}
```

`run` builds `PriceView` from `f`, calls `decide_price`, converts with the same `Cents::from_f64`. Hiring splits the same way: `decide_hire_target(&HireView, &LabourParams) -> HireTarget { target, skilled_vacancies, general_vacancies }` (labour.rs:40-63), then `decide_fire` (:116-124), then `decide_wage` (wraps `wage_step`). The matching of unemployed to vacancies (:65-113) is mechanics and stays in `run`.

## 5. How the registry feeds things

| Consumer | How | Effect |
|---|---|---|
| Agents explorer "Who decides" | `sim-api` builds `decides`, `observes`, `params` (with live values) from `DECISIONS` plus labels in `content/`; the hand-written `decides`/`observes`/`params` arrays in `assumptions.json` shrink to prose `summary` and `rule` text keyed by `DecisionId` | no drift; every decision appears |
| Wire / catalog | new additive `catalog().decisions[]` and `quantities[]`, `TS` derive, `npm run gen:contract`, contract minor bump (2.7.0) | web renders only |
| Causality graph (future) | `decisions()` and quantities give nodes; inputs/outputs give edges; a "causes of X" query is a reverse walk | UI not designed here |
| `assumptions::registry()` | worked examples call the same `decide_*` | one implementation |

Relation to other docs: **registry-driven-sim** (parked) makes the *sim* read `entities.json` for entity kinds; this feature registers *decisions*. They are independent; both avoid enum arms per kind, and `OwnerKind` should reuse the taxonomy's role names (`docs/taxonomy.md`: `employer`, `payer`, `emitter`, ...) so owners and quantities line up with roles. Decisions needing no role system stay bespoke.

## 6. Delivery plan

| Stage | Scope | Gate |
|---|---|---|
| 0 | `cargo run -p sim-cli --example statehash > before.txt`; note goldens | baseline |
| 1 | **Pilot: `labour.rs`** (decisions 2-5, the most buried). Add `DecisionSpec`, `QuantityId`, `OwnerKind` in `sim-types` (data only); `decide_*` fns | statehash identical; `tests/golden/seed7.json`, fixtures, recording and insta snapshot unchanged |
| 2 | `goods.rs` (1, 9, 10) | same |
| 3 | `investment`, `income`, `credit` (6-8, 13) | same |
| 4 | `class`, `lobbying`, `government`, `estate`, `nature` (11, 12, 14-17); RNG draws stay in `run` in the same order (class.rs:163 draws always, before use) | same, plus the 9-draw-site test |
| 5 | Wire: `catalog().decisions`, generate Agents text from it, shrink `assumptions.json` | contract bump, fixtures regenerated |

**Tests**
- Every `DecisionId` variant has exactly one spec in `DECISIONS`, and one `decide_*` (exhaustive `match`).
- Every `params` path resolves in `Params` (reuse the explorer's path check); every `inputs`/`outputs` id has a label and unit in `content/`.
- *View-only reads*: `decide_*` signatures take no `State`, `Params` or `Rng` beyond their own params struct (enforced by types); a property test perturbs each non-listed `View` field and asserts the Choice is unchanged, and perturbs each listed input and asserts it can change.
- *No hidden state*: the same `View` returns the same `Choice` on two calls; no `static` or `thread_local` in `sim-model` (grep test).
- Hash identity: `statehash` before/after at each stage, 3 seeds.

**Rules check (AGENTS.md):** systems stay `fn(&State,&Params,&mut Rng) -> Deltas`; `decide_*` are pure helpers, no mutation; Cents stay Cents inside `View` boundaries (convert at the same points as today); floats via existing `libm` calls, no `f32`; `BTreeMap`/`Vec` only; `forbid(unsafe_code)` untouched under option B; no new RNG source.

## 7. Risks

- Float order of operations: moving an expression into a function must not reorder arithmetic. Mitigation: cut and paste, statehash gate.
- Declared inputs drift from what the body reads. Mitigation: the perturbation test; review.
- `View` copies cost allocation in hot loops (`workers_of` collects per firm). Mitigation: views borrow slices; measure with the 40-turn run.
- Spec list becomes a second place to edit. Mitigation: it is the single place text, params and graph come from.

## 8. Open questions (default in bold)

1. Where do `DecisionSpec` and `QuantityId` live? **`sim-types` (data only).**
2. One `View` per decision or shared per owner? **Per decision, small.**
3. Is `Choice` the delta itself? **No: a small struct, then `run` maps it to deltas.**
4. Quantity labels and units in `content/ui.json` or a new file? **New `content/decisions.json`.**
5. Add the `decision!{}` convenience macro now? **No; revisit after stage 3.**
6. Do mechanics (matching, `allocate`) count as decisions? **Shop choice (10) yes; matching of hires no.**

## 9. Out of scope

The causality graph UI and any sign/strength data; player directives and overrides; new behaviour of any kind; balance changes.

## 10. As built (stage 1: the labour pilot)

Written from the code after stage 1. Where this and the code disagree, the code wins.

**What exists**
- `crates/sim-types/src/decisions.rs` (data only): `OwnerKind`, `DecisionId` (`owner()` is an exhaustive match), `QuantityId` (14 quantities), `DecisionSpec`. A 10-line `macro_rules! named_enum!` writes each enum's variants, `ALL` and snake_case `as_str()` once (no `unsafe`, passes clippy; it removes a three-times-repeated list that would otherwise drift). `DecisionSpec` itself is a plain `const` struct, as recommended (option B).
- `crates/sim-model/src/labour.rs`: `FIRM_HIRE`, `FIRM_FIRE`, `FIRM_WAGE` (specs) next to `decide_hire`, `decide_fire`, `decide_wage` and their views (`HireView`, `FireView`, `WageView`) and `*_view` builders. `run` is now: build view, decide, apply (matching, picking whom to fire, the job guarantee and person patches stay in `run`). The fire view reads the hire choice (`workers_wanted`), so the graph has a real edge `hire -> wanted_workers -> fire`.
- `crates/sim-model/src/decisions.rs`: `DECISIONS` (the const registry), `spec`, `readers_of`, `writers_of` (the graph queries), and the registry tests.
- `crates/sim-api/src/decisions.rs` + `content/decisions.json`: quantity labels, units and summaries, decision titles and rule text, parameter labels and units, and the owner to Agents-kind map. Joined with the registry at `catalog()` time and validated (missing or extra entries, rule id not matching the code, unknown parameter path: `bad_metadata`).
- Wire (contract 2.7.0, additive): `catalog.decisions: DecisionInfo[]`, `catalog.quantities: QuantityInfo[]`. The firms Agents kind's `decides`, `observes` and `params` now start with lines generated from the decisions; `content/assumptions.json` keeps only what no registered decision covers (price, invest, expected sales).
- Web: Agents explorer shows a "Decisions" list per kind (`inputs -> decision -> outputs`, rule and parameters in a disclosure) from `catalog.decisions`, filtered by `agentKind`. No decision is named in the web.

**Differences from the draft**
- Views carry state-derived quantities only; `decide_hire` also takes `&LabourParams` and `&ProductionParams` (for `beta_n`), `decide_wage` and `decide_fire` take `&LabourParams`. The sector's inventory cover and productivity growth are selected by the view builder (`inventory_cover`, `sector_tfp_growth` are quantities), so no decision matches on a sector.
- `output_capacity` is one quantity standing for capital, technology, weather and climate (`production::capacity`); splitting it is for the graph work, not now.
- `Choice` is a small struct, not the delta (open question 3 default). `HireChoice.max_hires` passes `labour.max_hire_per_month` through so that the declared parameter is genuinely read by the decision.
- Whom to hire and whom to fire are mechanics and stay in `run` (open question 6 default).
- `DecisionSpec` does not carry `OwnerKind` (derived) and `system` is `SystemId::Labour`.

**Tests (all run in `cargo test --workspace`)**
- `sim-model decisions::tests`: every `DecisionId` has exactly one spec; ids and quantity names unique; no `static` or `thread_local!` in the model crate; the graph queries find `hire -> wanted_workers -> fire`.
- `sim-model labour::decision_tests`: **view-only reads** for each decision: every view field is a quantity the spec lists (the table is built next to an exhaustive destructure, so a new view field fails to compile until added), every listed input is carried by a view field, perturbing each field changes the choice for at least one base view, and the same view gives the same choice.
- `sim-api tests/decisions.rs`: every listed parameter path resolves in `content/params.json`; every quantity and decision has text; the firms "Who decides" lines and parameters are generated and no longer hand-written in `content/assumptions.json`.
- Web `tests/agents.test.ts`: the wire carries the decisions; a decision added only in the catalogue appears with no web change.

**Gate results.** State hash (`statehash`, 3 seeds x plans) byte-identical before and after; baselines A-F identical (`cargo run --release -p sim-cli --example baselines`); insta snapshot and the golden's per-turn hashes unchanged; the golden changes only in the `agents` and `catalog` wire hashes. Recording, fixtures and generated contract regenerated.

**Known limits (see `docs/known-issues.md`, DP-1 to DP-3).** The perturbation test proves a declared input matters, not that no other input is read inside `decide_*`; the view builders read `State` and are not covered by it; the Agents "observes" list now has one line per input quantity (long).

## 11. Lessons from the pilot: the amended plan (stages 2-5)

Written after stage 1, before stage 2. These rules replace the matching lines of sections 6 and 8 where they differ.

**Stage 5 shrinks.** The wire (`catalog().decisions`, `quantities`, contract 2.7.0) and the generated Agents "Who decides" text already exist for firms. Stage 5 is now only: generate the text for every kind from the registry and delete the remaining hand-written `decides`/`observes` lines (with a test that none are left), an additive causal-edges list on the catalog (decision inputs -> decision -> outputs, and quantity links from `writers_of`/`readers_of`), and the usual regeneration and a contract bump.

**Each stage lists its new `QuantityId`s and `content/decisions.json` entries up front** (below); the as-built section then records any difference. Every new owner kind needs a row in `content/decisions.json` `owners` (DP-4); `catalog()` already fails without it.

| Stage | New decisions (ids) | New quantities (`QuantityId`) | New content entries |
|---|---|---|---|
| 2a (granularity) | none | remove `output_capacity`; add `capital_output`, `weather_factor`, `resource_factor`, `transition_factor` | 4 quantity entries (1 removed) |
| 2 goods | `firm_price`, `firm_expect`, `household_spend`, `household_shop` | `price`, `markup`, `unit_cost`, `demand_units`, `units_sold`, `savings`, `last_disposable`, `net_wealth`, `food_price`, `food_need`, `spending_propensity`, `wealth_spending_rate`, `consumption_budget`, `expectation_smoothing` is a parameter, not a quantity | quantities, 4 decisions, params, owner `household` |
| 3 firm money and bank | `firm_green`, `firm_capex`, `firm_payout`, `bank_lend`, `bank_spread`, `bank_default` | `footprint`, `carbon_price`, `revenue`, `emissions_cap`, `green_subsidy`, `treasury_cash`, `cash`, `cash_reserve`, `green_tech`, `green_plan`, `profit`, `capital`, `capital_per_sale`, `loan_rate`, `distressed`, `investment_plan`, `loan_request`, `payroll`, `debt_service`, `dividends`, `credit_headroom`, `debt`, `firm_assets`, `loan_approved`, `base_rate`, `distress_months`, `firm_open` | quantities, 6 decisions, params, owner `bank` |
| 4 households, state, estate, nature | `household_found_firm`, `household_lobby`, `government_fund`, `government_legitimacy`, `estate_pilot`, `estate_flight`, `nature_enclosure` | `sector_return`, `random_draw`, `capital_income`, `campaign_cap`, `lobby_spend`, `treasury_taxes`, `committed_spending`, `next_month_purchases`, `borrowing`, `repayment`, `spending_cut_ratio`, `legitimacy`, `wellbeing`, `inequality`, `public_debt_ratio`, `transition_resistance`, `bailout`, `pilot_unpaid_months`, `pilot_employed`, `estate_savings`, `flight`, `enclosure_chance`, `commons_open`, `commons_enclosed` | quantities, 7 decisions, params, owners `government`, `estate`, `nature` |
| 5 wire | none | none | none; `catalog().edges` (additive, contract 2.8.0) |

The lists are the plan; the as-built section of each stage records the final names. A decision that turns out not to be separable without changing behaviour stays in the system and is recorded in `docs/known-issues.md` with the reason.

**Quantity granularity (decided now; DP-2).** `output_capacity` hid weather, climate and technology in one node. It is split along the factors `production::capacity` multiplies: `capital_output` (technology x capital^alpha at one unit of labour), `weather_factor` (climate damage, drought, smoke), `resource_factor` (farm health, community-managed commons) and `transition_factor` (output lost while the economic system changes). The hire view carries the factors as separate fields and `decide_hire` multiplies them in the same order as `capacity` does (a factor that does not apply is exactly 1.0, so the product is bit-identical). Chosen for simplicity: four quantities, no per-source split of the weather factor (drought and smoke share one node), and `capacity` itself stays plain code used by production. Result: the future graph shows weather -> hire.

**The perturbation test is the gate for every `decide_*` and its view builder** (DP-1):
1. Per decision (existing `check_view_only`): every view field is a declared input, every declared input is a field, perturbing each field changes the choice for some base.
2. Per builder (new, `crates/sim-engine/tests/decision_builders.rs`): build the views from a real mid-run `State` and `Params`; serialise both to JSON; perturb each numeric or boolean leaf in turn (rebuild, rebuild the views); record which leaves change which decision's choice. Each decision lists the State and Params paths its builder is declared to read (array indices written `[]`). The test fails if a leaf outside the declared list changes the choice (an undeclared read), and if a declared path never changes any choice in any sampled state (a dead declaration, or one that needs a new sampled state).
3. What this can prove: for the sampled states and owners, no undeclared leaf of State or Params changes the choice, and each declared leaf can. What it cannot prove: absence of a read in states where the branch is not taken (a read behind a condition the sample never meets), leaves of other array elements (indices are collapsed), and RNG (draws are passed in as plain data, so the decision cannot draw). Reads inside the `decide_*` body are still limited by types, not tests.

**Firm pricing reads `Firm.markup` (state).** The price rule is `price + price_adjust x ((1 + markup) x unit_cost - price)`; `markup` is a per-firm state variable (set at start from content and copied to a new firm), not a parameter. It is a declared input quantity of `firm_price`; the spec's parameters are `goods.price_adjust` and `goods.min_price_usd`.

**RNG.** Draws stay in the system, in the same order, and enter the decision as plain data (`random_draw`, or the Bernoulli outcome as a boolean): `class::run` draws once always before use, `estate::run` first, `nature::run` the enclosure draw last.

**Not decisions (stay plain code):** production and capacity, emissions, weather, wellbeing, rent and eviction, the matching of hires and loans, who is fired, and the apportioning of money among households.

## 12. As built (stage 2: the firm decisions, plus founding a firm)

The user re-scoped stages: stage 2 is now "the firm decisions" and the household and bank decisions are the next stage. The table in section 11 is the old plan; this section is what exists.

**Decisions (9 registered in all: 3 labour from stage 1, 6 new).** `firm_price` (goods.rs `decide_price`), `firm_green` and `firm_capex` (investment.rs: the green step, then next month's capital purchase and the loan request that funds it), `firm_payout` (income.rs, the dividend), `firm_exit` (credit.rs, default after months of missed payments), `household_found_firm` (class.rs, the go or no-go for founding in the dormant slot). Quantity split: `output_capacity` is gone; the hire view carries `production::CapacityFactors` (capital_output, farm_health, climate, drought, community, smoke, transition) and the quantities `capital_output`, `weather_factor`, `resource_factor`, `transition_factor` (DP-2 closed: the graph now has weather -> hire).

**New `QuantityId`s (36 added, 1 removed).** price, markup, unit_cost, footprint, carbon_price, revenue, emissions_cap, green_subsidy, treasury_cash, cash, cash_reserve, green_tech, green_plan, profit, profit_trend, capital, capital_per_sale, loan_rate, distressed, invest_hurdle, green_cash_share, investment_plan, loan_request, payroll, debt_service, payout_ratio, dividends, debt, payment_shortfall, distress_months, firm_open, sector_return, base_rate, random_draw, new_firm (plus the four weather and resource quantities above). Quantities for the economic system's settings (`invest_hurdle`, `green_cash_share`, `payout_ratio`) are inputs because those values come from the selected system (`cfg(s, p)`), which is not a path in `params.json`.

**Content.** `content/decisions.json`: owner `household` -> Agents kind `households`; 6 decision entries; 35 quantity entries; 20 parameter entries. Hand-written lines for these decisions were removed from `content/assumptions.json` (firms: price, invest and borrow, unit cost, `goods.price_adjust`; households: founding a firm).

**Conventions settled.**
- A decision is `decide_x(&View, &ParamsStruct) -> Choice`; its builder is `x_view(s, p, f, ...)`. Where `run` computes an intermediate that the view needs (cash after the green step, capital after depreciation and this tick's purchases, wages paid, the interest due), `run` passes it to the builder as an argument; the builder gate feeds these from the state's own fields.
- The RNG draw is a plain `random_draw` field of the view (`FoundView`); `class::run` still draws it first, always. `decide_found` returns only the chance test; who may found (a Worker or Manager household with the savings) and which slot are matching, in `run`.
- `decide_exit` returns the new distress count and whether the firm closes; who gets the residual cash, the write-off and the workers losing jobs are accounting and stay in `run`.
- The views are plain `Copy` structs; the shared per-decision `check_view_only` helper now lives in `crates/sim-model/src/decision_tests.rs` (labour's tests use it too).
- The builder gate is `tests/decision_builders.rs` (registered as a test of `sim-engine`). It compares the view and the choice, perturbs every numeric or boolean leaf of `State` and of `params.json`, over 3 states of a seed-42 run and two variants (every sector has unmet demand; a carbon price with rich firms), about 6 seconds. It ran empty first and reported every read; the `reads()` table is that output curated.

**Gate results.** `statehash` (3 seeds x plans, 18 lines) byte-identical before and after (`cmp`); baselines A-F identical (`cmp` of the full output); `cargo test --workspace` 268 passed, 0 failed; fmt and clippy `-D warnings` clean. The golden changed only in the `agents` and `catalog` wire hashes; the per-turn state hashes did not change.

**Not extracted here (see `docs/known-issues.md`).** Production and capacity are physics, not a decision (the capex plan is the capacity decision); `firm_exit` has no `State` builder (its view is built from this month's payments inside `credit::run`).

## 13. As built (stage 3: household and bank decisions)

**Inventory check.** The inventory (section 2) was right about what exists; line numbers had drifted, the lobbying parameter section is `lobby` (not `lobbying`), the bank "lend" row is really five decisions (room, approval, rate, payout, rescue), and the smoothing of demand into expected sales (the one edge from households to hiring) was missing: it is now `firm_expect`. Section 2 rows 9, 10, 12 and 13 are corrected in place.

**Decisions registered (9 new, 18 in all).** `household_spend` (goods.rs: need first, then propensity and wealth, split into goods and services), `household_shop` (the stock and price weighted split; firms buying capital goods and the government use it too), `firm_expect` (smoothing of demand into expected sales), `household_lobby` (lobbying.rs), and the bank's `bank_headroom` (equity over the capital ratio, less loans), `bank_lend` (request within room and leverage cap), `bank_spread` (rate by leverage), `bank_dividend` (equity above the buffer), `bank_rescue` (bailout or depositor haircut when equity is negative). The graph now has `household_spend -> consumption budget -> household_shop -> demand -> firm_expect -> expected sales -> firm_hire`, and `firm_capex -> loan request -> bank_lend -> loan approved`, `bank_headroom -> lending room -> bank_lend`, `bank_spread -> loan rate -> firm_capex`.

**New `QuantityId`s (26).** savings, last_disposable, net_wealth, food_price, food_need, household_size, spending_propensity, wealth_spending_rate, consumption_budget, demand_units, units_sold, capital_income, campaign_cap, lobby_spend, bank_equity, bank_loans, bank_deposits, capital_ratio, credit_headroom, firm_assets, loan_approved, credit_spread_add, bank_dividend, policy_rate, bailout_policy, bank_failed. (`base_rate` stays the founder's cost of borrowing; the bank's input is `policy_rate`.) The system's extra credit spread (`cfg`) is an input quantity, like the stage-2 settings.

**Content.** Owner row `bank` -> Agents kind `bank` in `content/decisions.json`; 9 decision entries, 26 quantity entries, 16 parameter entries. The hand-written decides, observes and params of the Households and Bank kinds, the expected-smoothing parameter of Firms, and the lobbying line and `lobby.spend_rate` of Lobbyists were removed from `content/assumptions.json` (the Lobbyists kind keeps one line pointing to Households).

**Conventions added** (how-to section 27, step 4b): a list decision borrows slices through `ShopInputs`/`ShopView`; class-dependent parameters are chosen by the builder and enter as quantities; a not-applicable factor is exactly 0.0 or 1.0; `run` keeps the bank's running equity, loans and headroom and passes them to the builders.

**Gate results.** `statehash` (18 lines) byte-identical (`cmp`); baselines A-F identical; `cargo test --workspace` 277 passed, 0 failed; fmt and clippy `-D warnings` clean; web tests, typecheck, lint, build, check:contract (no wire type changed) and check:recording pass. The golden changed only in the `agents` and `catalog` hashes. The builder gate gained a bank-with-negative-equity variant; `household_spend` declares what `household_wealth` reads (`WEALTH`).

**Not extracted (see `docs/known-issues.md` DP-5 to DP-9).** `excess_demand` (beside expected sales), the household loan and deposit interest and the government's debt service (mechanics), the `apportion` of a firm's revenue to buyers, and the accounting of a default.

## 14. As built (stage 4: government, estate and nature)

**Inventory check.** Section 2 rows 14 to 17 were corrected in place. Row 14 is two decisions (fund, then cut), not one. The `gov.*` legitimacy parameters are `government.legit_*` (eight, including the wellbeing neutral point). Government spending by purpose was read for hidden choices and has none (DP-10): every payment is a lever or content figure times a head count, so it is policy execution and stays plain code. The estate row is two decisions (pilot, flight) and the pilot's pay and the carbon charge stay accounting (DP-11).

**Decisions registered (6 new, 24 in all).** `government_fund` (borrow the gap within the bank's lending room, or repay a share of a surplus; the room comes from `decide_headroom`, so the graph has `bank_headroom -> lending room -> government_fund`), `government_cut` (one pro-rata share for every committed payment), `government_legitimacy` (the existing step, now `decide_legitimacy(&LegitView, &GovParams)`; `legitimacy_step` is a wrapper that `breakdowns.rs` still calls), `estate_pilot` (keep, let go, hire: `PilotChoice`), `estate_flight` (draw below the monthly chance and a pilot on the payroll), `nature_enclosure` (open commons, chance above zero, draw below it; the richest Capitalist or Rentier household is matched in `run`).

**New `QuantityId`s (21).** committed_spending, next_month_purchases, treasury_taxes, public_debt, borrowing, repayment, spending_cut_ratio, legitimacy, wellbeing, inequality, public_debt_ratio, bailout, transition_resistance, pilot_employed, pilot_unpaid_months, pilot_free, estate_savings, flight, enclosure_chance, commons_open, commons_enclosed. Differences from the section 11 plan: `public_debt` and `pilot_free` were added (public debt differs in meaning from a firm's `debt`; the rehire test needs the pilot's freedom), `estate_savings` is the estate household's deposits, and `unemployment` is reused from firm wages. `pilot_employed` in the flight view is true also when the content has no pilot.

**RNG.** `estate::run` and `nature::run` take `r.next_f64()` (the one draw `r.chance` made, same order: estate first, nature last) and pass it as `random_draw`; the comparison with the chance moved into the decision.

**Content.** Owner rows `government`, `estate`, `nature` (each its own Agents kind); 6 decision entries, 21 quantity entries, 14 parameter entries. Removed from `content/assumptions.json`: the government's two borrowing and funding lines, its observes and its legitimacy parameters; the estate's pilot line, flight chance and wage parameters. Kept by hand: the government's and nature's "nothing on its own" lines, the estate's pay-in-full line, nature's observes and enclosure chance parameter, every `rule` string.

**Gate results.** `statehash` byte-identical, baselines A-F identical, workspace tests pass, fmt and clippy clean. Golden changed only in the `agents` and `catalog` hashes. The builder gate gained an indebted government, an estate whose pilot is due to be let go, and a pilot the estate can rehire; the system and transition leaves are declared and dormant (DP-12).

**Not extracted.** Spending by purpose (DP-10), the pilot's pay and the flight's carbon charge, who encloses the commons (DP-11), tax collection and debt service (DP-10).

## 15. As built (stage 5: every kind generated, the causal graph on the wire)

**Agents text.** `crates/sim-api/src/decisions.rs::merge_into_kind` now fills every kind: `decides` from each decision, `observes` from the labels and summaries of the inputs, `params` from the specs (live values) and `rule` from the decisions' own rule texts (`content/decisions.json`, one place per decision; the kind's `rule` is the decision titles and rule texts, one per line). It returns an error (so `catalog()` and `agents()` fail) if `content/assumptions.json` still holds hand-written `decides` for any kind, or `observes`, `params` or `rule` for a kind that owns registered decisions. Removed by hand from `content/assumptions.json`: every `decides` line, and for households, firms, bank, government, estate and nature the `observes`, `params` and `rule`. A kind with no decision (Lobbyists, Weather) must have a documented note.

**Documented exceptions (what is still authored, and why).**
- `content/decisions.json` `kind_notes`: per Agents kind, sentences served as the new `AgentKindInfo.notes` ("Not decided here" in the explorer). Government (policy execution and tax collection are not decisions, DP-10), estate (the pilot's pay in full and the carbon charge are accounting, DP-11), nature (it is not an agent, what it responds to, who encloses the commons is matching), Lobbyists and Weather ("nothing is decided here", why). This replaces the old hand-written "Nothing: ..." decides lines, which are no longer in `decides`.
- Lobbyists and Weather keep their own `observes`, `params` and `rule` in `content/assumptions.json`: they are mechanisms with no `DecisionSpec`, so nothing can generate them. The test pins that list (`lobbyists`, `weather`).
- Nature's enclosure chance is now in `NATURE_ENCLOSURE.params` (`systems.market.enclosure_prob`, labelled "Market system"; the other systems have their own value, read through `cfg`). The estate's `estate.flight_tonnes` is no longer a live parameter row (the flight's emissions are accounting, not read by the decision); the estate note names it without a number.
- Nothing else is hand-written in the kinds' decides, observes, params or rule.

**Test.** `crates/sim-api/tests/decisions.rs::agents_who_decides_is_generated_and_not_hand_written`: every registered decision is in exactly one kind (its `decides` line once, its rule text in the kind's `rule`, its params and input labels listed), the authored file has empty `decides` everywhere and empty `observes`, `params`, `rule` for kinds with decisions, the kinds without a decision are exactly lobbyists and weather and have notes, and the `decides` lines of all kinds add up to the registry (24). `every_quantity_has_label_unit_owner_and_text`: all 99 quantities have a label, unit, owner kind and text (`content/decisions.json` already required them; the test makes it explicit).

**Causal graph.** Pure Rust in `crates/sim-model/src/causal.rs` over `DECISIONS`: `nodes()`, `edges()` (reads quantity -> decision, writes decision -> quantity, then derived `drives` quantity -> quantity via the decision that reads one and writes the other), `role(q)` (source: no decision writes it; sink: written but read only by the decisions that write it; derived otherwise), `has_self_loop(q)`, `components()` (weakly connected, numbered in node order) and `causes_of(q, max_depth)` (the upstream quantities and decisions, depth = decisions crossed, cycle-safe). Order is stable (quantities in `QuantityId::ALL` order, then decisions in registry order; no hash maps). The wire (`catalog().causal`, contract 2.8.0) adds labels and the role words from content: `{upstreamDepth, roles, nodes, edges}`. A node is `{id, kind, role, refId, label, owner, selfLoop, component, upstream}`; ids are `quantity:<id>` and `decision:<id>` because a quantity and a decision can share a bare id (`bank_dividend`). An edge is `{from, to, kind, via, selfLoop}`. `upstream` (the quantity ids that can change a quantity, up to `causal.upstream_depth` = 3 decisions back, from `content/decisions.json`) is the exposed `causes_of`.
- Counts (verified by running the test): 24 decisions, 99 quantities, 331 edges, 65 source nodes, 21 sink nodes (the other 13 quantities are derived).
- Self-loops: a decision that reads and writes the same quantity (legitimacy, workers, base wage, price, months overstaffed, profit trend, expected sales, pilot on the payroll, ...) has both its `reads` and `writes` edge flagged `selfLoop`, and a `drives` edge from the quantity to itself. A quantity only its own writers read (legitimacy) is a sink with `selfLoop`.
- Tests: `sim-model/src/causal.rs` (every edge comes from a spec, every spec input and output has its edge, every drives pair exactly once, no dangling nodes, determinism, roles, self-loops, causes with depth and cycles) and `sim-api/tests/decisions.rs::causal_graph_matches_the_registry` (the wire is the registry joined with labels, ids unique and in registry order, no dangling ids, source and sink markers, no collision between a quantity and a decision named alike).

**Web.** `web/src/panels/explorer.ts::causalLines` reads `catalog().causal` only (nothing names a decision or a quantity): under each decision the explorer prints "Reads: ... Writes: ..." with "(Starting point)", "(End point)" and a loop mark, and, for each written outcome, "Causes of X (up to 3 decisions back): ...". `web/tests/agents.test.ts` proves a decision and edges added only to the catalogue appear with no web change. The explorer also shows "Not decided here" (the notes). It does not draw a graph.

**Gate results.** `statehash` (18 lines) byte-identical (`cmp`); baselines A-F (20 seeds) byte-identical (`cmp`); `cargo test --workspace` 291 passed, 0 failed; fmt and clippy `-D warnings` clean; web 506 tests passed, 12 skipped, typecheck, lint, build, check:contract and check:recording clean; golden changed only in the `agents` and `catalog` hashes (run twice); real-simulation Playwright (`web/playwright/decisions-shots.mjs`, 1440 wide) for Firms, Government, Estate, Nature and Lobbyists, no console errors. Contract 2.8.0 is additive (no `SCHEMA_VERSION` bump).

## 16. Future: causality graph (stages 0 and 1 of `docs/features/causality-graph.md` are built: live values per quantity, contract 2.9; mechanism specs for the plain code, contract 2.10)

Not built: the graph UI. What it can take from the registry today: nodes (quantities with owner and role, decisions with owner), edges (`reads`, `writes`, derived `drives`), source and sink markers, self-loop flags, components, and `upstream` for each quantity. A player-facing graph "why did X happen" would pick a quantity, take its `upstream` and the `drives` edges among them, and lay them out (the layout and any live values belong in the web; no rules).

What it still needs, and does not have:
- **Sign, strength and lag.** An edge says "reads", not "more of a raises b". A later `DecisionSpec` field per input (sign, and a lag in months) would be authored in `content/decisions.json`; strength would need to be measured (a perturbation sweep like the builder gate) and is not available.
- **Live values.** The graph is the static structure; showing "unemployment is 14%" needs the snapshot value of a quantity, and not every `QuantityId` has a snapshot field (many are internal to a month).
- **Granularity gaps (stage 1 closed most of them).** DP-2 is closed for output (weather, resource, transition factors, each written by a mechanism). "Starting point" now means "nothing in the registry writes it" (a decision or a mechanism, `MechanismSpec` in `MECHANISMS`, plain code declared next to itself, `docs/features/causality-graph.md` section 12): 57 of 129 quantities remain sources (levers, building-set values and the credit book, public finance and private accounting not yet described), down from 65 of 99. Emissions -> weather -> output -> hiring is connected. See DP-13.
- **Gate gaps.** The builder gate does not perturb the system files, enums or RNG (DP-12), so the dormant reads (transition resistance, enclosure chance) are proven only by the per-decision tests.
- **Cycles.** Several quantities are read and written by one decision (stocks that carry over a month); the graph is a loop over time, not a DAG. A view must unroll it (this month, last month) or hide the loop.

