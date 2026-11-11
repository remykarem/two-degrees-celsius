import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { AccountRef, LedgerEntry, LedgerPage, Snapshot } from '../src/contract';
import { reconcile } from '../src/panels/ledger';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import { ok, reconciles } from './helpers';

// Plays 12 turns on the REAL simulation (the wasm package) and checks the Town v5 wire against the viewer's rules.
// Skipped when the package is not built (npm run build:wasm).
const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
const present = existsSync(resolve(pkg, 'sim_wasm_bg.wasm'));
const TURNS = 12;

const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
const same = (a: AccountRef, b: AccountRef) => JSON.stringify(a) === JSON.stringify(b);

describe.skipIf(!present)(`the real sim after ${TURNS} turns (wasm)`, async () => {
  const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  const port = createWasmPort(mod);
  const start = ok(port.init(1));
  let snap: Snapshot = start;
  for (let t = 0; t < TURNS; t++) snap = ok(port.submitTurn([])).snapshot;
  const accounts: AccountRef[] = [
    ...snap.households.map((h) => ({ kind: 'household' as const, id: h.id })), ...snap.firms.map((f) => ({ kind: 'firm' as const, id: f.id })), { kind: 'government' }, { kind: 'bank' },
  ];
  const pages = new Map<string, LedgerPage>(accounts.map((a) => [JSON.stringify(a), ok(port.ledger(a))]));
  const gov = pages.get(JSON.stringify({ kind: 'government' }))!;
  const latestTick = Math.max(...gov.entries.map((e) => e.tick));

  it('is schemaVersion 4 with the v1.4 blocks', () => {
    expect(snap.schemaVersion).toBe(5);
    expect(Array.isArray(snap.energy.supply)).toBe(true);
    expect(snap.finance.quarter.govRevenue).toBeDefined();
    expect(snap.finance.government.cash).toBeGreaterThan(0);
  });

  it('every household, firm, government and bank ledger reconciles by the viewer rule (balance and debt) and by the independent rule', () => {
    expect(pages.size).toBe(accounts.length);
    for (const [k, p] of pages) {
      expect(p.reconciles, `sim flag ${k}`).toBe(true);
      const r = reconcile(p);
      expect(r, `viewer rule ${k}`).toMatchObject({ ok: true, brokenEntries: 0 });
      expect(Math.abs(r.diff) + Math.abs(r.debtDiff), k).toBeLessThan(0.01);
      expect(reconciles(p), `independent rule ${k}`).toBe(true);
      expect(p.detailFromTurn).toBeGreaterThan(1); // older periods were rolled up after 12 turns
    }
    for (const a of [{ kind: 'government' }, { kind: 'bank' }, { kind: 'household', id: 0 }, { kind: 'firm', id: 0 }]) expect(pages.get(JSON.stringify(a))!.summary.length, JSON.stringify(a)).toBeGreaterThan(0);
  });

  it('the bank page shows credit creation and destruction as external accounts, with the bank as a bystander', () => {
    const bank = pages.get(JSON.stringify({ kind: 'bank' }))!;
    const loans = bank.entries.filter((e) => e.kind === 'loan_new' || e.kind === 'loan_repay');
    expect(loans.length).toBeGreaterThan(0);
    expect(loans.every((e) => e.from.kind === 'external' || e.to.kind === 'external')).toBe(true);
    expect(loans.every((e) => !same(e.from, { kind: 'bank' }) && !same(e.to, { kind: 'bank' }))).toBe(true);
  });

  /** Sum of the amounts of entries at the latest tick, over all the pages, counted once (from the government's and households' point of view as stated). */
  const govIn = (f: (e: LedgerEntry) => boolean) => sum(gov.entries.filter((e) => e.tick === latestTick && same(e.to, { kind: 'government' }) && f(e)).map((e) => e.amount));
  const govOut = (f: (e: LedgerEntry) => boolean) => sum(gov.entries.filter((e) => e.tick === latestTick && same(e.from, { kind: 'government' }) && f(e)).map((e) => e.amount));

  it('finance totals equal the sum of ledger entries by kind for the latest month', () => {
    const f = snap.finance;
    const tax = (d: string) => govIn((e) => e.kind === 'tax' && e.detail === d);
    expect(f.government.revenue.wageTax).toBeCloseTo(tax('wage tax'), 1);
    expect(f.government.revenue.profitTax).toBeCloseTo(tax('profit tax'), 1);
    expect(f.government.revenue.capitalTax).toBeCloseTo(tax('capital tax'), 1);
    expect(f.government.revenue.carbon).toBeCloseTo(tax('carbon price'), 1);
    expect(f.government.spending.benefits).toBeCloseTo(govOut((e) => e.kind === 'benefit' && e.detail !== 'ubi'), 1);
    expect(f.government.spending.publicSpending).toBeCloseTo(govOut((e) => e.kind === 'purchase'), 1);
    expect(f.government.spending.upkeep).toBeCloseTo(govOut((e) => e.kind === 'upkeep'), 1);
    // revenue is every transfer into the treasury that is not a loan
    expect(sum(Object.values(f.government.revenue))).toBeCloseTo(govIn((e) => e.kind !== 'loan_new'), 1);
    // households: wages are inflows of kinds wage, lobby, build, upkeep, subsidy, over all households; taxes and spending are their outflows
    const hh = snap.households.map((h) => pages.get(JSON.stringify({ kind: 'household', id: h.id }))!);
    const hhAt = (f2: (e: LedgerEntry, me: AccountRef) => boolean) => sum(hh.flatMap((p) => p.entries.filter((e) => e.tick === latestTick && f2(e, p.account)).map((e) => e.amount)));
    const into = (e: LedgerEntry, me: AccountRef) => same(e.to, me), outOf = (e: LedgerEntry, me: AccountRef) => same(e.from, me);
    expect(f.households.income.wages).toBeCloseTo(hhAt((e, me) => into(e, me) && ['wage', 'lobby', 'build', 'upkeep', 'subsidy'].includes(e.kind)), 1);
    expect(f.households.taxes).toBeCloseTo(hhAt((e, me) => outOf(e, me) && e.kind === 'tax'), 1);
    expect(f.households.spending).toBeCloseTo(hhAt((e, me) => outOf(e, me) && e.kind === 'purchase'), 1);
    expect(f.households.income.benefits).toBeCloseTo(hhAt((e, me) => into(e, me) && e.kind === 'benefit'), 1);
    // firms: revenue is money received for purchases. (The Factory buying capital goods from itself moves no money, so this can sit below the model's own firm revenue.)
    const firms = snap.firms.map((x) => pages.get(JSON.stringify({ kind: 'firm', id: x.id }))!);
    const firmRev = sum(firms.flatMap((p) => p.entries.filter((e) => e.tick === latestTick && e.kind === 'purchase' && same(e.to, p.account)).map((e) => e.amount)));
    expect(f.firms.revenue).toBeCloseTo(firmRev, 1);
    const wageBill = sum(firms.flatMap((p) => p.entries.filter((e) => e.tick === latestTick && e.kind === 'wage' && same(e.from, p.account)).map((e) => e.amount)));
    expect(f.firms.wageBill).toBeGreaterThan(0);
    expect(wageBill).toBeGreaterThanOrEqual(f.firms.wageBill - 0.01);
  });

  it('the trailing quarter is the sum of the last three months', () => {
    const q = snap.finance.quarter;
    const quarterTax = sum(gov.entries.filter((e) => e.tick > latestTick - 3 && same(e.to, { kind: 'government' }) && e.kind === 'tax' && e.detail === 'wage tax').map((e) => e.amount));
    expect(q.govRevenue.wageTax).toBeCloseTo(quarterTax, 1);
    expect(q.govRevenue.wageTax).toBeGreaterThan(snap.finance.government.revenue.wageTax);
  });

  it('energy supply and demand are consistent', () => {
    const e = snap.energy;
    expect(e.unit).toBeTruthy();
    expect(e.served).toBeGreaterThanOrEqual(0);
    expect(e.served).toBeLessThanOrEqual(1);
    expect(e.demand.total).toBeCloseTo(e.demand.firms + e.demand.households, 6);
    expect(e.demand.households).toBeGreaterThan(0);
    expect(e.demand.firms).toBeGreaterThan(0);
    expect(e.supply.length).toBe(snap.plants.length);
    expect(sum(e.supply.map((p) => p.capacity))).toBeGreaterThan(0);
    expect(e.renewableShare).toBeGreaterThanOrEqual(0);
    expect(e.renewableShare).toBeLessThanOrEqual(1);
    expect(e.emissionsByPlant.length).toBeLessThanOrEqual(e.supply.length);
  });

  it('a person ledger carries event turns and monthly older earnings', () => {
    for (const p of snap.people.slice(0, 6)) {
      const l = ok(port.personLedger(p.id));
      expect(l.eventTurns).toHaveLength(l.events.length);
      expect(l.household).toBe(p.household);
    }
    const worker = snap.people.find((p) => p.job !== null)!;
    const l = ok(port.personLedger(worker.id));
    expect(l.earnings.length + l.summary.length).toBeGreaterThan(0);
  });
});
