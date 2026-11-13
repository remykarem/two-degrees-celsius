# Causality graph (the "Why?" view)

Status: **APPROVED (all open-question defaults).** Stage 0 (coupling first) DONE, contract 2.9 (section 11 "As built"); stage 1 (mechanism specs) DONE, contract 2.10 (section 12 "As built"); stage 2 (sign and lag on every spec, the perturbation sign test) DONE, contract 2.11 (section 13 "As built"); stage 3 (loops: SCC and R/B, replacing the hand-written loop table) DONE, contract 2.12 (section 14 "As built"); stages 4 and 5 not built. Builds on `docs/features/decision-points.md` (sections 15 and 16: the registry, `crates/sim-model/src/causal.rs`, `catalog().causal`, contract 2.8). Claims marked *(unverified)* were not checked against code or a running build; sizes marked *(npm)* come from npm registry metadata, not a bundle build.

## 1. Goal and hard rules

The player picks a quantity (unemployment, price, emissions) and sees its **causes as a graph**: `a -> b -> e`, `d -> e` (several causes converging), each edge saying which way it pushes ("if A goes up, B goes down"), and which parts are feedback loops. The user is new to economics, so every edge has a plain sentence.

- **Thin UI** (`docs/town-v6.md`, AGENTS.md web rule 8): the web lays out and draws; sign, lag, loops, depth, live values and sentences come from Rust and `content/`.
- **Coupling first**: where causal information is written twice, or in a place not tied to the code that makes it true, fix that before drawing anything (section 6).
- Determinism rules apply to any new Rust: no `HashMap`, `libm`, no RNG, no clock, `Cents` for money. State hash must stay byte-identical in every stage (section 9).

## 2. Visual conventions (question 1)

| Convention | What it shows | Fit here |
|---|---|---|
| Causal loop diagram (CLD): arrows with `+` / `-`, loop labels R (reinforcing) and B (balancing) | Direction of effect and feedback | **Adopt the vocabulary**: edge polarity and R/B badges. It is what the original design's loop table used |
| Stock-and-flow | Accumulations (savings, forest) and rates | Too heavy for a beginner; use only as a node icon (stock vs flow) later |
| DAG / influence diagram | Causes of one outcome, no cycles | **Adopt the layout idea**: causes left, effect right, layered |
| Layered "why tree" | Depth = steps from the effect | **Adopt**: depth control 1 to 3, already `causal.upstream_depth` = 3 |
| Sankey | Quantities flowing (money, tonnes) | Wrong tool for cause and effect; keep for Finance (exists) |

**Recommendation:** a layered DAG of the chosen quantity's causes (CLD notation on edges, R/B badges on loops). Cycles are shown by marking the loop, not by drawing the graph as a ring (decision-points section 16 "Cycles": the model is a loop over time).

Polarity must never rely on colour alone: `+` edges are solid with a "+" label and an up-up glyph; `-` edges are dashed with a "-" label and an up-down glyph; unknown sign is dotted with "?".

## 3. Libraries (question 2)

Web side, layout and drawing. Sizes are `unpackedSize` from the npm registry *(npm)*; gzip is only known for dagre (bundlephobia: dagre 0.8.5, 81 kB min, 25.7 kB gzip, *(unverified for 3.x)*). Current runtime deps: `three` only (`web/package.json`).

| Option | Unpacked *(npm)* | Svelte 5 fit | Accessibility | Verdict |
|---|---|---|---|---|
| elkjs 0.12.1 | 8.2 MB (large bundle, worker advised) *(unverified min size)* | Framework-free; you render | You render, so yours | Best layered layout, far too heavy for ~100 nodes |
| dagre 0.8.5 / @dagrejs/dagre 3.1.3 | 0.8 / 1.4 MB, ~26 kB gzip | Framework-free; you render | Yours | Good layered layout, small. Unmaintained 0.8.5; the scoped fork is current |
| d3-dag 1.3.0 | 0.5 MB, pulls an LP solver | Framework-free | Yours | Sugiyama layouts, heavier than needed |
| d3-force 3.0.1 | 55 kB | Framework-free | Yours | Force layout is non-deterministic in feel and gives no left-to-right layers. Wrong for "causes fan in" |
| cytoscape 3.35.0 | 5.8 MB | Canvas, own DOM; poor fit | Weak (canvas) | No |
| @xyflow/svelte 1.7.1 | 0.33 MB, peer `svelte ^5.25` | Native Svelte 5 | Keyboard focus on nodes built in *(unverified)* | Good interaction, but no layout (needs dagre or elk anyway), a full app-style canvas for a read-only view |
| Hand-rolled SVG, small layered layout | 0 deps, about 150 lines *(estimate)* | Plain Svelte | Full control: real `<button>` nodes, `<title>`, text twin | **Recommended** |

**Recommendation:** hand-rolled SVG with a small layered layout in `web/src/panels/` (rank = depth from the focus node; order within a rank by input order; no crossing minimisation beyond a barycentre sweep). The graph is small (depth 3, about 5 to 25 nodes shown, ~100 in the catalog), read-only, and must be accessible and testable as pure functions. If edge crossings become unreadable, swap in `@dagrejs/dagre` behind the same `layout(nodes, edges) -> positions` function; nothing semantic changes. Module boundaries: the layout is presentational and lives in `panels/` (Svelte and pure TS only); `scene/` (Three.js only) is not involved; no new import crosses a boundary.

## 4. Graph analysis in Rust (question 2, second half)

| Option | Gives | Cost | Verdict |
|---|---|---|---|
| petgraph (0.8.3 *(crates.io)*) | SCC (Tarjan), toposort, reachability, cycle detection | New dependency in `sim-model`; its default hasher use must be checked against clippy `disallowed-types` (HashMap) *(unverified)*; uses `HashMap` inside some algorithms *(unverified)* | Not needed: the graph is ~100 nodes, 331 edges |
| Hand-rolled over `causal.rs` (`Vec` edges) | Same, about 60 lines for Tarjan SCC and DFS | Zero deps, `BTreeSet`, stable order, already how `causes_of` works | **Recommended** |
| Autodiff crates (`autodiff`, `num-dual`) as graph representation | A computation graph | See below | No |

**What an autodiff graph is.** Reverse-mode autodiff records every arithmetic operation of one run as a *tape*: nodes are scalar operations (`mul`, `add`, `exp`), edges are operands. It answers "d output / d each input, at this point". It is the wrong structure for the player's graph: (1) nodes are arithmetic, not economic quantities, so a player sees thousands of `+` and `*` nodes; (2) the tape is one numeric run, not the model's structure; (3) it records only the path taken, so a branch not executed (a `min`/`max` clamp, an `if`) leaves no edge.

**Could dual numbers supply sign and strength automatically?** Forward-mode (dual numbers, e.g. `num-dual`) on the pure `decide_*` functions would give the exact derivative of an output with respect to an input at the current state, so sign and strength in one pass. Obstacles *(from the code and AGENTS.md; the decide_* signatures are not all f64 in, f64 out, unverified)*:

| Obstacle | Effect on dual numbers | Effect on finite differences on the real code |
|---|---|---|
| Clamps, `min`/`max`, thresholds | Derivative is 0 on the flat side; sign can be 0 at the point sampled | Detected by choosing the base state and step; flat side reads as "no effect here" (honest) |
| Discrete choices (hire or not, which shop, enum) | Derivative undefined | The gate already compares discrete `choices()` (`tests/decision_builders.rs`) |
| `Cents(i64)` integers | Not differentiable; a rewrite to a generic scalar touches money rules | Perturb in cents, read in cents; no change to code |
| `libm` calls (`exp`, `pow`) | Needs a dual-aware version, breaks bit-identity unless carefully mirrored | None |
| RNG | Out of scope | Fix the seed; identical draws |
| Determinism and hash | Generic scalar types in hot code risk changing float order | None: the test calls the real code |

**Recommendation:** do **not** put autodiff in the simulation. Measure sign (and later strength) by **finite-difference perturbation on the real `decide_*` code**, in a test, the same way `builders_read_only_what_they_declare` already perturbs `State` and `Params`. It tests the shipped code, and needs no code change.

## 5. Where polarity, lag and strength come from (question 3)

Authored **on the spec, next to the code** (coupled), with words in content:

```rust
// sim-types DecisionSpec, additive; one entry per input -> output pair
pub effects: &'static [Effect],   // Effect { from: QuantityId, to: QuantityId, sign: Sign, lag_months: u8 }
// Sign: Up (more from -> more to), Down, Unknown
```

| Item | Source | Where |
|---|---|---|
| Sign (`+`/`-`) | Authored in the `DecisionSpec` | `sim-model`, next to the `decide_*` |
| Lag in months | Authored in the spec; lead times of player levers already exist in `levers.json` and must be read, not retyped | spec; `content/levers.json` |
| Edge label and "because" | Template in `content/decisions.json`, filled in Rust | content |
| Strength | **Measured later**, a perturbation sweep (elasticity at baseline states), served as a number; never authored | test or tool, then wire |
| Path sign | Product of edge signs (computed in Rust) | `causal.rs` |
| Loop type | SCC; a loop is **B** if it has an odd number of `-` edges, else **R**; computed in Rust, served on the wire | `causal.rs` |

**Verification test (new, builder-gate style).** For every declared `Effect`, build states (the gate already makes `variants()`), nudge `from` up by a small step through the real builder, call the real `decide_*`, and read `to`. If the sign of the change disagrees with the declared sign in every variant where it moves, the test fails. Rules: a flat response is allowed (clamped), a response with both signs fails unless the effect is declared `Unknown`, and a declared sign never measured as moving in any variant fails (dead edge). Discrete outputs compare the choice (hire/no hire) as 0 or 1. Same machinery as DP-12, so it inherits DP-12's blind spot (system files, enums, RNG are not perturbed).

## 6. Coupling-first audit (question 4)

Duplicated or decoupled causal information today, the single source that should replace it, and whether it blocks the graph UI.

| # | Where it lives now (verified) | Problem | Single source | Before the UI? |
|---|---|---|---|---|
| C1 | the original hand-written loop table (removed from `docs/causal-model.md`; findings in section 14) (9 rows; one says it is "NOT MODELLED") | A prose copy of causal structure; drifts silently | Loops computed from the registry (SCC, section 5) and served; the doc table is replaced by a pointer. Loops outside the registry stay listed as such until C5 | Yes, for the R/B badges; the doc edit can follow |
| C2 | `snapshot.breakdowns` (`crates/sim-api/src/breakdowns.rs`): per headline a `formula` string, `inputs` and `parts`, built by hand; popover in `web/src/panels/explain.ts` | Inputs and formulas are a second statement of "what drives X", unconnected to the decision specs | Keep breakdowns as the arithmetic of a number; derive the *list of causes* from the graph, and link a breakdown input to its graph node by `quantity` id (add an optional `quantityId` to `BreakdownInput`). The graph does not replace the formula | Yes (the id link), else two causal explanations disagree |
| C3 | Markets cause words ("cost-driven", "scarcity-driven") and `drivers` in `content/ui.json` `markets`; inflation decomposition rows (`docs/features/inflation.md` section 4) | Cause words are authored per market, not read from edges | Each driver names a graph `quantity` id; the cause word comes from the edge or the path; the inflation decomposition is arithmetic and stays | No, but add the id so Markets cards can link "why?" |
| C4 | `content/assumptions.json` topics (68 `id` entries *(grep count)*) and `decides`/`observes` text | Already generated for decisions (stage 5). Topic prose still repeats rules in sentences | Only the Lobbyists and Weather exceptions remain hand-written; they become mechanism specs (C5) | No |
| C5 | DP-13: production, emissions, wellbeing, rent, tax collection, matching are plain code, so 65 "source" nodes include state they write and chains break (emissions -> weather -> output) | A graph with broken chains teaches wrong causes | Register **read-only "mechanism" specs** (`MechanismSpec` in the same `DECISIONS`-style registry: inputs, outputs, effects, no `decide_*`, no behaviour change) for production, emissions, weather, wellbeing, rent, tax collection, matching. They are declarations plus the sign test only; the real code is untouched | **Yes** for the main chains (emissions, output, wellbeing, price); the rest can follow |
| C6 | "Why is this changing?" (`web/src/panels/WhyModal.svelte`, `WhyView`, `why.ts`) shows the turn's `lastResult.trace` as bars | Not wrong (it is "what changed", from the trace), but it presents itself as the answer to "why" and shares no ids with the graph | Keep; relabel as "What changed this turn"; the graph is "What drives it". Link a trace record to a quantity id | No (naming only) |
| C7 | DP-2: `output_capacity` is one quantity for capital, technology, weather and climate | Graph cannot show weather -> hiring except through it | Split when C5 lands: mechanism specs for `production::capacity` give separate nodes | With C5 |
| C8 | Quantity id -> snapshot value: **no mapping exists** (`content/decisions.json` has labels, unit, owner, summary only; the breakdowns have their own ad hoc inputs) | Nodes cannot show live values; a web table would be a decoupled copy | A Rust function `live_value(&State, QuantityId) -> Option<f64>` next to the graph, `None` for internal-only quantities; the wire sends `value`, `unit`, `trend` per node | Yes |

## 7. Rust vs web (question 5)

| Concern | Rust (`sim-model`, `sim-api`) and content | Web (`panels/`) |
|---|---|---|
| Nodes, edges, sign, lag, strength | yes | no |
| Upstream/downstream sets, depth limit (`causal.upstream_depth`) | yes | passes the depth the player picked back as a request |
| Path sign, loop detection, R/B | yes | no |
| Live value, unit, trend per node | yes (`live_value`, section 6 C8) | formats with `fmtUnit` (existing) |
| Edge sentence ("when X goes up, Y goes down because ...") | yes: template in `content/decisions.json`, filled in Rust | prints it |
| "What would change this?": levers upstream | yes: nodes whose role is a lever, with the lever id from `levers.json` | links to the lever |
| Layout (ranks, order, coordinates), pan/zoom, focus, animation | no | yes (presentational only) |
| Glyph shape for `+`/`-`/`?`, colours, responsive layout | no | yes |

## 8. UI sketch (question 6)

```
+-- Why is [Unemployment 14%  (rising)] ?  ------------- Depth [1][2][3]  [x] -+
|                                                                               |
|  causes (layers, left to right)                              effect           |
|                                                                               |
|  [Wages 52/wk  v] ----(-)----> [Hiring    ^] ---(-)---> +--------------+     |
|                                  ^  (R) loop badge       | Unemployment |     |
|  [Demand  v]  ----(+)-------> [Sales  v ] ---(+)--+      |   14%  ^    |     |
|                                                    +----> +--------------+     |
|  [Carbon price $40]---(-)--> [Profit  v] ---(+)--> Hiring                      |
|                                                                               |
|  Legend:  --(+)-- same direction (solid)   --(-)-- opposite (dashed)  R / B   |
|  Hover or focus an edge:                                                      |
|  "When Wages go up, Hiring goes down, because dearer workers cut profit. ~2 mo"|
|                                                                               |
|  What would change this?   [Carbon price lever] [Job guarantee] [Wage floor]  |
|  [ Show as list ]                                                             |
+-------------------------------------------------------------------------------+
```

| Aspect | Design |
|---|---|
| Layout | Focus quantity at the right; causes fan in from the left by depth (1 to 3, default 2) |
| Edges | `+`/`-` as label and line style (solid/dashed) and colour; "?" dotted where sign is unknown; lag as a small "~2 mo" |
| Loops | R and B badges on the nodes of a strongly connected group; a tooltip explains "reinforcing: it feeds itself" in plain words (content) |
| Nodes | Label, live value, trend arrow with a word ("rising"); click re-centres on that node (back button keeps history) |
| Broken chains | A node that is a source because no code registers it shows "starting point (not explained further)"; honest until C5 |
| Entry points | A "why?" beside metrics and levers (next to the existing "how?"); the Agents explorer (replace the text "Causes of X" lines; keep them as the text view); Markets cards ("why is this price moving?") |
| Keyboard | Nodes are `<button>`s in rank order; Tab moves through, Enter re-centres, arrows follow edges, Escape closes; edges are focusable and reveal their sentence; `aria-live` announces re-centring |
| Text alternative | "Show as list": the same graph as lines ("Hiring: down when Wages go up (because ...)"), reusing `causalLines` in `web/src/panels/explorer.ts`; it is the accessible default for screen readers |
| Mobile / narrow | Layers stack vertically (effect on top, causes below); depth default 1; list view first |
| Performance | The view shows the focus node's upstream within depth 3, expected tens of nodes; the full catalog (99 quantities, 24 decisions, 331 edges) is not drawn at once; layout is O(n+e) per focus change *(estimate)* |
| Pressure mode | Opening the view pauses the clock like the breakdown popover (`PRESSURE_NOTE`) |

## 9. Staged delivery (question 7)

State hash must be byte-identical (`cargo run -p sim-cli --example statehash`, baselines A-F) in **every** stage. Mechanism specs (stage 1) are declarations that change no behaviour; if one ever needs a code change, it is split out and gated on its own.

| Stage | Work | Tests | Wire |
|---|---|---|---|
| 0 Coupling first | C8 `live_value` map; `quantityId` on `BreakdownInput` and Markets drivers (C2, C3); relabel "Why is this changing?" (C6); no new UI | every quantity with a snapshot value maps; breakdown `quantityId`s resolve; no dangling ids | additive (contract 2.9) |
| 1 Mechanism specs (**DONE**, section 12) | Register read-only specs for production, emissions, weather, wellbeing, rent, tax, matching (C5, C7) | `causal.rs` tests pass over the larger registry; statehash identical | additive (contract 2.10) |
| 2 Sign and lag (**DONE**, section 13) | `effects` on specs; template sentences in content | **Sign verification** (section 5); every effect has a sentence; every input-output pair has an effect | additive |
| 3 Loops (**DONE**, section 14) | SCC, R/B, path sign in `causal_loops.rs`; served on the wire | **Loop classification** on hand-built cases (parity of `-` edges); path sign is the product; determinism | additive (contract 2.12) |
| 4 Graph view | SVG layout, panel, entry points, list view, keyboard | layout pure tests; **thin-UI test**: a made-up edge and node added only to the catalogue renders with no web change (like `web/tests/agents.test.ts`); a11y checks | none |
| 5 Strength | Perturbation sweep served as a number; line thickness | sweep is deterministic and reproducible | additive |

Each stage that touches Rust or content follows the AGENTS.md checklist (fixtures, golden, recording, contract regeneration).

## 10. Risks, out of scope, open questions

| Risk | Mitigation |
|---|---|
| Sign is not constant (a clamp flips it, or two paths cancel) | Allow `Unknown`; the test reports both signs; show "it depends" in the sentence |
| Goods price ignores scarcity (MK-1), so a textbook edge would be wrong | Declare only what the code does; the graph then teaches this model honestly |
| Mechanism specs drift from plain code | The sign test runs on mechanisms too; DP-12 blind spots stay documented |
| Too many nodes overwhelm a beginner | Depth 1 to 2 by default; "important" causes first (needs strength, stage 5) |
| Sign authored wrongly and the test is weak | The test fails on disagreement; dead edges fail |

**Out of scope:** an autodiff or tape-based graph; a force-directed whole-model map; editing the graph; showing strength before stage 5; changing any game rule; daily ticks.

| Open question | Default |
|---|---|
| Library vs hand-rolled | Hand-rolled SVG; `@dagrejs/dagre` if crossings are bad |
| One registry for decisions and mechanisms | Yes, same `DECISIONS`-style registry, a `kind` flag |
| Where does "Why is this changing?" go | Stays as "What changed this turn"; the graph is "What drives it" |
| Depth default | 2 desktop, 1 mobile |
| Show unknown-sign edges | Yes, dotted, until stage 2 covers all |

## 11. Stage 0 as built (contract 2.9)

No behaviour change: `statehash` and baselines A-F are byte-identical before and after; the golden changed only in snapshot, breakdown, catalog and agents hashes (the wire grew).

| Audit item | Done | Where |
|---|---|---|
| C8 live value per quantity | **Yes.** `sim-api/src/live.rs`: `binding(QuantityId)` is an exhaustive match, so a new quantity is a compile error until it is bound (a snapshot path, an aggregate, an optional lever, an optional history series for a trend) or given an `Unbound` reason. `snapshot.quantities: QuantityLive[]` (`id, value, previous?, trend?`) serves the values of the bound quantities that have one now; `catalog().quantities[].live` (`bound, path?, aggregate?, lever?, hasTrend, unbound?`) says where each comes from or why it has none; words for aggregates and reasons are `content/decisions.json` `causal.live`. Values are read from the typed snapshot, never recomputed from `State`. 35 of 99 quantities are bound; 64 are unbound with a reason (`private`, `computed`, `not_published`, `fixed`, `random`) | `crates/sim-api/src/live.rs`, `tests/live.rs` |
| C2 breakdown input -> node | **Yes**, for inputs: `BreakdownInput.quantityId?` on 11 inputs (Gini, public debt, wellbeing, unemployment, debt ratio, bailout, bank equity, loans, capital ratio, headroom). A test checks no id dangles and that the legitimacy breakdown's linked ids are inputs of the `government_legitimacy` decision. **Not done for `parts`**: parts are per firm or per household (`"Factory sales"`), not quantities | `breakdowns.rs` |
| C3 Markets | **Partly.** `MarketView.priceQuantity?` and `MarketDriver.quantityId?` (`markets::price_quantity`, `driver_quantity`, in code next to the closed id sets). The cause word (`cost-driven`) is still authored per market in `content/ui.json`; deriving it from an edge needs stage 2 (signs and lags) and was not attempted. Housing rent and electricity have no registered quantity yet | `markets.rs` |
| C6 relabel | **Yes.** The "Why is this changing?" button, window title and per-metric "why?" are now `text.misc.what_changed_*` in `content/ui.json` ("What changed this turn", "what changed?"); the word "why" is free for the graph | `WhyModal.svelte`, `MetricsPanel.svelte` |
| C1 loop table, C4 topic prose, C5/C7 mechanism specs | Not stage 0 (C1 doc replaced in stage 3, C5/C7 in stage 1, C4 with C5) | n/a |

Rules that came out of it:
- **Trend baseline.** `previous` is the end of the previous turn: when no month of the current turn has been played yet the newest history point *is* now, so the one before it is used. `trend` ids are the Markets direction ids (`rising`, `steady`, `falling`) with the same `markets.steadyBand`; the words are `markets.directions`. Only quantities whose series the history keeps have a trend (unemployment, inequality, wellbeing, legitimacy, public debt ratio, food price, loan rate). More trends need a new `HistoryPoint` field (a saved-game change).
- **Aggregates.** Firm quantities are a sum or a mean over active firms; household money is a sum; levers show the effective (lobbied) setting; yes/no is 1 or 0. `Price` and `Wage` are firm means, so they differ from the Markets sector prices; `FoodPrice` and `LoanRate` read the market cards.
- **Web.** One generic use: the Agents explorer's "Causes of X" lines add each cause's value and trend word (`panels/explorer.ts` `liveLookup`; nothing there names a quantity). A made-up quantity with a binding and a value added only in the wire renders with no web change (`web/tests/agents.test.ts`). Units of five quantities were reworded in content (`on a 0 to 1 scale`, `share of the labour force`) so the phrase reads after a number.

### Handoff for stage 1 (mechanism specs, DP-13)
- New quantities (production, emissions, weather, wellbeing, rent, tax, matching) need a `QuantityId` variant, a `content/decisions.json` entry **and a `live::binding` arm** (the compiler forces it). Bind to an existing snapshot field where there is one (`nature.grossEmissionsMonthly`, `weather.stress`, `housing.rentPrivate`, `indicators.*`) so the node cannot disagree with its panel; otherwise choose an `Unbound` reason, and `not_published` is the honest one for state the snapshot does not show.
- Likely binds: `Footprint`/gross emissions, `WeatherFactor` (from `weather.stress`), a rent quantity (`housing.rentPrivate`, then Markets housing `priceQuantity` can stop being `None`), `Wellbeing` per person.
- Pitfalls: the test `every_bound_path_resolves_on_a_real_snapshot_and_the_value_matches` resolves paths with `a.b`, `xs[]` (active firms only) and `xs[id]` (markets); a new path shape needs the resolver in `tests/live.rs` extended. Golden and fixtures change on every wire edit (run `UPDATE_GOLDEN=1` twice). `QuantityLive` values come from the snapshot, so a binding that reads a field the snapshot builds is order-sensitive (`quantities` is filled last in `snapshot::build`).

## 12. Stage 1 as built (contract 2.10): mechanism specs (C5, C7)

No behaviour change: `statehash` (18 lines) and baselines A-F (20 seeds) are byte-identical before and after (`cmp`); the golden changed only in the snapshot-based hashes (`init`, `import`, `cancel2`, `finalSnapshot`, each turn's `fnv`) and the catalog hash (the agents hash is unchanged; no number in the golden moved).

**What a mechanism is.** `MechanismSpec { id, system, inputs, outputs, params, rule }` in `sim-types/src/decisions.rs` (the same shape as `DecisionSpec`), registered in `sim-model::decisions::MECHANISMS` next to `DECISIONS` (one registry, two lists, so "a `kind` flag" is the node kind). Each spec is a `pub const` in the module of the plain code it describes (`production.rs`, `nature.rs`, `weather.rs`, `wellbeing.rs`, `income.rs`, `government.rs`, `goods.rs`, `labour.rs`, `investment.rs`) and **changes no behaviour**: the code is untouched, no formula or float order moved. `MechanismId::owner()` is an exhaustive match (like a decision's), so a mechanism has exactly one owner kind. `OwnerKind` gained `Weather` (it owns mechanisms and quantities only; Agents kind `weather`).

**The 39 mechanisms (owner: what).**
- Firm (production): `production_technology` (Cobb-Douglas), `technology_growth`, `capital_accumulation`, `effective_labour_supply`, `farm_health_factor` (resource factor), `climate_damage`, `drought_loss`, `fire_smoke` (three writers of the weather factor), `transition_loss`, `production_output` (capacity, rationing, emissions cap), `food_cap`, `inventory_stock`, `electricity_demand`, `firm_emissions` (direct emissions and footprint), `wage_payment`, `firm_profit`, `goods_settlement`, `unit_cost_buildup`.
- Nature: `electricity_rationing`, `grid_intensity_mix`, `plant_emissions_output`, `emissions_accounting`, `carbon_budget`, `forest_sink_uptake`, `forest_stock_change`, `commons_stock_change`, `fire_burns_forest`.
- Weather: `climate_stress_curve`, `drought_onset`, `fire_ignition`.
- Household: `wellbeing_score`, `inequality_gini`, `last_disposable_record`, `household_income_flows`, `housing_rent`, `housing_moves`, `labour_matching`, `unemployment_rate`.
- Government: `tax_collection`.

**New quantities (30; 99 -> 129).** Nature and grid: forest and commons stock, community commons, commons harvest lever, emissions so far, gross, net, direct (firm) and plant emissions, forest sink, grid intensity, renewable share, plant capacity, electricity demanded and served. Weather: climate stress, drought, fire, resilience spending lever. Others: economic system change, output, technology, household spending, private rent, homes occupied, five tax levers (profit, wage, capital, wealth, wealth threshold). Each has a `content/decisions.json` entry and a `live::binding` arm: 28 are bound (`nature.grossEmissionsMonthly`, `weather.stress`, `housing.rentPrivate`, `energy.*`, firm sums, levers), 2 are `not_published` (technology, household spending). Live-bound quantities are now 63 of 129 (35 of 99). The Markets housing card now has `priceQuantity: "rent"` (C3).

**Graph (verified by running the tests).**

| | Stage 0 | Stage 1 |
|---|---|---|
| Decisions | 24 | 24 |
| Mechanisms | 0 | 39 |
| Quantities | 99 | 129 |
| Edges (reads, writes, drives) | 331 | 654 |
| Sources ("Starting point") | 65 | 57 (44 per cent of quantities, was 66) |
| Sinks ("End point") | 21 | 14 |

`causal.rs`: `Node::Mechanism`, `Edge.via` is now a `Node` (a `drives` edge goes through a decision or a mechanism), `writers(q)` and `readers(q)` over both, `role`, `has_self_loop`, `components` and `causes_of` cover both (`Causes.mechanisms`), `role_counts()`. Depth counts steps (decisions or mechanisms crossed); `causal.upstream_depth` is still 3, so the Agents "Causes of X" lines say "steps back". Order is stable: quantities, decisions, mechanisms. Wire: `catalog().mechanisms: MechanismInfo[]` (`{id, title, owner, agentKind, system, explains, rule, inputs, outputs, params}`), nodes of kind `mechanism` (id `mechanism:<id>`, role `mechanism`), `CausalEdge.via` may be a `mechanism:` id. Words are `content/decisions.json` `mechanisms` (title, one-line `explains`, rule label and text; the rule id must equal the spec's), `quantities`, `params` (79 new labels) and a fifth role word.

**Chains now connected (named test `the_dp13_chains_connect`, `sim-model/src/causal.rs`; and on the wire `the_emissions_to_hiring_chain_is_connected_on_the_wire`).**
- Emissions -> weather -> output -> hiring: firm output -> direct emissions -> gross emissions -> net emissions -> emissions so far -> climate stress -> drought -> weather factor -> output -> inventory -> wanted workers. Also fire -> smoke -> weather factor, fire -> forest -> sink -> net emissions.
- Cost push: wage -> unit cost -> price; wage -> payroll and savings -> spending; rent -> disposable income -> spending; occupancy -> rent.
- Nature -> wellbeing -> legitimacy (forest, gross emissions, fire into wellbeing); revenue -> profit -> taxes -> the treasury's funding decision; vacancies -> workers -> unemployment; investment plan -> capital -> output.

**Still broken (and why).** 57 sources remain. Real inputs: levers (carbon price, emissions cap, subsidy, min wage, taxes, base rate, resilience, commons harvest, bailout, campaign cap, capital ratio), the system change flag, plant capacity and renewable share (set by building), a firm's markup. Not explained yet (stage 1 did not describe them): the credit book (debt, bank loans and deposits, capital ratio, credit spread, loan approved, firm assets), the treasury (cash, public debt, committed spending, next month purchases), households' wealth (net wealth, spending propensity), the Markets food price (a mean of firm prices, not a quantity the code writes), firm private working variables (cash, cash reserve, capital per sale, distress, investment hurdle, payout ratio, sector return, payment shortfall) and the estate's savings and pilot flags. Those are credit, public finance and private accounting; add mechanisms with `docs/how-to.md` section 28.

**Test: `tests/mechanism_params.rs` (parameter gate).** It perturbs every numeric leaf of `params.json` (three ways) and runs the real system function on 161 states (mid-run states plus variants: a stressed climate with drought and fires, resilience spending, a flight, a transition, a depleted commons so the food cap binds, a community and a fenced-off commons, 150 weather seeds with fire stations, housing with unpaid rent, rent at the floor and homeless households). It fails if a parameter a mechanism declares is never read by that mechanism's system (100 declared paths checked; three fire-station paths are `DORMANT` with the reason), or if a system reads a parameter that no decision or mechanism of that system declares and the committed `UNDECLARED` list does not allow it (16 allowed, each with its reason: shared helpers, event-only thresholds, money flows no mechanism describes). **Not covered:** the quantity inputs and outputs of a mechanism are *not* perturbed (a quantity is an aggregate of many fields and one system writes several mechanisms' outputs together, so there is no per-quantity read test); a parameter is attributed to the system, not to one mechanism of it; the system files (`systems/*.json`, `transition_output_penalty`), enums and the RNG are not perturbed (DP-12); random outcomes (a fire spreading across a station's cover) are proven only by reading the code. The quantity lists are checked for being well formed, unique and joined to content (`causal.rs` and `tests/decisions.rs`), not for being complete.

**Web.** `panels/explorer.ts`: `mechanismsOfKind`, `mechanismLine`, `nodeLines` (the same Reads / Writes / upstream lines for any node; `causalLines` is its decision wrapper). `ExplorerModal.svelte`: a "Mechanisms (plain code, not choices)" section per Agents kind, one line per mechanism with its rule, explanation and parameters in a details row. Nothing in the web names a mechanism; `web/tests/agents.test.ts` proves a mechanism added only to the catalogue appears. Lobbyists and Weather keep their hand-written `observes`, `params` and `rule` (C4 is not done: the weather mechanisms are now listed beside them but the old text is not generated).

**Gate results.** `cargo test --workspace` 305 passed, 0 failed; fmt and clippy `-D warnings` clean; web 513 passed, 12 skipped, typecheck, lint, build, check:contract and check:recording clean; golden changed only in the snapshot-based and catalog hashes (run twice); real-simulation Playwright (`web/playwright/causal-stage1-shots.mjs`, 1440 wide) for Nature, Weather, Firms and Households, no console errors.

### Handoff for stage 2 (sign and lag on specs, the perturbation sign test)
- Add `effects: &'static [Effect]` to **both** `DecisionSpec` and `MechanismSpec` (`Effect { from, to, sign, lag_months }`); mechanisms are registered in the same files, so one test can walk `actors()` in `causal.rs`.
- **Easy to perturb (pure functions with a view):** the 24 decisions (`decide_*`, the builder gate's `choices()` and `variants()`), and the mechanisms that are already pure functions: `cobb_douglas`, `climate_factor`, `forest_sink`, `grid_intensity_of`, `plant_emissions`, `firm_direct_emissions`, `firm_energy_per_unit`, `household_energy_use`, `drought_p`, `ignition_p`, `spread_p`, `climate_stress`, `commons_regrowth`, `forest_regrowth`, `food_cap`, `price_target`. Nudge the input through the function; no `State`.
- **Hard to perturb:** mechanisms that live inline in a `run` (`production_output`, `emissions_accounting`, `carbon_budget`, `wellbeing_score`, `goods_settlement`, `wage_payment`, `tax_collection`, `housing_*`, `labour_matching`): a quantity is an aggregate of many fields, so "nudge the quantity" has no single `State` leaf. Either extract a small pure function per mechanism first (the decision recipe, with the float order copied unchanged, gated by statehash) or test the sign on the whole system with a hand-built state (raise `nature.cumulative`, read `weather.stress`). `drought_onset` and `fire_ignition` are random: test the chance (`drought_p`, `ignition_p`), not the outcome.
- **Pitfalls.** Many quantities are levels or flows of different units; a `+` edge means "more of the input, more of the output, other things equal" and must be measured in a state where the output is not clamped (food cap, emissions cap, rationing, `min` floors). Sign is not constant for `production_output` (the cap and rationing flatten it). Self-loops (technology, capital, forest, commons, drought, fire, savings) carry sign too: a stock's loop is `+` by itself, a net flow is what flips it. `Inequality`, `Wellbeing` and `Unemployment` are town indicators written by a mechanism whose real inputs are per person; declare the aggregate inputs and say so. Two mechanisms write the same quantity (`weather_factor` has three writers, `savings` three, `output` two, `inventory` two, `workers` a decision and a mechanism): the effect belongs to the (writer, input, output) triple, which is why `Edge.via` is a node. A `drives` edge from a mechanism and one from a decision to the same quantity can disagree in sign; the loop classifier (stage 3) must treat them as separate edges, as `edges()` already does. The recorded demo and `catalog` fixture will grow again; lags: the tick order (`TICK_ORDER`) decides whether an effect lands this month or next (weather runs before production, nature after; wellbeing last), read it before authoring `lag_months`.

## 13. Stage 2 as built (contract 2.11): sign and lag, the perturbation sign test

No behaviour change: `statehash` (18 lines) and baselines A-F (20 seeds) are byte-identical before and after (`cmp`). One tiny extraction, `production::climate_factor_of(p, cumulative)` (the arithmetic of `climate_factor`, same float order), so the sign test can call it; the golden changed only in the `catalog` hash (the recording, the snapshot and every state hash are unchanged).

**What was added.**
- `sim_types::Effect { from, to, sign, lag_months, basis }` in `crates/sim-types/src/decisions.rs`, and `effects: &'static [Effect]` on **both** `DecisionSpec` and `MechanismSpec`, written next to the code, one entry per input -> output pair (the graph's `drives` edge). `Sign` is `Up`, `Down`, `Varies(reason)` (a hump, a threshold or a category: the sign depends on the state) or `None(reason)` (the spec reads the input but it has no direct effect on this output; the edge stays so the cross product of inputs and outputs is complete, and path signs skip it). Constructors are `const`: `Fx::up(Q::Wage, Q::UnitCost)`, `.down`, `.varies(.., "why")`, `.none(.., "why")`, `.lag(1)`, `.system()`, `.unverified("why")`. A test (`causal.rs`) fails if any pair of any spec has no effect, or has two, or an effect names a pair the spec does not have, or a reason is not a sentence.
- **Lag.** `lag_months` is what the spec itself adds (0: the output is written in the tick the spec runs; 1: it lands next month; only `bank_spread` "from next month", and the capital added from last month's plan, set it). The wait imposed by the tick order is **derived, never authored**: `causal::tick_wait(writer_system, reader_system)` is 0 when the writer runs earlier in `TICK_ORDER` and 1 when it runs at or after the reader (the same system included: a system reads the state as it was when it started), and `causal::edge_lag_months(edge)` adds it to the spec's lag using the earliest registry writer of the input (0 for an input nothing writes, such as a lever). So "net emissions -> climate stress" is 1 month (nature runs after weather), "expected sales -> workers wanted" is 1 (goods runs after labour), "inventory cover -> workers wanted" is 0. **Not included:** a policy lever's lead time (`content/levers.json`, the player's queue) is not added to the edges whose input is a lever; that is a stage 3 or 4 job (read `lead_time`, never retype it).
- **Wire (contract 2.11, additive).** `CausalEdge.effect?: EdgeEffect { sign, lagMonths, lagLabel, basis, reason?, sentence, checked }` on `drives` edges. The sentence is generated in Rust from the templates in `content/decisions.json` `causal.effects` and the declared sign, so it cannot disagree with it: `{effect} ({lag}), {because}.` with `effect` from the sign template ("when {input} goes up, {output} goes up"), the lag word ("the same month", "the next month", "N months later") and a because: a hand-written reason from `because_overrides` (keyed by spec, from, to; 15 so far, validated at load against the real signed effects) or, by default, the spec's rule label ("following the rule ..."). A `varies` effect gives its reason in place of a because; a `none` effect says "X is read here but does not directly change Y: reason". The hand-written overrides are the teaching sentences; extending them is content work only.
- **Rust helpers** (`causal.rs`, pure, tested): `path_sign(&[Sign]) -> NetSign` (the product: two `-` make a `+`; any `Varies` makes the path `Varies`), `paths(cause, effect, max_steps) -> Vec<SignedPath>` (every simple path of `drives` edges, skipping `none` effects, with each path's quantities, the decisions and mechanisms crossed, and its sign), `net_sign(&[SignedPath])` (`Up` or `Down` when all agree, **`Mixed`** when some paths push up and others down, `Varies` when none disagrees outright but some can go either way, `NoPath`), and `sign_of(cause, effect, max_steps)`. A `Mixed` result is the honest answer to "does more of X raise or lower Y?" when two routes disagree, and the UI must say so rather than pick one.
- **Web.** The Agents explorer shows, under each decision and mechanism, a "How each input pushes each output (n)" row: one text line per edge, `Base wage (+) -> Unit cost, the same month` (sign glyph from the wire id, lag word from the wire; `none` reads "no direct effect"), with the sentence and how it was checked in an expandable "why" (and as the line's hover title). `panels/explorer.ts` `effectLines`; no effect, quantity or word is named in the web, and `web/tests/agents.test.ts` proves a sign added only in the catalogue renders with no web change.

**The sign test (what is measured, on the real code).** `Effect.basis` says how each sign is checked, and the counts are tested:

| Basis | What it means | Where | Effects |
|---|---|---|---|
| `Perturbed` | The real pure code: every `decide_*` through its view (raise one view field, call the real function, read the output), and the mechanisms that are pure functions (`cobb_douglas`, `climate_factor_of`, `climate_stress`, `drought_p`, `grid_intensity_of`, `plant_emissions`, `firm_direct_emissions`, `forest_sink`, `food_cap`, the tax helpers, `gini`, the energy helpers) | `crates/sim-model/src/sign_tests/{decisions,mechanisms}.rs`, rules in `crates/sim-model/src/sign_probe.rs` | 220 |
| `PerturbedSystem` | A mechanism inline in a `run`: raise one field of a real mid-run `State`, run the real system of `TICK_ORDER`, apply its deltas, read one field of the result (random mechanisms: the chance, `ignition_p` and `drought_p`, not the outcome) | `tests/mechanism_signs/` (`common.rs` the states and `after`, four groups) | 106 |
| `DeclaredUnverified` | Not measurable; the reason is in the spec | five effects, listed in `docs/known-issues.md` DP-18 | 5 |

Rules (`sign_probe.rs`, one place): a sample is **clamped** (a `min`, `max`, `clamp`, floor, cap or threshold on its flat side) when the output is exactly equal before and after the nudge, and clamped samples are ignored; the bases are chosen so some samples are live. `Up`/`Down` need at least one unclamped sample and every unclamped sample must move that way; `Varies` must show both signs; `None` must never move; a signed effect that never moves is a **dead edge** and fails; a `DeclaredUnverified` effect must have no probe and a `Perturbed` one exactly one (a stale marker would hide a verified effect).

**Declared from the code and economics, not back-filled.** Where a measurement disagreed, the declaration or the code was investigated; declarations were changed only with a reason, and nothing in the simulation was changed.

Totals (all 331 pairs of the 63 specs): **178 up, 104 down, 11 varies, 38 none**. Lags: 191 edges 0 months, 139 edges 1, 1 edge 2. Cause/effect pairs within 3 steps (`sign_of`, test `mixed_sign_pairs_are_counted_and_flagged`): 714 up, 632 down, 74 varies, **211 mixed** (routes disagree, e.g. expected sales -> vacancies), 15010 with no path. Timing: the in-crate sign test runs in 0.14 s and `tests/mechanism_signs` in 4.3 s on a warm build; a warm `cargo test --workspace` took 446 s in debug (dominated by the scenario suites and the two 20 s gates `decision_builders` and `mechanism_params`, 30 s each with linking; no pre-stage figure was captured), so stage 2 adds about 5 s; an incremental sim-model rebuild after touching one file is 2.5 s.

Declarations the measurement changed (each with its reason):
- `firm_fire` over-target months -> over-target months: `+` became `varies` (the counter adds a month while the surplus lasts but resets to 0 the month the firm fires).
- `firm_green` revenue -> green tech: `varies` became `down` (revenue raises the step's cost, not the carbon bill; the only upward case is revenue 0, where the step is never worthwhile).
- `household_spend` household size and food price -> consumption budget: `varies` became `up`. Food is bought first and the discretionary part falls only by propensity times the extra need (propensity below 1), so total spending never falls. "Less left over" is true only for the goods and services part.
- `forest_stock_change` and `commons_stock_change`, stock -> itself: `varies` became `up`. The output is next month's stock level, not the net change; the logistic hump is in the change, and the level's slope is 1 + g(1 - 2x/K), positive for the shipped growth rates.
- `food_cap` commons harvest lever -> output: `up` became `down`. The cap is the commons' take cap divided by (harvest per unit of food x the lever), so a higher lever makes each unit of food take more of the commons and the cap falls.
- `electricity_rationing` energy demand -> electricity served: `+` became `-`. The quantity is `energy.served`, the share of demand the plants meet, not the amount delivered, so more demand spreads the same supply thinner (an authoring error from reading the name instead of the wire binding).
- `goods_settlement` price -> inventory: stays `none` but is `declared_unverified`: the spec holds units sold fixed, which a whole-system run cannot do (a dearer price lowers the units a fixed nominal budget buys, so stock rose in 6 of 8 samples; that belongs to the shop decision).

**Surprising findings (teaching or bug candidates).**
- **Better weather means fewer workers.** `firm_hire`: every capacity factor (weather, resources, transition, capital output, effective labour) lowers the workers wanted for the same output: a better harvest needs fewer hands. The graph shows it as `-`, which a player may not expect.
- **The commons harvest lever tightens the food cap** (down), because each unit of food then takes more of the commons.
- **Wage -> household savings can reverse in the town total.** In a cash-rich firm sample savings fell about 4,250 cents when wages rose 10%: a higher payroll raises the firm's cash reserve and its idle-cash line (six months of outgoings), so `decide_payout` holds back more dividends than the wage adds. The wage payment's own effect is `+`; the reversal is the payout decision (the probe leaves that sample out, with a comment).
- **Grid intensity is 1.0 in every state of an idle run** (the town starts with only fossil plants and nothing builds renewables), so its effect on output and footprint is a dead edge until a renewable plant exists; the probe adds half-renewable variants.
- **Lone burning tiles lower the ignition chance** (fewer standing tiles) **but raise spread**: `fire_ignition` fire -> fire is `varies`, and measured both ways.
- **`decide_spread` with zero assets gives the lowest spread** (`safe_div` returns leverage 0): a firm with debt and no assets is charged least, and raising its assets raises its rate. Probably unreachable (firms hold capital plus cash) but a discontinuity (code, not changed).
- **`decide_price` at unit cost 0 keeps the price** (the firm made nothing), so a nudge from 0 lowers it; the probe's bases exclude that regime.
- **38 of 331 pairs are `none`.** The graph's `drives` edge is the cross product of a spec's inputs and outputs, and many specs have several outputs that depend on different inputs (`firm_capex` has 12 inputs and 3 outputs, 14 of its 36 pairs have no direct effect). They are measured (the output never moves) and kept so the edge list stays the complete cross product; the UI should hide them by default.
- **Rent -> rent flips on a 0.1% nudge.** The rent rule reads the town's mean rent burden over renters (not one household's), and a tiny rise can push the burden over the cap and turn a step up into a step down (`varies`).
- **Emissions caps in `production_output` bind only for very small caps** (0.2 to 20 t per firm per month, not 50): a cap of 0 means no cap, so the first tonne of cap is a tightening (`varies`, both directions seen).

**Limits (stated honestly).**
- Quantities are town aggregates and a few have no single state field, so some probes nudge a stand-in (units sold through households' disposable income, dividends through firm cash, output through firm capital, technology through skill); each probe's comment says which, and an approximate measure is still a measure of the pair's sign, not of its size. Strength is stage 5.
- Effects of one system's mechanisms share a run: where two mechanisms of a system write outputs together the probe picks fields that isolate the pair where it can and says so where it cannot.
- Five effects are `declared_unverified`, stated by reasoning only: `commons_open` into `food_cap` and `commons_stock_change` (open access is the baseline category, so there is nothing to nudge up; enclosure and the community cap carry the change), `wage_tax_rate` and `capital_tax_rate` into `last_disposable` (the wellbeing system copies `income.disposable()`; the rates act through `tax_paid`, which another system writes), and `price` into `inventory` in `goods_settlement` (above).
- DP-12's blind spot stands: system files, enums and the RNG are not perturbed.

### Handoff for stage 3 (loops: SCC and R/B, replacing the hand-written loop table)
- The signed edge list is complete: build the quantity graph from `causal::edges()` filtered to `kind == Drives` and `effect.sign` not `None`, **one edge per (via, from, to)** (two writers of one quantity are separate edges). Compute strongly connected components in Rust (Tarjan over `BTreeSet`s, about 60 lines, `docs/features/causality-graph.md` section 4: no petgraph), enumerate simple cycles inside each SCC with a bound (self-loops are cycles of length 1: technology, capital, forest, commons, drought, fire, savings, legitimacy), and classify each: **B** if the product of its edge signs is `-` (odd number of `-`), **R** if `+`, **"depends"** if any edge is `Varies` (`path_sign` already returns `Varies`; do not guess). `causal::path_sign` and `paths` are the building blocks.
- The loops that matter for the game and where they come from: wage -> unit cost -> price -> (nominal budgets) -> sales -> expected sales -> workers wanted -> wage; emissions -> stress -> drought -> weather factor -> output -> emissions (B through `-` weather factor, check the sign product); rent -> burden -> rent; forest/commons harvest. Compare the computed list with the original design's loop table (9 rows, one says "NOT MODELLED"), turn the doc table into a generated list (a test that compares the doc's loop ids with the computed ones, or generate the rows), and report loops the doc missed and rows the code does not support.
- Self-loops of stocks are `+` because the output is next month's level (stock -> itself is `Up`); the net-flow feedback (regrowth, harvest) shows in the other edges of the cycle, not in the self-loop.
- Mixed paths: use `net_sign`. A loop whose edges include a `varies` or a `none` is not classified; keep it listed as "depends" with the reason from the spec.
- Lags: sum `edge_lag_months` around a loop for its delay (a loop with delay 0 within one tick is an algebraic loop and would be a bug to report). Lever lead times are not in the edge lags yet (see above).

## 14. Stage 3 as built (contract 2.12): loops (SCC, reinforcing and balancing)

No behaviour change: `statehash` (18 lines) and baselines (`cargo run --release -p sim-cli --example baselines`) are byte-identical before and after (`cmp`); the golden changed only in the `catalog` hash (`d15d041eb058c7df` -> `b21e8a0c86553006`; the snapshot, turn and state hashes are unchanged). No simulation code changed.

**Where.** `crates/sim-model/src/causal_loops.rs` (pure, tested; Tarjan over `BTreeSet`s, no `petgraph`: 130 nodes and 293 edges need about 40 lines and every order stays ours, per section 4), one new helper `causal::system_of(node)`, words in `content/decisions.json` `causal.loops`, wire in `crates/sim-api/src/decisions.rs` (`build_loops`).

**The graph.** One node per quantity; one edge per `(via, from, to)` `Drives` edge whose sign is not `none`: **293** edges (331 less the 38 `none`). A starting point has no writer so it has no incoming edge: **a lever is never on a loop**, it acts on loops from outside (test `levers_are_never_on_a_loop`).

**Components.** 129 quantities: **6 strongly connected components that hold a cycle** and 78 quantities on no cycle. The big one has **46 quantities** (labour, prices, profit, savings, emissions, climate, forest and commons, rent); the other five are single quantities with a self-loop (`distress_months`, `bank_equity`, `legitimacy`, `pilot_employed`, `technology`). Test `scc_partition_is_complete_and_correct` checks every quantity is in exactly one component or none, by brute-force reachability that members reach each other, that two components never reach each other both ways, and that no "acyclic" quantity is on a cycle.

**Enumeration and its bounds.** Simple cycles by depth-first search from each quantity, visiting only larger quantities (`QuantityId` order), so each cycle is found once, from its smallest member (canonical rotation); edges are tried in registry order; the result is sorted shortest first, then by members, then by the decisions crossed. A self-loop is a cycle of length 1 (21 of them). **Bounds: at most `MAX_LOOP_LEN = 8` quantities and `MAX_LOOPS = 20,000`.** Why 8: the number of cycles explodes with length (measured with a cap of 2,000,000): 37 up to 3 quantities, 55 up to 5, 203 up to 8, 335 up to 9, 543 up to 10, 1,616 up to 12, 12,482 up to 16, 68,571 up to 20 and **886,269 in all** (the 46-quantity component has 231,164 reinforcing, 229,841 balancing and 425,264 mixed cycles). A player cannot read a 20-link loop, and the longer ones are combinations of the short ones. 8 gives 203 loops (about 3 ms to compute) and keeps the nine teaching loops of the original design, whose longest needs 8. `Loops.truncated` is set only when the count cap is hit (it is not: the test says 203 is far below it); cycles longer than 8 exist and are not listed (the wire says `loopMaxLen`).

**Classification** (`loop_kind`, using `causal::path_sign` on the signs around the cycle): **B** when the number of `-` edges is odd, **R** when even, **depends** when any edge is `varies` (never guessed). Test `classification_follows_the_count_of_minus_edges` re-derives it from the edges.

**Delay.** Each edge adds its spec's own lag plus the tick-order wait **between the system that writes its input in this loop (the previous edge's decision or mechanism) and the system that reads it** (`causal::tick_wait`). Summing the plain `causal::edge_lag_months` (which takes the *earliest* registry writer of each input) is wrong for a loop: **2 loops would get a delay of 0**, `forest_stock` alone and `forest_stock <-> output`, only because `forest_stock` has a second writer (`fire_burns_forest`) that runs before the reader. Within a cycle that is impossible (systems run in a fixed order, so a cycle must wait at least one tick somewhere), so `Loop.delay_months` uses the in-loop writer and is at least 1 for every loop (test `a_loop_cannot_close_inside_one_tick`); `Loop.summed_edge_lags` keeps the plain sum for this finding. The two differ for 48 of the 203 loops. **Zero-delay (same-month algebraic) loops: none.** Delays: 1 month: 26 loops (mostly the self-loops and the two-quantity ones), up to **7 months, the longest** (the reserve-army loop and the wage -> savings -> demand loop). Lever lead times (`content/levers.json`) are not in the delay, as stage 2 said.

**Counts (all measured, test `counts_are_pinned`).** **203 loops: 89 reinforcing (R), 107 balancing (B), 7 depends.** By length (1 to 8 quantities): R 17, 0, 1, 1, 6, 12, 18, 34; B 1, 8, 6, 5, 6, 11, 25, 45; depends 3, 1, 0, 0, 0, 1, 1, 1. Longest loop: 8 quantities (80 loops). Self-loops: 21 (17 R: a stock or a smoothed value carries over; 1 B: `bank_equity`; 3 depends: `over_target_months`, `fire`, `rent`). Note that a self-loop of a stock is `+` because the output is next month's level (section 13): it is persistence, not feedback, and the real feedback of a stock is in its longer loops; stage 4 should show self-loops last or hide them by default.

**Known examples (test `known_feedback_loops_have_the_expected_kind`).**
- Emissions -> climate stress -> drought -> weather factor -> output -> direct emissions -> gross emissions -> cumulative emissions: **B** (7 quantities, 5 months). The damage lowers output, and output makes the emissions.
- Wage -> unit cost -> price -> demand -> expected sales -> workers wanted -> workers -> unemployment -> wage: **B** (8, 7 months); the same chain through wage -> disposable income -> consumption budget -> demand is **R**: the wage is both a cost (balancing) and an income (reinforcing).
- Profit -> planned investment -> capital -> output -> stock -> units sold -> revenue -> profit: **R** (the growth loop).
- Forest burns -> smaller sink -> more emissions -> stress -> fire: **R** (5).
- Commons stock <-> output: **B**; footprint <-> green technology: **B**; rent: **depends** (the rent rule's sign flips on a 0.1% nudge, section 13).

**The nine hand-written rows of the original design's loop table (now removed from `docs/causal-model.md`), validated** (test `teaching_loops_are_validated_against_the_generated_loops` pins each row to the generated loop that carries it, or to the reason there is none):

| Row | Result |
|---|---|
| 1 Profit -> investment -> output -> profit (R) | Supported, R (8 quantities, 6 months) |
| 2 Wealth -> lobbying -> weaker policy -> profit (R) | **Not supported by the code**: `lobby_spend` has no outgoing edge (nothing reads it), so the capture loop does not exist in the graph |
| 3 Debt -> interest -> profit -> investment (R) | **Not supported**: `debt` is a starting point (nothing in the registry writes it): DP-13, the credit book |
| 4 Wage share falls -> demand falls -> layoffs (B) | **Only loosely supported**: the nearest loop is "the wage bill squeezes payouts" (workers -> payroll -> dividends -> savings -> demand -> hiring, B). The doc's capital-income-versus-wage-share route exists only in loops of 9 or more quantities |
| 5 Unemployment -> lower wages -> higher profit -> hiring (B) | Supported, B, but through **unit cost and price, not profit** (the "reserve army" loop). The shortest route through profit has 9 quantities and is R |
| 6 Scarcity -> price rises -> demand falls (B) | **Not modelled**, as the doc says: `price` reads only unit cost, the markup and itself (MK-1); the test asserts the three inputs, so a scarcity link would fail it |
| 7 Over-harvest -> commons collapses -> output falls (B) | Supported, B (2 quantities) |
| 8 Emissions -> budget used -> climate damage -> output and wellbeing (R) | **Kind disagrees**: through output it is **B** (output makes the emissions); the reinforcing overshoot is the forest-sink loop (R, 5). `wellbeing` drives only legitimacy, which feeds back only into itself, so "and wellbeing" is on no loop |
| 9 Carbon price -> green investment -> lower footprint (B) | Supported only as **footprint <-> green technology (B)**; the carbon price is a lever, so it feeds the loop from outside and is on no loop |

**Loops the doc missed.** The doc lists 9; the code has **203**. Beyond the pinned ones, notable loops the doc did not mention: the **shop** (inventory <-> units sold, B), **hiring and matching** (workers <-> vacancies, B; workers <-> over-target months, B), **rent against income** (rent <-> disposable income, B; rent <-> occupancy, B; rent -> capital income -> disposable income -> rent, R), **consumption against savings** (savings -> spending -> units sold -> inventory, B), **emissions through drought into electricity demand** (R, 6) and **fire** (smoke lowers the weather factor: B, 4 with the commons), the **farm and forest** (forest stock <-> output, B). All come from signed effects measured in stage 2.

**Incomplete loops.** `Loop.incomplete` is set when a loop passes through (a) a quantity bound to a player lever (`live.lever`), (b) a quantity plain code also writes (`causal.loops.gap_quantities` in content, the DP-13 list: debt, bank loans and deposits, credit spread, treasury cash, public debt, committed spending, net wealth, propensity, the food price, firms' private cash, reserve, capital per sale, hurdle, payout ratio, sector return and the estate's savings, all verified quantity ids) or (c) an edge whose sign is declared but not measured. **No loop is flagged today**: every quantity in the DP-13 list is a starting point or an end point (so cannot be on a cycle), the five unmeasured effects are not in any loop, and a lever has no writer. That is itself a finding: DP-13's incompleteness shows as **loops that are missing** (credit, lobbying, public finance), not as flagged loops. The flag logic is a pure function with its own test (`a_loop_through_a_lever_a_gap_or_an_unmeasured_link_is_incomplete`), so the first mechanism spec that puts such a quantity on a cycle will flag it with no other change.

**Wire (contract 2.12, additive).** `catalog().causal` gains `loops: CausalLoop[]` (`id`, `kind` `R`/`B`/`depends`, `members` quantity node ids in cycle order, `edges` one `{via, sign, lagMonths}` per member (member `i` to `i + 1`), `delayMonths`, `delayLabel`, `sentence`, optional `name` and `story`, `incomplete`, `incompleteReasons`, `scc`), `sccs: {id, members}[]`, `loopKinds` (label, badge letter `R`/`B`/`?`, a sentence), `loopIncomplete` (reason words), `loopMaxLen`, and every `CausalNode` gains `inLoops` (the loop ids a quantity is a member of, or a decision or mechanism carries an edge of). The loop **id** is `loop:` plus the member quantity names sorted and joined with `+` (stable, built from the node ids; `~2` for a second loop over the same quantities, one case today); teaching names are authored in content keyed by the members and validated at load against the real loops (7 so far: Growth, Wage bill squeezes payouts, Reserve army, Tragedy of the commons, Overshoot bites, Dying forests, Green investment). **The sentence is generated** from the cycle and its signs, so it cannot disagree with the kind: "This is a balancing loop: when carbon footprint goes up, green technology goes up, then carbon footprint goes down again." (R ends "goes up again", B "goes down again", a mixed loop "up or down"). Size: the catalogue grew from 434 KB to 871 KB compact JSON (the loop ids are long and repeat in `inLoops`); the recorded demo grew 437 KB raw and 20 KB gzipped.

**Web.** How it works > Agents: a **Feedback loops (n)** section under the kind's decisions and mechanisms, one text line per loop (R, B or ? as a badge whose shape differs: circle, square, diamond, with the word beside it), the name or the chain `A -> B -> ... -> A`, an expandable sentence, story, delay and incomplete reasons; the first 8, then "Show the other n"; and under each decision and mechanism a line "In 12 feedback loops: 5 reinforcing, 6 balancing, 1 depends". `panels/explorer.ts` `loopLines`, `loopSummary`; no quantity, loop or word is named in the web, and `web/tests/agents.test.ts` proves a loop added only to the catalogue renders with no web change.

**Not measured or not done (stated honestly).** Loop strength is stage 5. Loop delay ignores lever lead times. Loops longer than 8 quantities are not listed. A loop is a cycle of the *quantity* graph through *declared* effects (stage 2), so what the registry does not declare (DP-13, DP-16) is not in it. The look of the section was checked in a browser at 1440 px (see the stage 3 report).

### Handoff for stage 4 (the view: the Why? panel)
- **Wire data that exists:** nodes with `inLoops`, edges with sign, lag, sentence and basis, `loops` with kind, members in cycle order, per-edge sign and lag, delay, sentence, optional name and story, `loopKinds` words and badge letters, `sccs`, live values per quantity (`snapshot.quantities`, trend ids, contract 2.9) with `catalog().quantities[].live`. Layered layout needs only `causes_of` ranks, which `CausalNode.upstream` (depth 3, nearest first) already gives for the first level.
- **Wire data that does not exist yet:** (1) per-depth ranks for an arbitrary depth (the depth control): either request-time (`causes(q, depth)` on `SimPort`, not built) or compute ranks in the web from `edges` (layout only, allowed); (2) path sign / net sign from a cause to the focused quantity (`causal::sign_of` and `paths` exist in Rust but are not served; serve `netSign` per upstream node or the `Mixed` flag, never recompute in the web); (3) strength (stage 5); (4) a lever's lead time on edges from levers.
- **Entry points:** the Agents explorer lines (a quantity name could link to "Why?"), `BreakdownInput.quantityId`, `MarketView.priceQuantity`, `MarketDriver.quantityId` (stage 0 added them for this), and the headline numbers.
- **Pitfalls:** hide `none` edges by default (38 of 331); 21 of 203 loops are self-loops of persistence, show them last; a `Mixed` net sign must be said, not picked; two writers of one quantity are separate edges (never merge by pair); loop ids are long, key on them but do not display them; a loop's members are in cycle order starting at the smallest quantity, so for a loop through the focused node rotate for display; `depends` is not R or B; the loop list is bounded at 8 quantities, say so; do not draw a loop's `edges` from/to (they are implied by position in `members`).
