import type { ForestState, RiskLevel, Snapshot, TileRef } from '../contract';
import { tileKey } from './map';
import { forestStateOf, weatherOf } from './weather';

/**
 * The weather as the 3D scene needs it: plain data derived from the snapshot (memoised per snapshot object). No Three.js here.
 * The scene turns it into meshes and colours (scene/weatherLook.ts, scene/weather.ts).
 */
export interface WeatherLook {
  /** 0..1, the active drought's severity (0 when none). */
  drought: number;
  stress: number;
  fireRisk: RiskLevel;
  fires: { tile: TileRef; monthsLeft: number }[];
  regrowing: TileRef[];
  stations: TileRef[];
  /** Tiles a fire station covers and tiles smoke reaches from a fire (the wire's weather.coverageRadius, smokeRadius). */
  coverageRadius: number;
  smokeRadius: number;
  /** The simulation's odds that a fire reaches each standing forest tile next month (tile.fireRisk), keyed "x,y", and the largest of them (for colour scaling). */
  risk: Map<string, number>;
  maxRisk: number;
  /** State of every forest tile, keyed "x,y". */
  forest: Map<string, ForestState>;
}

const EMPTY: WeatherLook = { drought: 0, stress: 0, fireRisk: 'low', fires: [], regrowing: [], stations: [], coverageRadius: 0, smokeRadius: 0, risk: new Map(), maxRisk: 0, forest: new Map() };
const memo = new WeakMap<Snapshot, WeatherLook>();

export function weatherLook(snap: Snapshot | null): WeatherLook {
  if (!snap) return EMPTY;
  const hit = memo.get(snap);
  if (hit) return hit;
  const w = weatherOf(snap);
  const out: WeatherLook = { drought: w?.drought?.severity ?? 0, stress: w?.stress ?? 0, fireRisk: w?.forecast.fireRisk ?? 'low', fires: w ? w.fires.map((f) => ({ tile: { ...f.tile }, monthsLeft: f.monthsLeft })) : [], regrowing: [], stations: [], coverageRadius: w?.coverageRadius ?? 0, smokeRadius: w?.smokeRadius ?? 0, risk: new Map(), maxRisk: 0, forest: new Map() };
  for (const t of snap.map.tiles) {
    if (t.kind === 'forest') {
      const st = forestStateOf(snap, t) ?? 'standing';
      out.forest.set(tileKey(t), st);
      if (st === 'standing' && typeof t.fireRisk === 'number') { out.risk.set(tileKey(t), t.fireRisk); out.maxRisk = Math.max(out.maxRisk, t.fireRisk); }
      if (st === 'regrowing') out.regrowing.push({ x: t.x, y: t.y });
      // A burning tile on the map without a matching entry in weather.fires (a package inconsistency) still burns.
      if (st === 'burning' && !out.fires.some((f) => f.tile.x === t.x && f.tile.y === t.y)) out.fires.push({ tile: { x: t.x, y: t.y }, monthsLeft: 1 });
    } else if (t.kind === 'fire_station') out.stations.push({ x: t.x, y: t.y });
  }
  memo.set(snap, out);
  return out;
}

/**
 * Risk of one tile for the "fire and drought risk" lens, 0..1 for colouring: 1 where a fire burns, the simulation's own odds (tile.fireRisk, from its ignition and
 * spread rules) divided by the riskiest tile's odds elsewhere (chart scaling only), 0 for burnt or non-forest tiles.
 */
export function tileFireRisk(look: WeatherLook, t: TileRef): number {
  const st = look.forest.get(tileKey(t));
  if (st === 'burning') return 1;
  if (st !== 'standing' || look.maxRisk <= 0) return 0;
  return (look.risk.get(tileKey(t)) ?? 0) / look.maxRisk;
}
export const isCovered = (look: WeatherLook, t: TileRef): boolean => look.stations.some((s) => Math.hypot(s.x - t.x, s.y - t.y) <= look.coverageRadius + 1e-9);
/** Tiles within the smoke radius of any fire. */
export const nearFire = (look: WeatherLook, t: TileRef): boolean => look.fires.some((f) => Math.hypot(f.tile.x - t.x, f.tile.y - t.y) <= look.smokeRadius + 1e-9);
