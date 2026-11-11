import { describe, expect, it } from 'vitest';
import type { AccountRef } from '../src/contract';
import { createMockSim } from '../src/sim-port/mock';
import type { SimPort } from '../src/sim-port/port';
import { incomeOf } from '../src/store/sources';
import { reconciles } from './helpers';

const ok = <T>(r: { ok: true; value: T } | { ok: false; error: { message: string } }): T => { if (!r.ok) throw new Error(r.error.message); return r.value; };

const accounts = (sim: SimPort): AccountRef[] => {
  const s = ok(sim.init(1));
  return [...s.households.map((h) => ({ kind: 'household' as const, id: h.id })), ...s.firms.map((f) => ({ kind: 'firm' as const, id: f.id })), { kind: 'government' }, { kind: 'bank' }];
};

describe('mock ledger', () => {
  it('every account reconciles after every turn, including after roll-up', () => {
    const sim = createMockSim();
    const accts = accounts(sim);
    for (let t = 0; t < 14; t++) {
      ok(sim.submitTurn([]));
      for (const a of accts) {
        const page = ok(sim.ledger(a));
        expect(page.reconciles, `${JSON.stringify(a)} turn ${t}`).toBe(true);
        expect(reconciles(page), `own rule ${JSON.stringify(a)} turn ${t}`).toBe(true);
      }
    }
  });
  it('keeps detail for the last 8 turns and rolls older periods up', () => {
    const sim = createMockSim();
    const s0 = ok(sim.init(3));
    for (let t = 0; t < 12; t++) ok(sim.submitTurn([]));
    const page = ok(sim.ledger({ kind: 'household', id: s0.households[0]!.id }));
    expect(page.entries.length).toBeGreaterThan(0);
    expect(Math.min(...page.entries.map((e) => e.turn))).toBeGreaterThanOrEqual(page.detailFromTurn);
    expect(page.summary.length).toBeGreaterThan(0);
    expect(Math.max(...page.summary.map((r) => r.turn))).toBeLessThan(page.detailFromTurn);
  });
  it('household closing balance equals its savings, running balances chain, and entries cover wages, purchases and tax', () => {
    const sim = createMockSim();
    ok(sim.init(5));
    const snap = ok(sim.submitTurn([])).snapshot;
    const hh = snap.households.find((h) => incomeOf(h.income.sources, 'wages') > 0)!;
    const page = ok(sim.ledger({ kind: 'household', id: hh.id }));
    expect(page.closingBalance).toBeCloseTo(hh.savings, 1);
    const kinds = new Set(page.entries.map((e) => e.kind));
    expect(kinds.has('wage')).toBe(true);
    expect(kinds.has('purchase')).toBe(true);
    expect(kinds.has('tax')).toBe(true);
    expect(page.entries[page.entries.length - 1]!.balanceAfter).toBeCloseTo(page.closingBalance, 1);
    let bal = page.openingBalance;
    for (const e of page.entries) {
      bal += e.to.kind === 'household' && e.to.id === hh.id ? e.amount : -e.amount;
      expect(e.balanceAfter).toBeCloseTo(bal, 1);
    }
  });
  it('a person has earnings and employment events, and filters work', () => {
    const sim = createMockSim();
    const s = ok(sim.init(2));
    ok(sim.submitTurn([]));
    const worker = s.people.find((p) => p.job)!;
    const pl = ok(sim.personLedger(worker.id));
    expect(pl.household).toBe(worker.household);
    expect(pl.earnings.length).toBeGreaterThan(0);
    expect(pl.earnings.every((e) => e.kind === 'wage')).toBe(true);
    const tax = ok(sim.ledger({ kind: 'government' }, { kind: 'tax' }));
    expect(tax.entries.length).toBeGreaterThan(0);
    expect(tax.entries.every((e) => e.kind === 'tax')).toBe(true);
    expect(sim.ledger({ kind: 'external', label: 'x' }).ok).toBe(false);
    expect(sim.personLedger(999).ok).toBe(false);
  });
  it('save and load keeps the ledger; the snapshot carries finance and energy', () => {
    const sim = createMockSim();
    ok(sim.init(4));
    const snap = ok(sim.submitTurn([])).snapshot;
    expect(snap.finance.government.revenue.wageTax).toBeGreaterThan(0);
    expect(snap.finance.quarter).toBeDefined();
    expect(snap.energy.unit).toBe('MWh');
    expect(snap.energy.supply.length).toBeGreaterThan(0);
    const blob = ok(sim.exportState());
    const other = createMockSim();
    ok(other.importState(blob));
    expect(ok(other.ledger({ kind: 'government' })).closingBalance).toBe(ok(sim.ledger({ kind: 'government' })).closingBalance);
  });
});
