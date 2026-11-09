# Web notes: Scene for Town v2

Owner: scene builder. Files: `web/src/scene/**`, `web/src/app/SceneHost.svelte`, `web/src/store/scene-people.ts` (new readable), tests `web/tests/scene-*.test.ts`, `web/playwright/scene-*2.mjs`.

## What was built
- Look: sky gradient (canvas texture background), a meadow disc that fades into the sky, the town on a raised plot with soil sides, a ring of instanced trees and hedges (`island.ts`), warmer green ground tiles, dirt footpaths only where no road exists.
- Roads: one merged mesh for built roads (`roads.ts`): pad plus an arm per connected neighbour, so straight, corner, T, cross and end all come out of the neighbour mask (`roadMask`, `roadPiece` are pure and tested). Queued and planned roads are a translucent mesh plus road-works cones and the turns-left badge; the hovered tile gets a road ghost that joins its neighbours.
- People (`people.ts`, `peopleLayer.ts`): one figure per person, 11 draw calls in all. Child is smaller, retired is grey-haired with a stick and a slight stoop. Household cue: one of 8 colours AND one of 8 marker shapes above the head (ball, cube, cone, diamond, disc, pyramid, ring, gem). Idle figures stand on slots in front of their home; the unhoused household camps on the nearest free tile to the town hall, with a tent per household.
- Commutes: the wire has no path, so `route()` (Dijkstra, road tile cost 0.3, open 1, commons 1.3, buildings blocked) is used for everything. Each commuting person in `flows.commutes` walks home, work, home inside the month; the share of the month is proportional to `minutes`, so long commutes visibly take longer. Idle figures never move outside playback.
- Pointer: people win over the tile under them (15 px screen radius) unless the player is placing something. Hover writes `personHover` (store/scene-people.ts) and the tile hover; a click calls `game.select({ kind: 'person', id })`. The scene bus `EntityRef` has no person kind, so the person facts go in `bus.selection.facts`.
- Buildings: Shop (services) is a new market-stall model; tent; road works. Plants: smoke rate is the plant's `emissionsMonthly` (or the frame's `plant` entry with tile) over the heaviest firm or plant, so a renewable plant and a fossil plant with 0 t show no smoke; it also feeds the emissions lens.
- Flows: purchases are household -> firm (coin out, goods/food/service ticket back, services purple); rent goes to the home's owner household's home or the town hall for public housing; wages per person's household; plant emissions with tiles. Every list may be empty.
- Lenses: class, wealth, employment (per person: employed blue, looking for work orange, not in the labour force grey; homes by labour-force share), emissions (plants and firms), flows, and new `commute` (people by minutes, homes by longest commuter). `Lens` gained `'commute'` in `store/types.ts` (the only edit to an existing store file) and the legend objects keep the same shape.
- Keyboard: arrows (camera-relative), Enter (select or place), Esc (cancel build mode, then selection) verified in `playwright/scene-keys2.mjs` on both sims, plus person hover and click.

## Numbers (software GL / SwiftShader, 1440x900)
- Frame time during playback: about 33-40 ms per frame (25-30 fps); idle repaints are throttled for ambient smoke.
- Per frame: about 120 draw calls (the count includes the shadow pass), 43k triangles; instances: 134-138 idle, 174-180 in playback (people 20 x 3 + smoke + tokens + goods).

## Rough
- Roads have square corners (no rounded kerbs); no per-piece mesh variants, the pad-and-arms approach covers every case.
- Each building is still its own draw call (about 38); merging static buildings per kind would halve draws but breaks per-building tint and pop-in.
- Commute pace is "share of the month", not tiles per second; people vanish into the firm tile's doorstep rather than walking inside.
- Marker shapes are small at default zoom; they read at zoom 1.5 and up. The marker legend is not in the lens legend (panels).
- `personHover` is not read by panels yet. Pending roads read as pale grey squares at distance.
- The dirt paths are a decorative guess (home to its two nearest workplaces), not the sim's route.

## Contract issues (unchanged from web-notes-v2)
- `flows.commutes` has no path and `from` is nullable; person events carry no household. The unhoused household has no tile anywhere in the contract, so the camp tile is a scene invention.
- `EntityKind` in the scene bus has no `person`; `SceneBus.selection.tile` must be a tile, so a person without a home tile is reported on the camp tile.
