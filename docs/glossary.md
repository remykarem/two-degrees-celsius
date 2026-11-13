# Glossary

Domain terms first, then code terms. Values are the current content defaults (check `content/` if in doubt).

## Game and economy

| Term | Meaning |
|---|---|
| **Turn** | One player decision round = one quarter = 3 ticks. 40 turns per game (`run.turns`; was 160). `State.turn` starts at 1 |
| **Tick** | One simulated month. `step()` runs every system once. `ticks_per_turn` = 3. 48 extra warm-up ticks run before the game starts |
| **Month frame** | The per-tick slice of a `TurnResult` (`months[0..2]`): events plus flows, used for playback animation |
| **PC (political capital)** | The per-turn currency for player actions. Start 60, cap 100, regenerates `pc_base x legitimacy` (8 x legitimacy) each turn. Spell out "political capital" on first use in player text: "capital" alone is ambiguous (owner class, capital income, capital tax, capital goods, bank capital ratio) |
| **Lever** | A numeric (or on/off, or enum) policy setting the player can change (20 of them: taxes, minimum wage, carbon price, caps, ...). Listed in `content/levers.json`. "Policy" means the set of lever values |
| **Action** | A build or switch the player can queue: `switch_system`, `place:<kind>`, `remove:<kind>` (the `actions()` wire) |
| **Command** | What is submitted: a lever change or an action. Each has a PC cost and a lead time |
| **Pending (queue)** | Commands accepted but not yet in force. Each has `effectiveTurn = submitted turn + lead time` |
| **Lead time** | Turns between queueing and effect (per command kind, `params.turn.levers.<key>.lead_time`; carbon price 2, most policies 1) |
| **Cancel refund** | Cancelling a pending item returns 50% of its PC (never money) |
| **Opposition** | Extra PC cost factor `(1 + net_lobby)` on levers capital opposes |
| **Active (nominal) vs effective policy** | Active/nominal = what the player set and has taken effect (`State.policy`). Effective = after lobbying (`State.effective`): `nominal x (1 - net_lobby)` for carbon price, profit tax, wealth tax, caps |
| **Lobbying gap** | The difference between nominal and effective for those four levers (`policyGaps` in a turn result). Capital owners dampen rules that hurt them |
| **Economic system** | A bundle of ownership and distribution rules: Market, Regulated, Coop (`content/systems/*.json`). Switching is slow (transition months, output penalty, resistance). Not the same as a "tick system" |
| **Tick system (S0-S12)** | One rule module in `sim-model` (labour, production, ...). See `docs/architecture.md` section 4 |
| **Person** | One of 22 individuals: age band (child/adult/retired), skill, job, wage, commute, wellbeing. Work, skill, commuting and wellbeing are person-level |
| **Household** | One of 9 groups of 1-4 people (the estate household is one person). Money, spending, saving, rent, loans, assets and political power are household-level |
| **Labour force** | Adults who look for work. 11 of the 15 adults (owners and the estate owner are outside it; the jet's pilot is in it). Unemployment is shown "N of 11" |
| **Unemployment (smoothed)** | `unemployment_smooth` is an exponential average (weight 0.05) used for wages, legitimacy and the collapse test, because one job is 10 points |
| **Class** | Derived per household every tick: Capitalist, Rentier, Manager, Worker, Dependent. Based on ownership, income source, skill and pay |
| **Home** | A tile holding at most one household (public housing is a home whose owner is the public, not a separate kind). Private homes pay rent to an owner household; public housing pays a fixed rent. 8 homes for 9 households |
| **Unhoused** | A household with no home (starts with one). Pays no rent, suffers wellbeing and security penalties. Evicted after 3 months of unpaid rent |
| **Tile** | One cell of the 12 x 12 map owned by the sim. Holds at most one facility or stock (the estate is a bundle of several tiles, see Estate): home, firm, plant, bank, town hall, forest, commons, road, estate grounds, runway or nothing |
| **Road** | A tile that makes commutes faster (`fast_minutes` 2 vs `slow_minutes` 6 per tile). Costs $2,500 and $10 a month upkeep |
| **Commute** | Tiles and minutes from a person's home to their firm along the cheapest route. Long commutes cut effective hours and wellbeing |
| **Plant (power plant)** | Say "power plant" in player text. A power plant tile: fossil (emits `delivered x factor_fossil`) or renewable (emits 0). $40,000 each. Capacity 5 per plant |
| **Electricity rationing** | If demand exceeds total plant capacity, all output is scaled by `supply / demand` (`electricity_rationed` event) |
| **Firm slots** | Factory (goods), Farm (food), Shop (services), plus one dormant slot a household may found a firm in (`MAX_FIRMS` = 4) |
| **Maximum sustainable yield** | The most a regrowing stock can regrow in a month at its best (`growth rate x maximum / 4`, reached at half the maximum). Take no more than that every month and the stock can hold steady. It is not the same as 'sustainable at this stock': what counts is what the stock regrows at its present size |
| **Overshoot** | Taking more than a stock regrows. The stock shrinks, so it regrows less, so the same harvest is further over the line; below a low level it collapses. The Ecology view shows 'taken for every 1 regrown' and the months left at the present rate |
| **Tragedy of the commons** | Nobody owns a shared stock, so each user gains from taking more and all share the loss. Here the Farm does not choose to over-take: demand and the harvest lever set it |
| **Food cap (nature cap)** | The most food the Farm can grow next month, set by last month's commons and forest stocks (a set share of the stock over the stock taken per unit of food). It does not bind in the starting town; a run-down commons makes a real food shortage, not a price rise |
| **Forest, commons** | Natural stocks. Commons access is `open` or `community`; can be enclosed or collapse |
| **Budget b0** | `nature.budget_b0`, the town's cumulative-emissions allowance: 900 t. Exhausting it ends the game (`budget_exhausted`) |
| **Legitimacy** | 0..1 public support. Moves with wellbeing, unemployment, Gini, debt, bailouts. Scales PC regeneration. Zero = collapse |
| **Collapse** | Loss by `legitimacy_zero`, `mass_unemployment` (smoothed unemployment > 0.35) or `bank_failure_cascade` |
| **GPI** | Genuine progress indicator: `annual GDP x (1 - Gini) - social cost of carbon x emissions - depletion cost` (`sim-metrics`) |
| **Gini** | Wealth inequality over the 20 persons' wealth (household wealth shared equally), clamped 0..1. Start about 0.72 |
| **Top wealth share** | Richest household's share of positive household wealth. Start about 61% |
| **Wellbeing** | Per person 0..1; average feeds legitimacy and GPI |
| **Outcome** | `survived`, `budget_exhausted` or a collapse reason |

## Code and process

| Term | Meaning |
|---|---|
| **State** | Everything the sim knows (`sim-types/src/state.rs`). Serialised in saves |
| **Params** | Parsed `content/` numbers (`sim-params`) |
| **Deltas** | What a system returns: money operations, field changes, events, trace. Applied by the engine |
| **Cents** | `Cents(i64)` money type. Dollars only at the edges (wire, params) |
| **Trace** | Per-tick records explaining where numbers came from (emissions by source, income by type, policy gaps); feeds the "Why?" views |
| **Event** | Something notable that happened (hired, evicted, built, ...). Has an importance 1-3 on the wire |
| **Flow** | A movement of money, energy or emissions between places (commutes, purchases, wages, rent, emissions in a month frame) |
| **Frame** | Only the per-month animation record (`MonthFrame`), which carries the month's flows and events |
| **Emission source** | One entry of an emissions line (`breakdowns.emissions.month.sources`): firms, homes, power plants, private jet flights. Ids are a closed set in `crates/sim-api/src/sources.rs`; labels and categories (`industry`, `homes`, `energy`, `aviation`) are in `content/ui.json` `emission_sources`. Not the same as an income source |
| **Income source** | One entry of a household income list (`HouseholdView.income.sources`, `finance.households.income`): wages, dividends, rent received, interest, benefits, and domestic wages as a part of wages. Labels and categories (`labour`, `capital`, `transfer`) are in `content/ui.json` `income_sources` |
| **Contract** | The wire format between sim and UI: `docs/snapshot-contract.md` + `web/src/contract/index.ts`. Versions v0.1, v0.2, v0.3, v1.0 (Town v2) |
| **Schema version** | `SCHEMA_VERSION` = 2. Layout of state/wire; mismatching saves are refused |
| **Snapshot** | The full current view of the game sent to the UI (`Snapshot`) |
| **SimPort** | The TypeScript interface the UI uses to talk to a sim (`web/src/sim-port/port.ts`) |
| **Recorded demo vs real sim** | Recorded demo: when wasm is unavailable, a replay of one real run (`sim-port/replay`, `recording.generated.ts`), badged "Recorded demo"; choices cannot change it. Real: the Rust sim compiled to wasm (`sim-wasm`), "Real simulation" badge. (The hand-written TypeScript mock was deleted in the platform cleanup phase 2.) |
| **wasm pkg** | `web/src/sim-port/wasm/pkg/`, built by `npm run build:wasm`, gitignored |
| **Golden** | A recorded run (`tests/golden/seed7.json`, from `plan.json`) whose per-reply hashes must match natively and in wasm. Also an insta snapshot for seed 42 |
| **Fixtures** | Real wire JSON in `docs/fixtures/`, generated by the sim-api example; the web tests check their types against it |
| **Plan** | A JSON turn script (`content/scenarios/*.json`) the CLI and tests play |
| **Baselines A-D** | A no action, B sensible play, C neglect/extraction, D carbon tax only. 20 seeds each in `tests/scenarios.rs` |
| **Draft** | UI-only list of commands not yet submitted; each has a `preview()` |
| **Phase** | UI state machine step: review, plan, playback, results, ended (`store/phase.ts`) |
| **Lens** | A map colouring mode: none, emissions, class, wealth, employment, flows, commute |
| **Scene bus** | `store/scene-bus.ts`: channel where the scene publishes hover/selection and panels send camera requests |
| **Town v1 / v2** | v1: 6 cohorts standing for 1,000 people, UI-owned map. v2 (current; Town v7 made it 22 people in 9 households): 20 people in 8 households, sim-owned map, roads, plant emissions, Shop. Anything describing cohorts is historical |
| **Agents catalogue** | `content/assumptions.json` `agents` section, served by `agents()` (contract v1.7): agent kinds, what each decides and observes, its rule with live parameters, and the randomness table. Shown in the explorer's Agents tab |
| **Economic system (Market, Regulated, Coop)** | The town-wide rule set the player can switch (`content/systems/*.json`). Not a kind of firm |
| **Randomness table** | Every RNG draw in the model (drought, fire, home damage, enclosure, startup) with its distribution and parameter. A test checks it against the source. Everything else is deterministic |
| **Shared random stream** | The model has two seeded random number generators. The weather has its own (fire and drought draws only, stored in the game state), so nothing else can shift when hazards strike. Class (a household founding a firm), estate (the jet's flight) and commons enclosure share the other one, in a fixed order each tick, and each takes its draw every month whether or not it counts, so a player action never changes how many draws the others see. Adding a new draw to the shared stream still shifts every later shared draw (not the weather); the fix is a separate generator for the new feature |
| **Decision point / `DecisionSpec` / `QuantityId`** | An explicit, pure `decide_*` function over a read-only view, registered in `sim-model::decisions::DECISIONS` with its inputs and outputs (`QuantityId`s), parameter paths and default rule id. Stages 1-3 (18): firm hire, fire, wage, price, expected sales, green step, capex, payout, exit; household found-a-firm, spend, shop, lobby; bank headroom, lend, spread, dividend, rescue. Served as `catalog().decisions` and `catalog().quantities` (contract v2.7); feeds the Agents "Who decides" lines. `docs/features/decision-points.md` |
| **Decision tag** | One line in the person inspector: the class of their household and how such households decide, with a link to the Agents tab |
| **UI catalog (`catalog()`, contract v1.8)** | `content/ui.json` resolved against the live params: UI bands (crisis, health, word bands), lever group order and blurbs, authored text by group and key, and the economic-system cards. The web renders it and keeps no thresholds or model text of its own (`docs/town-v6.md`) |
| **Thin renderer** | The rule for `web/`: layout, formatting, animation, chart scaling, onboarding copy and the wording of wire ids stay in the web; simulation rules, assumptions, model descriptions, thresholds and explanatory text come over the wire |
| **Tile fire risk** | `Tile.fireRisk` on standing forest tiles: the model's monthly odds that a fire reaches the tile. The "Fire and drought risk" lens colours tiles by it, scaled to the riskiest tile |
| **Estate (Town v7)** | The bundle: the estate household plus its home, grounds, runway, hangar and jet (the household alone is "the estate household"). One very rich household (label "Estate owner", tag from `catalog.tags.estate_owner`) that does not work, owns the Farm, a 2-storey house, a private jet and a runway, in the south-west corner. `State.estate`; layout in `content/initial_town.json` `estate`; rules in `crates/sim-model/src/estate.rs`. Class is derived: Capitalist |
| **Jet / flight** | The estate's private jet. Each month one random draw (`estate.flight_prob_monthly`, 0.4) decides whether it flies, only if the pilot is employed. A flight emits `estate.flight_tonnes` (2 t) and costs the estate `carbon price x tonnes`. Event `private_jet_flight` |
| **Aviation** | The emission source for jet flights (`EmissionSource::Aviation`, source id `aviation`, category `aviation`): listed in an emissions line only when it is not zero |
| **Pilot / domestic wages** | The jet's pilot is a worker in Worker C's household, employed by the estate household (`Person.staff_of`). The estate pays `estate.pilot_wage_usd` (1,800) a month, household to household (`Purpose::DomesticWages`). Three months of short pay and the pilot is let go (`pilot_let_go`, the jet is grounded); rehired when the estate holds three months of wages |
| **Wealth band** | A word for a household's net worth ("Ultra-rich" from $1M), from `content/ui.json` `wealth_bands`, evaluated by the wire (`HouseholdView.wealthBand`) |
| **Net worth statement** | A household's net worth as assets minus liabilities, one line each (`byHousehold[].statement`), computed in Rust and flagged `reconciles` against the figure the Gini uses. A person's statement is the household's divided by its people (Town v8) |
| **Holder** | A group in "Who holds what": the estate, the bank's owner, firm owners, other households, plus the government as a memo row (`breakdowns.gini.town`) |
| **Market (Markets view)** | One card per market (food, goods, services, labour, housing, credit, electricity) on `Snapshot.markets`: what was **wanted** (demand), what was **available** (supply), what was **traded**, and the two gaps, **unmet** (wanted, not traded) and **unsold** (available, not traded). `demand = traded + unmet`, `supply = traded + unsold` in the market's own unit |
| **Supply / demand (in this game)** | Supply is what is on offer (shelf stock, people who can work, homes, the bank's room to lend, plant capacity); demand is what buyers would take at the going price before any stock limit. There is no demand curve: the cards show the real quantities |
| **Shortage / glut** | A shortage is unmet demand (a gap labelled "went without", "still open", "without a home"); a glut is unsold supply ("unsold", "idle"). Neither moves a goods price here (see cost-plus) |
| **Cost-plus pricing** | How firms set goods and services prices: each month the price moves a share of the way (`goods.price_adjust`) toward unit cost times one plus the firm's markup (`goods::price_target`). Demand never enters. Wages respond to unemployment, rents to occupancy and rent burden, loan rates to the base rate and the firm's borrowing |
| **Trade (market trade)** | One row of the list behind a market: a purchase (buyer, shop, units, unit price, total), a job (worker, employer, wage), a home (tenant, owner, rent) or a loan request (borrower, asked, approved, rate). Shown by "See all trades" on a market card and, for one party, in its inspector. Output only: it is kept beside the ledger, never in the simulation state. Rows add up to the card's own figure (`MarketTrades.reconciles`) |
| **Coordination mechanism** | A way of deciding what to make, how and for whom when nobody knows everything: a market (prices), rules or planning (an authority sets quantities or prices), a firm (managers direct work), community rules (shared norms). Real economies mix them; a market is not the same as capitalism. Each market card is tagged with the mechanisms it leans on; each economic system card says which it leans on (`catalog.markets.mechanisms`) |
