# Architecture

End to end, as implemented (verified against the code at the time of writing). Design intent is in `docs/town-v2.md`; this file is how the pieces fit.

## 1. Data flow

```
content/*.json ──include_str!──> sim-api (content.rs)
   params.json, initial_town.json,         │
   levers.json, systems/*.json             │ sim-params: Params::from_content (validation, deny_unknown_fields)
                                           v
 ┌──────────────────────── Rust, deterministic, no I/O ─────────────────────────┐
 │ sim-types (State, ids, Cents)   sim-rng (PCG32)   sim-params (Params)        │
 │      │                                                                       │
 │ sim-model: 12 pure systems  fn(&State,&Params,&mut Rng)->Deltas              │
 │ sim-metrics: indicators     sim-checks: invariants after every system        │
 │      │                                                                       │
 │ sim-engine: step() tick loop, apply() deltas, turn layer (PC, pending,       │
 │             lead times), submit_turn_with(), Plan, init + warm-up            │
 │      │                                                                       │
 │ sim-api: Api = wire JSON (camelCase), wire validation, frames, history,      │
 │          save/load, fixtures      <── docs/snapshot-contract.md (the wire)   │
 └──────┬───────────────────────────────────────────────────────────────────────┘
        │ native: sim-cli (headless), tests          wasm: sim-wasm (WasmSim, JSON strings in/out)
        v
 web/src/sim-port      port.ts (SimPort interface)
   ├─ wasm/   loads pkg/ built by `npm run build:wasm`
   └─ mock/   TypeScript stand-in; numbers from content/ via `npm run sync-mock`
        │ (only sim-port knows which one runs: selectSimPort())
        v
 web/src/store         game state machine, drafts, selection, save/load, sceneBus
        │                          ▲
        ├──────────> scene/  (Three.js: map, people, flows, lenses)    reads store, writes sceneBus
        └──────────> panels/ (Svelte: top bar, palette, drawer, ...)   reads store, calls game.*
 web/src/app           composes store + scene + panels (App.svelte, SceneHost.svelte, keys, playback)
```

Boundaries are enforced (see `AGENTS.md`). `web/src/contract/index.ts` holds the TypeScript mirror of the wire and is imported by sim-port, store, scene and panels; it imports nothing.

## 2. Crate responsibilities

| Crate | Owns |
|---|---|
| sim-types | `State` (people, households, homes, firms, bank, gov, nature, map, plants, pending, policy, effective policy), `Command`, `Event`, `TraceRecord`, `Delta`/`Deltas`, `SCHEMA_VERSION` (2), `MAX_FIRMS` (4) |
| sim-params | `Params` (run, turn, labour, production, space, energy, goods, income, credit, invest, nature, government, class, lobby, wellbeing, metrics, initial town, 3 systems). Content is passed in as JSON text; no file I/O |
| sim-model | One module per system; `TICK_ORDER`; `space.rs` (map, commute routes); `util.rs` |
| sim-engine | `init` (builds the town, runs 48 warm-up ticks, then `reset_clock`), `step`, `begin_turn` / `advance_month` / `finish_turn`, `submit_turn_with`, `turn.rs` (validate, preview, queue, activate_due, cancel, regenerate), `apply.rs`, `commands.rs`, `plan.rs` |
| sim-api | `Api::{init, reset, snapshot, levers, actions, preview, valid_tiles, submit_turn, begin_turn, advance_month, finish_turn, cancel_pending, export_state, import_state}`; `keys.rs` (closed enums), `meta.rs` (levers/actions from `content/levers.json` + params), `events.rs` (importance), `frames.rs`, `history.rs`, `persist.rs`, `fixtures.rs` |
| sim-wasm | `WasmSim` mirrors `Api` with JSON strings. Replies are `{"ok":true,"value":..}` or `{"ok":false,"error":{"code","message"}}`. Panic hook; panics abort in wasm, so `sim-api` validates input instead |

## 3. Turn flow (`Api::submit_turn` -> `sim_engine::submit_turn_with`)

1. **Guard.** `ensure_running`: after a game over, `submit_turn` and `cancel_pending` return `game_over`.
2. **Wire validation** (`meta.rs::to_sim`): out-of-range values, NaN, wrong types, off-map tiles become `invalid_command` / `tile_invalid` rejections. The engine never sees them (it cannot panic on them).
3. **Backup** of state, RNG and clamp counter. Any failure restores it (`violation` error); a failed turn leaves the game untouched.
4. **Queue** (`turn::queue`, shares `evaluate` with `preview`, so a preview always equals what a submit charges). Commands are costed in order; each sees the earlier ones as queued. Per command: `validate` (system rules, availability, transition in progress), `tile_problem`, PC check, funds check (gov cash + bank lending headroom - already queued builds). Accepted commands get a `PendingCommand` with `effective_turn = turn + lead_time` and are **charged PC now**. Rejected ones carry a `BlockReason`.
   - PC cost = `base_cost x max(|change| / unit, min_size) x (1 + opposition)`, where change is measured against the latest pending value of the same lever, and opposition is `net_lobby` for levers capital opposes. All from `params.turn.levers`.
5. **Activate** (`turn::activate_due`): every pending command with `effective_turn <= turn` is applied now (`commands.rs`), before the ticks. Levers change `policy`; builds are pushed to `gov.pending_build`; a system switch starts a transition. `CommandActivated` events are emitted.
6. **Three ticks** (`params.run.ticks_per_turn`), each `step()` (section 4). The api captures a `MonthFrame` per tick (events + flows). Events from steps 4-5 are added to month 0.
7. **Policy gaps** for the four lobbied levers (nominal vs effective), measured at end of turn.
8. **Regenerate PC**: `pc += pc_base x legitimacy`, capped at `pc_cap` (60 start, 100 cap, 8 base). `turn += 1`.
9. **History**: the api appends one `HistoryPoint` per played turn (kept in `Api`, saved in the blob, not in `State`).
10. **Result**: `TurnResult { accepted, rejected, activated, pcBefore, pcAfter, months[3], policyGaps, trace, snapshot }`.

**Live turn (Town v3, contract section 12).** `submit_turn_with` is `begin_turn` (steps 4-5) + `advance_month` x3 (step 6) + `finish_turn` (steps 7-8) in `sim-engine`; progress is `Sim::turn_phase` (`TurnPhase { open, months_done }`, outside `State` and saves). `Api::{begin_turn, advance_month, finish_turn}` run the same steps one call at a time, keeping the accumulating frames, events and trace in a private `Live` struct, and `finish_turn` returns the same `TurnResult` as `submit_turn` (step 9-10). Out-of-order calls return `turn_state` and change nothing; `submit_turn` and `export_state` are refused while a turn is open; `import_state` and `reset` close it. The split path is tested equal to `submit_turn` in `crates/sim-api/tests/turn_split.rs`.

Money for builds: PC is charged at queueing; build **money** is paid when the build activates and the next government tick (S8) pays it out (`gov.pending_build`). Cancelling a pending item refunds 50% of its PC and no money (none was charged). Capacity/tiles appear on activation: the lead time is the whole delay.

Game over: the first `Outcome` (`budget_exhausted`, or a collapse: `legitimacy_zero`, `mass_unemployment`, `bank_failure_cascade`) is recorded by `check_outcome`. The engine runs with `stop_on_loss = false`, so the rest of that turn still plays; the api then refuses further turns. `gameOver.turn` is derived from the outcome tick; a survived game reports turn 161.

## 4. Tick order (`sim-model::TICK_ORDER`, then engine)

Each system returns `Deltas`; `apply()` applies money operations, then field changes, then events and trace; `sim-checks` then runs (state invariants, money conservation, sector balance, unfunded-payment check when `strict_flows`).

| # | `SystemId` | Module | Doc label | Does |
|---|---|---|---|---|
| 1 | Policy | `policy.rs` | S0/system | Resets per-tick flows; advances an economic-system transition |
| 2 | Labour | `labour.rs` | S1 | Individual hiring/firing, wages, job guarantee |
| 3 | Production | `production.rs` | S2 | Output from effective labour (skill x commute-adjusted hours), electricity demand and rationing, firm direct emissions |
| 4 | Nature | `nature.rs` | S7 | Emissions by source (firm, households, plants, sink), forest, commons, cumulative emissions, atmosphere |
| 5 | Goods | `goods.rs` | S3 | Household spending, firm sales, housing market, rent, evictions and moves |
| 6 | Income | `income.rs` | S4 | Wages, dividends by system, rent, interest, taxes on income |
| 7 | Credit | `credit.rs` | S5 | Lending, repayment, defaults, bank failure/bailout |
| 8 | Government | `government.rs` | S8 | Taxes, benefits, public spending, builds (pays `pending_build`), road upkeep, legitimacy |
| 9 | Investment | `investment.rs` | S6 | Capital investment, green technology |
| 10 | Class | `class.rs` | S9 | Derived class per household, mobility, founding a firm in the dormant slot |
| 11 | Lobbying | `lobbying.rs` | S9b | Net lobbying; `effective = nominal x (1 - net_lobby)` for carbon price, profit tax, wealth tax, caps |
| 12 | Wellbeing | `wellbeing.rs` | S10 | Per-person wellbeing (incl. commute, unhoused, local pollution) |

Then, in `step()`: `sim_metrics::compute` (indicators), `check_outcome`, a money check, household income by type into the trace, `tick += 1`.

Note: `SPEC.md` section 11 lists a different, older ordering and "S" numbers do not match tick order (Investment S6 runs after Government S8). The table above is what runs. Effective policy computed at the end of a tick is used from the next tick (`deviations.md` #18).

## 5. Data ownership

| Owned by the sim (`State`) | Owned by the UI (store / scene / panels) |
|---|---|
| The map (12 x 12 tiles) and what stands on it; all buildings, roads, plants | Camera angle, zoom, lens, hover, selection |
| People, households, homes, firms, bank, government, nature | The **draft** (commands not yet submitted) and its previews |
| Policy (active), effective policy (after lobbying), pending queue, PC | Which phase the UI is in (briefing, plan, playback, results, end), playback month |
| Outcome, history series (in `Api`) | Text: formatting events, labels, "why" stories (`panels/format.ts`, `story.ts`, `why.ts`) |
| Random state (PCG32 `state`, `inc`) | Save file wrapper (`ecotown-save`, saveVersion 2): sim blob + last result + previous snapshot + lens/camera |
| Lever metadata (labels, ranges, arrows) via `content/levers.json` | Nothing about layout: since Town v2 the store derives the map from `snapshot.map` (`layoutOf`) |

The UI must never compute economy numbers. The mock is the one exception by design (it stands in for the sim) and must read its numbers from `content/`.

## 6. Determinism and the golden test

- All state is integer money (`Cents(i64)`) plus f64 quantities; no `HashMap`; `libm` for transcendentals; PCG32 seeded by `Rng::new(seed, 54)`. `serde_json` uses `float_roundtrip` so saves restore every f64 bit.
- `tests/golden/plan.json` (seed 7, scripted turns incl. a cancel, an export and an import) is replayed through the exact JSON strings the browser gets. `tests/golden/seed7.json` records an FNV-1a 64 hash of every reply plus headline numbers.
  - Native: `crates/sim-wasm/tests/golden.rs`. Wasm: `web/tests/wasm-golden.test.ts` (skipped if `web/src/sim-port/wasm/pkg` is not built).
  - Any intended simulation change makes both fail until `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` is run and the diff is understood.
- `tests/scenarios.rs` also has `same_seed_gives_identical_state`, invariants over several seeds, an insta snapshot (`golden_snapshot_seed_42`), and 20-seed baseline tests A-D.
- Seeds matter little: the RNG is used only for commons enclosure and firm founding (`deviations.md` T23).

## 7. Save/load and schema versions

| Version | Where | Value | Meaning |
|---|---|---|---|
| `SCHEMA_VERSION` | `sim-types/src/lib.rs` (re-exported by sim-api, sent as `Snapshot.schemaVersion`) | 2 | Layout of `State` and of the wire. Bump when either changes shape. Town v1 was 1 |
| Sim blob | `Api::export_state` | `{ schemaVersion, paramsHash, rngState, rngInc, initialPopulation, clampedTotal, state, history }` | Import refuses other versions (`schema_mismatch`), a different params hash (`params_mismatch`), and malformed structure or failed invariants (`bad_state`). A failed import leaves the game untouched |
| `SAVE_VERSION` | `web/src/store/save.ts` | 2 | The browser save file (`format: "ecotown-save"`) around the sim blob. `parseSave` returns plain-language errors |

`paramsHash` is FNV-1a 64 over the canonical params JSON, so **any change to `content/` invalidates existing saves** (`params_mismatch`). The mock keeps its own `schemaVersion` constant equal to the real one (2).
