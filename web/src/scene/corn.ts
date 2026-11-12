import { mix } from './palette';

/**
 * Corn on the commons (pure, no Three.js). The commons stock share (0..1, from the snapshot) is mapped to what the field looks like:
 * how many stalks stand, how tall, what colour and whether they carry cobs. These are VISUAL mappings only: the named constants below
 * are looks, not rules (the collapse threshold and the access rule come from the wire). Placement is seeded by the tile coordinates,
 * so the same tile always grows the same field and nothing uses Math.random.
 */

/** Stock share at which the field looks fully grown (the start state is about 0.8) and at which it is bare stubble. */
export const STOCK_FULL = 0.8;
export const STOCK_BARE = 0.05;
/** Smallest share of the stalk slots left standing as stubble, and smallest stalk height (share of full). */
export const STUBBLE_COUNT = 0.1;
export const STUBBLE_HEIGHT = 0.2;
/** Full stalk height in tile units (a tile is 1 wide; houses are about 0.7). */
export const STALK_HEIGHT = 0.6;
/** Rows x columns of stalk slots inside a tile (rows run along x). */
export const ROWS = 6;
export const COLS = 7;
export const SLOTS = ROWS * COLS;
export const ROW_X = 0.3;
export const ROW_Z = 0.3;

export const CORN_GREEN = 0x5fa83f;
export const CORN_YELLOW = 0xc9b94a;
export const CORN_DRY = 0x8a6a3c;
export const COB_GOLD = 0xf6c548;
export const COB_DRY = 0x7a5a2c;

const clamp01 = (v: number) => Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0));
const smooth = (t: number) => { const x = clamp01(t); return x * x * (3 - 2 * x); };

export interface CornLook {
  /** Share of the slots with a standing stalk (0..1). */
  density: number;
  /** Stalk height as a share of full (0..1). */
  height: number;
  /** Cob size as a share of full (0 = no cobs). */
  cob: number;
  /** 0 (bare, dry) .. 1 (full, healthy): the growth the other fields derive from. */
  growth: number;
}

/** Commons stock share -> look. Monotone in the stock: more stock never gives fewer, shorter or drier stalks. */
export function cornLook(stock: number): CornLook {
  const g = clamp01((stock - STOCK_BARE) / (STOCK_FULL - STOCK_BARE));
  return {
    density: STUBBLE_COUNT + (1 - STUBBLE_COUNT) * g ** 1.3,
    height: STUBBLE_HEIGHT + (1 - STUBBLE_HEIGHT) * smooth(g * 1.15),
    cob: smooth((g - 0.3) / 0.5),
    growth: g,
  };
}

/** Number of stalks standing out of `SLOTS` for a stock share. */
export const stalkCount = (stock: number): number => Math.round(cornLook(stock).density * SLOTS);

export interface Colours { stalk: number; cob: number }

/**
 * Stalk and cob colour: green when healthy, yellowing, then dry brown. A drought (0..1 look factor) dries it further. Under a lens
 * the lens tile colour is blended in so the field reads with the rest of the map.
 */
export function cornColours(stock: number, opts: { lensTint?: number | null; dry?: number } = {}): Colours {
  const g = cornLook(stock).growth;
  let stalk = g >= 0.5 ? mix(CORN_YELLOW, CORN_GREEN, (g - 0.5) / 0.5) : mix(CORN_DRY, CORN_YELLOW, g / 0.5);
  let cob = mix(COB_DRY, COB_GOLD, smooth(g / 0.6));
  const dry = clamp01(opts.dry ?? 0);
  if (dry > 0) { stalk = mix(stalk, CORN_DRY, 0.45 * dry); cob = mix(cob, COB_DRY, 0.3 * dry); }
  if (opts.lensTint !== null && opts.lensTint !== undefined) { stalk = mix(stalk, opts.lensTint, 0.6); cob = mix(cob, opts.lensTint, 0.45); }
  return { stalk, cob };
}

export interface Slot {
  /** Offset from the tile centre. */
  x: number;
  z: number;
  /** 0..1 order in which stalks disappear: the stalk stays while rank < density. Evenly spread, so the count is exact. */
  rank: number;
  /** Height variation (0.85..1.15), yaw, lean (radians), brightness (0.9..1.1) and cob chance rank. */
  hJit: number;
  yaw: number;
  lean: number;
  shade: number;
}

/** mulberry32: small seeded generator (deterministic). */
function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => { a = (a + 0x6d2b79f5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

/** The tile's stalk slots, in a seeded jittered row layout. Bare patches clump: the removal order blends noise with a per-tile blob field. */
export function stalkSlots(tx: number, ty: number): Slot[] {
  const r = rng((Math.imul(tx + 11, 73856093) ^ Math.imul(ty + 7, 19349663)) >>> 0);
  const ph = [r() * 6.28, r() * 6.28, r() * 6.28];
  const raw: Slot[] = [];
  for (let i = 0; i < ROWS; i++) {
    for (let j = 0; j < COLS; j++) {
      const x = -ROW_X + (2 * ROW_X * j) / (COLS - 1) + (r() - 0.5) * 0.04;
      const z = -ROW_Z + (2 * ROW_Z * i) / (ROWS - 1) + (r() - 0.5) * 0.03;
      const blob = 0.5 + 0.25 * Math.sin(x * 7 + ph[0]!) + 0.25 * Math.sin(z * 8 + x * 4 + ph[1]!);
      raw.push({ x, z, rank: 0.55 * r() + 0.45 * blob, hJit: 0.85 + 0.3 * r(), yaw: r() * 6.28, lean: (r() - 0.5) * 0.18, shade: 0.9 + 0.2 * r() });
    }
  }
  const order = raw.map((s, i) => [s.rank, i] as const).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  order.forEach(([, i], k) => { raw[i]!.rank = (k + 0.5) / raw.length; });
  return raw;
}

/** Height scale and cob scale (world, 0 = hidden) of one slot for a look. A slot beyond the density is hidden. */
export function slotScale(slot: Slot, look: CornLook): { stalk: number; cob: number } {
  if (slot.rank > look.density) return { stalk: 0, cob: 0 };
  const stalk = STALK_HEIGHT * look.height * slot.hJit;
  return { stalk, cob: stalk * look.cob };
}

/** Per-frame easing of the shown stock toward the target (exponential; reduced motion jumps). */
export function easeStock(cur: number, target: number, dt: number, rate = 2.5): number {
  if (Math.abs(target - cur) < 1e-4) return target;
  return cur + (target - cur) * (1 - Math.exp(-Math.max(0, dt) * rate));
}
