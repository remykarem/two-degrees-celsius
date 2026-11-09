import type { TileKind, TileRef } from '../../contract';
import { GEN } from './data';
import { cellAt, GRID, type MockState } from './state';

/** What a walker may step on (besides the start and the destination): open ground, roads and commons. */
const PASSABLE: TileKind[] = ['empty', 'road', 'commons'];

export interface Route { path: TileRef[]; tiles: number; minutes: number }

/** Cheapest walk from `a` to `b` over the grid: a road tile costs the fast minutes, any other tile the slow minutes. Dijkstra on 144 cells. */
export function route(s: Pick<MockState, 'grid'>, a: TileRef, b: TileRef): Route {
  const { slowMinutes: slow, fastMinutes: fast } = GEN.space;
  const idx = (t: TileRef) => t.y * GRID + t.x;
  const dist = new Array<number>(GRID * GRID).fill(Infinity);
  const prev = new Array<number>(GRID * GRID).fill(-1);
  const done = new Array<boolean>(GRID * GRID).fill(false);
  dist[idx(a)] = 0;
  for (;;) {
    let u = -1;
    for (let i = 0; i < dist.length; i++) if (!done[i] && dist[i]! < Infinity && (u < 0 || dist[i]! < dist[u]!)) u = i;
    if (u < 0 || u === idx(b)) break;
    done[u] = true;
    const ux = u % GRID, uy = Math.floor(u / GRID);
    for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]] as const) {
      const x = ux + dx, y = uy + dy;
      const cell = cellAt(s, x, y);
      if (!cell) continue;
      const v = y * GRID + x;
      const isDest = v === idx(b);
      if (!isDest && !PASSABLE.includes(cell.kind)) continue;
      const step = cell.kind === 'road' ? fast : slow;
      if (dist[u]! + step < dist[v]!) { dist[v] = dist[u]! + step; prev[v] = u; }
    }
  }
  if (!Number.isFinite(dist[idx(b)]!)) {
    const tiles = Math.abs(a.x - b.x) + Math.abs(a.y - b.y);
    return { path: [], tiles, minutes: tiles * slow };
  }
  const path: TileRef[] = [];
  for (let v = idx(b); v >= 0 && v !== idx(a); v = prev[v]!) path.unshift({ x: v % GRID, y: Math.floor(v / GRID) });
  return { path, tiles: path.length, minutes: dist[idx(b)]! };
}

/** Commute of a worker living in `homeId` (null = unhoused) to `firmId`. Unhoused people get the content's fixed long commute. */
export function commute(s: MockState, homeId: number | null, firmId: number): Route {
  if (homeId === null) return { path: [], tiles: GEN.space.unhousedTiles, minutes: GEN.space.unhousedMinutes };
  const home = s.homes.find((h) => h.id === homeId);
  const firm = GEN.firms.find((f) => f.id === firmId);
  if (!home || !firm) return { path: [], tiles: 0, minutes: 0 };
  return route(s, { x: home.x, y: home.y }, firm.tile);
}
