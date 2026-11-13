# Feature: explicit decision points (refactor, queryable, causal-graph ready)

Status: **APPROVED (all section 8 defaults). Stage 1 (labour pilot) in progress.** Written from `crates/sim-model/src/*.rs` at HEAD (`7107499`). Where this page and code disagree, code wins. Claims not checked in code are marked **(unverified)**. First step toward `docs/features/firm-agency.md`.

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
