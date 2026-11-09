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

/**
 * Shortest 4-neighbour route from a to b around buildings (both ends may be buildings). Includes both ends.
 * Falls back to an L-shaped line when no route exists. Deterministic.
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
  const prev = new Map<string, Tile | null>([[tileKey(a), null]]);
  const queue: Tile[] = [a];
  const dirs = [[1, 0], [0, 1], [-1, 0], [0, -1]];
  let found = false;
  for (let i = 0; i < queue.length && !found; i++) {
    const cur = queue[i]!;
    for (const [dx, dy] of dirs) {
      const nx = cur.x + dx!, ny = cur.y + dy!;
      const k = `${nx},${ny}`;
      if (nx < 0 || ny < 0 || nx >= n || ny >= n || blocked.has(k) || prev.has(k)) continue;
      prev.set(k, cur);
      if (nx === b.x && ny === b.y) { found = true; break; }
      queue.push({ x: nx, y: ny });
    }
  }
  let out: Tile[];
  if (found) {
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

/** Walkable tiles used by the everyday routes (homes to workplaces, shops, bank, town hall); drawn as footpaths. */
export function pathTiles(layout: MapLayout): Set<string> {
  const homes = layout.scenery.filter((s) => s.kind === 'home');
  const dest = layout.scenery.filter((s) => s.kind === 'firm' || s.kind === 'bank' || s.kind === 'government');
  const keep = new Set<string>();
  const blocked = blockedTiles(layout);
  for (const h of homes) for (const d of dest) for (const t of route(layout, h, d)) {
    const k = tileKey(t);
    if (!blocked.has(k)) keep.add(k);
  }
  return keep;
}
