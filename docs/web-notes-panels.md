# Web notes: panels (UI builder)

Scope: `web/src/panels/**`, `web/src/ui-kit/**`, `web/src/app/App.svelte`, tests `web/tests/panels-*`, scripts `web/playwright/panels-*.mjs`.

## Layout of the code
- Pure logic (no DOM, unit tested): `format.ts` (numbers, the one event/command formatter), `metrics.ts` (metric defs, runway projection), `crisis.ts`, `results.ts` (ranking, deltas, lobbying lines), `why.ts` (trace to bar data), `story.ts` (end-screen causal story), `policy.ts` (lever groups, disabled reasons, lobbying note), `inspect.ts` (selected tile to read-only model), `systemSwitch.ts`, `content/systems.ts` (system copy), `coach.ts`.
- `ui.ts`: panel-only UI state (drawer open, system modal, why panel, coach). Not game state.
- `ui-kit`: `Modal` (focus trap, Escape, focus return, topmost-only keyboard), `Segmented`, tokens (light, dark tokens ready).
- Checks: `npm test` (includes server-rendered component smoke tests), `node playwright/panels-shots.mjs <dir>` (screenshots at 1440x900 and 1024x768), `node playwright/panels-a11y.mjs` (focus trap, Escape, live regions, reduced motion). Both need a dev server on port 5175.

## Behaviour worth knowing
- Selection and hover come from `state.selection` / `state.hover` (the scene writes the same values to its bus). The inspector shows the selection, or the hovered tile when nothing is selected and no build tool is active.
- Removing a building: inspector button on a built building, or "Remove one" in the palette (keyboard route). One removal per kind per plan (the store's slot rule). The store removes the LAST placed building of that kind from the layout on activation, not necessarily the one that was selected. This is cosmetic (position has no effect) and the inspector says so.
- Direction arrows show arrows only, never ranges. The lobbying note uses `preview.opposition`: "you set X, expect about Y in force" interpolates between the current and drafted value.
- Money is shown as "on activation" (contract v0.2 item 9). The PC limit for a lever or building uses `baseCostPc` against PC not yet committed by the draft; the sim's `preview()` stays the authority and a blocked draft shows its reason.
- Crisis thresholds: legitimacy below 0.2, budget share below 15% (from the brief). Bank failed, commons collapsed, collapse, budget exhausted and policy_blocked come from events or the snapshot.
- Runway uses `nature.netEmissionsMonthly * 3` as the per-turn rate and `remainingBudget / budgetShare` as the starting budget. If the real sim's budget is consumed by gross rather than net emissions, change `runway()` in `metrics.ts`.
- Story templates only use `snapshot.history`, the last `TurnResult.trace` and `policyGaps`. The lobbying line therefore describes the final turn, not an average over the run.

## Placeholders / not done
- System copy: marked "placeholder" where `docs/causal-model.md` section 3 is silent (who resists a switch back to market, who resists the co-op switch). Transition months and output hits come from that table; the mock uses 12 months for every system.
- "Why?" mapping per metric is a judgement call (GDP, GPI, unemployment, Gini, wellbeing -> income trace; emissions, budget, temperature -> emissions trace; legitimacy -> lobbying gap). Metrics with no matching trace have no why link (debt, forest, commons).
- Only the mock's trace shape is exercised: emissions by source, wages only for income, and policy gaps. With the real sim, income by type will show more kinds without code changes.
- Dark theme: tokens exist and are used everywhere; not visually reviewed.
- Onboarding highlights elements via `data-coach` attributes; it is skippable and remembered in `localStorage` (try/catch guarded).
- Map keyboard operation (selecting tiles) belongs to the scene; the palette's remove buttons and the policy drawer are fully keyboard operable.

## Contract issues
None blocking. Observations:
1. `LeverInfo` has no unit label for `usd` levers (min wage, benefits, UBI): the UI shows plain dollars and cannot say "per hour" or "per month".
2. `ActionInfo.availableIn` for `switch_system` is interpreted as "systems you can switch FROM"; the contract does not say.
3. `TraceRecord` carries no period, so "tonnes" in the emissions why-panel has no time unit.
4. `Command.remove` has no tile, so the layout cannot know which building was removed (see above).
