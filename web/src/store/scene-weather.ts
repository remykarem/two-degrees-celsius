import type { ForestState, RiskLevel, Snapshot, TileRef } from '../contract';
import { tileKey } from './map';
import { forestStateOf, weatherOf, WEATHER_UI } from './weather';

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
  /** State of every forest tile, keyed "x,y". */
  forest: Map<string, ForestState>;
}

const EMPTY: WeatherLook = { drought: 0, stress: 0, fireRisk: 'low', fires: [], regrowing: [], stations: [], forest: new Map() };
const memo = new WeakMap<Snapshot, WeatherLook>();

export function weatherLook(snap: Snapshot | null): WeatherLook {
  if (!snap) return EMPTY;
  const hit = memo.get(snap);
  if (hit) return hit;
  const w = weatherOf(snap);
  const out: WeatherLook = { drought: w?.drought?.severity ?? 0, stress: w?.stress ?? 0, fireRisk: w?.forecast.fireRisk ?? 'low', fires: w ? w.fires.map((f) => ({ tile: { ...f.tile }, monthsLeft: f.monthsLeft })) : [], regrowing: [], stations: [], forest: new Map() };
  for (const t of snap.map.tiles) {
    if (t.kind === 'forest') {
      const st = forestStateOf(snap, t) ?? 'standing';
      out.forest.set(tileKey(t), st);
      if (st === 'regrowing') out.regrowing.push({ x: t.x, y: t.y });
      // A burning tile on the map without a matching entry in weather.fires (a package inconsistency) still burns.
      if (st === 'burning' && !out.fires.some((f) => f.tile.x === t.x && f.tile.y === t.y)) out.fires.push({ tile: { x: t.x, y: t.y }, monthsLeft: 1 });
    } else if (t.kind === 'fire_station') out.stations.push({ x: t.x, y: t.y });
  }
  memo.set(snap, out);
  return out;
}

/** Ignition risk of one tile, 0..1 (for the "fire and drought risk" lens): forecast level x fuel nearby x drought, halved under a fire station. */
export function tileFireRisk(look: WeatherLook, t: TileRef): number {
  const st = look.forest.get(tileKey(t));
  if (st === 'burning') return 1;
  if (st === 'regrowing') return 0.08;
  if (st !== 'standing') return 0;
  const level = look.fireRisk === 'high' ? 0.85 : look.fireRisk === 'medium' ? 0.55 : look.stress > 0 ? 0.25 : 0.1;
  let fuel = 0;
  for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (look.forest.get(tileKey({ x: t.x + dx, y: t.y + dy })) === 'standing') fuel++;
  const near = look.fires.some((f) => Math.abs(f.tile.x - t.x) <= 1 && Math.abs(f.tile.y - t.y) <= 1);
  return Math.min(1, level * (0.55 + 0.45 * (fuel / 9)) * (0.6 + 0.4 * look.drought + (near ? 0.4 : 0)) * (isCovered(look, t) ? 0.5 : 1));
}
export const isCovered = (look: WeatherLook, t: TileRef): boolean => look.stations.some((s) => Math.hypot(s.x - t.x, s.y - t.y) <= WEATHER_UI.coverageRadius + 1e-9);
/** Tiles within the smoke radius of any fire. */
export const nearFire = (look: WeatherLook, t: TileRef): boolean => look.fires.some((f) => Math.hypot(f.tile.x - t.x, f.tile.y - t.y) <= WEATHER_UI.smokeRadius + 1e-9);
