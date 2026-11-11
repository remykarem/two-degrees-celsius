# Web notes: Town v7 scene (the estate)

Scope: the Three.js scene only (`web/src/scene/**`, `web/src/store/scene-estate.ts`, `web/tests/scene-estate.test.ts`). Panels are in their own notes.

## What was built
- **Estate house** (`scene/estateModels.ts` `HOME_ESTATE`, chosen in `entities.ts` when `snapshot.estate.household` owns the home): two storeys, cream stone, slate roof with gable and gold finial, chimney, columned portico on the east front, balcony over the pool, warm light in the upper windows. Bigger than every other home. It is a normal tinted home entity, so lenses, hover and selection work as for any home; the `tag` ("Estate owner") is added to the facts.
- **Grounds** on the `estate_grounds` tiles: the wire gives no roles, so `estateRoles()` (store/scene-estate.ts) assigns them: nearest a road = gravel drive with gate pillars and a turning circle; next nearest the house = pool lawn (pool, loungers, parasol); the rest = garden (conifers, beds, a gravel path to the hangar).
- **Runway** on the `runway` tiles: asphalt strip along the south edge of the row with a dashed centre line, edge lines and threshold bars; an apron and a small open-front **hangar** on `estate.hangarTile` (in the real data this is also the first runway tile); a **wind sock** at the far end (sways unless Reduced motion). Everything is laid out in a runway frame (`estateLayout.ts`: `u` along the runway, `v` sideways), so a runway in another direction would still draw (untested beyond east-west).
- **Jet** (`estate.ts`, `jetPath.ts`): one merged mesh. Parked in front of the hangar; hidden while `estate.jet.flyingThisMonth` and no month is being played. In a month whose events hold `private_jet_flight` (turn playback: that month's frame; pressure mode: the live frame) the jet taxis, rolls east, climbs out over the meadow, is away, then lands from the far end, rolls back and parks. The flight is a pure function of month time (`jetPoseAt`), so it follows the existing clocks: 2 s per month in turn playback, the clock feed in pressure mode (it freezes on pause). If the flight is in the last month of the turn the jet stays away (matches `flyingThisMonth`).
- **Trail and emissions puff**: stateless particles sampled from the jet's own earlier positions (`jetTrail`): dark exhaust puffs near the ground and early in the climb (darkness scales with the month's aviation tonnes over `estate.flightTonnes`, both from the wire), white vapour after. Under the emissions lens the exhaust is tinted by the lens colour.
- **Reduced motion**: the jet stands on the runway for a moment with one puff, no flight path, no trail, no sock sway.
- **Pilot**: `flows.staffCommutes` are merged into the commute list in `planCommutes`, so the pilot walks from home along roads to the hangar tile with the existing commute animation (verified: real wire, positions sampled over a month).
- **Ninth household**: dark gold (`0xb8860b`) and a star marker (`people.ts`, `peopleLayer.ts`); nine colours and nine shapes.
- **Lenses**: employment and commute treat staff (`PersonView.employer`) as employed/commuting; the estate home is coloured by its footprint under the emissions lens; the month's aviation tonnes add a heat weight on the runway (hidden under the runway slab, so the visible effect is the house and the trail); class and wealth use the household as for any home; fire lens is unchanged (estate tiles are not forest).
- Debug hook for screenshots: `window.__scene.debugFlight(t | null)` (dev builds) shows the jet at month time `t`.
- Screenshots: `web/playwright/v7-scene-shots.mjs <outDir> [mock|real] [seed]` (needs `npm run dev -- --port 5180`).

## Fixed: wire bug
`MonthFlows` in `crates/sim-api/src/turn.rs` lacked `#[serde(rename_all = "camelCase")]`, so the real wire sent `staff_commutes`. Fixed (the other sim-api structs with multi-word fields already rename them); the scene now reads only `staffCommutes`, and `web/tests/fixtures.test.ts` checks that no snapshot, turn or month fixture key contains an underscore (the `policy` lever map excepted).

## Unverified / rough
- Only checked at 1440x900 in software GL (SwiftShader); frame rate with the jet not measured separately.
- Pressure mode (live clock) was not run with a flight; the code path uses the same pure functions as turn playback.
- Saved games and old wire packages: no `estate` means no estate scene (tested with `estate: undefined`).
- The jet is drawn larger than to scale (about half a tile long) so it reads at the default zoom.
- Estate tiles are not pickable entities: clicking grounds, runway or hangar selects the plain tile (`selectionAt`); keyboard description says "estate grounds" / "estate runway". The hangar and runway are not tinted by lenses (only ground tiles and homes are).
- The pilot's route crosses the pool-lawn-free garden and drive tiles because the routing layout (`store/map.ts`) does not know estate tiles; it looks right today but it is a coincidence of the fixed map.
- The three-point turn after landing is a swing-in (the jet rotates while taxiing to the stand); it is fast (about half a second at 2 s per month).
