# Glossary

Domain terms first, then code terms. Values are the current content defaults (check `content/` if in doubt).

## Game and economy

| Term | Meaning |
|---|---|
| **Turn** | One player decision round = one quarter = 3 ticks. 160 turns per game (`run.turns`). `State.turn` starts at 1 |
| **Tick** | One simulated month. `step()` runs every system once. `ticks_per_turn` = 3. 48 extra warm-up ticks run before the game starts |
| **Month frame** | The per-tick slice of a `TurnResult` (`months[0..2]`): events plus flows, used for playback animation |
| **PC (political capital)** | The per-turn currency for player actions. Start 60, cap 100, regenerates `pc_base x legitimacy` (8 x legitimacy) each turn |
| **Lever** | A global policy setting the player can change (19 of them: taxes, minimum wage, carbon price, caps, ...). Listed in `content/levers.json` |
| **Command** | A player action: set a policy lever, switch system, place or remove a building on a tile |
| **Pending (queue)** | Commands accepted but not yet in force. Each has `effectiveTurn = submitted turn + lead time` |
| **Lead time** | Turns between queueing and effect (per command kind, `params.turn.levers.<key>.lead_time`; carbon price 2, most policies 1) |
| **Cancel refund** | Cancelling a pending item returns 50% of its PC (never money) |
| **Opposition** | Extra PC cost factor `(1 + net_lobby)` on levers capital opposes |
| **Active (nominal) vs effective policy** | Active/nominal = what the player set and has taken effect (`State.policy`). Effective = after lobbying (`State.effective`): `nominal x (1 - net_lobby)` for carbon price, profit tax, wealth tax, caps |
| **Lobbying gap** | The difference between nominal and effective for those four levers (`policyGaps` in a turn result). Capital owners dampen rules that hurt them |
| **Economic system** | A bundle of ownership and distribution rules: Market, Regulated, Coop (`content/systems/*.json`). Switching is slow (transition months, output penalty, resistance). Not the same as a "tick system" |
| **Tick system (S0-S10)** | One rule module in `sim-model` (labour, production, ...). See `docs/architecture.md` section 4 |
| **Person** | One of 20 individuals: age band (child/adult/retired), skill, job, wage, commute, wellbeing. Work, skill, commuting and wellbeing are person-level |
| **Household** | One of 8 groups of 2-4 people. Money, spending, saving, rent, loans, assets and political power are household-level |
| **Labour force** | Adults who can be hired. 10 of the 13 adults (owners are outside it). Unemployment is shown "N of 10" |
| **Unemployment (smoothed)** | `unemployment_smooth` is an exponential average (weight 0.05) used for wages, legitimacy and the collapse test, because one job is 10 points |
| **Class** | Derived per household every tick: Capitalist, Rentier, Manager, Worker, Dependent. Based on ownership, income source, skill and pay |
| **Home** | A tile holding at most one household. Private homes pay rent to an owner household; public housing pays a fixed rent. 7 homes for 8 households |
| **Unhoused** | A household with no home (starts with one). Pays no rent, suffers wellbeing and security penalties. Evicted after 3 months of unpaid rent |
| **Tile** | One cell of the 12 x 12 map owned by the sim. Holds at most one thing: home, firm, plant, bank, town hall, forest, commons, road or nothing |
| **Road** | A tile that makes commutes faster (`fast_minutes` 2 vs `slow_minutes` 6 per tile). Costs $2,500 and $10 a month upkeep |
| **Commute** | Tiles and minutes from a person's home to their firm along the cheapest route. Long commutes cut effective hours and wellbeing |
| **Plant** | A power plant tile: fossil (emits `delivered x factor_fossil`) or renewable (emits 0). $40,000 each. Capacity 5 per plant |
| **Electricity rationing** | If demand exceeds total plant capacity, all output is scaled by `supply / demand` (`electricity_rationed` event) |
| **Firm slots** | Factory (goods), Farm (food), Shop (services), plus one dormant slot a household may found a firm in (`MAX_FIRMS` = 4) |
| **Forest, commons** | Natural stocks. Commons access is `open` or `community`; can be enclosed or collapse |
| **Budget b0** | `nature.budget_b0`, the town's cumulative-emissions allowance: 4,400 t. Exhausting it ends the game (`budget_exhausted`) |
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
| **Frame / flow** | Per-month animation data: commutes, purchases, wages, rent, emissions |
| **Contract** | The wire format between sim and UI: `docs/snapshot-contract.md` + `web/src/contract/index.ts`. Versions v0.1, v0.2, v0.3, v1.0 (Town v2) |
| **Schema version** | `SCHEMA_VERSION` = 2. Layout of state/wire; mismatching saves are refused |
| **Snapshot** | The full current view of the game sent to the UI (`Snapshot`) |
| **SimPort** | The TypeScript interface the UI uses to talk to a sim (`web/src/sim-port/port.ts`) |
| **Mock vs real sim** | Mock: TypeScript stand-in (`sim-port/mock`), numbers from `content/` via `sync-mock`, invented dynamics, shows a "Mock data" badge. Real: the Rust sim compiled to wasm (`sim-wasm`), "Real simulation" badge |
| **wasm pkg** | `web/src/sim-port/wasm/pkg/`, built by `npm run build:wasm`, gitignored |
| **Golden** | A recorded run (`tests/golden/seed7.json`, from `plan.json`) whose per-reply hashes must match natively and in wasm. Also an insta snapshot for seed 42 |
| **Fixtures** | Real wire JSON in `docs/fixtures/`, generated by the sim-api example; the web tests check their types against it |
| **Plan** | A JSON turn script (`content/scenarios/*.json`) the CLI and tests play |
| **Baselines A-D** | A no action, B sensible play, C neglect/extraction, D carbon tax only. 20 seeds each in `tests/scenarios.rs` |
| **Draft** | UI-only list of commands not yet submitted; each has a `preview()` |
| **Phase** | UI state machine step: review, plan, playback, results, ended (`store/phase.ts`) |
| **Lens** | A map colouring mode: none, emissions, class, wealth, employment, flows, commute |
| **Scene bus** | `store/scene-bus.ts`: channel where the scene publishes hover/selection and panels send camera requests |
| **Town v1 / v2** | v1: 6 cohorts standing for 1,000 people, UI-owned map. v2 (current): 20 people in 8 households, sim-owned map, roads, plant emissions, Shop. Anything describing cohorts is historical |
