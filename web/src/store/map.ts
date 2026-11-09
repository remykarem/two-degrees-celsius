import type { BlockReason, InfraKind, Snapshot, Tile as MapTile, TileRef } from '../contract';

/** A map position. (The contract's own `Tile` is the full snapshot tile; this is just x and y.) */
export type Tile = TileRef;
export const GRID_SIZE = 12;
export const sameTile = (a: Tile, b: Tile) => a.x === b.x && a.y === b.y;
export const tileKey = (t: Tile) => `${t.x},${t.y}`;
export const inBounds = (t: Tile) => t.x >= 0 && t.y >= 0 && t.x < GRID_SIZE && t.y < GRID_SIZE;

/** What the player has selected. A tile selection is for empty ground, roads, plants and scenery that has no entity of its own. */
export type Selection =
  | { kind: 'person'; id: number }
  | { kind: 'household'; id: number }
  | { kind: 'home'; id: number }
  | { kind: 'firm'; id: number }
  | { kind: 'tile'; x: number; y: number };

export type SceneryKind = 'home' | 'firm' | 'bank' | 'government' | 'forest' | 'commons';
export interface Scenery { kind: SceneryKind; ref?: number; x: number; y: number; label: string }
/** A finished building the player can place or remove (roads are in `roads`). */
export interface Placed { kind: Exclude<InfraKind, 'road'>; x: number; y: number; ref?: number }

/**
 * A DERIVED view of `snapshot.map.tiles` in the shape the scene draws from. Nothing here is stored or saved: the simulation owns the
 * map. Memoised per snapshot object, so the same snapshot gives the same layout object (identity checks stay cheap).
 */
export interface MapLayout {
  gridSize: number;
  scenery: Scenery[];
  placed: Placed[];
  /** Tile of every queued `place` command, by pending id. */
  pendingTiles: Record<number, Tile>;
  roads: Tile[];
  /** Tiles with a queued road. */
  pendingRoads: Tile[];
}

const EMPTY: MapLayout = { gridSize: GRID_SIZE, scenery: [], placed: [], pendingTiles: {}, roads: [], pendingRoads: [] };
const memo = new WeakMap<Snapshot, MapLayout>();

export function layoutOf(snap: Snapshot | null): MapLayout {
  if (!snap) return EMPTY;
  const hit = memo.get(snap);
  if (hit) return hit;
  const out: MapLayout = { gridSize: snap.map.width, scenery: [], placed: [], pendingTiles: {}, roads: [], pendingRoads: [] };
  for (const t of snap.map.tiles) {
    const at = { x: t.x, y: t.y };
    switch (t.kind) {
      case 'home': {
        const home = snap.homes.find((h) => h.id === t.ref);
        const hh = home?.household != null ? snap.households.find((h) => h.id === home.household) : undefined;
        out.scenery.push({ kind: 'home', ...(t.ref !== undefined ? { ref: t.ref } : {}), ...at, label: hh ? `${hh.label} home` : 'Vacant home' });
        break;
      }
      case 'firm': out.scenery.push({ kind: 'firm', ...(t.ref !== undefined ? { ref: t.ref } : {}), ...at, label: snap.firms.find((f) => f.id === t.ref)?.name ?? 'Firm' }); break;
      case 'bank': out.scenery.push({ kind: 'bank', ...at, label: 'Bank' }); break;
      case 'town_hall': out.scenery.push({ kind: 'government', ...at, label: 'Town hall' }); break;
      case 'forest': out.scenery.push({ kind: 'forest', ...at, label: 'Forest' }); break;
      case 'commons': out.scenery.push({ kind: 'commons', ...at, label: 'Commons' }); break;
      case 'public_housing': case 'fossil_plant': case 'renewable_plant': out.placed.push({ kind: t.kind, ...at, ...(t.ref !== undefined ? { ref: t.ref } : {}) }); break;
      case 'road': out.roads.push(at); break;
      case 'empty': break;
    }
  }
  for (const p of snap.pending) {
    if (p.command.type !== 'place') continue;
    out.pendingTiles[p.id] = { ...p.command.tile };
    if (p.command.kind === 'road') out.pendingRoads.push({ ...p.command.tile });
  }
  memo.set(snap, out);
  return out;
}

export function tileAt(snap: Snapshot | null, t: Tile): MapTile | undefined {
  return snap?.map.tiles[t.y * snap.map.width + t.x];
}
export function occupant(layout: MapLayout, t: Tile): Scenery | Placed | null {
  return layout.scenery.find((s) => sameTile(s, t)) ?? layout.placed.find((p) => sameTile(p, t)) ?? null;
}
/** Id of the queued build on this tile, if any. */
export function pendingAt(layout: MapLayout, t: Tile): number | null {
  for (const [id, tile] of Object.entries(layout.pendingTiles)) if (sameTile(tile, t)) return Number(id);
  return null;
}

/** The tile a selection sits on (for the scene's highlight). Unhoused households and unemployed people have no tile of their own: null for people without a home. */
export function selectionTile(snap: Snapshot | null, sel: Selection | null): Tile | null {
  if (!snap || !sel) return null;
  const homeTile = (id: number | null | undefined) => { const h = snap.homes.find((x) => x.id === id); return h ? { ...h.tile } : null; };
  switch (sel.kind) {
    case 'tile': return { x: sel.x, y: sel.y };
    case 'home': return homeTile(sel.id);
    case 'household': return homeTile(snap.households.find((h) => h.id === sel.id)?.home);
    case 'person': { const p = snap.people.find((x) => x.id === sel.id); return p ? homeTile(snap.households.find((h) => h.id === p.household)?.home) : null; }
    case 'firm': { const t = snap.map.tiles.find((x) => x.kind === 'firm' && x.ref === sel.id); return t ? { x: t.x, y: t.y } : null; }
  }
}

/** What a click on a tile selects. */
export function selectionAt(snap: Snapshot | null, t: Tile): Selection {
  const tile = tileAt(snap, t);
  if (tile?.kind === 'home' || tile?.kind === 'public_housing') return tile.ref !== undefined ? { kind: 'home', id: tile.ref } : { kind: 'tile', ...t };
  if (tile?.kind === 'firm' && tile.ref !== undefined) return { kind: 'firm', id: tile.ref };
  return { kind: 'tile', ...t };
}

export const selectionEquals = (a: Selection | null, b: Selection | null) => (!a || !b ? a === b : a.kind === b.kind && JSON.stringify(a) === JSON.stringify(b));

const REASON: Partial<Record<BlockReason | string, string>> = {
  tile_occupied: 'Something is already built here, or queued here',
  tile_invalid: 'Nothing can be built on this tile',
  nothing_to_remove: 'There is nothing to remove here',
};
export const describeTileReason = (reason: string | undefined): string => (reason && REASON[reason]) || 'This tile cannot take that';
