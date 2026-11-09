import { describe, expect, it } from 'vitest';
import { compact, formatLever, money, pct, signed, tonnes, trendOf } from '../src/panels/format';
import type { LeverInfo } from '../src/contract';

const lever = (p: Partial<LeverInfo>): LeverInfo => ({ key: 'tax_wage', label: 'x', unit: 'fraction', min: 0, max: 1, step: 0.01, baseCostPc: 1, leadTime: 1, lobbied: false, arrows: [], availableIn: ['market'], ...p });

describe('number formatting', () => {
  it('compacts magnitudes', () => {
    expect(compact(0)).toBe('0');
    expect(compact(12.5)).toBe('12.5');
    expect(compact(950)).toBe('950');
    expect(compact(1234)).toBe('1.2k');
    expect(compact(56000)).toBe('56k');
    expect(compact(2_500_000)).toBe('2.5M');
    expect(compact(5_000_000)).toBe('5M');
  });
  it('formats money, percent, tonnes and signs', () => {
    expect(money(-1500)).toBe('-$1.5k');
    expect(pct(0.256, 1)).toBe('25.6%');
    expect(tonnes(1200)).toBe('1.2k t');
    expect(signed(5, (v) => String(v))).toBe('+5');
    expect(signed(-5, (v) => String(v))).toBe('-5');
  });
  it('trendOf ignores tiny moves', () => {
    expect(trendOf(100, 100.01)).toBe('flat');
    expect(trendOf(100, 110)).toBe('up');
    expect(trendOf(0.5, 0.4)).toBe('down');
  });
  it('formats levers by unit', () => {
    expect(formatLever(lever({ unit: 'fraction' }), 0.25)).toBe('25%');
    expect(formatLever(lever({ unit: 'usd_per_tonne' }), 50)).toBe('$50/t');
    expect(formatLever(lever({ unit: 'tonnes' }), 0)).toBe('no cap');
    expect(formatLever(lever({ unit: 'bool' }), true)).toBe('on');
    expect(formatLever(lever({ unit: 'enum', options: ['open', 'community'] }), 1)).toBe('community');
    expect(formatLever(lever({ unit: 'usd' }), 12.5)).toBe('$12.5');
  });
});
