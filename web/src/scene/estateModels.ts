import type { Model, Part, Shape } from './models';
import { LANE_V } from './jetPath';
import { ASPHALT, GLASS, GOLD, HEDGE, LEAF, METAL, ROOF_DARK, STONE, WOOD } from './palette';

/**
 * Town v7 estate models as plain data (pure; geometry comes from geometry.ts). Same conventions as models.ts: unit primitives stand on
 * y = 0, `p` is the base centre, `z` the size. Tile models are in tile-local coordinates (x east, z south, tile edge at +-0.5).
 * Runway-frame models (strip, hangar, jet) use `u` = x along the runway and `v` = z sideways, and are rotated into place by `place`.
 */
const P = (s: Shape, x: number, y: number, z: number, w: number, h: number, d: number, c: number, o: Partial<Part> = {}): Part => ({ s, p: [x, y, z], z: [w, h, d], c, ...o });
const box = (x: number, y: number, z: number, w: number, h: number, d: number, c: number, tint = false) => P('box', x, y, z, w, h, d, c, { tint });
const cyl = (x: number, y: number, z: number, dia: number, h: number, c: number) => P('cyl', x, y, z, dia, h, dia, c);
const cone = (x: number, y: number, z: number, dia: number, h: number, c: number) => P('cone', x, y, z, dia, h, dia, c);
const ball = (x: number, y: number, z: number, dia: number, c: number) => P('sphere', x, y, z, dia, dia, dia, c);
const gable = (x: number, y: number, z: number, w: number, h: number, d: number, c: number, ry = 0) => P('gable', x, y, z, w, h, d, c, { ry });
const model = (parts: Part[]): Model => ({ parts, height: parts.reduce((m, p) => Math.max(m, p.p[1] + p.z[1]), 0) });

export const ESTATE_STONE = 0xf1eee6;
export const SLATE = 0x4a4f5a;
export const LAWN = 0xa8d37c;
export const GRAVEL = 0xe2d6b6;
export const WARM = 0xffd98a;
export const WATER = 0x4fb6d8;
export const WHITE = 0xfaf8f2;
export const RUNWAY_PAINT = 0xf5f2e6;

const hedge = (x: number, z: number, w: number, d: number) => box(x, 0, z, w, 0.12, d, HEDGE);
const conifer = (x: number, z: number, s: number): Part[] => [cyl(x, 0, z, 0.05 * s, 0.1 * s, WOOD), cone(x, 0.08 * s, z, 0.3 * s, 0.34 * s, LEAF), cone(x, 0.27 * s, z, 0.22 * s, 0.3 * s, LEAF)];

/**
 * The estate house: two storeys, cream stone walls (the tinted part), a slate roof with a gable and a gold finial, a stone chimney, a
 * columned portico on the east front (toward the drive), a balcony on the south side (over the pool), warm light in the upper windows.
 * Wider (0.86 x 0.7) and taller (to 0.62 under the roof) than any other home.
 */
export const HOME_ESTATE: Model = model([
  box(0, 0, 0, 0.9, 0.05, 0.74, 0xb9b2a2), // plinth
  box(0, 0.05, 0, 0.84, 0.58, 0.68, ESTATE_STONE, true), // two storeys of wall
  box(0, 0.33, 0, 0.86, 0.025, 0.7, 0xdcd6c8), // storey band
  gable(0, 0.63, 0, 0.92, 0.32, 0.76, SLATE), // roof, ridge north-south
  box(-0.2, 0.7, -0.2, 0.1, 0.3, 0.1, 0xcfc7b6), box(-0.2, 0.98, -0.2, 0.13, 0.03, 0.13, 0x8d867a), // chimney
  P('octa', 0, 0.95, 0, 0.11, 0.12, 0.11, GOLD), // gold finial: the owner marker
  // east front (the drive): portico with two columns, door, windows
  box(0.47, 0, 0, 0.1, 0.04, 0.34, 0xd2ccbc), cyl(0.48, 0.04, -0.13, 0.045, 0.3, WHITE), cyl(0.48, 0.04, 0.13, 0.045, 0.3, WHITE),
  box(0.47, 0.34, 0, 0.1, 0.03, 0.36, WHITE), gable(0.47, 0.37, 0, 0.38, 0.1, 0.1, SLATE, Math.PI / 2),
  box(0.43, 0.05, 0, 0.02, 0.22, 0.12, WOOD),
  box(0.43, 0.08, -0.24, 0.02, 0.16, 0.1, GLASS), box(0.43, 0.08, 0.24, 0.02, 0.16, 0.1, GLASS),
  box(0.43, 0.41, -0.2, 0.02, 0.16, 0.1, WARM), box(0.43, 0.41, 0, 0.02, 0.16, 0.1, WARM), box(0.43, 0.41, 0.2, 0.02, 0.16, 0.1, WARM),
  // south side (the pool): balcony with a rail, windows
  box(-0.05, 0.31, 0.43, 0.5, 0.03, 0.14, WHITE),
  ...[-0.28, -0.05, 0.18].map((x) => box(x, 0.34, 0.49, 0.015, 0.09, 0.015, WHITE)), box(-0.05, 0.43, 0.49, 0.5, 0.012, 0.012, WHITE),
  cyl(-0.28, 0.05, 0.47, 0.04, 0.26, WHITE), cyl(0.18, 0.05, 0.47, 0.04, 0.26, WHITE),
  box(-0.25, 0.08, 0.35, 0.12, 0.16, 0.02, GLASS), box(0.12, 0.08, 0.35, 0.12, 0.16, 0.02, GLASS),
  box(-0.25, 0.4, 0.35, 0.12, 0.16, 0.02, WARM), box(0.12, 0.4, 0.35, 0.12, 0.16, 0.02, WARM),
  box(-0.05, 0.38, 0.352, 0.1, 0.22, 0.02, WARM),
  // west and north windows
  box(-0.43, 0.1, 0, 0.02, 0.16, 0.14, GLASS), box(-0.43, 0.4, 0, 0.02, 0.16, 0.14, WARM), box(0, 0.4, -0.35, 0.16, 0.16, 0.02, WARM),
]);

/** Drive tile: gravel with a turning circle, a gate with two pillars on the road side (north), hedges. */
export const ESTATE_DRIVE: Model = model([
  box(0, 0, 0, 0.9, 0.012, 0.9, 0xb6c98a), // lawn border
  box(0, 0.012, -0.15, 0.2, 0.012, 0.7, GRAVEL), // drive north to the gate
  box(-0.2, 0.012, 0, 0.55, 0.012, 0.2, GRAVEL), // toward the house door (west)
  P('cyl', -0.02, 0.012, 0.02, 0.5, 0.014, 0.5, GRAVEL), // turning circle
  cyl(-0.02, 0.026, 0.02, 0.14, 0.04, 0x9fcb72), ball(-0.02, 0.06, 0.02, 0.12, HEDGE), // central bed
  box(-0.12, 0, -0.45, 0.09, 0.2, 0.09, WHITE), box(0.12, 0, -0.45, 0.09, 0.2, 0.09, WHITE), ball(-0.12, 0.2, -0.45, 0.09, GOLD), ball(0.12, 0.2, -0.45, 0.09, GOLD),
  box(-0.34, 0, -0.45, 0.3, 0.07, 0.04, WHITE), box(0.34, 0, -0.45, 0.3, 0.07, 0.04, WHITE),
  hedge(0.45, 0.05, 0.06, 0.8), hedge(-0.1, 0.45, 0.5, 0.06),
  cyl(0.3, 0, 0.18, 0.025, 0.2, METAL), ball(0.3, 0.2, 0.18, 0.06, WARM), // lamp
]);

/** Pool tile: lawn, a pool with white coping, two sun loungers, a parasol, a hedge on the south edge. */
export const ESTATE_POOL: Model = model([
  box(0, 0, -0.02, 0.9, 0.012, 0.9, 0xb6c98a),
  box(0.0, 0.012, 0.0, 0.7, 0.03, 0.46, WHITE), box(0.0, 0.03, 0.0, 0.58, 0.02, 0.34, WATER),
  box(-0.25, 0.012, -0.32, 0.14, 0.04, 0.07, WHITE), box(-0.05, 0.012, -0.32, 0.14, 0.04, 0.07, WHITE),
  cyl(0.25, 0, -0.33, 0.015, 0.22, WOOD), cone(0.25, 0.2, -0.33, 0.26, 0.07, 0xd1432f),
  hedge(0, 0.46, 0.9, 0.06), hedge(-0.46, 0, 0.06, 0.9),
  ...conifer(0.38, 0.36, 0.8),
]);

/** Garden tile: lawn, flower beds, conifers on the west and south edges, a gravel path to the hangar on the east edge. */
export const ESTATE_GARDEN: Model = model([
  box(0, 0, 0, 0.92, 0.012, 0.92, 0xb6c98a),
  box(0.12, 0.012, -0.34, 0.14, 0.012, 0.32, GRAVEL), box(0.2, 0.012, -0.02, 0.6, 0.012, 0.14, GRAVEL),
  ...conifer(-0.33, -0.33, 1.1), ...conifer(-0.35, 0.05, 1), ...conifer(-0.2, 0.36, 1.2), ...conifer(0.2, 0.38, 0.9),
  box(-0.08, 0.012, 0.25, 0.26, 0.04, 0.1, 0x7a5a3a), ...[-0.17, -0.09, -0.01].map((x, i) => ball(x, 0.05, 0.25, 0.07, [0xd55e00, 0xcc79a7, 0xf0e442][i]!)),
  hedge(0, 0.46, 0.9, 0.05),
]);

/** Apron under the hangar: a dark slab over the whole tile. In the runway frame. */
const APRON = box(0, 0, 0, 0.98, 0.022, 0.98, ASPHALT);

/** Hangar (north half of its tile, open on the runway side). Runway frame: x along the runway, z sideways, +z = the runway side. */
export const HANGAR: Model = model([
  APRON,
  box(0, 0.022, -0.46, 0.78, 0.36, 0.05, 0xd5d8de), // back wall
  box(-0.38, 0.022, -0.24, 0.05, 0.36, 0.45, 0xd5d8de), box(0.38, 0.022, -0.24, 0.05, 0.36, 0.45, 0xd5d8de), // side walls
  gable(0, 0.38, -0.24, 0.52, 0.16, 0.84, METAL, Math.PI / 2), // roof, ridge across the door
  box(0, 0.022, -0.24, 0.7, 0.01, 0.4, 0xb5b8be), // floor
  box(0, 0.3, -0.0, 0.84, 0.04, 0.04, ROOF_DARK), // door header
  box(-0.2, 0.16, -0.455, 0.12, 0.1, 0.01, GLASS), box(0.2, 0.16, -0.455, 0.12, 0.1, 0.01, GLASS),
  cyl(0.46, 0.022, 0.4, 0.06, 0.02, ASPHALT), // tie-down
]);

/** The runway strip over `tiles` tiles after the hangar tile, centre line on LANE_V; dashes, edge lines, threshold bars at the far end. */
export function runwayModel(len: number): Model {
  const w = 0.42, z = LANE_V;
  const x0 = 0.5, x1 = len + 0.5;
  const parts: Part[] = [box((x0 + x1) / 2, 0, z, x1 - x0, 0.02, w, ASPHALT)];
  for (let x = x0 + 0.2; x < x1 - 0.3; x += 0.34) parts.push(box(x, 0.02, z, 0.18, 0.004, 0.025, RUNWAY_PAINT));
  parts.push(box((x0 + x1) / 2, 0.02, z - w / 2 + 0.015, x1 - x0, 0.004, 0.012, RUNWAY_PAINT), box((x0 + x1) / 2, 0.02, z + w / 2 - 0.015, x1 - x0, 0.004, 0.012, RUNWAY_PAINT));
  for (let i = -3; i <= 3; i++) parts.push(box(x1 - 0.08, 0.02, z + i * 0.05, 0.1, 0.004, 0.03, RUNWAY_PAINT));
  return model(parts);
}

/** Windsock pole (static part). The sock itself is `WINDSOCK`, which the layer sways. Runway frame. */
export const WINDSOCK_POLE: Model = model([cyl(0, 0, 0, 0.025, 0.42, METAL), ball(0, 0.42, 0, 0.04, 0xd1432f)]);
/** The sock: alternating orange and white bands, pointing along +x from its pole (rotated by the layer). */
export const WINDSOCK: Model = model([0, 1, 2, 3].map((i) => P('cone', 0.06 + i * 0.07, 0, 0, 0.12 - i * 0.02, 0.07, 0.12 - i * 0.02, i % 2 ? WHITE : 0xe8731d, { rz: -Math.PI / 2 })));

/** The jet, nose toward +x, standing on the ground at y = 0 (its centre is the middle of the fuselage). */
export const JET: Model = model([
  P('cyl', -0.3, 0.07, 0, 0.1, 0.5, 0.1, WHITE, { rz: -Math.PI / 2 }), // fuselage
  P('cone', 0.2, 0.07, 0, 0.1, 0.16, 0.1, WHITE, { rz: -Math.PI / 2 }), // nose
  box(-0.08, 0.075, 0.051, 0.36, 0.014, 0.004, GOLD), box(-0.08, 0.075, -0.051, 0.36, 0.014, 0.004, GOLD), // gold cheat line
  box(0.02, 0.075, -0.2, 0.2, 0.012, 0.28, WHITE, false), box(0.02, 0.075, 0.2, 0.2, 0.012, 0.28, WHITE, false), // wings
  box(-0.28, 0.12, 0, 0.1, 0.2, 0.012, WHITE), // fin
  box(-0.3, 0.2, 0, 0.1, 0.012, 0.2, WHITE), // tailplane
  P('cyl', -0.22, 0.13, -0.07, 0.05, 0.14, 0.05, 0xb9bcc2, { rz: -Math.PI / 2 }), P('cyl', -0.22, 0.13, 0.07, 0.05, 0.14, 0.05, 0xb9bcc2, { rz: -Math.PI / 2 }), // engines
  box(0.14, 0.12, 0, 0.1, 0.026, 0.08, 0x2d4f7a), // cockpit glass
  box(0.0, 0.03, 0, 0.012, 0.045, 0.012, METAL), box(-0.22, 0.03, 0, 0.012, 0.045, 0.1, METAL), // gear
]);

export const GATE_STONE = STONE;
