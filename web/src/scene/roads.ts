import type * as THREE from 'three';
import type { Tile } from '../store';
import { mergeParts } from './geometry';
import type { Part } from './models';
import { ASPHALT, ROAD_LINE } from './palette';

/** Which of the four neighbours hold a road. n = y-1, e = x+1, s = y+1, w = x-1. Pure. */
export interface RoadMask { n: boolean; e: boolean; s: boolean; w: boolean }
export type RoadPiece = 'isolated' | 'end' | 'straight' | 'corner' | 't' | 'cross';

export function roadMask(roads: ReadonlySet<string>, t: Tile): RoadMask {
  const has = (x: number, y: number) => roads.has(`${x},${y}`);
  return { n: has(t.x, t.y - 1), e: has(t.x + 1, t.y), s: has(t.x, t.y + 1), w: has(t.x - 1, t.y) };
}

const ARMS: (keyof RoadMask)[] = ['n', 'e', 's', 'w'];

/**
 * Classify a mask. `rot` is in quarter turns: end = the arm that exists; straight 0 = north-south, 1 = east-west;
 * corner = the first arm of the clockwise pair (n+e 0, e+s 1, s+w 2, w+n 3); t = the stem (the arm opposite the missing one).
 */
export function roadPiece(m: RoadMask): { piece: RoadPiece; rot: number } {
  const on = ARMS.map((k) => m[k]);
  const count = on.filter(Boolean).length;
  switch (count) {
    case 0: return { piece: 'isolated', rot: 0 };
    case 1: return { piece: 'end', rot: on.indexOf(true) };
    case 2:
      if (on[0] && on[2]) return { piece: 'straight', rot: 0 };
      if (on[1] && on[3]) return { piece: 'straight', rot: 1 };
      for (let i = 0; i < 4; i++) if (on[i] && on[(i + 1) % 4]) return { piece: 'corner', rot: i };
      return { piece: 'isolated', rot: 0 };
    case 3: return { piece: 't', rot: (on.indexOf(false) + 2) % 4 };
    default: return { piece: 'cross', rot: 0 };
  }
}

const DIR: [number, number][] = [[0, -1], [1, 0], [0, 1], [-1, 0]];
const W = 0.62;
const box = (x: number, y: number, z: number, w: number, h: number, d: number, c: number): Part => ({ s: 'box', p: [x, y, z], z: [w, h, d], c });

/** Parts (world space) for a set of road tiles: a pad plus one arm per connected neighbour, with a centre line. Pure data. */
export function roadParts(tiles: readonly Tile[], connected: ReadonlySet<string>, size: number): Part[] {
  const out: Part[] = [];
  const half = (size - 1) / 2;
  for (const t of tiles) {
    const cx = t.x - half, cz = t.y - half;
    const mask = roadMask(connected, t);
    const { piece } = roadPiece(mask);
    out.push(box(cx, 0, cz, W, 0.045, W, ASPHALT));
    ARMS.forEach((k, i) => {
      if (!mask[k]) return;
      const [dx, dz] = DIR[i]!;
      const len = 0.5 - W / 2 + 0.02;
      const off = W / 2 + len / 2 - 0.01;
      out.push(box(cx + dx * off, 0, cz + dz * off, dx === 0 ? W : len, 0.045, dz === 0 ? W : len, ASPHALT));
      // dashed centre line on the arm
      out.push(box(cx + dx * (W / 2 + 0.06), 0.045, cz + dz * (W / 2 + 0.06), dx === 0 ? 0.035 : 0.1, 0.008, dz === 0 ? 0.035 : 0.1, ROAD_LINE));
    });
    if (piece === 'isolated' || piece === 'end') out.push(box(cx, 0.045, cz, 0.18, 0.008, 0.18, ROAD_LINE));
  }
  return out;
}

export const roadGeometry = (tiles: readonly Tile[], connected: ReadonlySet<string>, size: number): THREE.BufferGeometry | null =>
  tiles.length ? mergeParts(roadParts(tiles, connected, size)) : null;
