import { describe, expect, it } from 'vitest';
import { createMockSim } from '../src/sim-port/mock';
import { capitalGauge, circularFlow, financeEmpty, financeSections, fmtFin } from '../src/panels/finance';
import { energyBalance, energyText, intensityText, plantAt, plantRows, unitHelp, unitLabel } from '../src/panels/energy';
import { METRIC_LEDGER } from '../src/panels/metrics';
import { eventLedger } from '../src/panels/results';
import { fossilShareOf } from '../src/store';
import type { EnergyView, Snapshot } from '../src/contract';
import { ok } from './helpers';

const sim = createMockSim();
const start = ok(sim.init(4));
const played = ok(sim.submitTurn([])).snapshot;
const row = (s: Snapshot, sec: string, id: string) => financeSections(s).find((x) => x.id === sec)!.rows.find((r) => r.id === id)!;

describe('finance derivations', () => {
  it('has the six sections the design names', () => {
    expect(financeSections(played).map((s) => s.id)).toEqual(['government', 'bank', 'households', 'firms', 'money', 'prices']);
  });
  it('shows a monthly figure and the trailing quarter for flows, and none for stocks', () => {
    const wage = row(played, 'government', 'rev-wageTax');
    expect(wage.month).toBeGreaterThan(0);
    expect(wage.quarter).toBeGreaterThanOrEqual(wage.month);
    expect(wage.quarter).toBeCloseTo(wage.month * 3, -1); // three equal-ish months after one turn
    expect(row(played, 'government', 'gov-debt').stock).toBe(true);
    expect(row(played, 'bank', 'bank-cr').stock).toBe(true);
  });
  it('totals equal the sum of their parts', () => {
    const s = financeSections(played).find((x) => x.id === 'government')!;
    const rev = s.rows.filter((r) => r.id.startsWith('rev-')).reduce((a, r) => a + r.month, 0);
    expect(row(played, 'government', 'gov-in').month).toBeCloseTo(rev, 2);
    expect(row(played, 'government', 'gov-bal').month).toBeCloseTo(row(played, 'government', 'gov-in').month - row(played, 'government', 'gov-out').month, 2);
  });
  it('quarter is null (n/a) when the wire has no trailing-quarter figures', () => {
    const noQ = structuredClone(played);
    delete (noQ.finance as Partial<typeof noQ.finance>).quarter;
    expect(row(noQ, 'government', 'rev-wageTax').quarter).toBeNull();
    expect(circularFlow(noQ.finance, noQ.game.ticksPerTurn).approx).toBe(true);
    expect(circularFlow(played.finance, played.game.ticksPerTurn).approx).toBe(false);
  });
  it('ledger links: government revenue and spending rows open the government ledger with the right kind; the bank rows the bank', () => {
    expect(row(played, 'government', 'rev-wageTax').ledger).toEqual({ account: { kind: 'government' }, kind: 'tax' });
    expect(row(played, 'government', 'spend-benefits').ledger).toEqual({ account: { kind: 'government' }, kind: 'benefit' });
    expect(row(played, 'bank', 'bank-int').ledger).toEqual({ account: { kind: 'bank' }, kind: 'interest' });
    expect(METRIC_LEDGER.debt).toEqual({ account: { kind: 'government' }, kind: 'loan_new' });
    expect(financeSections(played).find((s) => s.id === 'households')!.ledgers).toHaveLength(played.households.length);
    expect(financeSections(played).find((s) => s.id === 'firms')!.ledgers!.length).toBe(played.firms.filter((f) => f.active).length);
  });
  it('sparklines come from history once there is some', () => {
    const more = ok(sim.submitTurn([])).snapshot;
    expect(row(more, 'government', 'gov-in').spark!.length).toBe(more.history.length);
    expect(row(more, 'bank', 'bank-cr').spark!.length).toBeGreaterThan(1);
  });
  it('the households quarter matches the household ledgers (flows come from the same bookings)', () => {
    const sim2 = createMockSim();
    ok(sim2.init(6));
    const s = ok(sim2.submitTurn([])).snapshot;
    let wages = 0;
    for (const h of s.households) for (const e of ok(sim2.ledger({ kind: 'household', id: h.id })).entries) if (e.kind === 'wage' && e.to.kind === 'household') wages += e.amount;
    expect(s.finance.quarter!.householdIncome.wages).toBeCloseTo(wages, 0);
  });
  it('formats and flags an empty first month', () => {
    expect(fmtFin(0.1234, 'pct')).toBe('12.3%');
    expect(fmtFin(1500, 'money')).toBe('$1.5k');
    expect(fmtFin(1.0234, 'index')).toBe('1.02');
    expect(financeEmpty(start.finance)).toBe(true);
    expect(financeEmpty(played.finance)).toBe(false);
  });
  it('the capital gauge compares the ratio with its limit', () => {
    const bank = played.finance.bank;
    expect(capitalGauge({ ...bank, capitalRatio: 0.2, capitalRatioLimit: 0.1 }).status).toBe('ok');
    expect(capitalGauge({ ...bank, capitalRatio: 0.11, capitalRatioLimit: 0.1 }).status).toBe('tight');
    const b = capitalGauge({ ...bank, capitalRatio: 0.05, capitalRatioLimit: 0.1 });
    expect(b.status).toBe('breach');
    expect(b.limitAt).toBeGreaterThan(b.at);
    expect(capitalGauge({ ...bank, capitalRatio: 0, capitalRatioLimit: 0 }).at).toBe(0);
  });
  it('the circular flow has only positive edges between the four sectors', () => {
    const { edges } = circularFlow(played.finance, played.game.ticksPerTurn);
    expect(edges.length).toBeGreaterThan(3);
    expect(edges.every((e) => e.amount > 0 && e.from !== e.to)).toBe(true);
    expect(edges.find((e) => e.from === 'households' && e.to === 'firms')!.amount).toBeCloseTo(played.finance.quarter!.householdSpending, 1);
  });
});

describe('events and metrics link into ledgers', () => {
  it('money events open the ledger they were booked in', () => {
    expect(eventLedger({ type: 'rent_shortfall', importance: 2, household: 3, amount: 10 })).toEqual({ kind: 'household', id: 3 });
    expect(eventLedger({ type: 'bank_bailout', importance: 3, amount: 10 })).toEqual({ kind: 'bank' });
    expect(eventLedger({ type: 'loan_defaulted', importance: 2, firm: 1 })).toEqual({ kind: 'firm', id: 1 });
    expect(eventLedger({ type: 'fire_out', importance: 1, tile: { x: 0, y: 0 } })).toBeNull();
  });
});

describe('energy formatting and the unit label', () => {
  const e = played.energy;
  it('labels the unit per month, keeps a unit that already has a rate, defaults when empty', () => {
    expect(unitLabel({ unit: 'MWh' })).toBe('MWh/month');
    expect(unitLabel({ unit: 'MWh/month' })).toBe('MWh/month');
    expect(unitLabel({ unit: '' })).toBe('MWh/month');
    expect(energyText(12.34, { unit: 'MWh' })).toBe('12.3 MWh/month');
    expect(unitHelp({ unit: 'MWh' })).toMatch(/MWh\/month.*megawatt-hours/);
    expect(intensityText({ ...e, intensity: 0.5, unit: 'MWh' })).toBe('0.50 t CO2 per MWh');
  });
  it('lists plants with capacity, share of the grid and emissions; shares add to 1', () => {
    const rows = plantRows(e);
    expect(rows).toHaveLength(e.supply.length);
    expect(rows.reduce((a, r) => a + r.outputShare, 0)).toBeCloseTo(1, 6);
    expect(rows.filter((r) => r.kind === 'fossil').every((r) => r.emissions > 0)).toBe(true);
    const first = rows[0]!;
    expect(plantAt(played, first.tile)).toEqual(first);
    expect(plantAt(played, { x: 0, y: 0 })).toBeUndefined();
  });
  it('warns when demand exceeds capacity and not otherwise', () => {
    const calm = energyBalance({ ...e, served: 1 });
    expect(calm.rationed).toBe(false);
    expect(calm.warning).toBeNull();
    const short: EnergyView = { ...e, served: 0.82 };
    const b = energyBalance(short);
    expect(b.rationed).toBe(true);
    expect(b.warning).toMatch(/82%/);
    expect(b.firmsShare + b.householdsShare).toBeCloseTo(1, 6);
  });
  it('the fossil share of capacity (used by the scene) follows the plants', () => {
    expect(fossilShareOf(e)).toBe(1); // two fossil plants, no renewables at the start
    expect(fossilShareOf({ supply: [] })).toBe(0);
    expect(fossilShareOf({ supply: [{ tile: { x: 0, y: 0 }, kind: 'fossil', capacity: 5 }, { tile: { x: 1, y: 0 }, kind: 'renewable', capacity: 5 }] })).toBe(0.5);
    expect(fossilShareOf(null)).toBe(0);
  });
});
