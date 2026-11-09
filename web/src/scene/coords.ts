import type { Tile } from '../store';

export type Pt = [number, number];

export const worldOf = (t: Tile, size: number, y = 0): [number, number, number] => [t.x - (size - 1) / 2, y, t.y - (size - 1) / 2];
export const centreOf = (t: Tile, size: number): Pt => [t.x - (size - 1) / 2, t.y - (size - 1) / 2];
export const tileKey = (t: Tile) => `${t.x},${t.y}`;

export function tileOf(wx: number, wz: number, size: number): Tile | null {
  const x = Math.floor(wx + size / 2);
  const y = Math.floor(wz + size / 2);
  return x >= 0 && y >= 0 && x < size && y < size ? { x, y } : null;
}

/** Deterministic hash in [0,1) so animation never needs Math.random (reproducible, testable). */
export function hash01(n: number): number {
  let x = Math.imul(n | 0, 0x9e3779b1) ^ 0x85ebca6b;
  x = Math.imul(x ^ (x >>> 15), 0x2c1b3c6d);
  x = Math.imul(x ^ (x >>> 12), 0x297a2d39);
  return ((x ^ (x >>> 15)) >>> 0) / 4294967296;
}
