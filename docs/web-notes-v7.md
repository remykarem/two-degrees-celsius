# Web notes, Town v7

## Panels (Town v7, contract v1.9)

What the panels render from the wire, and where. All wording is `catalog().text.estate` (`content/ui.json`); numbers are the wire's.

- `web/src/panels/estate.ts` (pure): `estateCard`, `pilotView`, `topHouseholdLine`, `giniRows`, `estateOwnerSections`, `pilotSections`, `wealthTaxShortfall`. `EstateCard.svelte` is the card at the top of the Society tab: tag and wealth band, "1 of 9 households, 1 of 22 people hold $X of $Y", what it owns (Gini `parts`), income by source, jet flights beside the town's emitted tonnes, the pilot (status, wage, short-paid months, a warning before dismissal, the grounded note), and the estate's money (last month and last 12 months).
- Finance: a "domestic wages" memo row inside the households section (it is part of `wages`, not added to it), an estate money table with an Estate ledger button, and the shortfall note. The estate's carbon charge also appears in the government's carbon revenue row, as before.
- Inspector: the estate owner (tag, band, Employs, flights, estate money) and the pilot (Job line "Employer: Estate owner (household)", wage, commute to the hangar, status, short-paid months, flights); a household card for the estate gets the same sections. `people.ts`: the pilot is "employed", not "looking for work", and is not counted unemployed.
- Gini view: richest first, the tag and wealth band, what each net worth is made of, the share as "$ of $" (counts, not bare percentages), the Lorenz curve labelled at its last step. The Society "wealth of the richest household" line now names the household with counts.
- Emissions: the aviation row and the reconciliation line were already in place (contract v1.9 work); checked in the real app that gross = firms + households + plants + aviation reconciles.
- Policies: a "paid in part" note under the wealth tax row and at the top when the estate's savings fall short; the unchanged-row filter still hides the row until the player changes the tax. Industry: the Farm's owner reads "Estate owner 1.00" (the household label, plus the tag when it differs).
- Events and crisis: `pilot_let_go`, `pilot_hired`, `private_jet_flight` were worded already (format.ts, coverage test). New: a warning banner `pilot_let_go` while the pilot is let go, and the end story's estate sentences.
- Explorer: the estate kind and the `estate_flight` draw come from `agents()` and render with no web change (checked in the generic list; not screenshotted).
- Mock: new text keys flow through `npm run sync-mock`; the parity tests (`mock-sync`, `thin-renderer`) pass. The mock still charges the estate no wealth tax or carbon charge (V7-8).
- Tests: `web/tests/panels-estate.test.ts`. Screenshots: `node playwright/v7-panels-shots.mjs <dir>` (needs `npm run dev -- --port 5191` and `npm run build:wasm`).
- Real-app findings: V7-6 (the owed wealth tax is estimated) and V7-10 (the pilot's earnings name the Government as payer) in `docs/known-issues.md`. One test of the scene agent's area (`scene-entities`, homes per class) was failing in the working tree mid-session; not touched.
