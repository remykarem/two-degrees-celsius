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

## Contract issues (followed as written; not changed)
1. `levers()` returns `LeverInfo[]` keyed by `LeverKey`, so `switch_system` and the three place/remove kinds cannot be described by it. Costs for those come only from `preview()`; palette labels and hints live in `layout.json`.
2. `MetricKey` is used by `LeverInfo.arrows` but never defined. The scaffold invented a working set (gdp, unemployment, gini, emissions, budget, legitimacy, wellbeing, gpi, debt, prices, housing, forest).
3. `SimEvent` payloads and `TraceRecord` fields are not specified beyond variant names. The scaffold uses a flat `{ type, importance, text?, household?, firm?, amount? }` and invented trace fields.
4. `commons_access` has `unit: 'enum'` but numeric `min/max/step`; the value on the wire is `CommonsAccessWire` and `policy.nominal` is a string. The mock uses 0 = open, 1 = community with `cap: 0.5`.
5. `TurnResult.pcBefore/pcAfter` is ambiguous. The mock uses: before = PC when the turn was submitted; after = PC at the start of the next turn (after spending and regeneration).
6. `Snapshot.pc.regenNextTurn` and `gameOver.turn` are not defined precisely; mock: regen = round(8 x legitimacy), gameOver.turn = the turn number the game stopped on (161 when survived).
7. `policy.effective` is `null` for non-lobbied levers and `TurnResult.policyGaps` has numbers only, so `bool`/`enum` levers cannot appear in gaps.
8. No year or turn-count in the snapshot: the UI hard-codes start year and last turn in `store/config.ts`.
9. Whether money for a command is charged at queue or activation time, and whether cancelling refunds it, is unspecified. Mock: charged at activation; cancel refunds PC only.
10. `bailout` is a one-off but is a persistent `LeverKey` with a value. Mock resets it to 0 on activation.
