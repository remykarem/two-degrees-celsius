// Emission and income sources are lists (contract v2.0): panels iterate them, totals reconcile, and a source the web has never heard of needs no web change.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { Snapshot } from '../src/contract';
import { emissionsView } from '../src/panels/emissions';
import { financeSections } from '../src/panels/finance';
import { householdIncome } from '../src/panels/people';
import { emissionOf, incomeOf, incomeTotal } from '../src/store/sources';
import { createGame } from '../src/store';
import { createSimPort } from '../src/sim-port';

const real = (): Snapshot => JSON.parse(readFileSync(resolve(__dirname, '../../docs/fixtures/snapshot-turn1.json'), 'utf8')) as Snapshot;
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
// The emissions panel reads the catalog for labels of ids only in why-bars; load it so the shared helpers have what they need.
createGame(createSimPort());

describe('sources on the wire', () => {
  it('household income is a list whose total is the sum of non-part entries', () => {
    const h = real().households[0]!;
    expect(h.income.sources.map((s) => s.id)).toEqual(['wages', 'dividends', 'rent', 'interest', 'benefits']);
    expect(householdIncome(h)).toBeCloseTo(h.income.sources.reduce((a, s) => a + s.amount, 0), 9);
    expect(incomeOf(h.income.sources, 'nope')).toBe(0);
  });
  it('a part-of entry (domestic wages) is not added twice', () => {
    const list = [
      { id: 'wages', label: 'Wages', category: 'labour' as const, amount: 100 },
      { id: 'domestic_wages', label: 'Domestic wages', category: 'labour' as const, partOf: 'wages', amount: 40 },
    ];
    expect(incomeTotal(list)).toBe(100);
  });
  it('the emissions table has one row per listed source and reconciles', () => {
    const e = real().breakdowns!.emissions;
    const v = emissionsView(e);
    const rows = v.rows.filter((r) => r.kind === 'source').map((r) => r.id);
    expect(rows).toEqual(expect.arrayContaining(e.month.sources.map((s) => s.id)));
    expect(v.reconciliation.ok).toBe(true);
  });
  it('a new emission source appears without any web change and still reconciles', () => {
    const e = clone(real().breakdowns!.emissions);
    for (const l of [e.month, e.turn]) { l.sources.push({ id: 'landfill', label: 'Landfill methane', category: 'waste', tonnes: 3 }); l.grossTotal += 3; l.net += 3; l.headline += 3; }
    const v = emissionsView(e);
    expect(v.rows.find((r) => r.id === 'landfill')?.label).toBe('Landfill methane');
    expect(v.reconciliation.ok).toBe(true);
    expect(emissionOf(e.month, 'landfill')).toBe(3);
  });
  it('a list that does not add up to the gross total is flagged by the table, not hidden', () => {
    const e = clone(real().breakdowns!.emissions);
    e.month.sources[0]!.tonnes += 5;
    expect(emissionsView(e).reconciliation.ok).toBe(false);
  });
  it('finance shows one income row per source, domestic wages as a memo', () => {
    const snap = clone(real());
    for (const x of snap.finance.households.income) x.amount = x.id === 'domestic_wages' ? 20 : 100;
    const rows = financeSections(snap).flatMap((s) => s.rows).filter((r) => r.id.startsWith('inc-'));
    expect(rows.map((r) => r.id)).toEqual(['inc-wages', 'inc-domestic_wages', 'inc-dividends', 'inc-rent', 'inc-interest', 'inc-benefits']);
    expect(rows[1]!.label).toMatch(/^of which /);
  });
});
