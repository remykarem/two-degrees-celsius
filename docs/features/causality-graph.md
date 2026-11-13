# Causality graph (the "Why?" view)

Status: **APPROVED (all open-question defaults).** Stage 0 in progress; later stages not built. Builds on `docs/features/decision-points.md` (sections 15 and 16: the registry, `crates/sim-model/src/causal.rs`, `catalog().causal`, contract 2.8). Claims marked *(unverified)* were not checked against code or a running build; sizes marked *(npm)* come from npm registry metadata, not a bundle build.

## 1. Goal and hard rules

The player picks a quantity (unemployment, price, emissions) and sees its **causes as a graph**: `a -> b -> e`, `d -> e` (several causes converging), each edge saying which way it pushes ("if A goes up, B goes down"), and which parts are feedback loops. The user is new to economics, so every edge has a plain sentence.

- **Thin UI** (`docs/town-v6.md`, AGENTS.md web rule 8): the web lays out and draws; sign, lag, loops, depth, live values and sentences come from Rust and `content/`.
- **Coupling first**: where causal information is written twice, or in a place not tied to the code that makes it true, fix that before drawing anything (section 6).
- Determinism rules apply to any new Rust: no `HashMap`, `libm`, no RNG, no clock, `Cents` for money. State hash must stay byte-identical in every stage (section 9).

## 2. Visual conventions (question 1)

| Convention | What it shows | Fit here |
|---|---|---|
| Causal loop diagram (CLD): arrows with `+` / `-`, loop labels R (reinforcing) and B (balancing) | Direction of effect and feedback | **Adopt the vocabulary**: edge polarity and R/B badges. It is what `docs/causal-model.md` section 4 already uses |
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
| C1 | `docs/causal-model.md` section 4, hand-written loop table (9 rows; one says it is "NOT MODELLED") | A prose copy of causal structure; drifts silently | Loops computed from the registry (SCC, section 5) and served; the doc table is replaced by a pointer. Loops outside the registry stay listed as such until C5 | Yes, for the R/B badges; the doc edit can follow |
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
| 1 Mechanism specs | Register read-only specs for production, emissions, weather, wellbeing, rent, tax, matching (C5, C7) | `causal.rs` tests pass over the larger registry; statehash identical | additive |
| 2 Sign and lag | `effects` on specs; template sentences in content | **Sign verification** (section 5); every effect has a sentence; every input-output pair has an effect | additive |
| 3 Loops | SCC, R/B, path sign in `causal.rs`; served on the wire | **Loop classification** on hand-built cases (parity of `-` edges); path sign is the product; determinism | additive |
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
