# Feature: Market trades (the list behind each market)

Status: **APPROVED by the user** (request: "see the ENTIRE LIST behind each market: the individual purchases and sales with units and price"). Builds on the Markets view (`docs/features/markets.md`, contract v2.3). Contract **v2.4**, additive. No simulation behaviour change: the state hash must stay byte-identical (`cargo run -p sim-cli --example statehash` before and after). Where this doc and the code disagree, the code wins.

## 1. Why
The Markets view shows only aggregates ("545 units bought, price $23"). A learner cannot see who bought what from whom. Each market card gets a **See all trades** list: one row per trade, with the people and firms as clickable references, units and prices, and a totals row that adds up to the number on the card.

## 2. What a "trade" is, per market (checked against the code)
| Market | Row | Columns | Counted in the totals |
|---|---|---|---|
| Food, Goods, Services | one purchase: a buyer (household, a firm buying capital goods, or the government) from a shop | Buyer, Seller, Units, Unit price, Total, Month | every row (units add up to the card's "bought") |
| Labour | one job: a person and an employer, plus open jobs and jobs lost | Worker, Employer, Wage a month, What changed | firm jobs (the card's "traded"); job guarantee, domestic staff, fired and open jobs are listed but not counted, and say so |
| Housing | one home: who lives there, who owns it, the rent | Home, Lives there, Owner, Rent a month, Status | occupied homes (the card's "housed"); vacant homes and homeless households are listed, not counted |
| Credit | one loan request of a firm: asked, approved, rate | Borrower, Lender, Asked for, Approved, Rate | approved dollars (the card's "traded") |
| Energy | not listed: the model has one town-wide supply and demand, no per-agent energy flows (verified: `State.energy` holds aggregates only), so the card says so | none | n/a |

## 3. Design decisions
1. **Capture, not state.** Per-trade detail is output only. Shops' purchases get a new `TraceRecord::Trade` pushed by `goods.rs` (pure addition to the already output-only trace; nothing reads it, so the hash cannot change). Labour, housing and credit rows are built in the wrapper from the state after each tick plus the tick's hire/fire events and the existing `CreditDecision` trace.
2. **Storage.** A `TradeStore` inside the API wrapper's `LedgerStore` (same clone-on-turn, same save blob, same `detail_turns`). Full rows for the last `ledger.detail_turns` turns. Older turns are rolled up per (market, buyer, seller, other party, note, turn) with exact unit and money totals; the month and the per-month price are dropped (unit price becomes total money over total units). Saved with the game; the field defaults to empty, so older saves still load (no `SCHEMA_VERSION` bump; the list simply starts at the load point).
3. **Units without drift.** A buyer's units are the shop's sold units times that buyer's share of the shop's revenue, so the rows add up to the card's "bought" exactly (up to float error). Money is the exact cents the ledger moved.
4. **Wire.** `marketTrades(market, query?)` (wasm `market_trades`) returns `MarketTrades`: period chosen, the periods available, `columns` (id, label, plain-English help, numeric), `rows` of `cells` (`text`, optional `sort` number, optional `entity` reference), a `totals` row, the card's own aggregate and `reconciles`. All labels and help come from `content/ui.json` (`markets.list[].trades`, `text.markets.trades_*`); a new market needs content and a Rust builder, and **no web change**.
5. **UI.** A **See all trades** button on each market card opens a modal: month/turn selector, sortable column headers (client-side only), the totals row, the reconcile line. Entities link to the inspector. Household and firm inspectors list the trades they took part in this month (the same call, with an `entity` filter).
6. **Recorded demo.** Like the ledger, the replay port refuses (`unsupported`), and the button says the list needs the live simulation. Measured in section 5.
7. No bare percentages anywhere in the new text; rates are "dollars a year for every $100 borrowed".

## 4. Tests
Rust: reconcile (rows' counted units and money equal the card's `traded`, and food money equals the ledger's purchase total) across seeds and all 40 turns and live months; roll-up keeps totals exact; save/load round trip; determinism (same seed, same trades). Web: model/sort/format; a made-up market renders from the wire alone. State hash identical.

## 5. Measurements
Filled in under "As built".
