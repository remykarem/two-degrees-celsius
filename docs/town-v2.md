# Town v2 design (draft for review)

Status: **draft, nothing implemented.** Replaces the cohort model (decision D1 in `docs/causal-model.md`) with **individual people living in households**, puts the **map inside the simulation**, and adds **roads**, **plants that emit directly**, a **services firm**, and a much harsher **starting inequality**. Every number is a placeholder to tune. Items marked **[DECIDED]** came from the user; **[PROPOSED]** are my choices for you to confirm or change.

## 1. What changes and why

| Today (v1) | Town v2 | Why |
|---|---|---|
| 6 cohorts standing for 1,000 people | **20 individual people in 8 households** [DECIDED: individuals; households of 2 to 4] | A real "person's stats" view; unemployment, class and inequality become things you can point at |
| Housing as an aggregate capacity | **7 homes on the map**, about 3 people each [DECIDED: roughly 1:3] | Housing becomes a placed, scarce thing |
| No space in the sim; UI owns the layout (decision A) | **The sim owns a 12 x 12 grid**; position has gameplay effects | Roads and commutes only mean something if place matters. This **reverses decision A** |
| Mild inequality (Gini about 0.40 to 0.47) | **Very rich and very poor from turn 1** (target start Gini about 0.65 to 0.75) | Makes the critique visible [DECIDED] |
| Plants add capacity; emissions come only from firms and households | **Plants emit directly** (fossil a lot, renewable none) [DECIDED] | Fossil build-out can now fail the player on its own |
| No roads | **Roads shorten commutes** [DECIDED: scoped small] | A first infrastructure type beyond housing and power |
| 2 firms (Factory, Farm) | **3 firms: Factory, Farm, Shop** [PROPOSED] | Services is the likely low-carbon sector; the shop gives households somewhere local to spend |

## 2. People and households

**Person** (20): `id`, `household`, `age_band` (child / adult / retired), `skill` (0..1), `job` (a firm slot or none), `wage`, `wellbeing`, `commute_tiles`, `commute_minutes`.
- Children and retired people do not work; adults are the labour force. Starting mix (placeholder): 13 adults, 4 children, 3 retired.
- **No births, deaths or ageing in v1.** The population is fixed at 20 so tests and balance stay tractable.

**Household** (8, sizes 2 to 4): `members[]`, `home` (a tile, or none), `savings`, `debt`, `assets` (firms, homes, bank shares), income by type (wages, dividends, rent, interest, benefits), `propensity_to_consume`, `political_power`, derived `class`.
- **Money, spending, saving, rent and loans are household-level.** **Work, skill, commuting and wellbeing are person-level.**
- Class stays **derived** from ownership and income (S9), per household.

**Starting households (placeholder sizes; sums to 20)**

| # | Household | People | Starting position |
|---|---|---|---|
| 1 | **Owner family** | 2 | Owns the Factory and most wealth. Super rich |
| 2 | **Landlord and investor** | 2 | Owns most homes, a stake in the Shop and the bank. Rich, indebted on paper |
| 3 | Manager A | 3 | Skilled, owns a home, some debt |
| 4 | Manager B | 3 | Skilled, rents |
| 5 | Worker A | 3 | Employed, rents, thin savings |
| 6 | Worker B | 3 | Employed, rents, in debt |
| 7 | Worker C | 2 | Precarious job at the Farm |
| 8 | **Destitute household** | 2 | No job, no savings, **no home** (7 homes for 8 households). Dependent on benefits |

The unhoused household makes the housing shortage visible from turn 1 and gives public housing an obvious first use. Target: the top household holds about 60 to 70% of wealth; the bottom has negative net worth.

## 3. Firms and the rest of the economy [PROPOSED]

| Firm | Sector | Role | Emissions |
|---|---|---|---|
| Factory | Goods | High output, the main process emitter | High (process) |
| Farm | Food | Uses forest and commons; low pay | Low, depends on land |
| **Shop** | **Services** | Local retail and services; the main employer for non-manual work | Low |
| (1 dormant slot) | any | For class mobility, a household founding a firm | – |

Bank, Government and Nature stay as in v1. **`Sector` gains a `Services` variant.** Households spend on food (need), goods and services (discretionary).

## 4. The map is in the simulation

- A **12 x 12 grid** of tiles. Every tile holds at most one thing: a home, a firm site, a power plant, a bank, the town hall, forest, commons, a road, or nothing.
- **Starting layout** comes from `content/initial_town.json` (tile positions for everything), no longer from the UI.
- **Placement commands carry a tile:** `Place { kind, tile }`, `Remove { tile }`. The sim validates tiles (empty, buildable) and returns `BlockReason::TileOccupied` / `TileInvalid`. `preview()` returns tile validity too, so the UI can shade the grid from the sim.
- **Distances** are measured in tiles along the grid, shortened by roads (section 5).

**Buildable things in v2:** public housing (a home), fossil plant, renewable plant, **road**. Firms, forest and commons stay fixed scenery. (More kinds can follow; each needs rules.)

## 5. Roads (scoped small) [DECIDED]

- A **road** is a tile. A commute from a home to a workplace follows the cheapest path; a tile with a road costs less time than one without.
- `commute_minutes = tiles_without_road x slow + tiles_on_road x fast`. Placeholder: slow 6, fast 2.
- **Effects, intentionally small:**
  1. **Productivity:** a worker's effective hours fall with commute time: `effective = 1 - k_commute x max(0, minutes - free_minutes)`.
  2. **Wellbeing:** a commute term in S10.
  3. **Cost:** build cost plus monthly **maintenance** paid by the government.
- **Not in v1:** traffic, congestion, trade between towns, transport emissions, road types, goods transport.

## 6. Plants emit directly [DECIDED]

- **Electricity demand** each month = firms' energy use + households' energy use. **Supply** comes from fossil and renewable plants by capacity share.
- `emissions_plant = fossil_output x factor_fossil`; renewables emit 0 (lifecycle emissions are out of scope).
- Firm and household emissions are **split** into process or direct emissions (stay with the firm or household) and **energy emissions** (now attributed to plants). This replaces the old `grid_factor` multiplier **without double counting**; the trace records the source so "why did emissions rise?" can name the plant.
- Building a renewable plant shifts the mix; removing fossil capacity raises the risk of a power shortage (a simple capacity check: if demand exceeds supply, output is rationed in proportion).
- **Local pollution [PROPOSED, optional]:** a small wellbeing penalty for homes within 2 tiles of a fossil plant or the Factory. This gives placement a trade-off beyond roads. Easy to drop.

## 7. Inequality from turn 1

- The starting wealth table above is set by `content/initial_town.json`; no code change needed to make it harsher.
- **New metrics** (see section 9): average and **median income per person**, top-household wealth share, income by class.
- Rules that now bite on the starting position: rent burden, debt, lobbying power (rich households fund it), unemployment risk for the poor.
- The **wealth tax** threshold must be rethought: it is currently inert because nobody exceeds it. Set it relative to the average household.

## 8. What happens to the ten systems

| System | Carries over | Must be rewritten |
|---|---|---|
| S1 Labour | Wage and hiring rules | Matching individual people to jobs; skill and commute effects per person |
| S2 Production | Production function | Effective hours from commute; energy split |
| S3 Goods and housing | Price rules | Spending by household; **housing by home and tile**; rent per home; the unhoused |
| S4 Income | Dividends by system | Per household, by owner |
| S5 Credit | Whole mechanism | Loans to households (mortgages) and firms |
| S6 Investment | Accelerator, green tech | Mostly unchanged |
| S7 Nature | Stocks, budget | Emissions now partly attributed to plants |
| S8 Government | Taxes, spending, legitimacy | Road maintenance; per-person benefits |
| S9 Class and mobility | Class function | Per household; founding a firm means a household takes a dormant slot |
| S9b Lobbying | Gap logic | Power from household wealth |
| S10 Wellbeing | Terms | Per person; commute and local pollution terms |

The Phase 2 balance fixes (green-investment smoothing, wage and price rules, the demand levers) are **kept as rules**, but all calibrated numbers must be redone.

## 9. Wire contract and UI consequences

**Contract v1.0 (breaking change).** `households[]` is replaced by `people[]`, `households[]` and `homes[]`; `layout` and `tiles` come from the snapshot; `Command::place` and `remove` take a tile; `preview()` reports tile validity; `HistoryPoint` gains median income and top wealth share. **Real `flows.purchases`** can now be filled (household to firm), and commutes become per person. The old "UI owns the map" section of the contract is withdrawn.

**UI changes**
- **Person view:** click a person on the map or in a household list to see their stats (job, wage, skill, commute, wellbeing); a household view shows the shared money, assets and class. Replaces today's cohort inspector.
- **Metrics:** average and median income per person; top-household wealth share; unemployment as "N of M".
- **Road tool** in the build palette; tile validity shading comes from `preview()`.
- **Green, practical background** [PROPOSED]: a grass-green ground plane with a soft sky gradient, the town on a raised island plot; dirt paths where there are no roads, asphalt where there are.
- Maps and people at 20 scale: **one figure per person** walking home to work becomes literally true.

## 10. Testing and balance

- A small population has **high variance**: one firm closing can swing unemployment by 5 points. Tests therefore run **many seeds** (say 20) and assert on distributions (survival rate, median outcomes), not single runs; golden tests stay single-seed.
- Re-establish the three baselines (A no action, B sensible, C neglect) plus D (carbon tax only), each over several seeds.
- Add checks: every person belongs to exactly one household; every home holds at most one household; no tile has two things; conservation of money as before.

## 11. Delivery plan

1. **You review this doc.** I then update `SPEC.md`, `docs/causal-model.md`, `docs/snapshot-contract.md` (v1.0) and `docs/ui-design.md` to match.
2. **Rust model agent:** people, households, map, roads, plant emissions, services firm; rewrite S1, S3, S4, S9, S10; keep systems pure. Tests: invariants, determinism, multi-seed baselines.
3. **Wire layer agent (`sim-api`)** after step 2: contract v1.0, fixtures, native-vs-wasm golden.
4. **Web agent(s):** person and household views, road tool, tile validity from the sim, metrics, green scene. The scene and the panels can run in parallel again.
5. **Balance pass** and an **independent review** at the end.
Steps 2 and 3 are sequential (the wire needs the model). The web agent can start on the mock once the contract v1.0 doc exists.

## 12. Decisions I made for you (please check)

1. **Firms:** Factory, Farm and Shop (services). You did not answer question 2, so this is my default.
2. **8 households but 7 homes**, so one household starts unhoused.
3. **Starting inequality target:** Gini about 0.65 to 0.75, top household about 60 to 70% of wealth.
4. **The map moves into the simulation**, reversing the earlier decision that position is cosmetic.
5. **No births, deaths or ageing** in v1.
6. **Local pollution** near plants and the factory is included as an optional small effect.
7. **Roads are only tiles that speed up commutes**, with build cost and maintenance.

## 13. Open questions

1. Should a household be able to **move** (rent a different home, lose its home)? Proposed: yes, evictions and moves happen in the monthly tick, but nobody is ever added or removed.
2. Should **people leave town** when wellbeing is very low? Proposed: not in v1.
3. How visible should **names** be? The spec chose abstract labels, so people are "Worker A, adult 2".
