> **Status: HISTORICAL** (v1 scene for the cohort model). Superseded by `docs/web-notes-scene2.md`; the scene bus description remains broadly valid.

# Web notes: 3D scene

Status: the placeholder scene is replaced (`web/src/scene/`). Procedural low-poly town, camera, lenses, playback animation, pending builds, hover and selection. Pure logic is unit tested (`web/tests/scene-*.test.ts`); WebGL is verified with Playwright screenshots (`web/playwright/scene-shots.mjs`, `scene-perf.mjs`).

## Files
- `models.ts` pure part lists per building (one silhouette per kind, per class for homes); `geometry.ts` merges a model into one geometry with vertex colours and a `tintW` mask (lens recolours walls only); `entityLayer.ts` meshes, merged outline, pop-in, construction crane and turns-left badge; `fx.ts` instanced figures, goods, coins, smoke, rings.
- `entities.ts` layout + snapshot + draft -> `EntityDesc[]` (pure). `lenses.ts` colour functions and legend objects (pure). `flows.ts` MonthFrame -> tracks, emitters, effects (pure). `paths.ts` BFS routes around buildings (pure). `cameraMath.ts` pure; `camera.ts` the Three.js rig.
- `index.ts` orchestrates; `timing.ts` is the playback pacing (`app/timing.ts` re-exports it).

## For the panels: the scene bus (`web/src/store/scene-bus.ts`)
`import { sceneBus } from '../store/scene-bus'` (not re-exported from `store/index.ts`; I did not edit existing store files). All fields are Svelte-compatible readables.
- `sceneBus.hover`: `{ tile, entity: EntityRef | null, invalidReason, buildKind }`, null when not hovering (also null during playback). `invalidReason` is set only in build mode.
- `sceneBus.selection`: `{ tile, entity, facts[] }` for the store's selected tile.
- `sceneBus.legend`: `LensLegend` for the current lens (`label`, `meaning`, `scale: none|gradient|categories`, `stops[{color,label}]`, `lowLabel`, `highLabel`, `notes`).
- `sceneBus.camera`: `{ angle, zoom, zoomMin, zoomMax, panned, reducedMotion }`. Buttons: `sceneBus.requestZoom(1.25)`, `requestZoom(0.8)`, `requestReset()` (resets pan, zoom and rotates back to view 1). Q/E and `game.rotate` still drive rotation.
- `sceneBus.playback`: `{ active, month, monthT, overall }`; `sceneBus.stats`: draw calls, triangles, instance count.
- The ground tiles are shaded green or red while a building kind is chosen in the plan phase; the reason text for a hovered tile is `hover.invalidReason`.

## Design decisions
- Class colours are Okabe-Ito. Each class also has its own house silhouette and rooftop marker (diamond, cube, ball, cone, none), and commuting figures differ in height. The class legend names these shapes.
- Everything is data-driven from the snapshot: home class, firm sector and active flag (closed = vacant lot with a sign), commons access (open, community, enclosed, collapsed), forest stock (green to dry straw), fossil plant smoke from `energy.fossilShare`.
- Playback is read-only: when the store's playback month changes the scene plans that `MonthFrame` and runs it for `TIMING.monthMs`. Skipping (phase leaves playback) drops all tracks; the final state is always the static scene. Caps: 48 figures, 36 goods, 80 coins, 140 smoke, 12 effect rings (max 316 instances).
- Drawing is on demand. Idle frames repaint only for ambient smoke or turbines (about 25 fps), full rate while the camera moves or playback runs.
- `prefers-reduced-motion`: rotation, zoom and reset snap instantly and pops finish at once. Playback animation still runs (it is skippable); it is not reduced further.

## Known limits (honest list)
- Firm closure and the construction-complete pop are approximated. The store already holds the final snapshot when playback starts, so a closed firm shows as a vacant lot from the first month and the closure is marked by a ring effect at the event time, not by a collapse animation. New firms and finished builds pop in about half a second after playback starts, not at their event month.
- Goods and purchase flows pick the nearest active firm of the sector (the frame names no firm for a purchase). Rent goes to the landlord home (or the town hall for dependents) because the frame names no payee. These are cosmetic.
- The money-flow lens colours buildings by net income against spending; with the mock's income numbers many homes sit near the neutral colour. The looping coins are the clearer cue.
- No label sprites for building names (the panels' inspector shows them). The outline is one merged line mesh; during pop-in the popping building has no outline until it settles.
- Software-GL frame rate was measured, not hardware (see the report).

## Contract issues
None blocking. Observations only:
1. `purchases` has no firm and `rent` has no payee, so the scene invents plausible endpoints (see above). A `firm` on purchases and a `to` on rent would make the animation exact.
2. `MonthFrame` events carry no tile; the scene derives tiles from firm and household ids via the UI layout. Fine for v1.
3. `snapshot.pending` names commands by tile only for `place`; remove has no tile so a pending removal is not shown on the map.
