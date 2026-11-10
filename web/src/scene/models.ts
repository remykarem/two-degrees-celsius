import { AWNING_A, BRICK, CRANE, DARK, FOOD, GLASS, GOLD, GRASS, LEAF, METAL, PANEL, ROOF, ROOF_DARK, SMOKE_DARK, STONE, WALL, WOOD } from './palette';

/**
 * Procedural building models as plain data (pure; geometry is built in geometry.ts).
 * Unit primitives sit on y = 0 and are centred on x/z. `p` = position of the base centre, `z` = size (sx, sy, sz).
 * `tint` parts take the lens/class colour; other parts keep their own colour.
 */
export type Shape = 'box' | 'cyl' | 'cone' | 'pyr' | 'sphere' | 'octa' | 'gable';
export interface Part { s: Shape; p: [number, number, number]; z: [number, number, number]; c: number; tint?: boolean; ry?: number; rz?: number }
export interface Model { parts: Part[]; spin?: { at: [number, number, number]; parts: Part[] }; height: number }

const P = (s: Shape, x: number, y: number, z: number, w: number, h: number, d: number, c: number, o: Partial<Part> = {}): Part => ({ s, p: [x, y, z], z: [w, h, d], c, ...o });
const box = (x: number, y: number, z: number, w: number, h: number, d: number, c: number, tint = false) => P('box', x, y, z, w, h, d, c, { tint });
const cyl = (x: number, y: number, z: number, dia: number, h: number, c: number, tint = false) => P('cyl', x, y, z, dia, h, dia, c, { tint });
const cone = (x: number, y: number, z: number, dia: number, h: number, c: number, tint = false) => P('cone', x, y, z, dia, h, dia, c, { tint });
const pyr = (x: number, y: number, z: number, dia: number, h: number, c: number) => P('pyr', x, y, z, dia, h, dia, c);
const ball = (x: number, y: number, z: number, dia: number, c: number, tint = false) => P('sphere', x, y, z, dia, dia, dia, c, { tint });
const gable = (x: number, y: number, z: number, w: number, h: number, d: number, c: number, ry = 0) => P('gable', x, y, z, w, h, d, c, { ry });
const height = (parts: Part[]) => parts.reduce((m, p) => Math.max(m, p.p[1] + p.z[1]), 0);
const model = (parts: Part[], spin?: Model['spin']): Model => ({ parts, spin, height: height(parts) });

function treeParts(x: number, z: number, s: number): Part[] {
  return [cyl(x, 0, z, 0.07 * s, 0.14 * s, WOOD), cone(x, 0.1 * s, z, 0.34 * s, 0.36 * s, LEAF, true), cone(x, 0.3 * s, z, 0.26 * s, 0.32 * s, LEAF, true)];
}
function fence(x0: number, z0: number, x1: number, z1: number, n: number): Part[] {
  const out: Part[] = [];
  for (let i = 0; i <= n; i++) out.push(box(x0 + ((x1 - x0) * i) / n, 0, z0 + ((z1 - z0) * i) / n, 0.04, 0.16, 0.04, WOOD));
  return out;
}

export const MODELS = {
  // Homes: one silhouette and one rooftop marker per class.
  home_capitalist: model([
    box(-0.05, 0, 0.05, 0.7, 0.34, 0.5, WALL, true), gable(-0.05, 0.34, 0.05, 0.74, 0.26, 0.54, ROOF_DARK),
    box(0.27, 0, -0.12, 0.2, 0.66, 0.2, WALL, true), pyr(0.27, 0.66, -0.12, 0.34, 0.22, ROOF_DARK),
    P('octa', 0.27, 0.9, -0.12, 0.13, 0.13, 0.13, GOLD), box(-0.2, 0.1, 0.32, 0.12, 0.14, 0.02, GLASS), box(0.05, 0.1, 0.32, 0.12, 0.14, 0.02, GLASS),
  ]),
  home_rentier: model([
    box(-0.27, 0, 0, 0.25, 0.3, 0.55, WALL, true), box(0, 0, 0, 0.25, 0.3, 0.55, WALL, true), box(0.27, 0, 0, 0.25, 0.3, 0.55, WALL, true),
    gable(-0.27, 0.3, 0, 0.27, 0.2, 0.58, ROOF), gable(0, 0.3, 0, 0.27, 0.2, 0.58, ROOF_DARK), gable(0.27, 0.3, 0, 0.27, 0.2, 0.58, ROOF),
    box(-0.27, 0.05, 0.29, 0.1, 0.14, 0.02, GLASS), box(0, 0.05, 0.29, 0.1, 0.14, 0.02, GLASS), box(0.27, 0.05, 0.29, 0.1, 0.14, 0.02, GLASS),
    box(0, 0.5, 0, 0.12, 0.12, 0.12, GOLD),
  ]),
  home_manager: model([
    box(-0.08, 0, 0, 0.55, 0.34, 0.5, WALL, true), gable(-0.08, 0.34, 0, 0.6, 0.24, 0.54, ROOF, 0),
    box(0.3, 0, 0.05, 0.22, 0.2, 0.34, WALL, true), box(0.3, 0.2, 0.05, 0.25, 0.04, 0.38, ROOF_DARK),
    box(-0.25, 0.5, -0.1, 0.1, 0.3, 0.1, BRICK), ball(0, 0.58, 0, 0.14, GOLD), box(-0.08, 0.08, 0.27, 0.12, 0.16, 0.02, GLASS),
  ]),
  home_worker: model([
    box(0, 0, 0, 0.5, 0.3, 0.46, WALL, true), gable(0, 0.3, 0, 0.56, 0.26, 0.5, ROOF), box(0, 0.05, 0.24, 0.12, 0.16, 0.02, WOOD),
    cone(0, 0.58, 0, 0.1, 0.18, GOLD),
  ]),
  home_dependent: model([
    cyl(0, 0, 0, 0.44, 0.24, WALL, true), cone(0, 0.24, 0, 0.58, 0.3, 0xcaa56a), box(0, 0.02, 0.21, 0.1, 0.14, 0.02, WOOD),
  ]),

  // Firms
  firm_goods: model([
    box(-0.02, 0, 0, 0.8, 0.34, 0.6, 0xc9ccd2, true),
    gable(-0.26, 0.34, 0, 0.26, 0.2, 0.62, METAL), gable(0.02, 0.34, 0, 0.26, 0.2, 0.62, METAL), gable(0.3, 0.34, 0, 0.26, 0.2, 0.62, METAL),
    cyl(-0.3, 0.3, -0.2, 0.13, 0.55, BRICK), cyl(-0.08, 0.3, -0.2, 0.13, 0.7, BRICK), box(0.1, 0, 0.31, 0.22, 0.2, 0.02, DARK),
    box(-0.3, 0.84, -0.2, 0.15, 0.04, 0.15, DARK), box(-0.08, 0.99, -0.2, 0.15, 0.04, 0.15, DARK),
  ]),
  firm_food: model([
    box(-0.15, 0, 0.02, 0.5, 0.3, 0.5, FOOD, true), gable(-0.15, 0.3, 0.02, 0.54, 0.26, 0.54, ROOF_DARK, 0),
    cyl(0.3, 0, -0.12, 0.22, 0.52, 0xd9d4c4), ball(0.3, 0.42, -0.12, 0.22, METAL), box(-0.15, 0, 0.29, 0.14, 0.2, 0.02, 0xf3e8d4),
    ...fence(0.05, 0.3, 0.42, 0.3, 4), box(0.28, 0, 0.14, 0.2, 0.14, 0.16, 0xe6c35c),
  ]),
  firm_vacant: model([
    box(0, 0, 0, 0.7, 0.04, 0.7, STONE, true), box(-0.3, 0, -0.3, 0.04, 0.3, 0.04, WOOD), box(-0.3, 0.2, -0.27, 0.26, 0.12, 0.02, 0xf3e8d4),
    box(0.2, 0.04, 0.2, 0.18, 0.1, 0.18, 0xb9b1a1),
  ]),
  bank: model([
    box(0, 0, 0, 0.8, 0.07, 0.62, STONE), box(0, 0.07, 0, 0.7, 0.4, 0.5, WALL, true),
    ...[-0.27, -0.09, 0.09, 0.27].map((x) => cyl(x, 0.07, 0.3, 0.08, 0.4, 0xffffff)),
    gable(0, 0.47, 0.02, 0.78, 0.2, 0.58, STONE), ball(0, 0.69, 0, 0.14, GOLD),
  ]),
  government: model([
    box(0, 0, 0, 0.76, 0.34, 0.56, WALL, true), gable(0, 0.34, 0, 0.8, 0.12, 0.6, ROOF_DARK),
    box(0, 0.34, 0, 0.26, 0.5, 0.26, WALL, true), cyl(0, 0.62, 0.14, 0.14, 0.02, 0xffffff), cone(0, 0.84, 0, 0.34, 0.26, 0x4f7a86),
    box(0, 1.08, 0, 0.02, 0.2, 0.02, DARK), box(0.07, 1.2, 0, 0.14, 0.08, 0.01, 0xd1432f), box(0, 0.05, 0.29, 0.14, 0.2, 0.02, WOOD),
  ]),

  // Nature
  forest_0: model([...treeParts(-0.22, -0.2, 1.1), ...treeParts(0.22, -0.1, 0.9), ...treeParts(-0.05, 0.25, 1.2), ...treeParts(0.3, 0.3, 0.8)]),
  forest_1: model([...treeParts(0.2, -0.22, 1), ...treeParts(-0.25, 0.05, 1.2), ...treeParts(0.1, 0.2, 0.85), ...treeParts(-0.15, -0.3, 0.8)]),
  forest_2: model([...treeParts(-0.1, -0.25, 1.15), ...treeParts(0.28, 0.0, 1), ...treeParts(-0.28, 0.2, 0.9), ...treeParts(0.05, 0.28, 0.8)]),
  // Weather (Town v4): a burning and a burnt (regrowing) forest tile. Fixed colours (no lens tint), so a fire always reads as a fire.
  forest_burning: model([
    box(0, 0, 0, 0.92, 0.03, 0.92, 0x2e2420), ...[[-0.22, -0.2, 1.1], [0.22, -0.1, 0.9], [-0.05, 0.25, 1.2], [0.3, 0.3, 0.8]].flatMap(([x, z, k]) => [
      cyl(x!, 0, z!, 0.07 * k!, 0.2 * k!, 0x2b2320), cone(x!, 0.14 * k!, z!, 0.3 * k!, 0.32 * k!, 0x3a2f2a), cone(x!, 0.32 * k!, z!, 0.18 * k!, 0.2 * k!, 0xc2541f)]),
  ]),
  forest_regrowing: model([
    box(0, 0, 0, 0.92, 0.03, 0.92, 0x5b4a3b), box(-0.25, 0.03, -0.22, 0.07, 0.1, 0.07, 0x2b2320), box(0.26, 0.03, 0.2, 0.08, 0.13, 0.08, 0x2b2320), box(0.2, 0.03, -0.3, 0.06, 0.08, 0.06, 0x2b2320),
    ...[[-0.05, 0.05], [0.28, -0.1], [-0.28, 0.25], [0.05, 0.3]].flatMap(([x, z]) => [cyl(x!, 0.03, z!, 0.025, 0.1, WOOD), cone(x!, 0.1, z!, 0.14, 0.16, 0x8cc46f)]),
  ]),
  commons_open: model([
    box(0, 0, 0, 0.9, 0.04, 0.9, GRASS, true), box(-0.2, 0.04, 0.1, 0.2, 0.12, 0.12, 0xf2f0e8), box(0.2, 0.04, -0.15, 0.18, 0.11, 0.11, 0xf2f0e8),
    ball(-0.3, 0.1, 0.1, 0.1, DARK), ...treeParts(0.28, 0.28, 0.7),
  ]),
  commons_community: model([
    box(0, 0, 0, 0.9, 0.04, 0.9, GRASS, true), box(-0.15, 0.04, 0.0, 0.2, 0.12, 0.12, 0xf2f0e8), box(0.15, 0.04, -0.12, 0.18, 0.11, 0.11, 0xf2f0e8),
    box(0.35, 0, 0.35, 0.04, 0.3, 0.04, WOOD), box(0.35, 0.22, 0.35, 0.16, 0.1, 0.02, 0xf3e8d4), ...fence(-0.4, 0.42, 0.2, 0.42, 3),
  ]),
  commons_enclosed: model([
    box(0, 0, 0, 0.9, 0.04, 0.9, GRASS, true), ...fence(-0.42, -0.42, 0.42, -0.42, 6), ...fence(-0.42, 0.42, 0.42, 0.42, 6),
    ...fence(-0.42, -0.42, -0.42, 0.42, 6), ...fence(0.42, -0.42, 0.42, 0.42, 6), box(0, 0.16, -0.42, 0.9, 0.03, 0.03, WOOD), box(0, 0.16, 0.42, 0.9, 0.03, 0.03, WOOD),
    box(-0.42, 0.16, 0, 0.03, 0.03, 0.9, WOOD), box(0.42, 0.16, 0, 0.03, 0.03, 0.9, WOOD), box(0, 0.04, 0, 0.14, 0.34, 0.02, 0xd1432f),
  ]),
  commons_collapsed: model([
    box(0, 0, 0, 0.9, 0.04, 0.9, 0xb89a6a, true), box(-0.2, 0.04, 0.15, 0.2, 0.05, 0.2, 0x9c8458), box(0.22, 0.04, -0.2, 0.16, 0.04, 0.16, 0x9c8458),
    box(0.05, 0.04, 0.0, 0.03, 0.2, 0.03, WOOD),
  ]),

  // Infrastructure
  public_housing: model([
    box(0, 0, 0, 0.72, 0.66, 0.5, WALL, true), box(0, 0.66, 0, 0.76, 0.05, 0.54, STONE), box(0, 0.71, 0, 0.5, 0.05, 0.3, 0xe69f00),
    ...[0.12, 0.34, 0.54].flatMap((y) => [-0.24, 0, 0.24].map((x) => box(x, y, 0.26, 0.1, 0.1, 0.02, GLASS))),
    ...[0.12, 0.34, 0.54].map((y) => box(0.3, y - 0.03, 0.29, 0.1, 0.02, 0.08, METAL)),
  ]),
  fossil_plant: model([
    box(-0.1, 0, 0.12, 0.7, 0.3, 0.44, 0x9a9a9e, true), box(-0.1, 0.3, 0.12, 0.74, 0.05, 0.48, DARK),
    cyl(-0.34, 0.2, -0.22, 0.14, 0.78, 0xb86f55), cyl(-0.1, 0.2, -0.22, 0.14, 0.9, 0xb86f55), cyl(0.14, 0.2, -0.22, 0.14, 0.7, 0xb86f55),
    box(-0.34, 0.9, -0.22, 0.16, 0.04, 0.16, DARK), box(-0.1, 1.02, -0.22, 0.16, 0.04, 0.16, DARK), box(0.14, 0.82, -0.22, 0.16, 0.04, 0.16, DARK),
    cyl(0.33, 0, 0.2, 0.2, 0.22, SMOKE_DARK),
  ]),
  renewable_plant: model([
    box(0, 0, 0, 0.9, 0.03, 0.9, 0xcfe3cc, true),
    cyl(0.22, 0.03, -0.18, 0.06, 0.72, 0xf2f0e8), ball(0.22, 0.72, -0.18, 0.1, 0xf2f0e8),
    ...[-0.3, -0.05].flatMap((x) => [0.12, 0.34].map((z) => P('box', x, 0.1, z, 0.22, 0.03, 0.18, PANEL, { rz: -0.5 }))),
    ...[-0.3, -0.05].flatMap((x) => [0.12, 0.34].map((z) => box(x, 0.03, z, 0.03, 0.08, 0.03, METAL))),
  ], { at: [0.22, 0.77, -0.15], parts: [0, 1, 2].map((i) => P('box', 0, 0, 0, 0.05, 0.38, 0.03, 0xf2f0e8, { rz: (i * 2 * Math.PI) / 3 })) }),

  // Fire station (Town v4): a garage with two red doors, a hose tower and a beacon.
  fire_station: model([
    box(-0.06, 0, 0.06, 0.7, 0.36, 0.5, 0xeae3d6, true), box(-0.06, 0.36, 0.06, 0.74, 0.05, 0.54, 0x7a2a22), box(-0.06, 0.2, 0.32, 0.7, 0.05, 0.02, 0xd1432f),
    box(-0.22, 0, 0.32, 0.22, 0.2, 0.02, 0xd1432f), box(0.08, 0, 0.32, 0.22, 0.2, 0.02, 0xd1432f), box(-0.22, 0.04, 0.33, 0.18, 0.02, 0.01, 0xf3e8d4), box(0.08, 0.04, 0.33, 0.18, 0.02, 0.01, 0xf3e8d4),
    box(0.3, 0, -0.14, 0.16, 0.72, 0.16, 0xeae3d6, true), pyr(0.3, 0.72, -0.14, 0.26, 0.2, 0xd1432f), ball(0.3, 0.96, -0.14, 0.09, 0xe8b100),
    box(0.3, 0.5, -0.05, 0.06, 0.06, 0.01, 0xd1432f), box(0.3, 0.3, -0.05, 0.1, 0.1, 0.01, 0x7fb0c9),
  ]),

  // Shop (services): a market stall with a striped awning, crates and a sign; not the goods model.
  firm_shop: model([
    box(0, 0, 0.05, 0.72, 0.26, 0.4, WALL, true), box(0, 0.26, 0.05, 0.76, 0.04, 0.44, ROOF_DARK),
    ...[-0.3, -0.18, -0.06, 0.06, 0.18, 0.3].map((x, i) => P('box', x, 0.32, 0.2, 0.12, 0.05, 0.34, i % 2 ? 0xffffff : AWNING_A, { rz: 0 })),
    ...[-0.34, 0.34].map((x) => box(x, 0, 0.36, 0.03, 0.34, 0.03, WOOD)),
    box(0, 0.1, 0.26, 0.6, 0.12, 0.14, WOOD), box(-0.2, 0.22, 0.28, 0.1, 0.07, 0.08, 0xe6c35c), box(0.02, 0.22, 0.28, 0.1, 0.07, 0.08, FOOD), box(0.22, 0.22, 0.28, 0.1, 0.07, 0.08, 0x8fc16a),
    box(0.3, 0, -0.22, 0.16, 0.14, 0.16, WOOD), box(0.1, 0, -0.26, 0.14, 0.1, 0.14, WOOD),
    box(-0.3, 0.3, -0.22, 0.02, 0.4, 0.02, DARK), box(-0.3, 0.6, -0.22, 0.26, 0.14, 0.03, GOLD),
  ]),
  // A tent for a household without a home, pitched on a free tile near the town hall.
  tent: model([
    P('pyr', 0, 0, 0, 0.5, 0.36, 0.5, 0xe9a23b, { }), box(0, 0, 0.18, 0.1, 0.16, 0.02, DARK), box(0.2, 0, -0.2, 0.02, 0.3, 0.02, WOOD), box(0.26, 0.24, -0.2, 0.12, 0.06, 0.01, 0xd1432f),
  ]),
  // Road works: cones and a barrier (drawn on a queued or planned road tile).
  road_site: model([
    ...[[-0.3, -0.3], [0.3, -0.3], [-0.3, 0.3], [0.3, 0.3]].map(([x, z]) => cone(x!, 0.04, z!, 0.1, 0.18, 0xf08a24)),
    box(0, 0.12, -0.34, 0.5, 0.05, 0.03, 0xf2c14e), box(-0.2, 0.04, -0.34, 0.03, 0.12, 0.03, DARK), box(0.2, 0.04, -0.34, 0.03, 0.12, 0.03, DARK),
  ]),

  // Under construction extras (drawn on top of a translucent building)
  crane: model([
    box(-0.38, 0, -0.38, 0.05, 0.75, 0.05, CRANE), box(0, 0.75, -0.38, 0.8, 0.04, 0.04, CRANE), box(0.25, 0.5, -0.38, 0.01, 0.25, 0.01, DARK),
    box(0.25, 0.42, -0.38, 0.1, 0.08, 0.1, CRANE),
  ]),
} satisfies Record<string, Model>;

export type ModelId = keyof typeof MODELS;
export const MODEL_IDS = Object.keys(MODELS) as ModelId[];
