> **Status: HISTORICAL** (web scaffold stage, contract v0.1, mock only). Current web notes: `docs/web-notes-scene2.md`, `docs/web-notes-panels2.md`, `docs/web-notes-v2.md`; architecture: `docs/architecture.md`.

# Web notes (scaffold stage)

Status: scaffold only. The turn loop runs end to end against a **mock** simulation (`web/src/sim-port/mock`, numbers in `data.json`, not an economic model). The scene and panels are placeholders.

## Commands (run in `web/`)
`npm run dev`, `npm run build`, `npm test`, `npm run typecheck`, `npm run lint`. Visual check: with dev running, `node playwright/shots.mjs <outDir>` (needs `npx playwright install chromium`).

## Boundaries
Enforced by ESLint `no-restricted-imports` per folder (`eslint.config.js`) and `tests/boundaries.test.ts`, which lints the real tree and also checks that deliberate violations are caught. Store may only import the type `sim-port/port`; only `sim-port` imports `sim-port/mock`.

## Placeholders to replace
- Scene: coloured boxes, no lens colouring, no playback animation (flows are not drawn), no zoom.
- Panels: no system-switch comparison modal, no crisis banner, results "why?" prints raw trace JSON, end screen has no causal story, inspector shows only tile contents.
- UI-owned layout: `store/layout.json`. Initial buildings must match the sim's starting infrastructure (mock: 2 public housing, 3 fossil, 1 renewable). `remove` has no UI; the store drops the last placed building of that kind when a remove activates.
- `sim-port/wasm/` is an empty documented stub.

## Contract issues (all resolved in contract v0.2, `docs/snapshot-contract.md` section 9)
1. `levers()` could not describe actions: resolved, section 9 item 1 (`actions()` / `ActionInfo`).
2. `MetricKey` undefined: resolved, item 2 (closed set in `contract/index.ts`).
3. `SimEvent` / `TraceRecord` payloads: resolved, item 3 (payload unions, no text; wording in `panels/format.ts`).
4. `commons_access` enum min/max: resolved, item 4 (`LeverInfo.options`, min/max/step 0).
5. `pcBefore` / `pcAfter`: resolved, item 5 (mock matches).
6. `regenNextTurn`, `gameOver.turn`: resolved, item 6 (float regen, 161 when survived).
7. `policyGaps` numeric only: resolved, item 7 (only carbon price, wealth tax, profit tax, emissions cap are lobbied in the mock).
8. No calendar: resolved, item 8 (`Snapshot.game`; `store/config.ts` has no constants).
9. Money timing: resolved, item 9 (charged at activation, pending counts as committed, cancel refunds PC only).
10. `bailout`: resolved, item 10 (persistent boolean, never reset).

Remaining UI default: the `cap: 0.5` sent with `commons_access: community` is chosen in `PolicyDrawer.svelte` (the contract gives no default).
