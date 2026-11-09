import type { MapLayout, Tile } from '../store';
import { tileKey } from './coords';

const cache = new WeakMap<MapLayout, Map<string, Tile[]>>();

/** Tiles people and goods cannot walk through: buildings and forest (commons are open meadow). */
export function blockedTiles(layout: MapLayout): Set<string> {
  const s = new Set<string>();
  for (const sc of layout.scenery) if (sc.kind !== 'commons') s.add(tileKey(sc));
  for (const p of layout.placed) s.add(tileKey(p));
  return s;
}

/** Cost of stepping onto a tile: built roads are cheap, so routes follow them when the detour is short; meadow costs a little more. */
export const STEP_COST = { road: 0.3, open: 1, commons: 1.3 } as const;

/**
 * Cheapest 4-neighbour route from a to b around buildings (both ends may be buildings), PREFERRING built roads
 * (STEP_COST). Includes both ends. Falls back to an L-shaped line when no route exists. Deterministic.
 */
export function route(layout: MapLayout, a: Tile, b: Tile): Tile[] {
  let memo = cache.get(layout);
  if (!memo) cache.set(layout, (memo = new Map()));
  const ck = `${tileKey(a)}>${tileKey(b)}`;
  const hit = memo.get(ck);
  if (hit) return hit;
  const n = layout.gridSize;
  const blocked = blockedTiles(layout);
  blocked.delete(tileKey(a));
  blocked.delete(tileKey(b));
  const roads = new Set(layout.roads.map(tileKey));
  const commons = new Set(layout.scenery.filter((s) => s.kind === 'commons').map(tileKey));
  const cost = (k: string) => (roads.has(k) ? STEP_COST.road : commons.has(k) ? STEP_COST.commons : STEP_COST.open);
  const dist = new Map<string, number>([[tileKey(a), 0]]);
  const prev = new Map<string, Tile | null>([[tileKey(a), null]]);
  const done = new Set<string>();
  const open: Tile[] = [a];
  const dirs = [[1, 0], [0, 1], [-1, 0], [0, -1]];
  while (open.length) {
    let bi = 0;
    for (let i = 1; i < open.length; i++) if (dist.get(tileKey(open[i]!))! < dist.get(tileKey(open[bi]!))!) bi = i;
    const cur = open.splice(bi, 1)[0]!;
    const ck0 = tileKey(cur);
    if (done.has(ck0)) continue;
    done.add(ck0);
    if (cur.x === b.x && cur.y === b.y) break;
    for (const [dx, dy] of dirs) {
      const nx = cur.x + dx!, ny = cur.y + dy!;
      const k = `${nx},${ny}`;
      if (nx < 0 || ny < 0 || nx >= n || ny >= n || blocked.has(k) || done.has(k)) continue;
      const d = dist.get(ck0)! + cost(k);
      if (d < (dist.get(k) ?? Infinity) - 1e-9) { dist.set(k, d); prev.set(k, cur); open.push({ x: nx, y: ny }); }
    }
  }
  let out: Tile[];
  if (done.has(tileKey(b))) {
    out = [];
    for (let t: Tile | null = b; t; t = prev.get(tileKey(t)) ?? null) out.unshift({ x: t.x, y: t.y });
  } else {
    out = [{ ...a }];
    for (let x = a.x; x !== b.x; x += Math.sign(b.x - x)) out.push({ x: x + Math.sign(b.x - x), y: a.y });
    for (let y = a.y; y !== b.y; y += Math.sign(b.y - y)) out.push({ x: b.x, y: y + Math.sign(b.y - y) });
  }
  memo.set(ck, out);
  return out;
}

/** Share of a route's inner tiles that are built roads (for tests and the commute readout). */
export function roadShare(layout: MapLayout, r: Tile[]): number {
  const roads = new Set(layout.roads.map(tileKey));
  return r.length ? r.filter((t) => roads.has(tileKey(t))).length / r.length : 0;
}

/** Walkable tiles used by the everyday routes (homes to workplaces, shops, bank, town hall) that are not roads; drawn as dirt paths. */
export function pathTiles(layout: MapLayout): Set<string> {
  const homes = layout.scenery.filter((s) => s.kind === 'home');
  const dest = layout.scenery.filter((s) => s.kind === 'firm' || s.kind === 'bank' || s.kind === 'government');
  const keep = new Set<string>();
  const blocked = blockedTiles(layout);
  const roads = new Set(layout.roads.map(tileKey));
  for (const h of homes) {
    // each home gets a track to its two nearest workplaces or services, so the dirt paths stay a network, not a plaza
    const near = [...dest].sort((p, q) => Math.abs(p.x - h.x) + Math.abs(p.y - h.y) - (Math.abs(q.x - h.x) + Math.abs(q.y - h.y)) || p.y - q.y || p.x - q.x).slice(0, 2);
    for (const d of near) for (const t of route(layout, h, d)) {
      const k = tileKey(t);
      if (!blocked.has(k) && !roads.has(k)) keep.add(k);
    }
  }
  return keep;
}
