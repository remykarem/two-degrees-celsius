# Town v5.3 design: agent choices explained (approved, small)

Status: **APPROVED and IMPLEMENTED (contract v1.7; user: "lets do it", no live trace)**. Builds on v5.2 (`docs/town-v5.md`, assumptions catalog in `content/assumptions.json`, wire v1.5). No simulation change, no schema bump.

## Goal
Let the player see what kinds of agents exist, how each decides, and exactly where randomness enters.

## Scope
Add an **Agents** section to the "How it works" explorer, plus a one-line decision tag and "how decisions are made" link in the person inspector.

1. **Kinds:** Households (derived class: Capitalist, Rentier, Manager, Worker, Dependent), Firms (Factory, Farm, Shop and the dormant slot), Bank, Government, Lobbyists; Nature and Weather listed as non-agent actors. Each: what it decides, what it observes.
2. **Model per kind:** short formula plus live parameter values read from `params.json` (no hard-coded numbers in the web app). Households: food need first, then propensity x (disposable income - food need), shop split `stock x (stock-weighted average price / price)^elasticity`. Firms: hire/fire from inventory cover and expected sales, wage rule (`u_ref`, `wage_k`), price closes a share of the gap to (1 + markup) x unit cost. Bank: spread by leverage. Government: legitimacy and lobbying gap.
3. **Randomness table:** every RNG draw, its distribution and live parameter: drought onset (Bernoulli), drought severity and duration (two uniform draws), fire spread per neighbouring tile (Bernoulli), fire ignition (Bernoulli) and its tile (uniform over standing forest), home damage (Bernoulli, off at the shipped value of zero), commons enclosure (Bernoulli, only the Market system has a nonzero chance), startup roll (Bernoulli). Nine draw sites in `sim-model`; a test counts them. State that everything else is deterministic, what a seed changes, and that starting heterogeneity comes from `initial_town.json`.

## Verification notes (code check, 2026-11)
The first draft of this doc was checked against `crates/sim-model/src`; corrections applied above:
- Market, Regulated and Coop are the town-wide economic system (`content/systems/*.json`), not kinds of firm. The firms are Factory, Farm, Shop and the dormant slot.
- Price does not respond to excess demand. `goods.rs` moves price toward (1 + markup) x unit cost. Excess demand feeds expected sales (smoothing `labour.expected_smoothing`), hiring and the startup test, not price directly.
- Shop weights use the stock-weighted average price, not the simple average.
- Households do not spend a fixed budget share: food need comes first, then `propensity x max(0, disposable - food need)`, plus a wealth term for Capitalists and Rentiers.
- The randomness list was incomplete (no drought onset draw, no per-neighbour fire spread, no home damage draw). The enclosure chance lives in `systems/*.json`, per economic system. Weather resilience cuts drought severity, not drought chance.
- `Rng` is drawn only in `weather.rs`, `nature.rs` and `class.rs`; every other system ignores it.

## Out of scope
Live per-choice trace (why this household chose this shop). Possible later v5.3b; needs new sim output and a schema bump.

## Rules
- Content-driven: text and parameter references live in `content/assumptions.json`; values resolved from params at load. Verify every claim against the code (`crates/sim-model/src/{goods,labour,production,credit,government,lobbying,class,nature,weather}.rs`).
- Modular: web changes only in panels/explorer; boundaries tests must pass. Extend the catalog file with an `agents` section and add a new `agents()` wire call (contract v1.7, additive; `assumptions()` is unchanged; documented in `docs/snapshot-contract.md`).
- No bare percentages. Regenerate fixtures and sync the mock; native-vs-wasm golden test must still pass; state-hash unchanged.

## Delivery
One agent, Rust catalog first, then web. Update docs index, glossary, how-to.
