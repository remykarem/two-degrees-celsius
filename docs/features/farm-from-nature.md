# Feature: the farm draws its food from nature (overconsumption and the tragedy of the commons)

Status: **APPROVED by the user with ALL open-question defaults (Q1 to Q8); IMPLEMENTED (see section 9, As built).** Written from the code at commit `30b7959`. Where this doc and the code disagree, the code wins (`docs/README.md`).

The game is pedagogical and the user is new to economics: this feature exists so the player can SEE overconsumption (taking from a shared stock faster than it regrows) and the tragedy of the commons. Thin UI / DRY applies (`docs/town-v6.md`): every number, sentence and flag below is computed in Rust or authored in `content/`; the web only renders.

## 1. Problem: nature does not limit food today

What the code does (verified):

- **Output** (`production::capacity`): `Y = tfp x K^alpha x L^beta`, then for the Farm only: `x resource_health^farm_health_exp` (0.5), `x climate_factor`, `x (1 - a_f x drought)` (a_f 0.3), `x (1 - community_coord_cost)` (0.03, community access only). `resource_health` = `sim_metrics::farm_health` = mean of forest share and commons share, each 0..1.
- **Harvest is a side effect** (`nature::run`): `harvest_c = sum(output) x farm_harvest_commons (0.007) x policy.commons_harvest`; `harvest_f = sum(output) x farm_harvest_forest (0.002)`. Access rule: Open (nothing), Enclosed (`x1.5`, fee $2 per unit to the owner), Community (`min(harvest_c, cap x MSY)`). Then `harvest_c = min(harvest_c, commons)`.
- **Stocks**: commons `dC = 0.02 x C x (1 - C/1000) x drought_factor - harvest_c` (drought_factor = `1 - a_c x drought x (1 - community_buffer)`, a_c 0.8, buffer 0.5); forest `dF = 0.004 x F x (1 - F/2000) - harvest_f - pending_clearing`. Collapse event when commons falls below 1% of max.
- **MSY** = `g x max / 4`: commons 0.02 x 1000 / 4 = **5 units/month** (at C = 500); forest 0.004 x 2000 / 4 = **2 units/month** (at F = 1000).

Consequence: the harvest is computed after the food was already made, so food is **not limited by what nature can give**. A dead commons only dampens output by a square-root multiplier (at C = 0 and F = 1400, health = 0.35, multiplier 0.59; the farm still makes more than half its food from a gone commons). The community cap limits only the depletion, not the food. Nothing here can create a hard shortage, so Markets and prices never show nature running out.

### Measured baseline (seed 42, no actions, ticks 4 to 40; `debug` example)

- Start state (`initial_town.json`): forest 1400 (70% of max), commons 800 (80%). `farm_health` 0.75, multiplier 0.87.
- Farm output `Y` about **512 to 515** food units/month early, 466 to 653 later (2 to 4 farm workers; the debug column `Y` of the Food sector). Household food need is 22 people x `food_need_units_pc` 25 = **550**/month, so the farm runs near the town's need and holds 400 to 1100 in inventory.
- Harvest at start = 512 x 0.007 = **3.6 commons units/month = 0.72 of MSY**; forest 512 x 0.002 = **1.0 = 0.5 of MSY**.
- But the sustainable line depends on the stock, not only MSY: regrowth at C = 800 is `0.02 x 800 x 0.2 = 3.2`/month, so 3.6 is already **above regrowth by 0.4/month** (about 1.5 commons units per turn). The commons drifts down slowly in a no-action game; it is below MSY the whole time, so it does not collapse in 10 years. This is the honest teaching point: "under MSY" is not "sustainable at this stock".
- To measure and put in the tuning log before building (cheap, read-only): commons and forest at turns 0, 10, 20, 30, 40 for baselines A to F (extend `baselines` or `tune_probe` output only after approval; no code now).

## 2. Design (1): make the stock the real source

Units: **1 unit of commons** is the stock unit the game already uses (`Units`, max 1000). **1 unit of food** is a Farm output unit (`Y`). Today's exchange rate is fixed by `farm_harvest_commons`: one commons unit sustains `1 / 0.007 = 143` food units (and one forest unit 500 food units). Keep that rate: it is what makes the start state unchanged.

### Formulations compared

**A. Schaefer / CPUE cap (recommended).** Fishing-style: catch `h = q x E x S` (catch per unit effort times effort times stock). Here effort is the farm's production capacity, and the cap says the harvest the stock can supply this month is `h_max = kappa x C` with `kappa` the share of the stock the farm can physically take in a month. Production is capped so its harvest fits:

```
h_max_c   = access_rule( kappa_c x C_prev )         # Open: as is; Enclosed: x enclosure_harvest_multiplier; Community: min(.., cap x MSY)
Y_cap_c   = h_max_c / ( farm_harvest_commons x commons_harvest )
Y         = min( Y_cobb_douglas x climate x drought x coord_cost , Y_cap_c )
```

`C_prev` is last month's end stock (production runs before `nature` in `TICK_ORDER`; a one-month lag is fine and avoids reordering). Keep the existing health multiplier unchanged (it models soil/forest condition); the cap is a second, harder limit. Harvest stays `Y x 0.007 x lever`, so with the cap not binding nothing changes.

- Math: the cap binds when `Y x 0.007 x lever > kappa_c x C`. With `lever` = 1 and Y = 512 the harvest is 3.6; choose `kappa_c = 0.0090` so `h_max = 7.2` at C = 800 (twice today's harvest, headroom for growth and the demand rising) and the cap binds below `C = 3.6 / 0.009 = 400`, i.e. just under MSY stock (500). At C = 100 the cap is 0.9 commons units = 129 food/month: a hard supply limit, about a quarter of need. At C = 0: Y = 0.
- Lever: `commons_harvest` (0 to 3, today 1.0) raises the commons taken per unit food (harsher extraction). At 2.0, harvest is 7.2 at start (at the cap), the stock falls fast, and the cap tightens as it falls: the lever buys food now and costs food later. That is overconsumption in one slider.
- Access rules finally bite on food: Enclosed raises `h_max` by 1.5 but the owner charges $2 per commons unit (a cost the farm passes into its price via cost-plus, section 5); Community caps `h_max` at `cap x MSY` (a physical food limit, today it limits only depletion).
- Forest gets the same cap with `kappa_f` (its harvest 1.0 vs MSY 2.0 is lighter): `Y_cap_f = kappa_f x F / farm_harvest_forest`. Set `kappa_f` so it is non-binding above F about 400. Y is the min of both caps. Soil: no soil stock exists in `State` (as far as read); out of scope.

**B. Hard cap by harvestable stock.** `Y <= (C - C_floor) x 143 x (1/lever)`: output cannot exceed what is physically there this month. Simple to explain ("you cannot take more than exists") but the cap is so loose until the last moment (C = 800 allows 114,000 food) that it never binds until collapse; it teaches nothing about overshoot and gives no early warning. Rejected.

**C. Replace the multiplier by a stock cap.** Delete `resource_health^0.5` and use only `min(Y, cap)`. Cleaner model, but it changes the start state (multiplier 0.87 vanishes, output rises 15%) and loses the smooth soil/forest decline. Rejected for balance (section 6); it is a possible later simplification.

### Recommendation: A

It uses a standard fishery model (a teachable concept), keeps today's 143:1 rate, leaves the start state unchanged because the cap is not binding there, makes the access rules and the lever physical, and binds exactly when overconsumption has gone far, which is when the player must see supply fall.

### Calibration against the baseline

Target: with default settings, start state and the 900 t, 40-turn no-action run barely change.

- Start: `h_max_c` 7.2 vs harvest 3.6 (cap not binding, ratio 0.5); forest similarly. Output, prices, state hash of the first turns identical except the hash changes by schema/new fields (section 7).
- No-action run: commons never fell below the cap-binding level 400 in the sampled seed (800 falling by about 0.4/month early; it slows as regrowth rises toward MSY at lower C). To confirm over seeds 1 to 20 with the probe; if any seed binds before turn 40, lower `kappa_c` (tuning knob 1).
- Pass bar: baselines B and F still survive 20 of 20; A, C, D, E budget run-out turns within about half a turn of the table in `docs/tuning-log.md` ("Independent review F3"): A 24.00, B no run-out, C 22.33, D 24.67, E 24.00, F no run-out.

## 3. Design (2): show overconsumption

Computed in Rust (`sim-metrics`, a small `pub fn` the model also uses, per `docs/how-to.md` section 14 step 1) and served as breakdowns/series; labels and sentences from `content/ui.json`; no percentages-as-numbers: counts and ratios worded as "X per Y".

**Sustainable-yield view** (a breakdown `commonsHarvest` plus a series, shown in Ecology; the Markets food card links to it):

| Row (list item, label from `ui.json`) | Value | Unit |
|---|---|---|
| Stock now | `C` | units of commons |
| Regrowth this month at this stock | `g C (1 - C/Cmax) x drought_factor` | units/month |
| Best possible regrowth (maximum sustainable yield) | `g Cmax / 4` | units/month |
| Taken this month | `harvest_c` | units/month |
| Taken per unit regrown | `harvest_c / regrowth` | "1.1 taken for every 1 regrown" |
| Over the sustainable line? | `harvest_c > regrowth` | flag |
| Months until the commons collapses at this rate | `(C - collapse_level) / (harvest_c - regrowth)` if over, else none | months |
| Food the commons can supply at most | `Y_cap_c` | food units/month |
| Food the town wants | household need + shop demand (Markets) | food units/month |

`parts` reconcile: regrowth minus taken equals the monthly change in the stock (tolerance as in `how-to` 14). A series (monthly) of stock, regrowth and taken lets the web draw "taken vs regrowth" without computing anything. The forest gets the same rows (its own labels) so the carbon sink link is visible.

**How demand drives harvest** (stated in the breakdown formula text): people (22 x 25 food units need) and income set food demand; the Farm hires and produces to expected sales (cost-plus quantity adjustment, `docs/features/markets.md` section 2); output sets harvest; the `commons_harvest` lever scales commons taken per unit food. Limits: lever clamp 0 to 10 in `apply.rs`, content range 0 to 3; setting it to 0 means the farm takes nothing from the commons yet still makes food (and with the cap, `Y_cap_c` is unbounded as lever to 0: define the cap as non-binding then; open question Q4). Food prices do not raise demand (need is fixed; the budget is min(need cost, savings)), so price is not a lever on harvest; population and wealth are.

## 4. Design (3): interactions

- **Weather / drought (S11):** drought already cuts output (`a_f` 0.3) and regrowth (`a_c` 0.8). With the cap, a drought during a low stock is a double hit: slower regrowth lowers next month's `C_prev`, which lowers the cap. The breakdown's regrowth row uses `drought_factor`, so the player sees why the sustainable line moved. No new draw; the weather stream is untouched (schema 5 rule: no draw added or reordered).
- **Community buffer:** a Community commons keeps half its regrowth in drought (`community_buffer` 0.5) and pays 3% output (`community_coord_cost`); under this feature it also has the `cap x MSY` food limit. Teaching point: a rule that limits yourself protects the stock and the food in a drought.
- **Enclosure fee:** owner takes $2 per commons unit (via `Purpose::CommonsFee`, unchanged). Enclosed raises `h_max` 1.5x, so more food now, a faster-falling stock, and the owner's income rises (inequality link). Fee is a cost to the Farm, so cost-plus pass-through raises the shelf price only if unit cost includes it (verify: unit cost = wage bill + carbon cost today; the fee may not be in it; open question Q5).
- **Forest clearing from building:** `forest_clear_per_build` 0.0002 per build removes forest, which lowers the forest cap and the sink; the breakdown's forest rows show "cleared by building" as a part.
- **Carbon sink:** `sink = sink_sigma x F`; forest taken for food and clearing lowers the sink and speeds the 900 t budget. Shown as the forest's last row.
- **Markets and trades (scope A, information only):** when `Y` is capped, Food supply falls, so the existing "unmet units" (demand minus sold) and the "Farm sold out" chip and the shortage rows in the trade list appear on their own. Add one cause word via the existing cause-word mechanism in `ui.json` ("the commons cannot supply more food") and a link from the food card to the Ecology view. No new Markets types needed beyond the cause word.
- **Prices:** cost-plus (`price_adjust` 0.05 of the gap to `(1+markup) x unit cost`): a shortage does NOT raise the price; it shows as unmet demand, and unit cost rises only if output falls (fixed wage bill over fewer units: yes, a smaller `Y` raises unit cost, so prices do rise slowly, by the cost route, not by scarcity). Say so in the UI, honestly. Scarcity pricing is scope B (`markets.md`), not approved and not part of this feature.

## 5. Design (4): pedagogy

Explorer topics (`content/assumptions.json`, `docs/how-to.md` section 15; no numbers in text, they come from `params` and `examples`; the worked example is a `pub fn` registered in `assumptions::registry()`):

1. **Maximum sustainable yield**: the most you can take every month without shrinking the stock; shown with the regrowth curve (stock on one axis, regrowth on the other, peak at half the maximum). Plain words: "a pond makes the most new fish when it is about half full".
2. **Overshoot and collapse**: taking more than the regrowth shrinks the stock, which shrinks the regrowth, which makes the same harvest even more over the line; below a threshold it does not come back. Explains the months-to-collapse row.
3. **The tragedy of the commons**: nobody owns the stock, each farm gains from taking more and all share the loss. Needs one sentence on why the game's farm does it (it does not decide to; demand and the lever do).
4. **Open vs enclosed vs community commons**: open (no limit), enclosed (an owner limits it by charging, but takes more), community (agreed cap, small coordination cost, drought buffer). Mechanism tags: open = market (no rules), enclosed = market (private ownership), community = community rules (Ostrom, tag already planned in `markets.md` section on mechanisms).

Mechanism tags: the commons card is tagged "community rules" or "market" per access rule (a list on the wire, label from `ui.json`). Wording: plain English; "units of commons" explained once as "a unit of the shared stock of land, water and wild food"; no bare percentages ("3.6 taken for every 5 the commons can regrow at best"). `usedBy` lists must include the new metric keys, the `commons_harvest` lever, and anchors `panel:ecology` and `markets:food`; `crates/sim-api/tests/assumptions.rs` enforces it.

## 6. Design (5): balance risk plan

- Re-run baselines before any code and after: `cargo run --release -p sim-cli --example baselines` (A to F, seeds 1 to 20), `tune_probe` for exhaustion turn and legitimacy. Record in `docs/tuning-log.md` as a new section; same table format as "Independent review F3".
- **Must not change** (default settings): start state; B and F survive 20 of 20; the A, C, D, E run-out turns within about half a turn; no event added or reordered; weather timing (own stream); the no-action game's weather visibility (event within 3 turns of onset in 19 of 20 seeds).
- **Tuning knobs** (content only, `content/params.json` `nature`): `farm_cap_share_commons` (kappa_c, start 0.009), `farm_cap_share_forest` (kappa_f), and, if wanted, `commons_collapse_share`. Do not touch `farm_harvest_*`, `commons_growth`, `forest_growth` (they define the calibration and other tests).
- **Expected effect on B/F winnability:** none by design (cap is not binding). A player who raises `commons_harvest` or lets the commons be enclosed sees food fall late in the game; that is a new way to lose well-being (`legit`, `wb`) and so a choice the player now faces. Sensible play (B, F) never raises the lever; expect unchanged. A to E may differ slightly late if the commons is low (drought plus open access); that is the point.
- **State hash:** the hash changes if (a) any new field enters `State` (schema bump) or (b) behaviour changes. Aim for (a) only with no stored cap value: compute `Y_cap` from `C_prev`, so no State field, and the hash is identical while the cap does not bind. If a field is added: bump `SCHEMA_VERSION`, run `cargo run -p sim-cli --example statehash` before and after, and re-bless on purpose: fixtures (`cargo run -p sim-api --example fixtures`), wasm golden, insta snapshot, recorded demo, as done for "F3" in the tuning log.

## 7. Design (6): wire, contract, tests, delivery

**Wire (additive, contract v2.6 proposed, next free after the inflation draft v2.5):** a breakdown `commonsHarvest` (and `forestHarvest`) via `make(id, title, formula, inputs, result, unit, parts)` with `result` = months until collapse or the taken-per-regrown ratio (pick one headline: taken per regrown), parts = regrowth, taken, change; plus a monthly series; rows as lists of `{key, label, value, unit}` (labels from `content/ui.json`, the web never names a row). Markets: one new cause word key and a link anchor on the food card. ts-rs generates TS (`cargo test -p sim-api --features ts`, `docs/how-to.md` section 19); edit only `web/src/contract/index.ts` aliases; update `docs/snapshot-contract.md` and regenerate fixtures and golden. Trace: harvest and cap per month as an output-only `TraceRecord` only if the series cannot be rebuilt from the end state (it can: stock, regrowth and harvest are stored: `harvest_commons`); avoid a log change (no schema bump).

**Tests (sign tests, mechanism sign-test style):**
1. Higher harvest effort (`commons_harvest` 2.0 vs 1.0) gives a lower commons stock after N months (same seed).
2. Stock at zero (commons 0, forest at 1400) gives `Y <= 0` from the commons cap (food output exactly capped).
3. Cap not binding at default start: output equals pre-feature output for the first 12 ticks (golden prefix).
4. Reconcile: `harvest_commons` = sum of per-firm harvests (if multiple Farms, 3 active in some runs) and breakdown parts sum to the headline.
5. Community cap: food output with cap 0.5 x MSY is lower than open at a low stock.
6. Drought at low stock lowers next month's cap (sign).
7. Invariants (`sim-checks`): finite numbers; `Y_cap >= 0`; stocks within `[0, max]`.

**Delivery steps:** (1) measure baselines and stock paths (read-only probe, record); (2) params + `nature.rs`/`production.rs` cap, behaviour test 3 first; (3) `sim-metrics` pure fns; (4) breakdowns and series in `sim-api`; (5) `ui.json` labels, cause word, topics in `assumptions.json`; (6) web Ecology panel and Markets link (render only); (7) regenerate TS, fixtures, golden, recording; (8) re-run baselines, tuning-log section; (9) docs: contract section, glossary (maximum sustainable yield, overshoot), markets.md Q5 note, README row status.

**Out of scope:** scarcity pricing (Markets scope B); a market or price for the commons; a soil stock; new random draws; daily ticks; changing `farm_harvest_*`, growth rates or `commons_max`; the estate or carbon price rules.

## 8. Open questions (defaults)

- Q1. Formulation? **Default: A (Schaefer-style stock cap).**
- Q2. Keep the `resource_health^0.5` multiplier? **Default: yes, unchanged (option C is a later cleanup).**
- Q3. Cap on the forest as well? **Default: yes, with `kappa_f` non-binding above F of about 400.**
- Q4. What does the cap do when `commons_harvest` is 0? **Default: cap non-binding (the farm takes nothing from the commons, so the commons cannot limit it).**
- Q5. Does the enclosure fee enter the Farm's unit cost (so the price rises)? **Default: check the code first; if not, leave as is and report as a known issue, not fixed here.**
- Q6. Headline for the breakdown: taken per regrown or months to collapse? **Default: taken per regrown; months to collapse as a row.**
- Q7. One-month lag (`C_prev`) acceptable? **Default: yes.**
- Q8. Should the demo/recording or the Markets food card warn before the cap binds? **Default: yes, a "past the sustainable line" flag in Ecology only; no popup.**

## 9. As built

Status: IMPLEMENTED (contract v2.6, `docs/snapshot-contract.md` section 27, recipe `docs/how-to.md` section 26). Where the build differs from the draft:

- Cap: `sim_metrics::food_cap`, applied in `production::run` to the farm's output before rationing; several farms share it pro rata; no State field, no schema bump. `kappa_c` 0.009 and `kappa_f` 0.0026 (forest non-binding above about 400). `capacity()` itself is unchanged so hiring does not see the cap.
- Q5: checked in the code: the enclosure fee is NOT in the Farm's unit cost (`goods.rs`). Left as is (default), listed as known-issues FN-2.
- Wire: its own `natureYield` list (not a `Breakdown`, because the headline is a worded ratio and the parts reconcile to the monthly change); series per turn from the history (optional history fields); the forest rows omit "cleared by building" (FN-5); the Markets signal is a chip `commons_limit` (forward-looking) plus a `link` on the card, not a price cause word (the cause word is a per-market price rule).
- Calibration result: baselines A, B, D, E, F unchanged; C moves 3.3 turns later (`docs/tuning-log.md`, FN-1).
- Finding: with `kappa_c` below the growth rate an open commons cannot collapse from harvest alone (FN-3).
