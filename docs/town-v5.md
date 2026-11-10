# Town v5 design: information release (approved)

Status: **APPROVED (user: make v5 "more info-friendly" and expose more metrics; daily ticks are deferred to `docs/town-v6.md`).** No economic rules change in v5: it makes the existing model **visible and traceable**. Numbers and labels are defaults for the user to adjust.

## 1. What the user asked for
1. A **Finance tab**.
2. For **every entity (government, bank, household, person) the past transactions**, so any number can be traced back to its original value.
3. A unit of measurement for the **power grid** (it is not shown anywhere).
4. Clarity on whether **people spend at shops**.

## 2. Answers about the current model (checked in the code)
- **Power grid units.** Electricity is in abstract "energy units" per month (`energy.plant_capacity = 5.0` per plant; households use `0.1 + 0.15 x consumption/2000` per person; firms use `energy_per_unit` per output). Fossil plants emit `factor_fossil = 1.0` t CO2 per unit. **The UI shows none of this and no unit.** The magnitudes fit **MWh per month** (a person uses about 0.1 to 0.25 MWh a month; 1 t CO2 per MWh is coal-like), so v5 labels the unit **MWh/month** [PROPOSED] and surfaces supply, demand, rationing and intensity. No model change is needed; the label lives in one config field (`energy.unit_label`) so it can be changed.
- **People and shops.** Yes in the model: households split their monthly budget between food (Farm), goods (Factory) and **services (Shop)** (`goods.rs`, one `Purchase` event per household and firm). It is **not visible**: the scene draws coins flowing to firms, but figures never walk to a shop. v5 adds shopping trips (cosmetic, driven by real purchase flows) and, more importantly, shows where each household's money went in its ledger.

## 3. Finance tab (new panel, all values from the sim)
Sections, each with a monthly value, a trailing-quarter total and a sparkline from history:
- **Government:** revenue by source (wage tax, profit tax, capital tax, wealth tax, carbon revenue), spending by category (benefits, UBI, public purchases, job guarantee, road/plant upkeep, subsidies, interest), balance, debt, debt/GDP, legitimacy.
- **Bank:** equity, loans, deposits, capital ratio vs the limit, lending headroom, default rate, interest income, bailouts.
- **Households (sector and per household):** income by type, taxes, rent, loan payments, spending, saving rate, debt, net worth.
- **Firms (sector and per firm):** revenue, wage bill, profit, investment, debt, cash.
- **Money:** supply, created and destroyed this turn, with the sector-balance identity the sim already checks.
- **Circular flow:** a simple flow diagram (households, firms, government, bank) for the last quarter.
- **Prices and distribution:** price index, wage share, profit share, Gini, median vs average income.

## 4. Transaction ledgers (the traceability feature)
**Every money movement becomes a ledger entry**, and each account's ledger **reconciles**: `opening balance + sum of entries = closing balance`, checked per account on every tick.

- **Accounts with a ledger:** each household, each firm, the government, the bank. **Persons have no purse in the model** (money is household-level), so a person's ledger lists **wages earned** (each credited to their household), **employment events** (hired, fired, commute changes) and their share of nothing else; it says so. [PROPOSED]
- **Entry fields:** turn, month, tick, from, to, amount ($), kind, memo (for example "wage, Factory, Adult 2", "rent, home 3", "interest, loan"), and the account's running balance after the entry.
- **Kinds:** wage, dividend, rent, benefit, tax (by type), purchase (by sector), lobby, interest, loan_new, loan_repay, loan_writeoff, build, upkeep, subsidy, bailout, wealth_consumption, neutral (mobility, liquidation, payout).
- **Coverage rule:** all balance changes must be expressed as ledger entries (transfers, and bank money creation and destruction recorded against an `external` account such as "bank credit creation"). The Rust agent audits every delta that touches a balance. Anything it cannot attribute is reported as a bug.
- **Storage [PROPOSED]:** the ledger lives in the **API wrapper** (like history), not in the model state, so the simulation and its golden hashes are unaffected. Full-detail entries for the last **8 turns** (config `ledger.detail_turns`); older periods are kept as **monthly aggregates per (account, counterparty, kind)** for the whole game, plus **opening balances at turn 0** and closing balances per turn, so a value can be traced all the way back to the start with exact totals even when old detail is rolled up. Saved with the game.
- **UI:** a **Transactions** section in the inspector for household, firm, government and bank (and a person's earnings and events), filter by kind and period, with "open counterparty" links, a running balance column, and a reconciliation badge (opening + entries = closing). Metrics link to the ledger where they come from (for example debt, savings, tax revenue).

## 5. Energy panel
Supply (per plant, kind, MWh/month), demand (firms, households), served share, rationing events, grid intensity (t CO2 per MWh), renewable share, emissions by plant. Unit label from config. Ecology tab or its own tab. Hover text explains the unit.

## 6. Wire (contract v1.4, additive; section 14 of `docs/snapshot-contract.md`)
`snapshot.finance`, `snapshot.energy`, `SimPort.ledger(account, query)` returning a `LedgerPage` (opening, entries, rolled-up summary rows, closing, `reconciles`), the existing `purchase` flows, and new per-kind totals in `HistoryPoint`. Schema version unchanged unless the saved wrapper blob changes (it does: the ledger is saved; bump `schemaVersion` to 4 and refuse older saves cleanly).

## 7. Delivery plan
1. **Rust agent:** ledger capture in the engine (per-tick transaction list alongside events), audit of every balance-changing delta, ledger store with roll-up in `sim-api`, finance aggregates (monthly and trailing quarter), energy metrics, wire v1.4, reconciliation and coverage tests, fixtures, golden, wasm.
2. **Web agent (parallel, on the mock first):** Finance tab, ledger viewers in inspectors, energy panel, shopping trips in the scene, contract types, mock ledger with the same rules.
3. You play and tell me which numbers you still cannot trace; independent review.

## 8. Tests
- Per-account reconciliation every tick, native and wasm; total of all ledgers equals total money movement; roll-up preserves totals exactly; save/load round trip; ledger memory bounded; every `Delta` variant that changes a balance has a ledger kind (a test enumerates them).
- Web: ledger viewer logic, roll-up display, filters, finance derivations, energy formatting, shopping-trip planner.

## 9. Risks
- **Coverage gaps:** a balance change with no ledger entry would break reconciliation; the strict per-tick check turns that into a test failure, not a silent gap.
- **Data volume:** entries per turn could reach a few thousand; the detail window and roll-up bound it.
- **No behaviour change:** state golden hashes should be unchanged; wire hashes change.
- **Unit label is cosmetic:** if the user wants realistic physical units everywhere, plants and demand would need re-scaling and a balance pass.
