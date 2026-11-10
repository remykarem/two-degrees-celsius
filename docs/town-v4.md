# Town v4 design: weather (draft for review)

Status: **DRAFT, nothing implemented. Planned after Town v3 (the pressure clock, `docs/town-v3.md`).** Scope **[DECIDED]**: **forest fires and drought only**; no floods in v4. Severity **scales gradually** with climate stress **[DECIDED]**. Thresholds are measured as carbon budget **remaining** **[DECIDED]** and are configurable. All numbers are placeholders.

## 1. Goal

Make the carbon budget felt before it hits zero: as the budget runs down, weather gets erratic and then damaging, with real consequences for the forest, the commons, the farm, wellbeing and emissions, not just visuals.

## 2. Climate stress (the gradual dial)

`stress s` in 0..1 from the carbon budget remaining `r` (share of `budget_b0`):

```
s = clamp( (onset - r) / (onset - full), 0, 1 ) ^ curve
```
Placeholders: `onset = 0.50` (erratic weather begins at 50% remaining) [DECIDED], `full = 0.10` (maximum severity at 10% remaining), `curve = 1.5`. All in `content/params.json` under `weather`.

Before onset there are no hazards. After it, hazard odds and severity rise smoothly with `s`; there is no step change.

## 3. Hazards

### Forest fire
- **Ignition (monthly roll, seeded RNG):** `p_ignite = base x (1 + a x s) x dryness x forest_fraction`, where `dryness` rises during a drought and with low commons health, and `forest_fraction` is the share of forest tiles still standing. Cannot ignite on a burnt tile.
- **A fire** is a set of burning forest tiles with `months_left`. Each month it burns forest stock, may spread to an adjacent forest tile with probability `spread_p` (raised by drought, reduced by fire station coverage), and ends after `duration` months or when out of fuel.
- **Effects:**
  - forest stock falls per burning tile (the **carbon sink shrinks**, so net emissions rise: a feedback loop, shown in the trace as "sink lost to fire");
  - burnt tiles regrow slowly (a `regrowing` state, then forest again);
  - **smoke** gives a wellbeing penalty to people near a burning tile (radius `smoke_radius`) and a small productivity penalty;
  - the farm loses output through the existing resource-health factor (it averages forest and commons shares);
  - homes within 1 tile of a fire may be damaged (a repair cost, and an eviction risk); keep optional.

### Drought
- **Onset (monthly roll):** `p_drought = base x (1 + b x s)`; severity `d` in 0..1 and duration `D` months drawn from ranges that widen with `s`.
- **Effects while active:** commons regrowth `x (1 - a_c x d)`; farm productivity `x (1 - a_f x d)` (food output down, so food prices rise through the existing price rule; low-income households feel it most); electricity demand slightly up; wildfire dryness up. An over-harvested commons can cross its collapse threshold, which already raises an event.
- Ends when `months_left` reaches 0, after which commons regrowth recovers on its normal curve.

### Ordinary weather
Seasons and mild variability are **cosmetic only** (scene lighting, clouds); no simulation effect before onset.

## 4. Forecast and adaptation (so it is fair, and fun)

- **Forecast:** each quarter the snapshot reports `forecast.fireRisk` and `forecast.droughtRisk` (low / medium / high), computed deterministically from current conditions (stress, dryness, forest density, coverage). It reports probabilities, not the RNG's outcome, so it never spoils events.
- **Adaptation [PROPOSED, small]:**
  - **Fire station** (a buildable tile): reduces spread and duration for fires within `coverage_radius` tiles; build cost, upkeep, PC cost. Reuses the existing buildable pipeline.
  - **Resilience spending** (a policy lever, $/month, lead time 1 turn): lowers ignition odds and drought damage by a diminishing amount. Reuses the lever pipeline.
- **Interplay with existing choices:** community-managed commons keep a health buffer against drought; replanting or protecting forest lowers fire risk; cutting emissions slows stress itself.

## 5. State, events, wire

**State (`sim-types`):** `Weather { stress, drought: Option<Drought{severity, months_left}>, fires: Vec<Fire{tile, months_left}> }`; forest tiles gain a state (`standing | burning | regrowing`). A new pure system `S11 weather` runs **before nature and production** each tick; it returns deltas (stock changes, tile states, new events). **RNG:** it draws from the sim RNG, so weather finally makes **seeds differ** (today runs are near-identical across seeds because little uses the RNG).

**Events:** `fire_started{tile}`, `fire_spread{from,to}`, `fire_out{tile}`, `forest_burned{tile}`, `drought_started{severity, months}`, `drought_ended`, `home_damaged{home}` (optional). **Trace:** emissions record "sink lost to fire".

**Wire (contract v1.2, additive):** `snapshot.weather = { stress, drought, fires, forecast }`; tile state on forest tiles; events as above; `nature` gains `forestHealth`, `commonsHealth`, `farmHealth` (0..1: the shares and the resource-health factor the farm already uses). Schema version stays 2 if additive; if the State layout changes saves from v3 are refused cleanly (bump to 3 and say so).

## 6. UI and scene

- **Health meters (needed regardless):** forest, commons and farm health bars in the Ecology tab and in a small status cluster by the carbon-budget gauge. These stocks exist already but the panels do not show a farm or commons reading yet.
- **Climate stress** gauge and the **forecast** card (a calm "Dry spell likely" style warning with a risk level and an Adapt shortcut).
- **Scene:** haze and dry ground tint under drought; flames, smoke columns and darkened burnt tiles for fires; a regrowing tile look. Respect reduced motion.
- **Crisis banner and results** read the new events; the end-screen story names fires and droughts that mattered.
- A new lens **"fire and drought risk"** is optional.

## 7. Testing and balance
- Pure-function tests: stress curve, ignition and spread probabilities, drought effects on regrowth, fire station coverage.
- Property tests: forest stock never negative, tiles never double-burning, determinism per seed.
- Sign tests (as in `docs/causal-model.md` section 5): higher stress raises fire frequency; more fire raises net emissions; fire station lowers fire damage.
- **Re-run the baselines over many seeds.** Weather adds a new negative feedback on A and C and will shift outcomes; `budget_b0` and several parameters will need a pass. Weather must hurt, but a good player must still be able to win (baseline B), so adaptation is not optional design.
- Golden snapshots and fixtures regenerate; the native-vs-wasm golden test must still pass bit for bit.

## 8. Delivery plan
1. You review this doc; I update contract, causal-model (a new S11 section), architecture and how-to docs.
2. **Rust agent:** weather system, state, events, fire station and resilience lever, baselines and tests.
3. **Wire agent:** contract v1.2 and fixtures.
4. **Web agents:** health meters, forecast, scene effects (scene and panels in parallel).
5. Balance pass and independent review.

## 9. Decisions I made for you
1. `full` severity at 10% remaining, `curve` 1.5, onset 50% (the first and the 10% figure came from your earlier floods idea, now reused for fire and drought).
2. Ordinary weather is cosmetic only.
3. Adaptation is a fire station plus one resilience lever. Floods, storms and heatwaves are left for later.
4. Damage to homes from fire is optional.

## 10. Open questions
1. Should a **fire station** be a buildable the player must place (position matters, as with roads), or should fire defence be only the lever?
2. How hard should drought hit **households' food costs**: a visible price rise, or only lower farm output?
3. Do you want **cooperative or community commons** to give extra drought resilience (a reward for the community option)?
