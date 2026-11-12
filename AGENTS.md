# AGENTS.md: start here

Entry point for coding agents and contributors. Read this, then `docs/README.md` (which doc is the source of truth for what), then `docs/architecture.md` and `docs/how-to.md` before changing anything.

## What this is

- A turn-based game: the player runs a small town's economy and must stay under a 1.5 C carbon budget while keeping the town livable.
- 1 turn = 1 quarter = 3 monthly ticks; a game is 40 turns (10 years, 2026 to 2036; `run.turns`, 900 t budget; weather starts at 75% budget left). Player actions cost political capital (PC) and take effect after a lead time.
- Town v2 world (Town v7 adds the estate): 22 people in 9 households, 8 homes, a 12 x 12 tile map owned by the simulation, 3 firms (Factory, Farm, Shop) plus one dormant slot, a bank, a government, forest and commons, and one very rich estate household with a private jet and its pilot (`docs/town-v7.md`).
- Rust simulation (deterministic, no I/O) -> JSON wire layer -> compiled to wasm -> TypeScript/Svelte/Three.js front end. When the wasm package is missing the web replays a recorded run of the real sim (the "Recorded demo"); there is no second implementation of the rules.
- All game numbers live in `content/*.json`. The code holds rules, not values.

## Repo map

| Path | Purpose | Allowed dependencies |
|---|---|---|
| `crates/sim-types` | Data only: typed IDs, `Cents`, units, enums, `State`, commands, events, deltas. `SCHEMA_VERSION` lives here | serde |
| `crates/sim-rng` | Seeded PCG32 generator | none |
| `crates/sim-params` | Parameter structs, JSON loading and validation (no file I/O) | sim-types, serde, serde_json |
| `crates/sim-metrics` | Derived indicators: Gini, GDP, GPI, budget, temperature | sim-types, sim-params |
| `crates/sim-model` | The rules: one pure system per module, `TICK_ORDER` in `lib.rs` | sim-types, sim-rng, sim-params, sim-metrics, libm, serde |
| `crates/sim-checks` | Invariants run after every system (money, finite numbers, structure) | sim-types |
| `crates/sim-engine` | Tick loop, delta application, turn layer (commands, PC, pending queue), `Plan`, init | all of the above (not sim-api) |
| `crates/sim-api` | Wire layer: contract JSON shapes, validation, frames, history, save/load, fixtures; source of the generated TS contract (feature `ts`, ts-rs) | engine and below. No model logic, no I/O |
| `crates/sim-wasm` | wasm-bindgen shell over `sim_api::Api` (`WasmSim`). `unsafe_code = "deny"` (macro output) | sim-api, sim-params |
| `crates/sim-cli` | Headless runner (`sim`) and `examples/debug.rs` | engine, params, metrics, types |
| `content/` | All numbers: `params.json`, `initial_town.json`, `levers.json` (lever and action metadata), `entities.json` (the entity registry: kinds, labels, roles, tiles, scene models, inspector sections; contract v2.1), `ui.json` (UI bands, authored UI text, system cards; Town v6; net worth statement lines and holders, Town v8; the Markets view cards, mechanisms and cause words, contract v2.3), `assumptions.json` (explorer), `systems/*.json`, `scenarios/*.json` (turn plans) | n/a |
| `tests/` | `scenarios.rs` (engine suite, run as a test of sim-engine), `golden/` (wasm golden), `snapshots/` (insta) | n/a |
| `docs/` | Design docs, contract, notes, generated `fixtures/`. Index: `docs/README.md` | n/a |
| `web/` | Browser app (Vite, Svelte 5, Three.js, Vitest). See below | n/a |
| `.github/workflows/` | `ci.yml` (every pull request): `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace`, contract and recording staleness, build wasm, web test/typecheck/lint/build; plus a non-blocking `audit` job (`npm audit --omit=dev`, `cargo audit`). `deploy.yml` (push to `main`): cargo test, build wasm, web test/typecheck/lint, build, publish `web/dist` to `gh-pages` (only this job has `contents: write`). Actions are pinned to commit SHAs; `rust-toolchain.toml` pins the compiler | n/a |
| `.idea/` | IDE files. Untracked and git-ignored (they were tracked by accident until the review fix); never commit | n/a |

`web/src` folders:

| Folder | Purpose | May import |
|---|---|---|
| `contract/` | TypeScript types of the wire. `generated.ts` is generated from the Rust wire types (never hand-edit); `index.ts` holds aliases. Types only | nothing (`index.ts` may import only `./generated`) |
| `sim-port/` | `SimPort` interface (`port.ts`), recorded-demo replay port (`replay/`, with the generated `recording.generated.ts`), wasm wrapper (`wasm/`), `selectSimPort()` | contract (replay also `sim-port/port`) |
| `store/` | Framework-agnostic game state, phases, drafts, selection, save/load, `sceneBus` | contract, `sim-port/port` only |
| `scene/` | Three.js 3D map, camera, lenses, flows, people | three, store, contract |
| `panels/` | Svelte UI panels and their pure logic (`format.ts`, `metrics.ts`, ...) | svelte, store, ui-kit, contract |
| `ui-kit/` | Generic Svelte widgets and `tokens.css` | svelte |
| `app/` | Composition root: `App.svelte`, `SceneHost.svelte`, keyboard, playback timing | everything except `sim-port/replay` and `sim-port/wasm` (use `selectSimPort`) |

## Hard rules: Rust

Enforced by `Cargo.toml` workspace lints, `clippy.toml`, or tests unless marked "convention".

1. IDs are typed newtypes (`HouseholdId`, `PersonId`, `FirmId`, ... in `sim-types/src/ids.rs`). Never pass bare integers for ids.
2. Money is `Cents(i64)`. No `f64` for money in state. Convert with `Cents::from_dollars` / `.dollars()`.
3. No `f32` anywhere (convention; a grep finds none).
4. No `HashMap` / `HashSet` (clippy `disallowed-types`; use `Vec`, `BTreeMap`, `BTreeSet`).
5. Systems are pure: `fn(&State, &Params, &mut Rng) -> Deltas`. They never mutate state; `sim-engine/src/apply.rs` applies the deltas. After every system `sim-checks` runs (money conservation, finite numbers, structure).
6. Transcendentals go through `libm` (clippy `disallowed-methods` bans `powf`, `powi`, `exp`, `ln`, `sqrt` on f64). This keeps native and wasm bit-identical.
7. `#![forbid(unsafe_code)]` in every logic crate. Only `sim-wasm` relaxes it to `deny`.
8. Randomness only from `sim_rng::Rng` (PCG32). No other source, no clock, no I/O in logic crates.
9. Every number is in `content/` JSON and read through `sim-params`. No magic numbers in rules. `Params` uses `deny_unknown_fields`.
10. Model crates never depend on `sim-api`. The wire is a view; it holds no rules.

## Hard rules: web

Enforced by ESLint (`web/eslint.config.js`, `no-restricted-imports` plus a local dynamic-import rule) and `web/tests/boundaries.test.ts` (lints the real tree and proves deliberate violations are caught).

1. `contract/` imports nothing (except `index.ts` re-exporting `./generated`). It is types only (that it holds no logic is a convention; do not add functions).
2. Only `sim-port/` knows the replay port and the wasm package exist. Nothing else may import `sim-port/replay` or `sim-port/wasm`.
3. `store/` may import only `sim-port/port` (the type), never the factory, replay or wasm. No Svelte, no Three.js in the store.
4. `scene/` is Three.js only: it must not import `panels`, `ui-kit`, `app`, `sim-port` or Svelte. `panels/` must not import `scene`, `three`, `app` or `sim-port`. They talk through the store (`sceneBus` is the channel).
5. The store is the only meeting point of scene, panels and sim-port. `app/` wires them together.
6. Dynamic `import()` needs a literal path and obeys the same boundaries.
7. Nothing in `web/` simulates. The recorded demo is data the Rust sim produced (`npm run gen:recording`); never hand-edit `recording.generated.ts`.
8. The web is a thin renderer (Town v6, `docs/town-v6.md`): no simulation rules, thresholds, model text or copies of `content/` numbers in `web/src`. Read them from the wire (the snapshot, or `catalog()` from `content/ui.json`; recipe in `docs/how-to.md` section 17). The web keeps layout, formatting, animation, chart scaling, onboarding copy and the wording of wire ids.

## Commands (all verified on this checkout)

Rust (repo root):

```
cargo test --workspace                          # all simulation, wire and golden tests
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p sim-cli -- --seed 42 --turns 40    # headless run; --plan <json>, --csv <file>, --set a.b=v, --quiet
cargo run -p sim-cli -- --seed 7 --plan content/scenarios/baseline_b_sensible.json --turns 40
cargo run -p sim-cli --example debug -- 12 3    # per-tick debug print (TURNS EVERY [plan.json])
```

Web (in `web/`; needs Node 22+ and `npm install` once):

```
npm run dev                # http://localhost:5173 ; wasm when built, else the recorded demo; ?sim=replay forces the demo, ?seed=42
npm test                   # vitest (43 files, 468 tests plus 12 skipped at last count with the wasm package built; without it the files that need the real sim are left out and the run says so)
npm run typecheck          # svelte-check + tsc
npm run lint               # eslint src tests (includes the module boundaries)
npm run build              # production bundle in web/dist (VITE_BASE sets the base path)
npm run gen:contract       # regenerate web/src/contract/generated.ts from the Rust wire types (after ANY change to a wire type in crates/sim-api)
npm run check:contract     # fails if generated.ts is stale (cargo test --workspace also does)
npm run build:wasm         # REBUILD WITH THIS AFTER ANY RUST CHANGE (else the browser runs an old package or the recorded demo); builds sim-wasm into web/src/sim-port/wasm/pkg (gitignored)
npm run gen:recording      # regenerate the recorded demo (web/src/sim-port/replay/recording.generated.ts) after ANY sim, content or wire change
npm run check:recording    # fails if it is stale (cargo test --workspace also does)
```

`npm run build:wasm` needs `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version <version in Cargo.lock> --locked` (the script prints the exact command). `wasm-opt` is optional.

Regenerating generated files (only after an intended change):

| Generated file | Command | Checked by |
|---|---|---|
| `docs/fixtures/*.json` | `cargo run -p sim-api --example fixtures` | `crates/sim-api/tests/fixtures.rs`, `web/tests/fixtures.test.ts` |
| `tests/golden/seed7.json` | `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` | same test natively; `web/tests/wasm-golden.test.ts` in wasm (skipped when `pkg/` is absent) |
| `web/src/sim-port/replay/recording.generated.ts` | `cd web && npm run gen:recording` | `crates/sim-api/tests/recording.rs` (runs in `cargo test --workspace`), `npm run typecheck` |
| `web/src/contract/generated.ts` | `cd web && npm run gen:contract` | `crates/sim-api/tests/contract_ts.rs` (runs in `cargo test --workspace`), `npm run typecheck` |
| `tests/snapshots/scenarios__market_seed_42.snap` | `INSTA_UPDATE=always cargo test -p sim-engine --test scenarios golden_snapshot_seed_42` (standard insta behaviour; not run here, `cargo-insta` is not installed) | `golden_snapshot_seed_42` in `tests/scenarios.rs` |

Playwright scripts in `web/playwright/*.mjs` are manual screenshot and check tools (need `npm run dev` running and `npx playwright install chromium`). They are not part of `npm test` and were not re-run when this file was written.

## Verification checklist before any commit

1. Rust touched: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
2. `content/` or Rust sim touched: regenerate in this order, then re-run tests. `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden`; `cd web && npm run gen:recording`; insta snapshot if the scenario output changed. Read the diff of each generated file and be able to explain every change.
3. Wire or contract touched: derive `TS` on the type and run `npm run gen:contract` (see `docs/how-to.md` section 19), `docs/snapshot-contract.md`, `docs/fixtures/` (see `docs/how-to.md`, "Change the contract").
4. Web touched or Rust sim touched: `cd web && npm run build:wasm` (so the wasm golden test actually runs), then `npm test`, `npm run typecheck`, `npm run lint`, `npm run build`.
5. `git status` shows only files you meant to change; no `.idea/`, no `pkg/`, no `target/`, no `dist/`.
6. CI (`ci.yml` on pull requests) runs `cargo fmt --check` and clippy as well as the tests; `deploy.yml` on `main` does not repeat them, so run them before pushing to `main`.

## Git conventions

- Stage explicit paths: `git add path1 path2`. Never `git add -A` or `git add .`. Never commit `.idea/` changes or the wasm `pkg/`.
- Message style: `Area: imperative summary` (history: `TownV2: ...`, `WireV2: ...`, `WebV2: ...`, `SceneV2: ...`, `PanelsV2: ...`, `Balance: ...`, `Docs: ...`). One logical change per commit; generated files go in the same commit as the change that caused them.
- End every commit message with the trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` (or the attribution the current session gives).
- Pushing `main` triggers the deploy workflow. Do not push unless asked.

## Do not

- Do not tune balance to hit a number. Report what the baselines show (`docs/known-issues.md`). Balance changes are design decisions; test across seeds and baselines A to D, not one run.
- Do not change the contract (the Rust wire types) without updating `docs/snapshot-contract.md`, the fixtures, the recording, and bumping `SCHEMA_VERSION` when saves or shapes break.
- Do not hand-edit generated files: `web/src/contract/generated.ts`, `web/src/sim-port/replay/recording.generated.ts`, `docs/fixtures/*`, `tests/golden/seed7.json`, `tests/snapshots/*`.
- Do not commit `web/src/sim-port/wasm/pkg/`, `target/`, `web/dist/`.
- Do not put numbers in code that belong in `content/`. Do not add `HashMap`, `f32`, wall-clock time or unseeded randomness to sim crates.
- Do not rewrite historical docs; add a status banner and fix verified errors only. Update the current docs listed in `docs/README.md` instead.
- Do not invent facts in docs. If unverified, say so.

## Honesty and verification

- Never say tests, lint, build or a command passes unless you ran it in this session and saw it pass. Quote the result (counts, exit status).
- If a command fails or you could not run it, say so plainly in your report; do not hide it or substitute a different check.
- Say what you did not verify (for example "not tested in a browser"). Do not claim a screenshot or playtest you did not do.
- Verify claims against the code before writing them in docs. When docs and code disagree, the code wins; fix the doc or list the mismatch in `docs/known-issues.md`.
