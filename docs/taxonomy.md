# Taxonomy: a classification of every entity and concept in the game

Status: **APPROVED by the user with every open-question default (section 11).** Phases 1 and 2 of the platform cleanup are implemented (section 12 has the status per step); `Purpose` categories and the optional Rust unification (step 7) are open. Written from the code at commit `8f4d565`. Where this doc and code disagree, code wins (`docs/README.md`). This doc itself changes no code; every proposal below says what it would touch.

## 1. Why

Today each concept is classified by whichever enum grew first: `TileKind` (14 variants) mixes buildings, natural stocks and fixed scenery; `Owner` (firms) and `HomeOwner` (homes) are two spellings of "who owns this"; `EmissionSource` is one enum on the Rust side, a different `SourceKind` plus fixed-field `EmissionsLine` on the wire, and a third copy in `web/src/panels/emissions.ts`; `IncomeFlows` is a fixed struct of five income fields; `Purpose` has 27 variants that each must be mapped by hand to a ledger kind and label. Adding the private jet (v7) touched all of these: a new `EmissionSource::Aviation`, a new `SourceKind::Aviation`, a new `aviation` field in `EmissionsLine`, a new web label, a new `TileKind::Runway`, `EstateGrounds`, a new `Purpose::DomesticWages`, a new `Person.staff_of`. None of that was wrong; it shows that the unit of classification is wrong. A single tree ("Entity -> Building -> Plant") does not fit because one thing is several things: the Farm is a firm (actor), sits on a tile (facility), emits (source), is owned by the estate (ownership edge), employs people (employer) and is hit by drought (hazard-exposed).

Goal: classify along **axes** (what kind of thing it is) plus **roles** (what it can do in the model). Roles are what the code and UI share; kinds are what stays bespoke.

## 2. Inventory (from code and content)

Sources: `crates/sim-types/src/{state,enums,ledger}.rs`, `crates/sim-model/src/*.rs`, `crates/sim-api/src/{emissions,keys,snapshot,catalog}.rs`, `content/{initial_town,levers,ui,assumptions}.json`, `web/src/{scene,panels,contract}`.

### 2.1 Actors (things that decide)
| Entity | Where | Notes |
|---|---|---|
| Person (22) | `State.people`, `Person` | job, skill, commute, wellbeing, `staff_of` (employed by a household, v7), `AgeBand` |
| Household (9) | `Household`, `Class` derived each tick | decides spending, moving, lobbying, founding a firm. Tag "Estate owner" from `catalog.tags`. Class: Capitalist, Rentier, Manager, Worker, Dependent |
| Firm (3 + 1 dormant slot) | `Firm`, `Sector` Goods/Food/Services | Factory, Farm, Shop (UI names); owner is `Owner::{Household, Workers, State, Bank}` |
| Bank | `Bank` | lends, owns some firms (`Owner::Bank`), can fail |
| Government | `Government` | taxes, benefits, builds, upkeep, legitimacy (a state, not a decision-maker the player controls directly) |
| Lobbyists | not an entity: `lobbying.rs`, `net_lobby`, `Purpose::LobbySpend`, `IncomeFlows.lobby_spent` | capital households' pooled pressure; an aggregate, not a thing on the map |
| Player | `Command`, PC (political capital) | not in `State` as an actor; owns `pc`, `pending` |

### 2.2 Facilities and places (sit on tiles; `TileKind`)
`Home` (private), `PublicHousing`, `Firm` (one tile per firm), `Bank`, `TownHall`, `FossilPlant`, `RenewablePlant`, `Road`, `FireStation`, `EstateGrounds`, `Runway`, plus the hangar (`Estate.hangar`, a tile with no `TileKind` of its own) and the estate's 2-storey house (a `Home` with a household). Non-building tiles: `Forest`, `Commons`, `Empty`. Farmland is not a tile kind (the Farm is a `Firm` tile). `InfraKind` (road, fire station, ...) is a separate enum for buildable things.

### 2.3 Objects owned by actors, not on the map as their own kind
Jet (`Estate.jet`, owner `HouseholdId`, value), the estate bundle (`State.estate`: household, home, grounds, runway, hangar, pilot, jet, counters), loans and deposits (bank), power capacity (`Plant`, tile + kind).

### 2.4 Natural stocks
Forest (stock, per tile `fireRisk`, regrowing), commons (`CommonsAccess` open/community, enclosure, collapse), atmosphere (cumulative emissions against budget b0 = 900 t; `net_emissions`), the sink (`sigma x forest`).

### 2.5 Hazards
Drought (`Drought`), fire (`Fire`, `Regrowing`), climate stress (a function of budget left: onset 0.75, full 1.0), smoke (`smokeRadius`), home damage, plus RNG-driven startup and enclosure events. Held in `State.weather`; exposure is per tile (forest, farm output, firms in smoke, homes).

### 2.6 Institutions, rules and levers
Levers (19 in `content/levers.json`, 4 lobbied: carbon price, wealth tax, profit tax, emissions cap), economic systems (`EconomicSystem`: Market, Regulated, Coop; `content/systems/*.json`; transition state), policy vs effective policy, command kinds (`Command`: set lever, switch system, build, remove), pending queue and lead times, PC rules, outcomes (`Outcome`, `CollapseReason`), class rules, legitimacy, budget.

### 2.7 Flows and records
- Money: `MoneyOp`, `Transaction`, `Purpose` (27), `LedgerKind`, `LedgerAccount`, `External`, `Account`, `FlowKind`, `GdpPart`, `IncomeKind`, `IncomeFlows`.
- Emissions: `EmissionSource` (Firm(id), Households, Plant(id), Sink, FireSinkLoss, Aviation); wire `SourceKind`, `SourceLine`, `EmissionsLine`.
- Energy: demand by households and firms, plant capacity, rationing, `energy.rs` wire.
- Events (`Event`, ~100 variants, importance 1-3), `TraceRecord`, `PolicyGap`, `MonthFrame`.
- Derived: `Indicators` (GDP, Gini, GPI, wellbeing, legitimacy), `Class`, `RiskLevel`.

## 3. Proposed axes

Five **kinds** (what it fundamentally is; exactly one per entity) and a set of **roles** (what it does; any number).

**Kinds**
1. **Actor**: has state and decides each tick (person, household, firm, bank, government). The player is outside the sim.
2. **Facility**: built or fixed thing on one or more tiles (home, plant, road, hangar, runway, fire station, town hall, bank building, estate grounds, public housing).
3. **Natural stock**: renewable or depletable resource with a stock and dynamics (forest, commons, atmosphere budget, sink).
4. **Institution / rule**: levers, systems, command kinds, class rules, lobbying, outcomes. No location, no money of its own.
5. **Flow / record**: money transactions, emissions lines, energy, events, trace. Derived from the above; never owned.

Hazards are a sixth, thin kind: **Process** (drought, fire, smoke, climate stress): they act on stocks and facilities, are random or state-driven, and have an exposure set. Keeping them apart from "natural stock" matters because they are the things draws are taken from (the shared RNG stream order, `docs/glossary.md`).

**Roles** (cross-cutting; each is a small interface, not a base class)
| Role | Data it carries | Today's equivalent |
|---|---|---|
| `located` | tile list, footprint, buildable flag | `Tile`, `TileKind`, `Estate.{grounds,runway,hangar}` |
| `owned` | owner edge: `Actor` id or `Public`/`Workers`/`Bank`/`State` | `Owner`, `HomeOwner`, `Jet.owner`, `Estate.household` |
| `emitter` | source category and tonnes per month | `EmissionSource` + trace |
| `sink` | negative tonnes | `EmissionSource::Sink` |
| `payer` / `payee` | money edges, with purpose | `Purpose`, `Posting`, `LedgerAccount` |
| `employer` | jobs, wage | `Firm`, estate household (`staff_of`), government (job guarantee) |
| `income_source` | category of income it produces for a recipient | `IncomeFlows`, `IncomeKind` |
| `energy_producer` / `energy_consumer` | MWh-like units | `Plant`, households, firms |
| `hazard_exposed` | which hazards, how | forest (fire), Farm (drought), firms (smoke), homes (damage) |
| `buildable` | cost, lead time, valid tiles | `InfraKind`, `Command::Build` |
| `lever_target` | which levers touch it | `levers.json` |
| `labelled` | label, plural, scene model id, inspector sections | scattered (section 7) |

## 4. Matrix: today's entities against roles
Legend: X = yes, o = partly or implicit, blank = no. Columns: Loc(ated), Own(ed), Emit, Sink, Pay, Empl(oyer), Inc(ome src), En(ergy), Haz(ard exposed), Bld(buildable).

| Entity | Kind | Loc | Own | Emit | Sink | Pay | Empl | Inc | En | Haz | Bld |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Person | Actor | o (home, job tile) | | | | | | X wages | X cons | | |
| Household | Actor | o (home) | X (homes, firms, jet) | X (heating, transport) | | X | X (estate staff) | X | X | | |
| Firm | Actor | X | X | X | | X | X | X (dividends) | X cons | X | o (startup) |
| Bank | Actor + Facility | X | X (firms) | | | X | | X (interest) | | | |
| Government | Actor | o (town hall) | X (public housing, state firms) | | | X | X (JG) | X (benefits out) | X (public) | | |
| Lobbyists | aggregate | | | | | X | | | | | |
| Home / public housing | Facility | X | X | | | | | X (rent) | | X | X |
| Fossil plant | Facility | X | | X | | | | | X prod | | X |
| Renewable plant | Facility | X | | | | | | | X prod | | X |
| Road | Facility | X | | | | X (upkeep) | | | | | X |
| Fire station | Facility | X | | | | X (upkeep) | | | | | X |
| Town hall | Facility | X | | | | | | | | | |
| Estate grounds, runway, hangar | Facility | X | X | | | | | | | | fixed |
| Jet | Facility-like asset | o (hangar) | X | X (aviation) | | X (carbon charge) | | | | | |
| Pilot | Person (staff) | | | | | | | X | | | |
| Forest | Stock | X | | | X | | | | | X fire | |
| Commons | Stock | X | o (access) | | | X (fee) | | X | | X | |
| Atmosphere / budget | Stock | | | accumulates | | | | | | | |
| Drought, fire, smoke | Process | X (tiles, radii) | | | o | | | | | | |
| Lever, system | Rule | | | | | | | | | | |

Observation: the empty columns are informative. The only things that emit are households, firms, plants and the jet; the only things that sink are forest. That is the list `EmissionSource` already encodes by hand.

## 5. How this simplifies code

### 5.1 Emissions as a list of sources
Today: `EmissionsLine { firms, households, plants, aviation, gross_total, sink, net, headline }` (fixed fields, `crates/sim-api/src/emissions.rs`), the Rust `EmissionSource` enum, wire `SourceKind`, and web labels in `panels/emissions.ts`. A new emitter needs a new field in four places.

Proposed (already half true: `SourceLine` is a list on the wire):
```
EmissionSourceLine { id: SourceId, kind: "firm"|"household"|"plant"|"aviation"|"landfill"|..., 
                     label: string, category: "industry"|"homes"|"energy"|"aviation"|"waste"|"sink", tonnes }
Emissions { month: [EmissionSourceLine], turn: [EmissionSourceLine], grossTotal, sink, net, headline, reconciles }
```
The category is the roll-up key (replaces the four fixed fields). `category` lives in content (`content/ui.json` or the new registry, section 6), so the web groups by it without a switch. `grossTotal` and `net` stay as derived numbers; `reconciles` stays a test. The Rust enum `EmissionSource` can stay (it is the tick-level trace key and part of save compatibility); the change is at the wire: `EmissionsLine` becomes `categories: [{category, tonnes}]`. The explorer's "emissions by source" table then needs no per-source code.

### 5.2 Income sources as a list
Today `IncomeFlows` has five income fields and wire `wages/dividends/rent/interest/benefits` (`contract/index.ts` line ~153). Proposed: `income: [{source: "wages"|"dividends"|"rent"|"interest"|"benefits"|"domestic_wages"|"commons"|..., payerKind, amount}]` on household views, derived from the ledger (`LedgerKind`, which already exists). Internally `IncomeFlows` stays (hot path, hashed). The wire list is built from it plus the ledger, so a new income type needs a `LedgerKind` plus a label in content, not a new field everywhere.

### 5.3 Money flows
`Purpose` (27) -> `LedgerKind` + label is already a table in `Purpose::resolve`. Proposal: add the role pair to each purpose: `(payer_role, payee_role, category)`, in one table. Then the Finance panel and sector-balance checks read categories (`tax`, `transfer`, `wage`, `purchase`, `upkeep`, ...) instead of enumerating variants. Group `WageTax, CapitalTax, WealthTax, ProfitTax, CarbonTax` as category `tax` with a `lever` link; `Wages, DomesticWages, JobGuarantee` as `labour_income`.

### 5.4 Ownership
Unify `Owner` (firms) and `HomeOwner` (homes) and `Jet.owner` into `OwnerRef = Household(id) | Public | State | Workers | Bank`. On the wire it already reads nearly uniformly (`FirmOwner {kind,id}`; homes use `owner: id | 'public'`). Inspector "Owner" rows (`panels/inspect.ts` lines 34, 106, 132, 198) then share one formatter.

### 5.5 Entity registry in `content/`
One file `content/entities.json` describing **kinds of facility and actor**, not instances:
```
{ "id": "fire_station", "kind": "facility", "label": "Fire station", "plural": "Fire stations",
  "tile_kind": "fire_station", "footprint": [[0,0]], "roles": ["located","buildable","payer","hazard_mitigator"],
  "buildable": {"cost_ref": "infra.fire_station.cost", "lead_time_ref": "..."},
  "emits": null, "scene_model": "fire_station", "inspector": ["owner","upkeep","coverage"],
  "glossary": "fire_station" }
```
Instances stay in `initial_town.json`. The wire serves the registry through `catalog()` (the v6 mechanism). The scene maps `scene_model` to a builder in `scene/models.ts`/`estateModels.ts`; the inspector builds sections from the role list; the build palette and glossary read label and blurb from it. Numbers stay in `params.json`; the registry holds references, never values (AGENTS rule 9).

### 5.6 Generic panels and inspector
`panels/inspect.ts` has one function per kind (person, household, home, firm, plant, ...), each assembling `{label, value}` rows by hand. Under roles: a section per role (`Owner`, `Location`, `Emissions`, `Energy`, `Money in and out`, `Employer`, `Hazard exposure`), generated from the entity's role set; bespoke sections remain for what is truly special (estate card, pilot lines). Similarly the lens list (emissions, class, wealth, employment, flows, commute) becomes "lenses keyed by role" (the emissions lens applies to any `emitter`).

### 5.7 Adding a new entity: today vs under the taxonomy
Example: a **landfill** (facility, emitter of methane, buildable, pays upkeep, hazard: fire).

Today (rough touch list): `TileKind::Landfill` in `enums.rs`; `InfraKind`; `Command` validation; `params` (cost, emission factor); `EmissionSource::Landfill`; `nature.rs` tally; `sim-api` `SourceKind`, `EmissionsLine` field, `keys.rs` strings; `breakdowns` rows; `tiles` wire; contract TS (`index.ts`) mirror; mock (`sim-port/mock`, its own emissions and tiles); scene model (`models.ts`) and tile colour (`palette.ts`, `lenses.ts`); build palette; inspector function; `panels/emissions.ts` label; glossary; explorer text; fixtures regeneration; golden hash. About 15 files in 3 languages.

Under the taxonomy: one registry entry (kind facility, roles located, buildable, emitter(category `waste`), payer(upkeep)), one params block, one Rust function that yields `EmissionSourceLine`s for entities with the `emitter` role and a model system only for what is genuinely new (the methane rule). Scene gets a model id; the rest (palette, inspector, emissions table, glossary, lens) is derived. Still bespoke: the emission rule, the 3D model, the balance numbers. Estimated 5 to 6 files.

**NGO**: an actor (new kind in registry) with roles payer/payee (donations) and lever_target (lobbying pushes the other way). Needs a model system and a tile or none; the roles reuse lobbying's `net_lobby` as a signed sum of lobbyist contributions rather than the current capital-only formula. Honest note: this changes `lobbying.rs` and balance; the taxonomy makes it additive in data, not free.

**Bank tower**: a facility decoration of the existing Bank actor. Under the registry it is a second tile of the same actor (`located` with a footprint); today `Bank` has one `TileKind::Bank` tile. Cheap under the taxonomy (footprint), a new `TileKind` today.

## 6. Vocabulary audit and renames
Checked `docs/glossary.md`, `content/ui.json`, `content/assumptions.json` (explorer), the catalog and web labels.

| Problem | Where | Proposed |
|---|---|---|
| "Capital" means five things: capital owners (class Capitalist), capital income, capital tax, capital goods (`Purpose::CapitalGoods`), bank capital ratio; plus "political capital" (PC) | glossary, levers, explorer ids `tax_capital`, `capital_ratio`, `class_politics` | Say "owner class" for people, "capital income" kept, "bank buffer" for the capital ratio in player text, "political capital (PC)" spelled out on first use; keep ids |
| "Firm" vs "Industry" vs "Business" vs "Sector" (`IndustryPanel`, `Sector` enum) | panels, explorer | Firm = the actor, Sector = Goods/Food/Services, "Industry" panel title -> "Firms" |
| "Plant" (power) vs "plant" (botanical, none) but "Power plant", "Plant", "Fossil plant" all appear | glossary, TileKind, ui.json (55 "plant" hits) | "Power plant" in player text, kind id `power_plant` with `fuel: fossil|renewable` (registry attribute, not two tile kinds) |
| Estate: the household, the whole bundle and the house are all "estate"; label "Estate owner" is a tag, not a class | glossary, `State.estate` | Estate = the bundle (household + home + grounds + runway + hangar + jet); the household is "the estate household"; keep tag text |
| Home / House / Housing / Public housing / Vacant home | glossary, inspector | Home = tile; "public housing" = a home with `Public` owner (one kind, owner attribute) |
| "Households" is both the emissions source and the actor | `EmissionSource::Households`, panel | Source label "Homes (heating, transport)" with category `homes`; actor stays "Household" |
| "Emissions" vs "Carbon" vs "CO2": lever "carbon price", tax "carbon tax" (`tax_carbon`), `Purpose::CarbonTax` vs lever `CarbonPrice` | ui.json, ledger | Emissions = tonnes; carbon price = the lever; carbon charge = what is paid (estate flights say "charge"; the ledger says "carbon tax"): pick "carbon charge" |
| "Source" means emission source, income source (`IncomeKind`), and "source" of a trace | glossary | Qualify: "emission source", "income source" |
| "Lever", "policy", "setting", "command", "action" (`actions()` wire) | glossary, contract | Lever = a numeric policy setting; Action = a build/switch; Command = either, when submitted. Make "policy" mean the set of lever values only |
| "Economic system" vs "Regime"/"Market" as if a firm kind | glossary already warns | Keep "economic system"; never "market" alone |
| "Active/nominal vs effective" vs "Lobbying gap" | glossary | fine; add "effective" to the Policies panel header consistently |
| "Wealth" (net worth) vs "Wealth band" vs "Wealth tax" | glossary, ui.json | fine, but glossary should link them |
| "Tile" holds "at most one thing" but estate bundle spans many | glossary | "one facility or stock" and note the estate bundle |
| "Frame / flow / MonthFrame" vs "money flow" vs "energy flow" | glossary | "Flow" = money/energy/emissions movement; "frame" only the per-month animation record |

## 7. Migration plan (small steps; each is independently shippable and testable)
0. **Docs only**: adopt the vocabulary renames in glossary, explorer text and `ui.json` (text-only; no state hash change).
1. **Wire-only**: add `SourceLine`-based categories to `breakdowns.emissions` (`category` on each line, from a small table in `ui.json`). Keep `EmissionsLine` fixed fields beside it. Web emissions panel reads categories. Additive, contract minor bump.
2. **Income list**: add `income: [...]` to household views, built from `IncomeFlows`/ledger. Web finance reads it. Additive.
3. **Registry v0**: `content/entities.json` holding labels, plurals, scene model id, role lists for the 14 `TileKind`s. Served by `catalog()`. Web labels (`infraLabel`, `classLabel`, ownerWord) read it. Delete the scattered label tables. No sim change.
4. **Generic inspector sections**: role-driven sections replacing per-kind row assembly; keep estate and pilot bespoke.
5. **Purpose categories**: add `(payer role, payee role, category)` to the `Purpose` resolver; Finance groups by category. Ledger tests unchanged.
6. **Remove the duplicated fixed fields** (`aviation`, `firms`, `households`, `plants`) in a contract major step, after the web and mock no longer read them. Use the same step as the TS contract generation (section 8) so the break lands once.
7. **Internal Rust unification (optional, last)**: `OwnerRef`; `TileKind` split into `Facility`/`Stock`/`Scenery` attributes. Touches `State` and saves: bump `SCHEMA_VERSION`, regenerate golden; do only with strong need.

## 8. Interaction with the planned platform cleanup

### 8.1 Generating the TS contract with ts-rs
Findings (no `ts-rs` dependency exists today; `grep` of all Cargo.toml finds none):
- The wire types live in `crates/sim-api` (`snapshot.rs`, `emissions.rs`, `ledger.rs`, ...) with `#[serde(rename_all = "camelCase")]` on structs and `snake_case` on keyed enums (`keys.rs`, `events.rs`, `ledger.rs`). ts-rs honours `rename_all`, `tag`, `default` and `skip_serializing_if` (as optional fields via `#[ts(optional)]`/attributes), so most types would generate correctly; the `Event` enum (`#[serde(tag = "type")]`) becomes a discriminated union, which matches what the web already has.
- **Money.** `Cents(i64)` is `#[serde(transparent)]` in `sim-types`. ts-rs maps `i64` to `bigint`, which `JSON.parse` never produces. But the wire sends dollars as `f64` (`snapshot.rs` converts `.dollars()`), so wire structs hold `f64` and ts-rs emits `number`: no problem on the wire. The problem is only in saves and `State` if ever exposed: put `#[ts(type = "number")]` on any `Cents` that reaches a wire struct, or never derive `TS` on `sim-types`. Recommendation: derive `TS` only in `sim-api` wire structs, not in `sim-types`, so `sim-types` keeps "serde only" (AGENTS dependency table; ts-rs would be a new dependency of `sim-api`, optional behind a feature, so wasm builds do not pull it).
- IDs: `u32` newtypes -> `number`; with `#[serde(transparent)]` ts-rs generates `type HouseholdId = number` (branded types are lost; the current `contract/index.ts` should be checked for whether it brands them).
- `f64` -> `number`; `NaN` is not valid JSON; the existing finite-number checks cover it.
- `Option<T>` with `skip_serializing_if` gives `field?: T`; with `serde(default)` on a non-optional field (`aviation`) the TS says it is required but older fixtures omit it: decide per field.
- Open items: the hand-written `contract/index.ts` (570 lines) has comments and doc text that ts-rs copies only from doc comments; "types only, no functions" rule means the generated file can replace it directly. The `boundaries` test already requires `contract/` to import nothing, which generated files satisfy.
- The test strategy: generate to `web/src/contract/generated.ts`, add a CI check (`git diff --exit-code`) and keep the fixtures typecheck in the web tests as the second gate.

### 8.2 Emissions and income as lists
This is the single biggest wire win to do before the generation step: a list type is one generic definition in ts-rs, while the fixed fields (`aviation`, ...) were exactly the kind of churn that made the hand-written mirror drift. Do section 7 steps 1 and 2 first (additive), generate the contract (including them), then drop the fixed fields in one major bump. Reasons: generation is cheaper on a stable list shape; the mock must change anyway.

### 8.3 Replacing the hand-written mock with recorded real-sim output
The mock is ~2,700 lines of TypeScript plus `data.generated.json` (2,822 lines) plus `mock-only.json`; it carries its own dynamics, estate, weather, ledger and finance, which duplicate rules and need parity tests (v6 finding 6). A recorded-sim port would replay a seeded run: run the real sim natively (like `docs/fixtures/` and the golden `tests/golden/seed7.json`), save per-turn `Snapshot`, `TurnResult`, `finance`, `energy`, `ledger` replies, and let a `ReplayPort implements SimPort` serve them. Trade-offs:
- Pros: no rule duplication; contract drift cannot hide; the taxonomy's lists are exercised by real data.
- Cons: choices are not interactive (a replay cannot answer "what if I set carbon price to 80"); command previews need either the real sim in wasm (already available) or a handful of recorded branches. Offline fallback is the original reason for the mock; wasm already runs in the browser, so the fallback is needed only if wasm fails to load, which is a rare case.
- Order: contract generation first (so recordings are typed), taxonomy wire lists second (so recordings do not have to be regenerated twice), recording third. Keep the mock only until `ReplayPort` passes the existing store and panel tests; then delete `sync-mock` and the parity test.
- Fixtures drift: recorded files must be regenerated by one command, tied to `SCHEMA_VERSION` and the state hash; a CI step fails when they are stale.

## 9. Risks
- **Type safety.** Lists with a `kind: string` key lose exhaustiveness in Rust and TS. Keep closed enums for `kind` and `category` (the `keys.rs` pattern: closed enum, snake_case strings) and generate the TS union; the registry validates that every registry id is a known enum value at load (like `Params` with `deny_unknown_fields`).
- **Wire churn.** Each step above is additive until step 6; contract version bumps; fixtures and the web typecheck catch breakage. The web mock must follow every wire change, which is another argument for 8.3.
- **Determinism and state hash.** Steps 0-6 do not touch `State` or the RNG: `cargo run -p sim-cli --example statehash` must stay byte-identical (the v6 acceptance test). Step 7 changes serialisation and requires a `SCHEMA_VERSION` bump and golden regeneration. A new emitter or actor that draws randomness shifts the shared stream (v7 jet precedent): put new draws last in the order and list them in the randomness table, whose test checks the source.
- **Registry becoming a second source of truth.** Rule: registry holds identity, labels, role flags and references; numbers stay in `params.json`; Rust asserts every `TileKind` has a registry entry.
- **Over-abstraction.** Roles as traits in Rust would add indirection to hot systems that are deliberately explicit. Keep roles as **data** (registry and wire), not as Rust traits, at least at first.

## 10. What stays bespoke
The model rules (labour, goods, credit, weather, lobbying formulas); the estate's jet logic and its card; the pilot's employment; the 3D models and layout of the estate; the economic-system cards; the agents catalogue's prose; the commute router; the turn layer. The taxonomy classifies and labels them; it does not generalise their behaviour.

## 11. Open questions (with defaults)
1. Should `Power plant` be one tile kind with a `fuel` attribute? Default: yes in the registry and wire, keep two `TileKind`s in Rust until step 7.
2. Is the estate one entity or a bundle? Default: a bundle, with the household as the actor and the rest as facilities carrying an `owner` edge.
3. Are lobbyists an actor kind? Default: no; they remain an aggregate institution until an NGO or a second lobbying group exists.
4. Should the player appear in the registry? Default: no.
5. Category set for emissions (`industry`, `homes`, `energy`, `aviation`, `waste`, `sink`)? Default: this list; `fire_sink_loss` stays informational and outside sums.
6. Where does the registry live: its own file or `ui.json`? Default: own file `content/entities.json`; text blurbs stay in `ui.json`.
7. Do hazards get registry entries? Default: yes, labels only; their rules stay in `weather.rs`.
8. Does generating the contract replace `docs/snapshot-contract.md`? Default: no; the doc keeps the semantics, the generated file the shapes.
9. Rename text now (step 0) before the structural work? Default: yes; it is free and reduces review noise.

## 12. Platform cleanup: status

Tracked here and in `docs/README.md`, not as a Town version. Rule for every step: no sim behaviour change (state hash byte-identical), additive or version-bumped contract, small commits.

| Step | What | Status |
|---|---|---|
| A. TS contract generation (section 8.1) | ts-rs in `sim-api` behind feature `ts`; `web/src/contract/generated.ts`; `npm run gen:contract` / `check:contract`; `cargo test --workspace` fails when it is stale; hand-written duplicates deleted | DONE (commit `e731406`) |
| B. Emissions and income as lists (sections 5.1, 5.2, 7 steps 1, 2 and 6) | `EmissionsLine.sources`, `income.sources`, content-driven labels and categories, fixed fields removed, contract v2.0, `SCHEMA_VERSION` 6 (`docs/snapshot-contract.md` section 20) | DONE for emissions, household income and finance household income (commit `472afd6`). Not done: `TraceRecord::Income` kinds and `TraceRecord::Emissions` sources still use their own closed enums; government revenue and spending are still fixed records (not in scope) |
| C. Vocabulary renames (section 6), display only | Done: households source "Homes (heating, transport)", income "Rent received", explorer text says homes, glossary (lever / action / command, emission and income source, flow vs frame, home vs public housing, estate, PC spelled out, "power plant" in player text, tile = one facility or stock). Skipped, with reasons: see `docs/known-issues.md` PC-3 | PARTLY DONE |
| Phase 2, step 3: entity registry v0 (`content/entities.json`) | `content/entities.json` (22 kinds), validated at load (`crates/sim-api/src/entities.rs`), served in `Catalog.entities` (contract v2.1, `docs/snapshot-contract.md` section 21); labels, plurals, scene models read from it; recipe `docs/how-to.md` section 21; limits in `docs/known-issues.md` PC-9 | DONE |
| Phase 2, step 4: role-driven inspector sections | The registry lists each entity's inspector sections; `panels/inspect.ts` has one builder per section id (a closed set). `emissions` is generic; the others read the snapshot of their kind. A made-up facility test: `web/tests/entities.test.ts` | DONE (bespoke sections remain, PC-9) |
| Phase 2, step 5: `Purpose` categories | not started (not part of the registry step) | OPEN |
| Phase 2, section 8.3: replace the hand-written mock with recorded real-sim output (`ReplayPort`) | Mock, `sync-mock` and the parity tests deleted. `sim-port/replay` replays `recording.generated.ts` (a cargo example, `content/demo_script.json`, seed 7, 12 turns), badged "Recorded demo"; stale-recording test in `cargo test --workspace`; `docs/how-to.md` section 22; limits PC-7, PC-8 | DONE |
| Step 7: Rust `OwnerRef`, `TileKind` split | not started, optional | OPEN |
