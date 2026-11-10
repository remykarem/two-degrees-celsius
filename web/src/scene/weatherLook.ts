import type { ForestState } from '../contract';
import { tileKey } from '../store/map';
import type { WeatherLook } from '../store/scene-weather';
import { tileFireRisk, isCovered } from '../store/scene-weather';
import type { ModelId } from './models';
import { mix } from './palette';

/** Pure weather looks (no Three.js): which mesh a forest tile gets, how dry the ground looks, colours for the fire and drought lens. */

export const DRY_GROUND = 0xd9c27a;
export const EMBER_GROUND = 0x3b2b24;
export const BURNT_GROUND = 0x6a5a45;
export const COVER_GROUND = 0xb9d3e8;
/** Ramp for the "fire and drought risk" lens: low (green) through amber to high (deep red). */
export const FIRE_RAMP = [0xcfe8b8, 0xf2d27a, 0xe58a3a, 0xb3261e] as const;

/** Forest tile state to mesh. `standing` keeps the three varied tree models. */
export function forestModelFor(state: ForestState | null | undefined, x: number, y: number): ModelId {
  if (state === 'burning') return 'forest_burning';
  if (state === 'regrowing') return 'forest_regrowing';
  return `forest_${(x * 7 + y * 3) % 3}` as ModelId;
}

export interface DroughtLook {
  /** How far the ground colour moves toward dry straw (0..1). */
  ground: number;
  /** Fog (haze) range in camera depth units, or null for none. Nearer and thicker as severity rises. */
  fog: { near: number; far: number; color: number } | null;
  /** Sunlight colour: warmer and dimmer as it gets drier. */
  sun: number;
  sunIntensity: number;
}
/** Everything scales smoothly with severity (0..1); 0 means no change at all. */
export function droughtLook(severity: number): DroughtLook {
  const s = Math.min(1, Math.max(0, Number.isFinite(severity) ? severity : 0));
  return {
    ground: 0.5 * s,
    fog: s > 0.02 ? { near: 22 - 6 * s, far: 44 - 4 * s, color: mix(0xf6f0e3, 0xe3cf94, s) } : null,
    sun: mix(0xfff0d8, 0xffd9a0, s),
    sunIntensity: 1.9 - 0.35 * s,
  };
}

/** Ground colour of one tile: dry straw under a drought, embers under a fire, burnt brown while regrowing. */
export function weatherGround(look: WeatherLook, x: number, y: number, base: number): number {
  let c = base;
  const st = look.forest.get(tileKey({ x, y }));
  if (look.drought > 0) c = mix(c, DRY_GROUND, droughtLook(look.drought).ground);
  if (st === 'burning') c = mix(c, EMBER_GROUND, 0.85);
  else if (st === 'regrowing') c = mix(c, BURNT_GROUND, 0.7);
  else if (look.fires.some((f) => Math.abs(f.tile.x - x) <= 1 && Math.abs(f.tile.y - y) <= 1)) c = mix(c, EMBER_GROUND, 0.18); // scorched edge
  return c;
}

/** "Fire and drought risk" lens: ground and forest are coloured by ignition risk; tiles under a fire station get a cool tint. */
export function fireLensGround(look: WeatherLook, x: number, y: number, base: number): number {
  const t = { x, y };
  const st = look.forest.get(tileKey(t));
  if (st) return mix(base, rampFire(tileFireRisk(look, t)), 0.85);
  const dry = look.drought > 0 ? mix(base, DRY_GROUND, 0.4 * look.drought) : base;
  return isCovered(look, t) ? mix(dry, COVER_GROUND, 0.55) : dry;
}
export function rampFire(t: number): number {
  const c = Math.min(1, Math.max(0, t)) * (FIRE_RAMP.length - 1);
  const i = Math.min(FIRE_RAMP.length - 2, Math.floor(c));
  return mix(FIRE_RAMP[i]!, FIRE_RAMP[i + 1]!, c - i);
}
