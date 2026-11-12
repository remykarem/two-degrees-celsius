# Web notes: corn on the commons

The commons is drawn as a corn field that thins as the commons stock share falls. Scene only; no wire or content change.

## What it does
- `web/src/scene/corn.ts` (pure, unit-tested in `web/tests/scene-corn.test.ts`): `cornLook(stock)` gives density, height and cob size; `cornColours(stock, {lensTint, dry})` gives green, then yellow, then dry brown (drought dries it more; a lens blends its tile colour in); `stalkSlots(tx, ty)` gives 42 seeded slots (6 rows x 7, jittered) per tile. No `Math.random`: the same tile always grows the same field. Slots disappear in a fixed order that blends noise with a per-tile blob field, so bare soil shows in clumps and a thinner field is always a subset of a fuller one.
- The mapping uses named VISUAL constants only (`STOCK_FULL` 0.8 = fully grown, `STOCK_BARE` 0.05, `STUBBLE_COUNT`, `STUBBLE_HEIGHT`, ...). The scene invents no rule: the collapsed model still comes from `entities.ts` (`nature.commons < 0.12`, pre-existing), and the access rule from `nature.commonsAccess`.
- `web/src/scene/cornLayer.ts` (`CornLayer`): two `InstancedMesh`es (stalks, cobs) for ALL commons tiles, so two draw calls whatever the tile count. It writes matrices and colours from `EntityDesc.stock`, eases the shown stock toward a new one (about 1 s; jumps under Reduced motion), and sways the stalks in the vertex shader (no per-frame matrix work). Sway is off under `prefers-reduced-motion`; when on it asks for the 25 fps ambient repaint like turbines do.
- `models.ts`: `commons_open` is the tile plus soil furrows; `commons_community` adds a sign and a fence segment; `commons_enclosed` the full fence and a red owner marker; `commons_collapsed` bare cracked earth (the few stubble stalks are still instanced). The grazing animals and tree were removed.
- `lenses.ts`: under no lens the commons ground now goes from soil brown (was straw) to green so bare patches read as soil under thin corn.

## Verified (real wasm, seed 42, 1440x900, SwiftShader)
`web/playwright/corn-shots.mjs <outDir> [url] [poke|over]` (needs `npm run dev -- --port 5190`). Default run (commons 0.77), stock poked to 0.8, 0.5, 0.25, 0.13, 0.05 (dev poke: the live snapshot object is mutated in place and a draft change re-renders; no dev hook exists for stock), community and enclosed at high and middle stock, 4 rotations, wealth, emissions and fire lenses, and an overconsumption run (`commons_harvest` 3 for 29 turns: the commons only fell to 0.56 and the town enclosed it, see FN-3, so it never reached collapse by itself).
- Draw calls 149 -> 151 (the two corn meshes); triangles 56k -> 99k. Software GL at device scale 2: 27 -> 20 fps in one sample, noisy (second sample 14 -> 12.5). A GPU run was not measured.

## Unverified
- Reduced motion was not run in a browser (code path: `uAmp` 0, no easing); not tested on a real GPU or a phone-sized viewport.
- The ease between two stock values was not captured as frames.
