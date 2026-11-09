import raw from './layout.json';
import type { InfraKind, Snapshot } from '../contract';

export type SceneryKind = 'home' | 'firm' | 'bank' | 'government' | 'forest' | 'commons';
export interface Scenery { kind: SceneryKind; ref?: number; x: number; y: number; label: string }
/** A finished building. Which tile it sits on is cosmetic. */
export interface Placed { kind: InfraKind; x: number; y: number }
export interface Tile { x: number; y: number }

/**
 * The UI-owned map. `scenery` and `placed` are DERIVED from the snapshot (households, firms, buildings) and reconciled whenever the
 * counts change; only `placed` and `pendingTiles` carry choices that must persist. Position never affects the economy.
 */
export interface MapLayout {
  gridSize: number;
  scenery: Scenery[];
  /** Finished buildings (count per kind equals Snapshot.buildings). */
  placed: Placed[];
  /** Tile reserved for each queued build, by pending id. */
  pendingTiles: Record<number, Tile>;
}

interface Geometry {
  gridSize: number; bank: Tile; government: Tile; forest: Tile[]; commons: Tile[]; homeSlots: Tile[]; firmSlots: Tile[];
  buildZones: Record<InfraKind, Tile[]>;
}
const GEO = raw as unknown as Geometry;

export const sameTile = (a: Tile, b: Tile) => a.x === b.x && a.y === b.y;
const key = (t: Tile) => `${t.x},${t.y}`;
export const inBounds = (layout: Pick<MapLayout, 'gridSize'>, t: Tile) => t.x >= 0 && t.y >= 0 && t.x < layout.gridSize && t.y < layout.gridSize;

export function emptyLayout(): MapLayout {
  return { gridSize: GEO.gridSize, scenery: [], placed: [], pendingTiles: {} };
}

/** Fixed things plus homes (one per household) and firm slots (one per firm), all placed on the geometry's slots. */
export function derivedScenery(snap: Snapshot): Scenery[] {
  const out: Scenery[] = [];
  const taken = new Set<string>([key(GEO.bank), key(GEO.government), ...GEO.forest.map(key), ...GEO.commons.map(key)]);
  const spill = freeTiles(taken);
  snap.households.forEach((h, i) => {
    const t = GEO.homeSlots[i] ?? spill();
    taken.add(key(t));
    out.push({ kind: 'home', ref: h.id, x: t.x, y: t.y, label: `${h.label} homes` });
  });
  snap.firms.forEach((f, i) => {
    const t = GEO.firmSlots[i] ?? spill();
    taken.add(key(t));
    out.push({ kind: 'firm', ref: f.id, x: t.x, y: t.y, label: f.name });
  });
  out.push({ kind: 'bank', ...GEO.bank, label: 'Bank' }, { kind: 'government', ...GEO.government, label: 'Town hall' });
  for (const t of GEO.forest) out.push({ kind: 'forest', ...t, label: 'Forest' });
  for (const t of GEO.commons) out.push({ kind: 'commons', ...t, label: 'Commons' });
  return out;
}

/** Row-major free tiles that are not in `taken`, skipping the fixed geometry and the build zones' reserved corners. Deterministic. */
function freeTiles(taken: Set<string>): () => Tile {
  let i = 0;
  return () => {
    for (; i < GEO.gridSize * GEO.gridSize; i++) {
      const t = { x: i % GEO.gridSize, y: Math.floor(i / GEO.gridSize) };
      if (!taken.has(key(t))) { i++; return t; }
    }
    return { x: 0, y: 0 };
  };
}

export interface BuildHint { kind: InfraKind; tile: Tile }
const KIND_COUNT: Record<InfraKind, (b: Snapshot['buildings']) => number> = {
  public_housing: (b) => b.publicHousing, fossil_plant: (b) => b.fossilPlant, renewable_plant: (b) => b.renewablePlant,
};
const KINDS: InfraKind[] = ['public_housing', 'fossil_plant', 'renewable_plant'];

/**
 * Brings the layout in line with the snapshot: scenery from households/firms, one placed tile per built building (extra tiles are
 * dropped last-first, missing ones added on `hints` tiles first, then the kind's zone, then any free tile), and a reserved tile for
 * every queued build. Returns the SAME object when nothing changed so identity checks stay cheap.
 */
export function reconcileLayout(prev: MapLayout | null, snap: Snapshot, hints: BuildHint[] = []): MapLayout {
  const scenery = derivedScenery(snap);
  const taken = new Set(scenery.map(key));
  const next: MapLayout = { gridSize: GEO.gridSize, scenery, placed: [], pendingTiles: {} };
  const anyFree = freeTiles(new Set());
  const free = (t: Tile | undefined) => !!t && inBounds(next, t) && !taken.has(key(t));
  const claim = (kind: InfraKind, preferred?: Tile): Tile => {
    let t = free(preferred) ? preferred! : GEO.buildZones[kind].find(free);
    if (!t) { for (let n = 0; n < GEO.gridSize * GEO.gridSize && !t; n++) { const c = anyFree(); if (free(c)) t = c; } }
    t ??= { x: 0, y: 0 };
    taken.add(key(t));
    return t;
  };

  for (const kind of KINDS) {
    const want = Math.max(0, Math.round(KIND_COUNT[kind](snap.buildings)));
    const mine = (prev?.placed ?? []).filter((p) => p.kind === kind && free(p));
    for (const p of mine.slice(0, want)) { taken.add(key(p)); next.placed.push({ kind, x: p.x, y: p.y }); }
    const kept = Math.min(mine.length, want);
    const queue = hints.filter((h) => h.kind === kind);
    for (let n = kept; n < want; n++) {
      const t = claim(kind, queue.shift()?.tile);
      next.placed.push({ kind, x: t.x, y: t.y });
    }
  }
  for (const p of snap.pending) {
    if (p.command.type !== 'place') continue;
    const prevTile = prev?.pendingTiles[p.id];
    const t = claim(p.command.kind, prevTile ?? p.command.tile);
    next.pendingTiles[p.id] = t;
  }
  return prev && JSON.stringify(prev) === JSON.stringify(next) ? prev : next;
}

export function occupant(layout: MapLayout, t: Tile): Scenery | Placed | null {
  return layout.scenery.find((s) => sameTile(s, t)) ?? layout.placed.find((p) => sameTile(p, t)) ?? null;
}
/** Id of the queued build reserved on this tile, if any. */
export function pendingAt(layout: MapLayout, t: Tile): number | null {
  for (const [id, tile] of Object.entries(layout.pendingTiles)) if (sameTile(tile, t)) return Number(id);
  return null;
}

/** Why a tile cannot take a building, or null when it is valid. `drafted` are tiles already in the draft plan. */
export function tileBlock(layout: MapLayout, t: Tile, drafted: Tile[]): string | null {
  if (!inBounds(layout, t)) return 'Outside the map';
  const o = occupant(layout, t);
  if (o) return 'label' in o ? `Occupied by ${(o as Scenery).label}` : 'Already built here';
  if (pendingAt(layout, t) !== null) return 'A building is already under construction here';
  if (drafted.some((d) => sameTile(d, t))) return 'Already in this turn\'s plan';
  return null;
}
