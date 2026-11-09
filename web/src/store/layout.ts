import raw from './layout.json';
import type { InfraKind } from '../contract';

export type SceneryKind = 'home' | 'firm' | 'bank' | 'government' | 'forest' | 'commons';
export interface Scenery { kind: SceneryKind; ref?: number; x: number; y: number; label: string }
export interface Placed { kind: InfraKind; x: number; y: number; effectiveTurn: number }
export interface Tile { x: number; y: number }

export interface MapLayout { gridSize: number; scenery: Scenery[]; placed: Placed[] }

export function initialLayout(): MapLayout {
  const l = raw as unknown as { gridSize: number; scenery: Scenery[]; placed: Placed[] };
  return { gridSize: l.gridSize, scenery: l.scenery.map((s) => ({ ...s })), placed: l.placed.map((p) => ({ ...p })) };
}

export const sameTile = (a: Tile, b: Tile) => a.x === b.x && a.y === b.y;

export function occupant(layout: MapLayout, t: Tile): Scenery | Placed | null {
  return layout.scenery.find((s) => sameTile(s, t)) ?? layout.placed.find((p) => sameTile(p, t)) ?? null;
}

export const inBounds = (layout: MapLayout, t: Tile) => t.x >= 0 && t.y >= 0 && t.x < layout.gridSize && t.y < layout.gridSize;

/** Why a tile cannot take a building, or null when it is valid. `drafted` are tiles already in the draft plan. */
export function tileBlock(layout: MapLayout, t: Tile, drafted: Tile[]): string | null {
  if (!inBounds(layout, t)) return 'Outside the map';
  const o = occupant(layout, t);
  if (o) return 'kind' in o && 'label' in o ? `Occupied by ${(o as Scenery).label}` : 'Already built here';
  if (drafted.some((d) => sameTile(d, t))) return 'Already in this turn\'s plan';
  return null;
}
