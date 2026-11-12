# Docs index

Start with `/AGENTS.md`. This page says which doc to trust for what. Where two docs disagree, the order of authority is: **code and `content/` > generated fixtures > current docs below > historical docs**.

## Source of truth, by topic

| Question | Go to |
|---|---|
| What the game is, goals, scope, milestones | `SPEC.md` sections 1-4, 8-10, 14-16 (read its "Current state" block first) |
| Town v2 design (people, households, map, roads, plants, Shop, inequality) | `docs/town-v2.md` |
| The rules of each system and sign tests | `docs/causal-model.md` section 10 (Town v2), then sections 7 and 8 (turn layer, demand side); sections 0-3 are cohort-era |
| What the code deliberately does differently from the design | `docs/deviations.md`, the "Town v2" table (T1-T23) |
| Numbers, tuning history, baselines A-D | `content/*.json` (values), `docs/tuning-log.md` "Town v2" (history) |
| The wire (JSON shapes) | `crates/sim-api` (Rust types) and the TS generated from them, `web/src/contract/generated.ts` (do not edit; `index.ts` holds aliases), then `docs/snapshot-contract.md` section 27 (v2.6, the sustainable-yield view), section 24 (v2.4, trade lists) and section 23 (v2.3, the Markets view), section 22 (v2.2, the net worth statement), section 21 (v2.1, the entity registry), section 20 (v2.0, sources as lists), then `docs/snapshot-contract.md` sections 9-11, 18 (v1.8) and 19 (v1.9, the estate), then `docs/sim-api-notes.md` "Town v2" |
| Where UI text, thresholds and bands live | `docs/town-v6.md`, `content/ui.json`, `docs/how-to.md` section 17 |
| Sample wire output | `docs/fixtures/` (generated; do not hand-edit) |
| How the pieces fit, turn flow, tick order, save schema | `docs/architecture.md` |
| `docs/town-v3.md` | APPROVED, IMPLEMENTED (Pressure mode: the live clock, `web/src/store/clock.ts`, `live.ts`; recipe `docs/how-to.md` section 13) | Working on the clock or Pressure mode |
| `docs/town-v4.md` | APPROVED, IMPLEMENTED (weather: fires and drought; `crates/sim-model/src/weather.rs`) | Working on weather |
| `docs/town-v5.md` | APPROVED (information release: Finance tab, per-entity ledgers, energy units, shopping trips). In progress (section 10: the 'How it works' explorer is queued as v5.1) | Working on finance/ledger/energy |
| `docs/town-v5-3.md` | APPROVED, IMPLEMENTED (agent choices explained: Agents section in the explorer, person decision tag, `agents()` wire call, contract v1.7). Includes the code-check corrections | Changing agent rules or the randomness table |
| `docs/town-v6.md` | APPROVED (DRY cleanup: the web is a thin renderer; model text, thresholds and UI bands move to `content/` and the wire; contract v1.8) | Changing where text or thresholds live |
| `docs/town-v7.md` | APPROVED, Rust/content/wire/recorded demo IMPLEMENTED (the elite estate and its pilot; contract v1.9, `docs/snapshot-contract.md` section 19). The web panels and the scene are DONE (`docs/web-notes-v7.md`) | Working on the estate in the web, or its rules |
| `docs/town-v8.md` | APPROVED (net worth breakdown: per-household and per-person statement, town split by holder, reconciled to the Gini; contract v2.2). IMPLEMENTED | Working on net worth, wealth or the Who holds what view |
| `docs/features/gst-and-state-ownership.md` | DRAFT (GST/VAT and state ownership; was v7, then v8). Not implemented; after v5.1/v5.2 | Planning fiscal and ownership work |
| `docs/features/daily-ticks.md` | DRAFT, DEFERRED (daily ticks; was v5, v6, v8, then v9) | Planning the time-step refactor |
| `docs/features/markets.md` | APPROVED, scope A IMPLEMENTED (Markets view: supply, demand, price and who is short per market, information only; contract v2.3, `docs/snapshot-contract.md` section 23, recipe `docs/how-to.md` section 24). Scope B, demand-responsive goods prices, not approved | Planning the Markets tab or price rules |
| `docs/features/market-trades.md` | APPROVED, IMPLEMENTED (the list of individual trades behind each market, "See all trades"; contract v2.4, `docs/snapshot-contract.md` section 24, recipe `docs/how-to.md` section 25) | Planning market or ledger drill-downs |
| `docs/features/inflation.md` | DRAFT for review (measure inflation: CPI from household spending, real wages, and an exact cost pass-through decomposition; output only; contract v2.5 proposed) | Planning inflation, price indices or real wages |
| `docs/features/farm-from-nature.md` | APPROVED (all defaults), IMPLEMENTED (food physically drawn from the commons: Schaefer-style stock cap, a sustainable-yield view of overconsumption and the tragedy of the commons; contract v2.6, `docs/snapshot-contract.md` section 27, recipe `docs/how-to.md` section 26) | Planning nature limits on food, the commons or overconsumption |
| `docs/features/firm-agency.md` | DRAFT for review (let the player act as firms: inventory of today's firm decisions, gap analysis against real firms, options A directives, B play one firm, C role switch) | Planning firm actions, firm-side play or firm lobbying |
| `docs/features/registry-driven-sim.md` | PARKED (idea: the simulation reads `content/entities.json`, not only Rust enums) | Planning the sim/registry link |
| **Naming convention** | From now on new work is a named **feature**, not a version: `docs/features/<name>.md`. The old `town-vN.md` files stay as history | Starting new work |
| How to make a change | `docs/how-to.md` |
| Vocabulary | `docs/glossary.md` |
| Open problems and debt | `docs/known-issues.md` |
| UI layout, phases, module rules | `docs/ui-design.md` (with its Town v2 amendment), `web/eslint.config.js` |
| Building and running wasm, determinism, golden | `docs/integration-notes.md`, `AGENTS.md` |

## All documents

| File | Status | Read it when |
|---|---|---|
| `AGENTS.md` (root) | CURRENT | Always, first |
| `README.md` (root) | CURRENT | Running the game |
| `SPEC.md` | PARTLY CURRENT. Sections 2 (agents), 5, 6, 12 and 13.1 (map ownership) are SUPERSEDED BY section 17 and `docs/town-v2.md` | Concept, goals, scope |
| `docs/town-v2.md` | CURRENT (approved design) | Changing anything about people, households, map, roads, plants, firms |
| `docs/causal-model.md` | Sections 0-3 SUPERSEDED BY section 10 and `docs/town-v2.md` (cohort model). Sections 4-9 HISTORICAL numbers, rules still mostly valid. Section 10 CURRENT | Changing or checking a system's rule |
| `docs/snapshot-contract.md` | Sections 1-8 HISTORICAL (v0.1: UI owns the map, cohort households). Sections 9-11 CURRENT (v0.2 fixes, v0.3 alignment, v1.0 Town v2) | Changing the wire |
| `docs/sim-api-notes.md` | Top half (v0.3 era) HISTORICAL, some values stale (corrected inline). "Town v2 (contract v1.0)" section CURRENT | Wire details: shapes, frames, save, metadata |
| `docs/integration-notes.md` | CURRENT for wasm build, determinism, golden. "Contract fixes" and "Contract issues left" sections are v0.3-era (schemaVersion 1) and HISTORICAL | Building wasm, golden test |
| `docs/deviations.md` | Items 1-46 HISTORICAL (cohort model; 39-46 superseded by Town v2). "Town v2" table T1-T23 CURRENT | Why the code differs from the design |
| `docs/tuning-log.md` | Everything before "Town v2" HISTORICAL. "Town v2" section CURRENT | Parameter history, baselines |
| `docs/balance-backlog.md` | HISTORICAL (Phase 1-2, cohort numbers). Open items carried into `docs/known-issues.md` | Background only |
| `docs/ui-design.md` | CURRENT with Town v2 amendment at the bottom. Its "Decisions" (map ownership, palette v1) are SUPERSEDED BY the amendment | UI layout, phases |
| `docs/web-notes.md` | HISTORICAL (web scaffold, contract v0.1) | Never needed |
| `docs/web-notes-scene.md` | HISTORICAL (v1 scene). SUPERSEDED BY `docs/web-notes-scene2.md` | Scene design rationale only |
| `docs/web-notes-panels.md` | HISTORICAL (v1 panels). SUPERSEDED BY `docs/web-notes-panels2.md` | Panels rationale only |
| `docs/web-notes-align.md` | HISTORICAL (alignment to contract v0.3, schema 1) | Never needed |
| `docs/web-notes-v2.md` | Foundation notes. Its "Left rough" list is mostly fixed by scene2/panels2; its "Contract issues" list is still valid | Store/contract changes for v2 |
| `docs/web-notes-scene2.md` | CURRENT (scene notes) | Working in `web/src/scene` |
| `docs/web-notes-panels2.md` | CURRENT (panels notes) | Working in `web/src/panels` |
| `docs/town-v5-3.md` | CURRENT (agents catalogue, contract v1.7) | Changing agent rules |
| `docs/town-v6.md` | CURRENT (approved and implemented: thin renderer, UI catalog `content/ui.json`, contract v1.8) | Deciding where text, a threshold or a number lives |
| `docs/town-v7.md` | CURRENT (approved; Rust, content, wire, recorded demo, web panels and scene implemented; see `docs/web-notes-v7.md`) | Estate rules and design; the "As built" section lists what differs from the draft |
| `docs/town-v8.md` | APPROVED (net worth breakdown; contract v2.2). IMPLEMENTED (`docs/snapshot-contract.md` section 22, `docs/how-to.md` section 23) | Net worth, wealth, Who holds what |
| `docs/features/gst-and-state-ownership.md` | DRAFT (GST and state ownership; was v7, then v8) | Planning fiscal and ownership work |
| `docs/features/daily-ticks.md` | DRAFT, DEFERRED (daily ticks; was v5, v6, v8, then v9) | Planning the time-step refactor |
| `docs/features/markets.md` | APPROVED scope A, IMPLEMENTED (Markets view; contract v2.3) | Planning the Markets tab or price rules |
| `docs/features/market-trades.md` | APPROVED, IMPLEMENTED (trade lists behind each market; contract v2.4) | Planning market or ledger drill-downs |
| `docs/taxonomy.md` | APPROVED (axes and roles classification of every entity; migration plan; ts-rs and mock-replay interaction). Phases 1 and 2 of the platform cleanup are implemented (section 12 has the status per step) | Planning shared abstractions, the entity registry, or vocabulary renames |
| `docs/web-notes-v7.md` | CURRENT (Town v7 web notes: Panels and Scene sections) | Working on the estate in `web/src` |
| `docs/architecture.md` | CURRENT | Understanding the system end to end |
| `docs/how-to.md` | CURRENT | Making a change |
| `docs/glossary.md` | CURRENT | Any unfamiliar term |
| `docs/known-issues.md` | CURRENT | Before tuning, before reporting a bug |
| `docs/fixtures/*.json` | GENERATED (`cargo run -p sim-api --example fixtures`) | Seeing real wire output |

## Reading order for a new agent

1. `AGENTS.md` 2. `docs/architecture.md` 3. `docs/town-v2.md` 4. `docs/how-to.md` for your change 5. `docs/known-issues.md` 6. The code you will touch.

Historical docs are kept as a record, not rewritten. Each has a status banner at the top pointing to its replacement.

## Platform cleanup (no Town version; status per step in `docs/taxonomy.md` section 12)

Phase 1, done: (A) the TS contract is generated from the Rust wire types (`web/src/contract/generated.ts`; recipe `docs/how-to.md` section 19); (B) emissions and income sources are lists on the wire (contract v2.0, `docs/snapshot-contract.md` section 20; `SCHEMA_VERSION` 6); (C) the display-only vocabulary renames that were low risk (skipped ones: `docs/known-issues.md` PC-3). State hash unchanged. Phase 2, done: the entity registry (`content/entities.json`, contract v2.1, `docs/snapshot-contract.md` section 21, recipe `docs/how-to.md` section 21) with registry-driven inspector sections, labels and scene models; and the hand-written mock replaced by a recorded demo of the real sim (`docs/how-to.md` section 22). Still open: `Purpose` categories, PC-3 renames, PC-4 lists, the optional Rust unification (taxonomy section 12; limits in `docs/known-issues.md` PC-7 to PC-9).
