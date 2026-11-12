# Feature: registry-driven simulation (parked)

Status: **PARKED, idea only, nothing designed or built.** The user approved scoping it ("yup lets do that, park it in another feature").

## Idea
Today `content/entities.json` (22 kinds, see `docs/taxonomy.md`) drives the wire, the panels, the scene and the inspector, but the simulation itself still uses Rust enums (`TileKind`, `InfraKind`, `Owner`). Make the sim read the registry too, so a new entity kind with a known role set needs no new enum variant or per-kind match arm in `sim-model`.

## Why it is parked
It touches determinism (stable ordering, no HashMap, state hash), save compatibility and the typed-ID rules in `AGENTS.md`. Design it before any code: which roles are rules the sim can execute from data (emitter, upkeep payer, energy producer) and which stay bespoke code.

## Known related gaps
- Money-flow `Purpose` categories are not in the taxonomy yet.
- Government revenue and spending lines are fixed fields (known-issues PC-4).
- Tile-to-selection routing and inspector section headings still named in the web (PC-9).
