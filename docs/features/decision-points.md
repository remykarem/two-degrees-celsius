# Feature: explicit decision points (refactor, queryable, causal-graph ready)

Status: **APPROVED (all section 8 defaults). Stage 1 (the labour pilot) IMPLEMENTED; stages 2-5 under way, amended by section 11 (the pilot's lessons).** See section 10, "As built (stage 1)" and section 11. Written from `crates/sim-model/src/*.rs` at HEAD (`7107499`). Where this page and code disagree, code wins. Claims not checked in code are marked **(unverified)**. First step toward `docs/features/firm-agency.md`.

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
| 9 | Household | Spending split (food, goods, services) | goods.rs:255-273 | savings, last disposable, class, wealth | three budgets | `goods.propensity.*`, `food_need_units_pc`, `services_share` | no |
| 10 | Household | Shop choice | goods.rs:23 (`allocate`) | stock, price per firm | budget split | `goods.price_elasticity` | yes (private) |
| 11 | Household | Found a firm | class.rs:138-165 | savings, sector return, base rate, RNG | new firm | `class.mobility_rate`, `startup_cost_usd` | no |
| 12 | Household (capital class) | Lobby spend | lobbying.rs:24-56 | capital income, `campaign_cap` | fund, `net_lobby` | `lobbying.spend_rate` | no (`expected_effective` yes) |
| 13 | Bank | Lend to firms | credit.rs:229-250 | requests, headroom, leverage | approved loans | `credit.max_firm_leverage`, spread params | no |
| 14 | Government | Fund spending (cash, tax, borrow) | government.rs:203-225 | cash, taxes, bank headroom | borrowing, pro-rata cuts | levers | no |
| 15 | Government | Legitimacy | government.rs:251, `legitimacy_step` :271 | gaps, bailout | legitimacy | `gov.*` **(unverified names)** | step yes |
| 16 | Estate | Flight, pilot dismissal | estate.rs:26-67 | RNG draw, pilot pay | flight, jet | `estate.flight_prob_monthly` | no |
| 17 | Nature (non-agent) | Commons enclosure | nature.rs:148-160 | system chance, RNG, richest capitalist | access rule | system `enclosure_prob` | no |

Not decisions (rules of physics or accounting; stay plain code): output (production.rs `capacity`, `cobb_douglas`), emissions, weather draws (weather.rs, own RNG stream), wellbeing, rent and eviction (goods.rs `housing`, :82). Government spending allocation by purpose (government.rs:55-200) is policy execution of player levers; whether it holds hidden choices is **(unverified)**.

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
