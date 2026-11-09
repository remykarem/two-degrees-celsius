/** mulberry32. State is a single uint32 so it can be exported with the game state. */
export interface Rng { state: number }

export function nextFloat(r: Rng): number {
  r.state = (r.state + 0x6d2b79f5) >>> 0;
  let t = r.state;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

/** Symmetric noise in [-amp, amp]. */
export function noise(r: Rng, amp: number): number {
  return (nextFloat(r) * 2 - 1) * amp;
}

export function pickInt(r: Rng, n: number): number {
  return Math.floor(nextFloat(r) * n);
}
