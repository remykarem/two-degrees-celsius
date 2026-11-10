> **Status:** foundation notes for Town v2. The "Left rough" list is mostly fixed by `docs/web-notes-scene2.md` and `docs/web-notes-panels2.md`; the "Contract issues" list is still valid. Consolidated list: `docs/known-issues.md`.

# Web notes: Town v2 foundation

Status: foundation done (types, port, mock, store). Scene and panels are only minimally adapted; two more builders (scene, panels) take over from here. Contract v1.0 is `docs/snapshot-contract.md` section 11, with the real wire shapes in `docs/sim-api-notes.md` ("Town v2").

## What changed
- `contract/index.ts`: v1.0 types (`map`, `Tile`, `PersonView`, new `HouseholdView`, `HomeView`, `PlantView`, tile-carrying `place { kind, tile }` and `remove { tile }`, `tile_occupied` / `tile_invalid`, `CommandPreview.tileValid/tileReason`, Sector `services`, new indicators and history fields, new events, trace source `plant`, `schemaVersion` 2). The cohort types are gone. Where the real wire (fixtures, `sim-api-notes.md`) carries more than section 11 lists, the types follow the wire (see Contract issues).
- `sim-port`: `SimPort.validTiles(kind)`. The wasm wrapper calls `valid_tiles` when the package has it, otherwise derives the answer from the snapshot. `loadWasmPort` refuses a package that does not report `schemaVersion` 2 (so an old build falls back to the mock with the visible notice). Without a package the mock runs with the "Mock data" badge as before.
- Mock (`sim-port/mock`): owns a 12 x 12 grid from `content/initial_town.json` (via `npm run sync-mock`, which now reads households, people, homes, map, plants, firms and the road costs). 20 people, 8 households, 7 homes (one household unhoused), 3 firms plus a dormant Shop 2, roads, plants. Real rules kept: PC, lead times, costs, funds check, tile validation (same order as `tile_problem` in `crates/sim-engine/src/turn.rs`), activation-time re-validation, cancel refund. Invented and labelled mock: all economy dynamics, household money, wellbeing per person, job matching, evictions (3 months of arrears) and moves, emissions split (plants share), commutes (Dijkstra, slow 6 / fast 2 minutes per tile from content, unhoused get 36 minutes). `mock-only.json` lever ranges now equal `content/levers.json`.
- Store: no `layout.json`, no reconcile. `store/map.ts` has `layoutOf(snapshot)`, a pure, memoised VIEW of `snapshot.map.tiles` in the shape the scene draws from (scenery, placed, pendingTiles, roads, pendingRoads). Drafts carry a tile; one change per tile. `GameState.validTiles` (from `port.validTiles`) feeds `tileError()`. `Selection` is person / household / home / firm / tile; `selectionTile()` maps it to a map tile. `game.removeAt(tile)` drafts a removal. Save file version 2 has no layout; old saves are refused with a clear message (file version, or the sim's schema message).
- Scene: draws buildings from `layoutOf(snapshot)`; real roads are asphalt tiles, queued and drafted roads tint the tile. Panels: inspector shows person, household (with clickable people), home, firm, tile cards and a Remove button per tile; build palette gains Build road.

## Left rough (for the scene and panels builders)
Scene
- Services firms (Shop) reuse the goods model; the dormant Shop 2 is the vacant lot. Roads are tinted ground tiles, no road meshes, no junction logic. Road ghost model: none (tile tint only).
- People are not drawn as markers; commute figures are still the old per-household figures (one per commute row, aggregated route drawn by the scene's own router, not the sim's path: the wire has no path).
- Homes: colour by household class, vacant homes use the worker model. No indicator for the unhoused household. No green island or sky.
- `pathTiles` footpaths are still computed in the scene; they are decorative.
- Plant emissions flows (source `plant`) are not used; plant smoke comes from the placed plants and the fossil share.
- Purchases animate as goods from the named firm to the home (real flows), nothing special for rent to public housing.
Panels
- Inspector is a basic card: no wellbeing breakdown, no commute-time chart, no household list panel on its own (only through a home click).
- Metrics panel does not show average income, median income, top-household wealth share or "N of M" unemployment yet (the keys and labels exist in `format.ts`).
- Briefing text lists "Lobbying weakened X: set A, about A in force" even when nominal equals effective (existing behaviour, visible with both sims).
- Remove for a public-housing tile that is occupied is only reachable from the tile hover/vacant-home card, not from the household card.
- Event ranking (`panels/events`-style helpers) still treats `commuted` as noise; person events show person labels such as "Manager A Adult 1".
- Lens "employment" uses adults with jobs over adults (PersonView has `inLabourForce` but the scene lens helper does not read it).

## Contract issues
1. Section 11 under-specifies the wire: the real `PersonView`, `HouseholdView`, `HomeView`, `Snapshot` (`plants`), `HousingView`, `EnergyView`, `ActionInfo` carry extra fields (`inLabourForce`, `jobGuarantee`, `effectiveHours`, `wellbeing`, `footprintTonnes`, `foodMet`, `value`, `plants[]`, `needsTile`, `tileRule`, ...). The TypeScript types include them as required; the mock fills them. `HousingView` and `EnergyView` changed shape (capacities replaced); the old fields are gone.
2. `flows.commutes` has no tile path (`from`, `to`, `tiles`, `minutes` only); `flows.wages` is per person, `flows.rent` names the home; `commuted` events carry no `household`. Section 11 says "with a tile path".
3. `household_moved.to` is a home id; `built`, `removed`, `electricity_rationed` events are additions.
4. `Tile.buildable` is static (empty); only `validTiles(kind)` and `preview().tileValid` know about queued builds. `validTiles` is the same for every kind today.
5. `remove` carries only a tile, so its price depends on the kind on the tile; `ActionKey` keeps `remove:<kind>`. A remove on an empty tile is `nothing_to_remove` (previewed with the `remove_road` lead time on the real wire).
6. `tileReason` is typed `string` (a BlockReason code on the wire). The dormant firm slot appears as a `firm` tile on both sims.
7. The real sim's `buildings` counts and `housing` no longer include capacities; panels that used `publicCap` / `privateCap` were changed to `homes` and `unhousedHouseholds`.
8. Mock only: `schemaVersion` of the mock's own save equals the snapshot's (2).

## Verification (this stage)
`npm test`, `npm run typecheck`, `npm run lint`, `npm run build` green. Playwright on port 5179 against both the mock and the new wasm package: load, road and renewable-plant planning with tile validity messages, end turn, skip, select tile, save (file version 2, no layout) and load. Not verified: the keyboard tile cursor with the new selection model, touch input, the wasm `valid_tiles` path in the browser (the package in this checkout may predate it; the wrapper falls back to the snapshot).
