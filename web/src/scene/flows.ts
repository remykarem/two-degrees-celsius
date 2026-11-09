import type { ClassKey, MonthFlows, MonthFrame, PlantView, Sector, SimEvent, Snapshot } from '../contract';
import type { MapLayout, Tile } from '../store';
import { centreOf, hash01, tileKey, type Pt } from './coords';
import { route } from './paths';

/** Pure planner: MonthFrame -> animation tracks. No Three.js, no clock. Reads frames, never changes state. */
export const CAPS = { goods: 36, tokens: 80, smoke: 140, fx: 12 } as const;
export const CLASS_ORDER: ClassKey[] = ['capitalist', 'rentier', 'manager', 'worker', 'dependent'];

export type TrackKind = 'good' | 'token';
export interface Track {
  id: string;
  kind: TrackKind;
  /** good: 0 goods / 1 food / 2 service; token: 0 wage / 1 rent / 2 purchase. */
  style: number;
  path: Pt[];
  /** Month-local time window, 0..1. */
  t0: number;
  t1: number;
}
export type EmitterKind = 'smoke' | 'sink' | 'house';
export interface Emitter { id: string; kind: EmitterKind; at: Pt; top: number; rate: number }
export type FxType = 'closed' | 'founded' | 'hired' | 'fired' | 'forest' | 'commons' | 'enclosed' | 'bank' | 'class' | 'shortfall' | 'default';
export interface Fx { id: string; type: FxType; at: Pt; t: number; tile: Tile }
export interface FramePlan { tracks: Track[]; emitters: Emitter[]; fx: Fx[] }

export interface PlanCtx {
  layout: MapLayout;
  households: { id: number; class: ClassKey; home: number | null }[];
  firms: { id: number; sector: Sector; active: boolean }[];
  homes: { id: number; owner: number | 'public'; tile: Tile }[];
  plants: PlantView[];
  fossilShare: number;
  turn: number;
}

export function ctxFromSnapshot(layout: MapLayout, s: Snapshot): PlanCtx {
  return {
    layout, turn: s.turn, fossilShare: s.energy.fossilShare,
    homes: s.homes.map((h) => ({ id: h.id, owner: h.owner, tile: { ...h.tile } })), plants: s.plants,
    households: s.households.map((h) => ({ id: h.id, class: h.class, home: h.home })),
    firms: s.firms.map((f) => ({ id: f.id, sector: f.sector, active: f.active })),
  };
}

/** Point along a polyline at fraction u (0..1) of its length. */
export function pointAt(path: Pt[], u: number): Pt {
  if (path.length === 0) return [0, 0];
  if (path.length === 1) return path[0]!;
  const lens: number[] = [];
  let total = 0;
  for (let i = 1; i < path.length; i++) { const l = Math.hypot(path[i]![0] - path[i - 1]![0], path[i]![1] - path[i - 1]![1]); lens.push(l); total += l; }
  let d = Math.min(1, Math.max(0, u)) * total;
  for (let i = 0; i < lens.length; i++) {
    if (d <= lens[i]! || i === lens.length - 1) {
      const f = lens[i]! > 0 ? Math.min(1, d / lens[i]!) : 0;
      return [path[i]![0] + (path[i + 1]![0] - path[i]![0]) * f, path[i]![1] + (path[i + 1]![1] - path[i]![1]) * f];
    }
    d -= lens[i]!;
  }
  return path[path.length - 1]!;
}

/** Commuters walk out to work, stay a moment, and walk back: u in 0..1 -> position fraction along the outward path. */
export const commuteFraction = (u: number) => (u < 0.4 ? u / 0.4 : u < 0.6 ? 1 : 1 - (u - 0.6) / 0.4);

/** Local progress of a track at month time t (null when not yet started or already finished). */
export function trackProgress(tr: Track, t: number): number | null {
  if (t < tr.t0 || t > tr.t1) return null;
  return tr.t1 > tr.t0 ? (t - tr.t0) / (tr.t1 - tr.t0) : 1;
}

/** Fair split of a cap across weighted flows: every flow with weight gets at least one, no flow more than `maxEach`, total never above `cap`. */
export function allocate(weights: number[], cap: number, maxEach = 8): number[] {
  const sum = weights.reduce((a, w) => a + Math.max(0, w), 0);
  const counts = weights.map((w) => (w > 0 && sum > 0 ? Math.min(maxEach, Math.max(1, Math.round((w / sum) * cap * 0.8))) : 0));
  let total = counts.reduce((a, c) => a + c, 0);
  const order = weights.map((w, i) => i).sort((a, b) => weights[a]! - weights[b]!);
  for (const i of order) { if (total <= cap) break; total -= counts[i]!; counts[i] = 0; }
  return counts;
}

/** The tile of a household's home (undefined when it has none: the unhoused). */
const homeTile = (c: PlanCtx, household: number): Tile | undefined => {
  const home = c.households.find((h) => h.id === household)?.home;
  return home == null ? undefined : c.layout.scenery.find((s) => s.kind === 'home' && s.ref === home);
};
const firmTile = (c: PlanCtx, id: number): Tile | undefined => c.layout.scenery.find((s) => s.kind === 'firm' && s.ref === id);
const firstOf = (c: PlanCtx, kind: 'bank' | 'government'): Tile | undefined => c.layout.scenery.find((s) => s.kind === kind);

function pathBetween(c: PlanCtx, a: Tile, b: Tile, seed: number): Pt[] {
  const n = c.layout.gridSize;
  const ox = (hash01(seed) - 0.5) * 0.22, oz = (hash01(seed + 77) - 0.5) * 0.22;
  const tiles = route(c.layout, a, b);
  return tiles.map((t, i): Pt => {
    const [x, z] = centreOf(t, n);
    const end = i === 0 || i === tiles.length - 1;
    return [x + (end ? 0 : ox), z + (end ? 0 : oz)];
  });
}

function window01(seed: number): [number, number] {
  const t0 = 0.03 + 0.5 * hash01(seed);
  return [t0, Math.min(1, t0 + 0.34 + 0.14 * hash01(seed + 5))];
}

/** Smoke below this share of the heaviest emitter is not drawn (a plant that emits nothing shows no smoke). */
export const SMOKE_MIN = 0.02;

/**
 * Emissions list for a frame -> emitters (chimneys, house puffs, forest sinks, plants). Each emitter's rate is its tonnes over the
 * heaviest firm or plant, so the player SEES which one emits. Plants use the frame's `plant` entries when present, else the snapshot's
 * `emissionsMonthly`; a renewable plant (0 t) gets no smoke.
 */
export function planEmitters(emissions: NonNullable<MonthFlows['emissions']> = [], c: PlanCtx): Emitter[] {
  const n = c.layout.gridSize;
  const plantT = new Map<string, number>();
  for (const p of c.plants) plantT.set(tileKey(p.tile), p.emissionsMonthly);
  for (const e of emissions) if (e.source === 'plant' && e.tile) plantT.set(tileKey(e.tile), Math.max(0, e.tonnes));
  const max = Math.max(1e-9, ...emissions.filter((e) => e.source === 'firm').map((e) => Math.max(0, e.tonnes)), ...plantT.values());
  const maxSink = Math.max(1e-9, ...emissions.map((e) => Math.max(0, -e.tonnes)));
  const out: Emitter[] = [];
  for (const e of emissions) {
    if (e.source === 'firm' && e.id !== undefined) {
      const t = firmTile(c, e.id);
      const active = c.firms.find((f) => f.id === e.id)?.active ?? false;
      const sector = c.firms.find((f) => f.id === e.id)?.sector;
      if (t && active && e.tonnes / max >= SMOKE_MIN) out.push({ id: `f${e.id}`, kind: 'smoke', at: centreOf(t, n), top: sector === 'food' ? 0.55 : sector === 'services' ? 0.45 : 1.0, rate: Math.min(1, e.tonnes / max) });
    } else if (e.source === 'households' && e.tonnes > 0) {
      for (const h of c.layout.scenery.filter((s) => s.kind === 'home')) out.push({ id: `h${h.ref}`, kind: 'house', at: centreOf(h, n), top: 0.62, rate: Math.min(1, (e.tonnes / max) * 0.5) });
    } else if (e.source === 'sink' && e.tonnes < 0) {
      c.layout.scenery.filter((s) => s.kind === 'forest').slice(0, 5).forEach((f, i) => out.push({ id: `s${i}`, kind: 'sink', at: centreOf(f, n), top: 0.9, rate: Math.min(1, -e.tonnes / maxSink) }));
    }
  }
  for (const p of c.layout.placed) {
    if (p.kind !== 'fossil_plant') continue;
    const tonnes = plantT.get(tileKey(p));
    const rate = tonnes === undefined ? 0.3 + 0.6 * c.fossilShare : tonnes / max;
    if (rate >= SMOKE_MIN) out.push({ id: `p${p.x},${p.y}`, kind: 'smoke', at: centreOf(p, n), top: 1.1, rate: Math.min(1, rate) });
  }
  return out;
}

/** Emissions as a flow list straight from a snapshot, used for ambient smoke outside playback. */
export function emissionsFromSnapshot(s: Snapshot): NonNullable<MonthFlows['emissions']> {
  const sum = s.nature.grossEmissionsMonthly * 0.1; // a rough ambient share for household puffs
  return [
    ...s.firms.map((f) => ({ source: 'firm' as const, id: f.id, tonnes: f.active ? f.emissions : 0 })),
    ...s.plants.map((p) => ({ source: 'plant' as const, tile: { ...p.tile }, tonnes: p.emissionsMonthly })),
    { source: 'households' as const, tonnes: sum },
    { source: 'sink' as const, tonnes: -s.nature.sinkMonthly },
  ];
}

const FX_OF: Partial<Record<SimEvent['type'], FxType>> = {
  firm_closed: 'closed', firm_founded: 'founded', hired: 'hired', fired: 'fired', person_hired: 'hired', person_fired: 'fired', household_evicted: 'shortfall', forest_shrank: 'forest', commons_collapsed: 'commons',
  commons_enclosed: 'enclosed', bank_failed: 'bank', class_changed: 'class', rent_shortfall: 'shortfall', loan_defaulted: 'default',
};

function fxTile(e: SimEvent, c: PlanCtx): Tile | undefined {
  switch (e.type) {
    case 'firm_closed': case 'firm_founded': case 'hired': case 'fired': case 'person_hired': case 'person_fired': case 'loan_defaulted': return firmTile(c, e.firm);
    case 'class_changed': case 'rent_shortfall': case 'household_evicted': return homeTile(c, e.household);
    case 'bank_failed': return firstOf(c, 'bank');
    case 'forest_shrank': return c.layout.scenery.find((s) => s.kind === 'forest');
    case 'commons_collapsed': case 'commons_enclosed': return c.layout.scenery.find((s) => s.kind === 'commons');
    default: return undefined;
  }
}

/** Events -> short effects at tiles, spread over the month. Most important first when there are too many. */
export function planFx(events: SimEvent[], c: PlanCtx): Fx[] {
  const n = c.layout.gridSize;
  const picked = events.filter((e) => FX_OF[e.type] && fxTile(e, c)).sort((a, b) => b.importance - a.importance).slice(0, CAPS.fx);
  return picked.map((e, i) => {
    const tile = fxTile(e, c)!;
    return { id: `${e.type}:${i}`, type: FX_OF[e.type]!, at: centreOf(tile, n), tile: { x: tile.x, y: tile.y }, t: 0.08 + (0.6 * i) / Math.max(1, picked.length) };
  });
}

/** One month of flows -> tracks (figures, goods, tokens) + emitters + effects. Pure and deterministic. */
export function planFrame(frame: MonthFrame, c: PlanCtx): FramePlan {
  // Contract v0.3 section 10.8: every flow list may be missing or empty.
  const f = { commutes: [], purchases: [], rent: [], wages: [], emissions: [], ...frame.flows } as Required<MonthFrame['flows']>;
  const tracks: Track[] = [];
  const seed0 = frame.month * 1009 + c.turn * 31;
  const bank = firstOf(c, 'bank'), gov = firstOf(c, 'government');

  // Purchases (real, household -> firm): goods, food or a service ticket from the firm to the home, plus a coin the other way.
  const buys = f.purchases.flatMap((p) => {
    const h = homeTile(c, p.household);
    if (!h) return [];
    const firm = c.firms.find((q) => q.id === p.firm && q.active && firmTile(c, q.id));
    return firm ? [{ p, h, ft: firmTile(c, firm.id)! }] : [];
  });
  allocate(buys.map((b) => b.p.amount), CAPS.goods, 3).forEach((cnt, i) => {
    const b = buys[i]!;
    for (let k = 0; k < cnt; k++) {
      const seed = seed0 + 5000 + i * 17 + k, [t0, t1] = window01(seed);
      tracks.push({ id: `g${frame.month}:${i}:${k}`, kind: 'good', style: b.p.sector === 'food' ? 1 : b.p.sector === 'services' ? 2 : 0, path: pathBetween(c, b.ft, b.h, seed), t0, t1 });
      if (k % 2 === 0) tracks.push({ id: `bp${frame.month}:${i}:${k}`, kind: 'token', style: 2, path: pathBetween(c, b.h, b.ft, seed + 1), t0: Math.max(0, t0 - 0.05), t1: Math.max(0.2, t1 - 0.05) });
    }
  });

  const tokenBudget = CAPS.tokens - tracks.filter((t) => t.kind === 'token').length;
  // Wages: firm -> home. Rent: home -> landlord home (or town hall for public housing tenants).
  const wages = f.wages.filter((w) => homeTile(c, w.household) && firmTile(c, w.firm));
  allocate(wages.map((w) => w.amount), Math.floor(tokenBudget * 0.55), 4).forEach((cnt, i) => {
    const w = wages[i]!;
    for (let k = 0; k < cnt; k++) {
      const seed = seed0 + 9000 + i * 13 + k, [t0, t1] = window01(seed);
      tracks.push({ id: `w${frame.month}:${i}:${k}`, kind: 'token', style: 0, path: pathBetween(c, firmTile(c, w.firm)!, homeTile(c, w.household)!, seed), t0, t1 });
    }
  });
  const rents = f.rent.flatMap((r) => {
    const from = homeTile(c, r.household);
    const home = c.homes.find((h) => h.id === r.home);
    const to = home ? (home.owner === 'public' ? gov ?? bank : homeTile(c, home.owner) ?? gov ?? bank) : gov ?? bank;
    return from && to ? [{ r, from, to }] : [];
  });
  allocate(rents.map((r) => r.r.amount), Math.floor(tokenBudget * 0.35), 3).forEach((cnt, i) => {
    const r = rents[i]!;
    for (let k = 0; k < cnt; k++) {
      const seed = seed0 + 13000 + i * 11 + k, [t0, t1] = window01(seed);
      tracks.push({ id: `r${frame.month}:${i}:${k}`, kind: 'token', style: 1, path: pathBetween(c, r.from, r.to, seed), t0, t1 });
    }
  });

  return { tracks, emitters: planEmitters(f.emissions, c), fx: planFx(frame.events ?? [], c) };
}

/** Looping token streams for the money-flow lens: all three months of the last turn merged into one repeating loop. */
export function planFlowLoop(frames: MonthFrame[], c: PlanCtx): Track[] {
  if (frames.length === 0) return [];
  const hasFlows = (m: MonthFrame) => Object.values(m.flows ?? {}).some((a) => (a?.length ?? 0) > 0);
  const first = [...frames].reverse().find(hasFlows) ?? frames[frames.length - 1]!;
  return planFrame({ ...first, events: [] }, c).tracks.filter((t) => t.kind === 'token');
}

/** Stateless smoke particle: position and size for particle `i` of an emitter at time `sec`. Reproducible, no per-particle state. */
export function smokeParticle(e: Emitter, i: number, sec: number): { x: number; y: number; z: number; scale: number; life: number } {
  const seed = i * 131 + e.id.length * 17 + Math.round((e.at[0] * 7 + e.at[1] * 13) * 10);
  const period = e.kind === 'house' ? 3.2 : 2.6;
  const life = (((sec / period + hash01(seed)) % 1) + 1) % 1;
  const jx = (hash01(seed + 1) - 0.5) * 0.12, jz = (hash01(seed + 2) - 0.5) * 0.12;
  const grow = life < 0.15 ? life / 0.15 : 1 - (life - 0.15) / 0.85;
  if (e.kind === 'sink') {
    const k = 0.5 + 0.5 * e.rate;
    return { x: e.at[0] + (hash01(seed + 3) - 0.5) * 0.7, y: e.top - life * 0.8, z: e.at[1] + (hash01(seed + 4) - 0.5) * 0.7, scale: 0.05 * k * (1 - Math.abs(life - 0.5) * 2) + 0.01, life };
  }
  const size = e.kind === 'house' ? 0.07 : 0.1 + 0.14 * e.rate;
  return { x: e.at[0] + jx + life * (0.35 + 0.3 * e.rate), y: e.top + life * (0.9 + 0.8 * e.rate), z: e.at[1] + jz + life * 0.12, scale: Math.max(0.005, size * grow * (0.7 + life)), life };
}

/** How many smoke particles each emitter gets, never more than `cap` in total. */
export function smokeCounts(emitters: Emitter[], cap: number = CAPS.smoke): number[] {
  const want = emitters.map((e) => (e.kind === 'house' ? Math.ceil(e.rate * 3) : Math.ceil(e.rate * 14)));
  const total = want.reduce((a, b) => a + b, 0);
  if (total <= cap) return want;
  const k = cap / total;
  return want.map((w) => Math.floor(w * k));
}
