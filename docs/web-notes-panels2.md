# Web notes: panels for Town v2

Scope: `web/src/panels/**`, tests `web/tests/panels-v2.test.ts`, scripts `web/playwright/panels-people-shots.mjs` and `panels-end-shots.mjs` (dev server on 5181, screenshots outside the repo).

## What was added
- People tab (4th tab of the right column, `PeoplePanel.svelte`, logic in `people.ts`): 8 clickable household rows (label, class, people, home or "no home", net worth, monthly income; sortable), members grouped by household (narrowed to the selected household). Lists are button groups with `aria-pressed`, Up/Down move between rows, selection is announced through a polite live region. Selecting goes through `game.select`, so the scene highlights the home; selecting on the map (home, person, household) highlights the list row (`selectedHouseholdId`). Read-only.
- Inspector: person (age band, household link, job and firm, wage, skill, commute tiles and minutes, wellbeing, home and rent or UNHOUSED) and household (members with jobs, home, owner, rent, rent arrears, savings, debt, net worth, income all sources and the five-way split, tax, rent paid, loan payments, spending, wellbeing, political power). The unhoused household says "UNHOUSED (no home)".
- Metrics: average and median income per person (Economy), unemployment with "N of M unemployed" (people with `inLabourForce`), wealth of the richest household (Society), sparklines from the new HistoryPoint fields. Income metrics link to the income trace.
- Fixes: briefing, policy row and why-bars mention lobbying only when nominal differs from effective (`lobbyingDiffers`); save/load message is a floating toast (header no longer wraps); event wording for household_moved / evicted, person_hired / fired, purchase, built, removed, electricity_rationed uses household and person labels; the emissions why-chart names a plant ("Fossil plant at (10,2)"); header does not wrap at 1024 px.
- End screen: median and average income, richest-household wealth share (start and end), unhoused households and evictions in the final turn. Templates only; numbers come from history, the final snapshot and the last result.

## Cost mismatch (plant "about $40k" versus about $2M)
Findings, measured on the current wasm package (seed 7):
- `actions()` says `place:fossil_plant` moneyCost 40,000; `preview()` says money 40,000; the activation turn raises government debt by about 38,000. Public housing 40,000, road 2,500 on all three. So on this package the palette and preview agree and the real charge is about $40k.
- The $2M (and 14,000 for housing) figures are Town v1 values still written in `docs/sim-api-notes.md` (lines 13 and 42). `content/params.json` was rebalanced to 40,000 in the Town v2 content commits. Any build from before that, or a reader going by the notes, sees a $2M plant.
- Structural root cause: the palette showed `ActionInfo.moneyCost`, a static table the sim fills in `crates/sim-api/src/meta.rs` from `params.government`, while the charge comes from `build_cost` in `crates/sim-engine/src/turn.rs`. Two code paths read the same parameters; nothing forces them to stay equal, and a stale table is shown with no label.
Fix (panels and store only): `game.estimateBuild(kind, tile?)` asks `port.preview()` for a single place command (the hovered valid tile while building, else the first of `validTiles(kind)`) without drafting. The palette shows that price; the action table is only a fallback, labelled "Est." and "(estimate)". `costLine` and `estimateDisagrees` (panels/costs.ts) are tested, plus a test that `estimateBuild` equals the drafted preview. Plan tray already used preview values. Not changed: `meta.rs` should derive moneyCost from the same function as `build_cost` (contract owner), and `sim-api-notes.md` should be corrected (not my file).

## Rough
- Inspector is a card over the map; the household card is long and scrolls at 1024x768.
- People tab at 280 px is dense; no person sort or filter; no wellbeing breakdown or commute chart.
- Removal of occupied public housing is still only through the tile or vacant-home card.
- Evictions in the end story only cover the final turn (no per-game count on the wire).
- Wealth-tax threshold shows as money; with net worths of tens of thousands the lever range (up to $5M) is mostly beyond the town; not changed.
- A11y: focus is not moved into the inspector on select (it stays on the list row on purpose); the inspector close button has no Escape handler.

## Contract issues
1. Price duplication between `ActionInfo.moneyCost` and `preview().money` (above). Suggest dropping `moneyCost` from the static table or documenting it as "typical".
2. `docs/sim-api-notes.md` costs are stale (14,000 / 2,000,000).
3. `Indicators.avgIncomePerPerson` and `medianIncomePerPerson` have no unit in the contract; they are per month (checked: average times people equals the sum of household incomes).
4. No evictions or moves count in `HistoryPoint`.
