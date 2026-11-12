import { describe, expect, it } from 'vitest';
import { cornColours, cornLook, easeStock, slotScale, stalkCount, stalkSlots, SLOTS, STALK_HEIGHT, STUBBLE_HEIGHT } from '../src/scene/corn';

const stocks = Array.from({ length: 21 }, (_, i) => i / 20);

describe('corn look from the commons stock', () => {
  it('is dense, tall and cobbed when healthy, and stubble when collapsed', () => {
    const full = cornLook(0.9), bare = cornLook(0);
    expect(full.density).toBe(1); expect(full.height).toBe(1); expect(full.cob).toBe(1);
    expect(bare.density).toBeLessThan(0.15); expect(bare.height).toBeCloseTo(STUBBLE_HEIGHT, 5); expect(bare.cob).toBe(0);
    expect(stalkCount(1)).toBe(SLOTS);
    expect(stalkCount(0)).toBeGreaterThan(0);
  });
  it('never gives more stalks, height, cobs or green when the stock falls', () => {
    for (let i = 1; i < stocks.length; i++) {
      const a = cornLook(stocks[i - 1]!), b = cornLook(stocks[i]!);
      expect(b.density).toBeGreaterThanOrEqual(a.density); expect(b.height).toBeGreaterThanOrEqual(a.height); expect(b.cob).toBeGreaterThanOrEqual(a.cob);
      expect(stalkCount(stocks[i]!)).toBeGreaterThanOrEqual(stalkCount(stocks[i - 1]!));
    }
  });
  it('survives odd input', () => { expect(cornLook(NaN).density).toBeGreaterThan(0); expect(cornLook(-3).height).toBeCloseTo(STUBBLE_HEIGHT, 5); expect(cornLook(7).density).toBe(1); });
  it('goes green to yellow to brown, drier under drought, and blends the lens colour', () => {
    const g = (c: number) => (c >> 8) & 255, r = (c: number) => (c >> 16) & 255;
    const hi = cornColours(0.9).stalk, mid = cornColours(0.4).stalk, lo = cornColours(0).stalk;
    expect(g(hi)).toBeGreaterThan(r(hi)); expect(r(mid)).toBeGreaterThan(r(hi)); expect(r(lo)).toBeLessThan(r(mid));
    expect(cornColours(0.9, { dry: 1 }).stalk).not.toBe(hi);
    expect(cornColours(0.9, { lensTint: 0x808080 }).stalk).not.toBe(hi);
    expect(cornColours(0.9, { dry: 0 }).stalk).toBe(hi);
  });
});

describe('stalk slots', () => {
  it('are deterministic per tile and differ between tiles', () => {
    expect(stalkSlots(3, 4)).toEqual(stalkSlots(3, 4));
    expect(stalkSlots(3, 4)).not.toEqual(stalkSlots(4, 3));
    expect(stalkSlots(3, 4)).toHaveLength(SLOTS);
  });
  it('stay inside the tile and have evenly spread ranks', () => {
    const s = stalkSlots(5, 6);
    for (const q of s) { expect(Math.abs(q.x)).toBeLessThan(0.34); expect(Math.abs(q.z)).toBeLessThan(0.34); }
    expect([...s.map((q) => q.rank)].sort((a, b) => a - b).map((v, i) => Math.abs(v - (i + 0.5) / SLOTS)).every((d) => d < 1e-9)).toBe(true);
  });
  it('shows exactly the counted stalks and a thinning field is a subset of the fuller one', () => {
    const s = stalkSlots(2, 2);
    const shown = (st: number) => s.map((q, i) => (slotScale(q, cornLook(st)).stalk > 0 ? i : -1)).filter((i) => i >= 0);
    expect(shown(0.8)).toHaveLength(SLOTS);
    for (const st of [0.6, 0.3, 0.1]) expect(Math.abs(shown(st).length - stalkCount(st))).toBeLessThanOrEqual(1);
    const lo = new Set(shown(0.3));
    expect(shown(0.6).filter((i) => lo.has(i))).toHaveLength(lo.size);
    for (const q of s) expect(slotScale(q, cornLook(0.8)).stalk).toBeLessThanOrEqual(STALK_HEIGHT * 1.15 + 1e-9);
  });
});

describe('easing', () => {
  it('moves toward the target and settles exactly', () => {
    let v = 0.8; for (let i = 0; i < 400; i++) v = easeStock(v, 0.3, 0.016);
    expect(v).toBe(0.3);
    expect(easeStock(0.8, 0.3, 0.016)).toBeLessThan(0.8);
    expect(easeStock(0.8, 0.3, 0)).toBe(0.8);
  });
});
