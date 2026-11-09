import type { Tile } from '../store';

export const worldOf = (t: Tile, size: number, y = 0): [number, number, number] => [t.x - (size - 1) / 2, y, t.y - (size - 1) / 2];

export function tileOf(wx: number, wz: number, size: number): Tile | null {
  const x = Math.floor(wx + size / 2);
  const y = Math.floor(wz + size / 2);
  return x >= 0 && y >= 0 && x < size && y < size ? { x, y } : null;
}
