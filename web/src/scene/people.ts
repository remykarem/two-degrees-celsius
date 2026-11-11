import type { AgeBand, MonthFrame, PersonView, Snapshot } from '../contract';
import type { MapLayout, Tile } from '../store';
import { centreOf, hash01, type Pt } from './coords';
import { commuteFraction, pointAt, trackProgress } from './flows';
import { route } from './paths';
import { staffCommutesOf } from '../store/scene-estate';

/** Pure people planning: who stands where, and who walks where during a month. No Three.js, no clock. */

/** Okabe-Ito plus grey, plus dark gold for the ninth (the estate owner): nine household colours. Every household also has its own marker shape, so colour is never the only cue. */
export const HOUSEHOLD_COLORS = [0xe69f00, 0x56b4e9, 0x009e73, 0xf0e442, 0x0072b2, 0xd55e00, 0xcc79a7, 0x8d8d8d, 0xb8860b] as const;
export type MarkerShape = 'sphere' | 'cube' | 'cone' | 'octa' | 'disc' | 'tetra' | 'torus' | 'dodeca' | 'star';
export const MARKER_SHAPES: readonly MarkerShape[] = ['sphere', 'cube', 'cone', 'octa', 'disc', 'tetra', 'torus', 'dodeca', 'star'];
export const SHAPE_LABEL: Record<MarkerShape, string> = { sphere: 'ball', cube: 'cube', cone: 'cone', octa: 'diamond', disc: 'disc', tetra: 'pyramid', torus: 'ring', dodeca: 'gem', star: 'star' };
export const AGE_SCALE: Record<AgeBand, number> = { child: 0.68, adult: 1, retired: 0.92 };

export interface PersonStand {
  id: number;
  household: number;
  /** Position of the household in the snapshot list: picks the colour and marker shape. */
  hIndex: number;
  ageBand: AgeBand;
  /** Idle position (world x, z). */
  at: Pt;
  /** The tile the person lives on (or camps on), for selection and hover. */
  tile: Tile;
  unhoused: boolean;
}
export interface Camp { household: number; tile: Tile }

const SLOTS: Pt[] = [[-0.3, 0.38], [0, 0.4], [0.3, 0.38], [0.4, 0.1], [-0.4, 0.1]];

/** Free tile for each unhoused household, nearest the town hall first (or the map centre without one). Deterministic. */
export function campTiles(layout: MapLayout, snap: Snapshot): Camp[] {
  const un = snap.households.filter((h) => h.home === null);
  if (!un.length) return [];
  const hall = layout.scenery.find((s) => s.kind === 'government') ?? { x: 5, y: 5 };
  const taken = new Set<string>();
  const mark = (t: Tile) => taken.add(`${t.x},${t.y}`);
  layout.scenery.forEach(mark); layout.placed.forEach(mark); layout.roads.forEach(mark); layout.pendingRoads.forEach(mark);
  Object.values(layout.pendingTiles).forEach(mark);
  const free = snap.map.tiles.filter((t) => t.kind === 'empty' && !taken.has(`${t.x},${t.y}`))
    .sort((a, b) => Math.hypot(a.x - hall.x, a.y - hall.y) - Math.hypot(b.x - hall.x, b.y - hall.y) || a.y - b.y || a.x - b.x);
  return un.map((h, i) => ({ household: h.id, tile: free[i] ? { x: free[i]!.x, y: free[i]!.y } : { x: hall.x, y: hall.y } }));
}

export function planStands(layout: MapLayout, snap: Snapshot): PersonStand[] {
  const n = layout.gridSize;
  const camps = new Map(campTiles(layout, snap).map((c) => [c.household, c.tile]));
  const out: PersonStand[] = [];
  snap.households.forEach((h, hIndex) => {
    const home = h.home === null ? null : snap.homes.find((x) => x.id === h.home);
    const tile = home ? home.tile : camps.get(h.id);
    if (!tile) return;
    const [cx, cz] = centreOf(tile, n);
    h.members.forEach((pid, k) => {
      const p = snap.people.find((x) => x.id === pid);
      if (!p) return;
      let off: Pt;
      if (home) off = SLOTS[k % SLOTS.length]!;
      else { const a = (k / Math.max(3, h.members.length)) * Math.PI * 2 + 0.6; off = [Math.cos(a) * 0.34, Math.sin(a) * 0.34]; }
      out.push({ id: p.id, household: h.id, hIndex, ageBand: p.ageBand, at: [cx + off[0], cz + off[1]], tile: { x: tile.x, y: tile.y }, unhoused: !home });
    });
  });
  return out;
}

export interface CommutePlan { person: number; path: Pt[]; t0: number; t1: number; minutes: number; /** Town v5: a shopping trip (same walk as a commute, to a firm the household spent money at). Absent means a commute. */ kind?: 'shop' }

/** Move `p` a distance `d` toward `to`. */
const toward = (p: Pt, to: Pt, d: number): Pt => { const l = Math.hypot(to[0] - p[0], to[1] - p[1]); return l < 1e-9 ? p : [p[0] + ((to[0] - p[0]) / l) * Math.min(d, l), p[1] + ((to[1] - p[1]) / l) * Math.min(d, l)]; };

/**
 * One commute plan per commuting person in the frame (the wire has no path, so the scene routes itself, preferring roads).
 * Longer commutes take a longer share of the month, so the pace is the same and the minutes show.
 */
export function planCommutes(frame: Pick<MonthFrame, 'month' | 'flows'>, layout: MapLayout, stands: PersonStand[], turn: number): CommutePlan[] {
  const list = [...(frame.flows?.commutes ?? []), ...staffCommutesOf(frame)]; // Town v7: the pilot's commute to the hangar
  if (!list.length) return [];
  const n = layout.gridSize;
  const by = new Map(stands.map((s) => [s.id, s]));
  const maxMin = Math.max(1, ...list.map((c) => c.minutes));
  const out: CommutePlan[] = [];
  for (const c of list) {
    const st = by.get(c.person);
    if (!st) continue;
    const from: Tile = c.from ?? st.tile;
    const tiles = route(layout, from, c.to);
    const seed = frame.month * 1009 + turn * 31 + c.person * 7;
    const dur = Math.min(0.62, 0.2 + 0.4 * (c.minutes / maxMin));
    const t0 = 0.03 + (0.95 - dur - 0.03) * hash01(seed);
    const pts: Pt[] = tiles.map((t) => centreOf(t, n));
    const ox = (hash01(seed + 3) - 0.5) * 0.3, oz = (hash01(seed + 4) - 0.5) * 0.3;
    for (let i = 1; i < pts.length - 1; i++) pts[i] = [pts[i]![0] + ox * 0.5, pts[i]![1] + oz * 0.5];
    if (pts.length === 1) pts.push([pts[0]![0], pts[0]![1]]);
    pts[0] = st.at;
    const last = pts.length - 1;
    const door = toward(pts[last]!, pts[last - 1]!, 0.42);
    pts[last] = [door[0] + ox, door[1] + oz];
    out.push({ person: c.person, path: pts, t0, t1: Math.min(1, t0 + dur), minutes: c.minutes });
  }
  return out;
}

/** Most shopping figures walking in one month. Cosmetic: the real purchases are in the ledgers, not on the map. */
export const MAX_SHOPPING_TRIPS = 5;

/**
 * Town v5 shopping trips: for the biggest real `flows.purchases` (household at a firm), one member of the household who is not already
 * walking to work goes to that firm's tile and back, along the same road-preferring router as a commute. At most `cap` trips, one per
 * household, at most one per firm per sector first so the Shop, the Farm and the Factory all see visitors. Empty or missing purchases give no trips.
 */
export function planShopping(frame: Pick<MonthFrame, 'month' | 'flows'>, layout: MapLayout, stands: PersonStand[], busy: ReadonlySet<number>, turn: number, cap = MAX_SHOPPING_TRIPS): CommutePlan[] {
  const buys = (frame.flows?.purchases ?? []).filter((p) => p.amount > 0);
  if (!buys.length || cap <= 0) return [];
  const n = layout.gridSize;
  const firmTile = new Map(layout.scenery.filter((x) => x.kind === 'firm' && x.ref !== undefined).map((x) => [x.ref!, { x: x.x, y: x.y }] as const));
  const ranked = [...buys].sort((a, b) => b.amount - a.amount || a.household - b.household || a.firm - b.firm);
  const chosen: typeof ranked = [];
  const usedHh = new Set<number>();
  const take = (p: (typeof ranked)[number]) => { chosen.push(p); usedHh.add(p.household); };
  const sectorsSeen = new Set<string>();
  for (const p of ranked) { if (chosen.length >= cap) break; if (!usedHh.has(p.household) && !sectorsSeen.has(p.sector)) { sectorsSeen.add(p.sector); take(p); } }
  for (const p of ranked) { if (chosen.length >= cap) break; if (!usedHh.has(p.household)) take(p); }
  const out: CommutePlan[] = [];
  const walking = new Set(busy);
  for (const p of chosen) {
    const to = firmTile.get(p.firm);
    if (!to) continue;
    const member = stands.filter((s) => s.household === p.household && s.ageBand !== 'child' && !walking.has(s.id)).sort((a, b) => a.id - b.id)[0];
    if (!member) continue;
    walking.add(member.id);
    const tiles = route(layout, member.tile, to);
    const seed = frame.month * 1013 + turn * 37 + member.id * 11 + 5;
    const dur = Math.min(0.5, 0.22 + 0.04 * tiles.length);
    const t0 = 0.05 + (0.93 - dur - 0.05) * hash01(seed);
    const pts: Pt[] = tiles.map((t) => centreOf(t, n));
    const ox = (hash01(seed + 3) - 0.5) * 0.3, oz = (hash01(seed + 4) - 0.5) * 0.3;
    for (let i = 1; i < pts.length - 1; i++) pts[i] = [pts[i]![0] + ox * 0.5, pts[i]![1] + oz * 0.5];
    if (pts.length === 1) pts.push([pts[0]![0], pts[0]![1]]);
    pts[0] = member.at;
    const last = pts.length - 1;
    const door = toward(pts[last]!, pts[last - 1]!, 0.42);
    pts[last] = [door[0] + ox, door[1] + oz];
    out.push({ person: member.id, path: pts, t0, t1: Math.min(1, t0 + dur), minutes: 0, kind: 'shop' });
  }
  return out;
}

/** Commutes first, then shopping trips for people who are not commuting this month. */
export function planTrips(frame: Pick<MonthFrame, 'month' | 'flows'>, layout: MapLayout, stands: PersonStand[], turn: number): CommutePlan[] {
  const commutes = planCommutes(frame, layout, stands, turn);
  return [...commutes, ...planShopping(frame, layout, stands, new Set(commutes.map((c) => c.person)), turn)];
}

export interface Pose { x: number; z: number; heading: number; walking: boolean }

/** Where a person is at month time t: on their commute while it runs, otherwise at their idle spot. */
export function poseAt(stand: PersonStand, plan: CommutePlan | undefined, t: number): Pose {
  if (plan) {
    const u = trackProgress({ id: '', kind: 'token', style: 0, path: plan.path, t0: plan.t0, t1: plan.t1 }, t);
    if (u !== null) {
      const f = commuteFraction(u), f2 = commuteFraction(Math.min(1, u + 0.015));
      const [x, z] = pointAt(plan.path, f), [x2, z2] = pointAt(plan.path, f2);
      const moving = u < 0.4 || u > 0.6;
      return { x, z, heading: x2 === x && z2 === z ? 0 : Math.atan2(x2 - x, z2 - z), walking: moving };
    }
  }
  return { x: stand.at[0], z: stand.at[1], heading: 0, walking: false };
}

export const personById = (snap: Snapshot, id: number): PersonView | undefined => snap.people.find((p) => p.id === id);
