# Town v7 design: the elite estate (draft for review)

Status: **DRAFT for review, nothing implemented.** Numbers are placeholders in `content/`. **[DECIDED]** = from the user; **[PROPOSED]** = my default, for the user to change. Every claim about the code below was checked against the checkout at commit 3df5e76; figures marked "measured" come from `cargo run -p sim-cli --example debug -- 2 3` (seed 42, no player action). This release was numbered v8 for GST/state ownership before this doc existed; the GST and state-ownership draft is now `docs/town-v8.md` and daily ticks are `docs/town-v9.md`.

## 1. Goal and decisions

One more household, one person, very rich, who does not work, lives on a 2 x 2 estate in a forest corner of the 12 x 12 map, owns one private jet and a short runway, and flies now and then. The point is to make inequality and its carbon cost **visible and playable**: the Lorenz curve gets a real outlier, the wealth tax and the carbon price visibly bite one specific agent, and every flight is an event.

- **[DECIDED]** One household, one person, net worth far above every current household, never counts as unemployed or labour supply.
- **[DECIDED]** The class label `Capitalist` is **not** renamed. No "Elite" class.
- **[DECIDED]** 4-tile estate with a posh 2-storey house in a map corner, forest around it, a road to the town centre; a jet owned by this agent; a runway of about 3 tiles; each flight emits CO2 (about 2 t, about 40% a month); estate household emissions about 0.5 t a month; all tunable in `content/`.
- **[PROPOSED]** No new `Class`. The estate household is `Capitalist` by derivation (section 3) and carries a separate wire tag "Estate owner" plus a wealth band, both read from `content/ui.json`.
- **[PROPOSED]** Income: the estate household is the single `Owner::Household` of an existing firm. The best choice by the numbers is the **Farm**, not the Shop (section 4). Interest on deposits is not a usable income source (section 4).

## 2. What the code allows (checked)

| Fact | Where | Consequence |
|---|---|---|
| A household has 2 to 4 people; the validator rejects fewer | `crates/sim-params/src/lib.rs` `validate_town` (`(2..=4).contains(&hh.people.len())`), comment on `Household` in `sim-types/src/state.rs` | Relax to 1 to 4. Content check only, no rule depends on it |
| `Person.in_labour_force = false` never counts as unemployed or labour supply | `Person::unemployed`, `State::labour_force` (`sim-types/src/state.rs`); benefits go to `unemployed()` or `Retired`, so an Adult outside the labour force gets none | The "does not need to work" rule needs no new code. Use `age_band: Adult`, `labour_force: false`, `job: null`, as the Owner family already does |
| Map tiles are placed in `init` from `initial_town.json`; scenery is `Road`, `Forest`, `Commons`; one thing per tile, checked at load | `sim-engine/src/init.rs` (`scenery`), `validate_town` ("tile ... holds both ...") | The sim owns the map. New tile kinds must be added to `TileKind` (`sim-types/src/enums.rs`) and the wire `TileKind` union |
| Forest tiles are the fire substrate: `forest_tiles()`, ignition picks one standing tile uniformly, spread is per neighbour | `weather.rs` | Adding forest tiles changes the odds and the uniform pick for every seed (balance, section 8) |
| Homes: one `Home` per tile, `owner` and `household`; the owner-occupier pays no rent | `init.rs`, `income.rs` rent loop (`home.owner == HomeOwner::Household(h.id)` -> skip) | The estate house is an ordinary `Home` owned by its occupant |
| Startup site search uses the first `TileKind::Empty` tile | `class.rs` `site` | New non-Empty tile kinds are safe; the estate tiles are never picked |
| Roads cost upkeep per tile (`government.road_upkeep_usd` = 10) and speed commutes | `params.json`, `space.rs` | 7 road tiles add $70 a month of public upkeep (verify it counts initial roads) |
| `Owner` is `Household(id) | Workers | State | Bank`; one owner per firm; every firm starts `Owner::Household(founder)` | `sim-types/src/enums.rs`, `init.rs` | Ownership is set by `founder` in `initial_town.json` (section 4) |
| The bank holds only loans: `equity = loans - deposits`, and init fails if equity is negative or below `bank_capital_ratio` of loans | `init.rs` (end of `init`) | **Big deposits are impossible** without matching loans (section 4) |
| Household carbon price: none. The carbon price is charged to firms only (`income.rs`) | `grep carbon_cost` | "Carbon price hits the estate" needs a new, explicit charge on flights (section 6) |
| Wealth tax = annual rate on net worth above `wealth_threshold`, one twelfth a month, paid from savings only (what savings cannot cover is not collected) | `government.rs`, `util::wealth_tax_monthly` | The estate pays it if it holds enough savings. Net worth counts the home and firm equity, not just deposits |
| `household_wealth` = savings - debt + owned home value + bank equity (bank owner) + firm equity | `sim-metrics/src/lib.rs` | A jet is not counted today. It needs an asset line (section 5) |
| Political power = share of positive net wealth; lobbying spend = `spend_rate` x capital income of Capitalists and Rentiers, paid to the richest Manager | `lobbying.rs` | The estate dominates the `political_power` display at once; the lobbying effect only changes through capital income (section 6) |
| One shared `Rng`; all draws `r.chance` / `r.next_f64` / `r.below`; the table in `content/assumptions.json` `agents.randomness` (8 entries) must match a source scan | `crates/sim-api/tests/assumptions.rs` `randomness_table_matches_the_draw_sites_in_the_model` | One new entry and a new count |

## 3. The household, its class and its tag

Starting values (all in `content/initial_town.json`; `[PROPOSED]`):

| Field | Value | Why |
|---|---|---|
| label | `Estate owner` | The household label; the tag below is separate |
| person | `Adult 1`, `Adult`, skill 0.5, `labour_force: false`, `reservation_usd: 0`, no job | Never unemployed, never labour supply |
| savings | $20,000 | Upper bound set by the bank identity (section 4) |
| debt | 0 | |
| home | the estate house (new `Home`, value $1,200,000, owner = this household) | |
| firm owned | the Farm (section 4) | The income source |
| jet value | $800,000 (new, section 5) | |
| net worth | about $2.1M: 20k + 1,200k + 800k + Farm equity (about 77k, computed from `initial_town.json` with `capital_price_usd` 1500) | Owner family, the richest today, is about $458k (measured); all eight current households together are about $750k |

**Derived class (verified in `class.rs`).** `classify_with` returns `Capitalist` when `owns_assets` holds (it owns an active firm) and capital income is above `capital_share_threshold` (0.5) of gross income over the last six months, and `Rentier` only if rent received exceeds `rentier_share_threshold` (0.5) of capital income. The estate household receives only dividends (the Farm) and a trivial amount of interest; it rents out no home. So it is `Capitalist` by derivation, with no `class.rs` change, and the `Capitalist` label stays.

**Edge case to decide.** `owns_assets` excludes the home a household lives in, and counts firms only while `active`. If the Farm defaults and closes, the estate owns no counted asset, has no job, and the function returns `Dependent`: a $2M household labelled Dependent. Options: (a) accept, it is rare and the tag still says "Estate owner" (default); (b) add a content threshold `class.asset_wealth_threshold_usd` so that large net worth also counts as owning assets (a small `class.rs` change, one more param). Open question 3.

**Tag and band (wire, thin renderer).** The wire `HouseholdView` gets two additive optional fields: `tag` (string, from `content/ui.json` `tags.estate_owner` = "Estate owner", present when `state.estate.household == h.id`) and `wealthBand` (id and label from a new `ui.json` `wealth_bands` table, for example "Ultra-rich" at $1M, evaluated by the wire from `household_wealth`; the web only renders them). The web shows the class label (Capitalist) and the tag next to it everywhere a class chip is shown (people panel, inspector, Gini table).

## 4. Income: how the estate lives without a shareholding system

What flows exist today (from `income.rs`, `credit.rs`, `class.rs`):

1. **Dividends.** After wages, carbon cost and profit tax, a firm pays `payout_ratio` of profit (limited by a cash reserve), plus a share of idle cash, to `match f.owner`: `Household(h)` gets it all. Taxed as capital income by `tax_capital` (0.4).
2. **Deposit interest.** Each month every household earns `(base_rate - deposit_spread)` on savings (0.04 - 0.01 = 3% a year), paid **out of bank equity** and capped by it (`pay = due.min(eq)`).
3. **Rent** from homes owned and let to others (only the Landlord today).
4. Wages: not applicable.

What the existing Capitalist owns today: the **Factory** and the **Farm** (both `founder: 0`), the home they live in, and the **bank** (`bank_owner: 0`, so bank dividends also reach them). The Landlord owns the **Shop** (`founder: 1`) and five rented homes. Measured, month 3: Owner family dividends about $9.7 to $10.0k, Landlord dividends $0.3 to $0.4k, firm profits Factory $4.9k, Farm $6.7k, Shop $0.5k.

**Why the Shop does not work.** Making the estate the Shop's owner gives it about $0.4k a month. Its food need alone is 25 units x $24 = $600 (`goods.food_need_units_pc`), so with $20k savings it runs out in about 100 months and the "richest agent" ends the game broke; it also takes the Landlord's only dividend (about $0.35k of a disposable income of about $1.1k while the Landlord pays $0.75k a month on $111k of debt and holds about $1k of savings), a likely arrears and default. **Why deposits do not work.** Interest on $20k is about $50 a month, and a bigger deposit is impossible: init requires `bank.equity = loans - deposits >= 10% of loans`; loans are $345k (firm and household debt plus government debt), deposits $289k, equity $56k against a $34.5k minimum, so only about $21k more can be deposited. Even if the check were bypassed, interest comes out of bank equity.

**Proposal [PROPOSED]: the estate household is the `Owner::Household` of the Farm** (set `firms[1].founder` to the estate household index; no code change, `init.rs` copies `founder` to `owner`). Result: the estate earns about $5k a month in dividends (Farm profit $6.7k x the payout ratio, then 40% capital tax), similar to a Manager household, and its wealth is dominated by assets, not cash, which is how a real estate is held. **What changes for the existing Capitalist:** they keep the Factory and the bank, lose the Farm's dividends (about $5k of $9.9k a month), stay `Capitalist` (the Factory alone gives capital share 1.0; still `owns_assets`), and lose $77k of net worth. Their consumption (wealth effect, propensity 0.45) falls, which lowers demand a little. The Shop alternative is kept as a content switch (`founder` of firm 2), with the numbers above, if the user prefers it. **Both versions are a content edit; the choice is open question 1.**

**Later (v8, `docs/town-v8.md`).** State ownership and shareholding will generalise `Owner` to shares and let the estate hold a stake in several firms or the bank. Nothing in v7 blocks that: v7 uses only `Owner::Household`. Deposit interest as a real income source would need the bank identity relaxed (a loan book backing a large deposit), also a v8 question.

## 5. Estate, jet and runway: state and rules

**Map [PROPOSED].** I checked every tile against `initial_town.json`: the south-west corner (columns 0 to 5, rows 8 to 11) holds no forest, home, firm or commons. Nearest existing things: forest at (0,7), home (3,7), shop (5,7), home (5,8). Proposed (x,y), origin top-left:

| What | Tiles | Kind |
|---|---|---|
| Estate house (2 storeys) | (0,10) | `Home` (id = the estate home) |
| Grounds: pool and garden, drive, lawn | (1,10), (0,11), (1,11) | new `EstateGrounds` (3 tiles) |
| Runway (3 tiles, east of the house, west-east) | (2,11), (3,11), (4,11) | new `Runway` |
| Road to the town centre (drive gate at (1,9)) | (1,9), (1,8), (2,8), (3,8), (4,8), (4,7), (4,6), joining the existing road at (5,6) | `Road` (7 new tiles) |
| Forest ring (new, fixed scenery) | (0,8), (0,9), (2,9), (3,9), (4,9), (2,10), (3,10), (4,10), (5,9), (5,10), (5,11) | `Forest` (11 new tiles) |

All 25 tiles are free today and distinct, so `validate_town` passes. The 2 x 2 estate is `(0..1, 10..11)`; the map edge closes it on the west and south, forest closes the north (row 9 and the drive gap), east beyond the runway and the apron. The 11 forest tiles take the forest count from 15 to 26. `initial.forest` (stock, 1400) is a separate scalar, so no stock changes; the visible tile count and the fire odds do (section 8). No existing home or road is displaced. Also checked: the commute table is unaffected (the estate person has no job; `space::commutes` skips people without a job).

The estate layout lives in `content/initial_town.json` under a new `estate` block (house tile, grounds tiles, runway tiles in take-off order, jet name, jet value, household index), so the map stays content-driven and the scene never invents positions.

**State (sim-types, `SCHEMA_VERSION` 3 -> 4).** `State.estate: Option<Estate>` with `household: HouseholdId`, `home: HomeId`, `grounds: Vec<Tile>`, `runway: Vec<Tile>`, `jet: Jet`. `Jet { owner: HouseholdId, value: Cents, flight: Option<Flight> }`; `Flight { phase: Takeoff | Cruise | Landing, month: u32, tonnes: Tonnes }` exists only in the month of a flight (the sim is monthly; the animation phases are the scene's, see section 7, so the state needs only `flying_this_month: bool`, `flights_total: u32`, `tonnes_total: Tonnes`). `None` when the content has no estate, so old content and tests still load.

**Wealth.** `household_wealth` adds the jet `value` to its owner. Static, no depreciation (`estate.jet_depreciation` = 0, a param so v8 can change it).

**New system `estate` (sim-model/src/estate.rs, `SystemId::Estate`)**, placed in `TICK_ORDER` **after Weather and before Production**, so a flight is known before Nature totals the month's emissions. Pure: `fn(&State, &Params, &mut Rng) -> Deltas`. Each month: one Bernoulli draw `r.chance(estate.flight_prob_monthly)`; on success emit `Event::PrivateJetFlight { tonnes }`, add `flight_tonnes` to the month's emissions (`EmissionSource::Aviation`, a new variant so the breakdown shows it separately from "Households"), and charge the carbon price (section 6). No draw happens if there is no estate, so towns without one keep the old stream exactly.

**Emissions.** `nature.rs` currently gives every household `members x household_footprint_base` (0.12 t, so 20 people = 2.4 t a month). For the estate household the monthly direct figure becomes `estate.household_emissions_t` (0.5, replacing its 0.12), via a per-household override read from `state.estate`, not a special case in the numbers. Proportions from `params.json` and the measured run:

| Quantity | t a month | Share of gross 11.6 to 11.8 | Share of the 900 t budget over 120 months |
|---|---|---|---|
| Current gross (measured 11.6 to 11.8) | 11.7 | 100% | 1,404 t if sinks are ignored |
| Current household sector (20 x 0.12) | 2.4 | 20% | |
| Estate household (replaces 0.12 with 0.5) | +0.38 | +3.2% | +46 t (5.1%) |
| Jet, expected (2 t x 0.4) | +0.8 | +6.8% | +96 t (10.7%) |
| Estate total, expected | +1.18 | +10.1% | +142 t (15.7%) |
| Household sector after (incl. jet) | 2.4 + 1.18 = 3.58 | +49% | |
| One flight | 2 | 17% of a month's gross | 0.22% of the budget |
| Estate person vs an average person | 0.5 vs 0.12 per head | 4.2x; with the jet 11x | |

Monthly jet emissions have a standard deviation of 2 x sqrt(0.4 x 0.6) = 0.98 t, so in a given quarter the town's gross swings by about 2 t. Single person, 11 times an average person's footprint, is the intended message. The no-action budget (baseline A) is used up about 2.5 turns earlier (900 / 11.7 = 77 months; 900 / 12.9 = 70 months, ignoring the forest sink).

## 6. Making inequality and the carbon cost visible

**Gini, Lorenz, top share (verified, `sim-metrics`, `breakdowns.rs`).** Gini is over people, each holding an equal share of their household wealth. I recomputed from the measured month-3 wealth (Farm moved to the estate, estate $2.1M): Gini goes from 0.72 to about 0.87; the Lorenz curve ends in a 1/21 step holding about 76% of all wealth; `top_wealth_share` goes from 0.60 (Owner family) to about 0.76 (estate). Nothing to change in the formulas; the Gini breakdown table (`by_household`) gets the tag column.

**Legitimacy impact (balance, not a bug).** `government.legit_gini` 0.012 above `legit_gini_threshold` 0.45 costs legitimacy every month. Moving from 0.72 to 0.87 adds about 0.15 x 0.012 = 0.0018 a month, about 0.2 over the game, against a baseline B minimum legitimacy of 0.44 (`docs/tuning-log.md`). Expect baselines to lose legitimacy faster; this is what the player should feel, but it needs a tuning decision (section 9).

**Where it shows.**
- Society and people views: the estate row with class chip "Capitalist" and tag "Estate owner", wealth band, net worth, monthly dividend income, footprint (tonnes a month).
- Wealth and income breakdowns: new rows from the wire (estate home, jet, firm equity in the wealth breakdown; Farm dividends in income). They arrive in the existing `breakdowns` shapes, with labels from `content/ui.json`.
- Person inspector: person label, "Does not work: outside the labour force", home "Estate", jet, last flight, footprint, wealth tax paid and carbon charge paid, last 12 months.
- Policies and Finance tabs: the wealth tax row already shows the revenue per month; add "of which the estate" (the wire gives the per-household tax already in `household.income` / the ledger). Cheap: the ledger already records `Purpose::WealthTax` per household.
- **Carbon price on the jet.** Because the carbon price hits firms only, the estate would never feel it. New rule: a flight pays `effective.carbon_price x flight_tonnes` from the estate's savings to the government (`FlowKind::Tax`, `Purpose::CarbonTax`, the existing purpose), capped by savings like the other taxes in `government.rs`; the estate household's other emissions are not charged (so ordinary households stay unaffected; a household carbon price is out of scope). At $100 a tonne, a flight costs $200. This respects lobbying (it uses `s.effective.carbon_price`, which is already eroded by the net lobby).
- **No jet running cost.** Fuel and crew are not modelled in v7 (no counterparty without inventing a sector). A flight costs money only through the carbon charge. Listed in Out of scope.
- **Wealth tax.** Already applies (net worth above `wealth_threshold` 150k, paid from savings only). At $2.1M and a 1% annual rate it is about $1.6k a month, covered by about $5k of dividends and savings. Because net worth is mostly illiquid, a high wealth tax rate can exceed the cash available and is silently capped; the Policies tab should say "paid in part" (the existing "unfunded" idea in the ledger); open question 7.
- **Optional follow-up (not in scope):** elite lobbying weight: weight `lobby_capital` by wealth share instead of capital income. Today the estate's political power display is about 0.76, but real lobbying still uses 2% of its capital income, about $100 a month.

## 7. Scene and web (thin renderer, Town v6 rules)

The sim owns the map; the scene only renders what the wire says.

**Wire (contract v1.9, additive only; `snapshot-contract.md` gets a section 19).** `TileKind` adds `estate_grounds` and `runway`. `Snapshot.estate?: { household, home, groundsTiles, runwayTiles, jet: { value, flyingThisMonth }, lastFlight?: { turn, month, tonnes }, flightsTotal, tonnesTotal }`. `HouseholdView.tag?`, `wealthBand?`. Event kind `private_jet_flight` `{ tonnes, household }`. Breakdown source `aviation`. `catalog()` gains `tags` and `wealth_bands`. Fixtures regenerate from the sim; no hand edits. Optional fields, so older readers and the mock keep working.

**Mock.** `content/` stays the source: `npm run sync-mock` carries the estate block and the new `ui.json` entries into `data.generated.json`. The mock reproduces a flight with its own seeded draw at the same probability (labelled in `mock-only.json`, like the weather).

**Estate model (`web/src/scene/models.ts`, `geometry.ts`; Three.js primitives, no assets).** A 2 x 2 footprint on tile (0,10) to (1,11):
- House, tile (0,10): two storeys, off-white stone walls (`0xf1eee6`), a pitched slate roof (`0x4a4f5a`) with a gable, a stone chimney, a portico with two columns, a glass-blue band for windows, a warm light in the upper windows.
- Grounds: (1,10) a lawn with a rectangular pool (flat blue box, white coping); (0,11) a garden with three small conifers and a hedge; (1,11) a gravel driveway with a turning circle running north to the gate at (1,9); a low wall and a gate on the road side.
- Runway: three tiles, dark asphalt strip with white dashes and threshold bars, a windsock at the east end, a small hangar (open front) on the apron tile (2,11), where the jet is parked.
- Lens support in `lenses.ts`: class lens tints the house like any Capitalist home; wealth lens uses the household wealth (always the top colour); emissions lens weights the estate by its footprint; the jet's tonnes appear as a smoke weight on the runway; the commute, employment and flows lenses ignore it (no job, no commute).

**Jet and animation.** The jet is a Three.js group (fuselage cylinder, swept wings, tail, two engines), and it is **hidden between flights**: visible only during a flight month's playback. When `private_jet_flight` arrives in a frame's events (the frames already carry per-month events; `playback` timing in `app/` plays them), the scene runs, over about 6 seconds: taxi from the hangar to the west end of the runway, take-off roll east along the three tiles, climb on a curve out of the scene, with an exhaust vapour trail of instanced white particles (the existing `FxLayer` smoke pool, a new `trail` type with a longer life and a fade), then, after a short pause, the return: a landing approach from the east, a touchdown roll, a taxi back, and the jet hides. The trail colour follows the emissions lens. Reduced-motion users get the jet placed on the runway for one second with a puff, no flight path. The flight happens only when playback shows the month (not when scrubbing history quickly). The log entry: "Private jet flight from the estate: 2.0 t CO2" from the event, wording from `content/ui.json` (`events.private_jet_flight`).

**Panels.** People, Society (Gini view with the tag column), Inspector, Finance and Policies as in section 6, all labelled from the wire. No numbers (probabilities, tonnes, thresholds) in `web/src`; the explorer topic "Private jet" is generated from `content/assumptions.json`.

## 8. Balance and tests

**Balance.** Known effects, each to be measured, not assumed:
1. Emissions +10% expected (section 5); no-action budget exhausted about 2.5 turns sooner; baselines A, B, D, E, F (and C) all move, and the pass/fail margins in `tests/scenarios.rs` (`baseline_*` tests) must be re-read. Possible levers, in order: `nature.budget_b0` (900 t, `docs/tuning-log.md` records the history), then `estate.flight_prob_monthly`, then the weather onset.
2. Forest +11 tiles: ignition is `fire_base x ... x standing share`, and the target tile is a uniform pick, so about 42% of ignitions (11/26) now land near the estate; a fire there raises smoke (`smoke_radius` 2) over the estate and may remove the 11 tiles' sink capacity in effect. Re-check the fire counts in the weather baselines (`docs/tuning-log.md`, weather section).
3. RNG: the new draw is on the shared stream, so every later draw shifts and fire and drought timing change for every seed. Alternative (open question 5): a dedicated stream stored in state, which keeps old weather sequences exactly.
4. Demand: the estate's wealth effect (`goods.wealth_consumption_rate` 0.005 x $2.1M = $10k a month wish) is capped by savings, so it spends roughly its income, about $5k a month, a rise of about 25% in household spending (21.4k measured) funded by a smaller Owner family income; net demand roughly neutral, composition changes (services share). Measure.
5. Legitimacy (section 6) and bank: the estate adds $20k of deposits (equity falls from $56k to about $36k, just above the $34.5k requirement). **Tight**: lower the estate savings to $15k, or raise one firm's debt in the content, if init rejects it.
6. The Landlord is untouched in the Farm option; in the Shop option they are likely to default (section 4).

**Tests.**
- Content: the estate loads; tile conflicts fail validation; household of 1 allowed; a town without `estate` still runs.
- Rules: the estate person is never in `unemployed()` or in `labour_force()`; derived class is `Capitalist`; flights only when the draw succeeds; the emissions line `Aviation` equals flights x tonnes; carbon charge equals `effective.carbon_price x tonnes` and is capped by savings; wealth includes the jet.
- Randomness: add the `estate_flight` entry to `content/assumptions.json` `agents.randomness` (`bernoulli`, `estate.flight_prob_monthly`, system `estate`, code `crates/sim-model/src/estate.rs`, "each month while the estate exists") and raise the expected draw-site count in `randomness_table_matches_the_draw_sites_in_the_model` (`crates/sim-api/tests/assumptions.rs`) by one; the explorer test also requires every param path to resolve.
- Money: ledger reconciliation runs inside every simulation test (flights move money only through the carbon charge).
- Web: lens, tag and band rendering from fixtures; `thin-renderer.test.ts` (every catalog key the code names exists); boundaries test unchanged (the scene imports no panels); a scene unit test for the flight timeline as a pure function (phase by elapsed time).

**Re-blessing goldens.** The state hash changes now (new household, tiles, firm owner, RNG shift). After the Rust agent finishes, in the order of `docs/how-to.md` and `AGENTS.md`: `cargo run -p sim-api --example fixtures`; `UPDATE_GOLDEN=1 cargo test -p sim-wasm --test golden` (twice if the two goldens race); `cd web && npm run sync-mock`; the insta snapshot (`INSTA_UPDATE=always cargo test -p sim-engine --test scenarios golden_snapshot_seed_42`); `cargo run -p sim-cli --example statehash` before and after, and the diff of each generated file must be read and explained (new household, new tiles, the Farm owner, new draws). Any params or content change also invalidates saves (`params_mismatch`, a known V6-6 behaviour), and `SCHEMA_VERSION` goes to 4.

## 9. Delivery plan

1. **Rust agent.** `sim-types` (`TileKind` variants, `Estate`, `Jet`, `Event::PrivateJetFlight`, `EmissionSource::Aviation`, `SystemId::Estate`, `SCHEMA_VERSION` 4); `sim-params` (`estate` params, `initial_town.estate`, household size 1 to 4, `ui.json` tags and bands); `sim-engine/init` (tiles, household, firm owner); `sim-model/estate.rs` + `TICK_ORDER` (13 -> 14 entries, update the const length) + `nature.rs` override + `sim-metrics` jet value + the carbon charge; `sim-checks` (money, no change expected); `sim-api` (wire v1.9, fixtures, assumptions entry, breakdown rows). Content: `initial_town.json`, `params.json`, `ui.json`, `assumptions.json`. Deliver with the Rust and API tests green, goldens re-blessed, `docs/snapshot-contract.md` section 19 and the sim-api notes.
2. **Web agents, in parallel** once the fixtures exist: (a) scene: estate model, runway, jet, flight timeline, trail, lens support, event log; (b) panels: tag and band, Gini table column, inspector, Finance and Policies rows, explorer topic, mock sync.
3. **Balance pass.** Re-run baselines A to F over 20 seeds, record in `docs/tuning-log.md` "Town v7"; tune `budget_b0`, `flight_prob_monthly`, `legit_gini` only with the user's decision on each; update `docs/known-issues.md`, `docs/deviations.md`, `docs/glossary.md` (Estate, Jet, Aviation), `AGENTS.md` (people and household counts: 21 people in 9 households), `docs/architecture.md` (tick order and map), the README index.
4. **Review and playtest** with the real wasm build, screenshots of the estate, a flight, the Gini view and the inspector.

## 10. Risks

- Baselines shift in several directions at once (emissions, forest tiles, legitimacy, RNG). Mitigation: one balance pass with a written record; no tuning inside the feature work.
- The bank identity is tight (section 8, point 5); a too-large estate deposit fails at init.
- An estate that owns a Farm that defaults becomes class `Dependent` (open question 3).
- The jet's visual weight: animation must not obstruct the playback timing in `app/`; flights are capped at one per month and the scene must be idempotent when frames are skipped.
- The estate dominates `top_wealth_share` and `political_power` bars; these read as 0.76 from the first frame, which may look like a bug unless the tag explains it.
- Scene tile kinds are a wire change; older saved wire snapshots in `localStorage` lack them, and fall back to `empty`.

## 11. Open questions (defaults)

1. Income firm: Farm (default, about $5k a month) or the Shop (the user's example, about $0.4k, endangers the Landlord)?
2. Wealth figures: home $1.2M, jet $0.8M, savings $20k (default), or smaller (net worth about $1M) so Gini rises less?
3. If the Farm closes: accept `Dependent` (default) or add `class.asset_wealth_threshold_usd`?
4. Should the jet cost anything besides the carbon price (default: no)?
5. Shared RNG stream (default) or a dedicated estate stream to keep old weather sequences?
6. Should the estate's 0.5 t a month replace the per-person 0.12 (default) or be added to it?
7. Wealth tax shortfall: silently capped like today (default) or shown in the Policies tab as "unpaid"?
8. Road to the town: 7 new tiles at the road upkeep of $10 each (default), or fewer by moving the estate closer?

## 12. Out of scope

Renaming the `Capitalist` class; a new class; share ownership, several owners per firm, nationalisation and GST (`docs/town-v8.md`); deposit-interest income from a large balance (needs the bank identity relaxed); jet fuel, crew, hangar and maintenance costs; selling or buying the jet, depreciation; more than one flight a month, route or destination choices; a household-level carbon price; elite lobbying weight by wealth; the estate founding firms or buying homes; multiple estates; sound.
