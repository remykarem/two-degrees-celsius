# Town v8: net worth breakdown

Status: **APPROVED (user: "just do net worth").** No new simulation behaviour: the state hash (`cargo run -p sim-cli --example statehash`) must be byte-identical before and after. The wire grows additively to contract v2.2. (GST and state ownership, which used to be v8, is now `docs/town-v9.md`; daily ticks is `docs/town-v10.md`.)

## 1. Goal
Show a statement of net worth per household and per person: assets minus liabilities, every line with its own "how?". Show the town's net worth split by holder. Both reconcile exactly to the headline net worth that the Gini and the Lorenz curve use, and say so with a `reconciles` flag, like the emissions tally.

## 2. What net worth is today (checked in code)
`sim_metrics::household_wealth` (`crates/sim-metrics/src/lib.rs`), in cents per household:
- `savings - debt` of the household;
- homes owned (`HomeOwner::Household`) at `Home.value`, the occupied one included;
- the bank's `equity` for the bank's owner (`bank.owner`, household 0);
- the jet's `value` for its owner (the estate household, Town v7);
- for each active firm, equity = `capital x capital price + cash + inventory x price - debt`, to the owner household, to the bank's owner for `Owner::Bank`, shared equally among the households of the workers for `Owner::Workers`, and to nobody for `Owner::State` (no system sets it today).
The Gini and Lorenz curve (`breakdowns.gini`) give each person an equal share of their household's figure (`person_wealth`); `byHousehold[].netWorth` is that figure and v1.9 already lists coarse `parts` (savings minus debt, homes, jet, bank, firms).

Not in net worth, so not a line: unpaid rent (the model keeps `arrears_months`, a count of months, not an amount owed); government cash and debt; state-owned firms; pensions, possessions, human capital. The statement says so (`text.networth.unpaid_rent_note`) rather than inventing a figure.

## 3. The statement
Lines are a list `[{id, label, category: asset|liability, value, perPerson, count?, howId?}]`; zero lines are left out; liabilities are positive magnitudes; `netWorth = assets - liabilities`.

| id | category | what |
|---|---|---|
| `savings` | asset | deposits at the bank |
| `homes` | asset | homes owned at value (`count` = how many) |
| `firm_capital`, `firm_cash`, `firm_inventory` | asset | the household's share of the books of the firms it owns (`count` = how many firms) |
| `bank_equity` | asset | the bank's equity, for the bank's owner |
| `jet` | asset | the estate's jet |
| `loans` | liability | the household's own debt |
| `firm_loans` | liability | the same share of those firms' debt |

Labels, categories and `howId` (an explorer anchor) come from `content/ui.json` `net_worth_lines`, validated at load against the closed set the model totals (like `emission_sources`). A new kind of asset is: a `WealthLines` field and its `match` arm in `sim-metrics`/`sim-api/src/networth.rs`, one `ui.json` entry, an assumptions anchor; the web needs no change because it iterates the list. `howId`s are covered by two new explorer topics in `content/assumptions.json` (`net_worth`, `net_worth_firms`), checked by `crates/sim-api/tests/assumptions.rs`.

Per person: the Gini treats a person as an equal share of the household, so each line carries `perPerson` (value / people). The person inspector shows the household's statement at `perPerson` with the sentence "1 of N people".

## 4. The town view: who holds what
`breakdowns.gini.town`: holders `households`, `firm_owners`, `bank`, `estate`, plus the memo row `government`, each with households and people counts, assets, liabilities and net worth. Every household is in exactly one group (estate first, then the bank's owner, then owners of or shareholders in a firm, then the rest). The four household groups add up to the headline (`holdersTotal` vs `headline`, `reconciles`). `government` (cash minus debt, plus any state-owned firms) is `inHeadline: false`: shown as a memo, outside the Gini. Labels and notes: `content/ui.json` `net_worth_holders`.

## 5. Where it shows
Person inspector (the household's statement per person) and household inspector (full statement); the household cards in Society (Gini view); the estate card; a Society section "Who holds what". Dollar amounts and counts everywhere, no bare percentages. The web reads the lists from the wire; it computes no net worth and recomputes no sum (a "reconciles" badge reads the wire flag).

## 6. Wire change (additive, contract v2.2)
- `breakdowns.gini.byHousehold[].statement` (`NetWorthStatement`) and `.holder`; `breakdowns.gini.town` (`TownNetWorth`); `catalog.netWorthLines`, `catalog.netWorthHolders`.
- ts-rs types in `crates/sim-api/src/networth.rs`, listed in `contract_ts.rs`; `npm run gen:contract`. `parts` (v1.9) stays and is now derived from the same lines.
- Regenerate the fixtures, the golden (`UPDATE_GOLDEN=1`, twice) and the recording (`npm run gen:recording`); every changed golden hash is the new fields, and the state hash is unchanged. `SCHEMA_VERSION` (saves) is unchanged: saves hold state, not the wire.

## 7. Tests
Rust (`crates/sim-api/tests/networth.rs`): every household reconciles to `household_wealth` and to `HouseholdView.netWorth` in five seeds, at start and over 40 turns, in live months, and in a cooperative economy; lines sum to assets and liabilities; the town reconciles and counts every household and person once; unknown or missing line ids in content are refused. Web: panel logic reads the wire; a made-up asset line added to a snapshot (and to the catalog) appears in every view without a web change. Explorer coverage test for the new anchors.

## 8. Out of scope
No new simulation behaviour (state hash identical); no change to what counts as net worth, no new tax or balance effect; no unpaid-rent liability (not modelled); state ownership and shares (v9).
