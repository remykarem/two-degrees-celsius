import { describe, expect, it } from 'vitest';
import type { AccountRef, LedgerEntry, LedgerPage, Snapshot } from '../src/contract';
import { createSimPort } from './port';
import {
  accountLabel, accountSelection, counterpartiesIn, earningsRows, entryEffect, externalHint, turnSpan, exact, kindsIn, ledgerRows, monthLabel, NO_FILTER, paginate, reconcile, rollupGroups, rollupTotals, rowTotals, selectionAccount, signedExact, turnsIn,
} from '../src/panels/ledger';
import { ok, reconciles as sameRule } from './helpers';

const HH: AccountRef = { kind: 'household', id: 2 };
const FIRM: AccountRef = { kind: 'firm', id: 0 };
const GOV: AccountRef = { kind: 'government' };
const BANK: AccountRef = { kind: 'bank' };
const e = (turn: number, month: 0 | 1 | 2, from: AccountRef, to: AccountRef, amount: number, kind: LedgerEntry['kind'], balanceAfter: number, detail = ''): LedgerEntry =>
  ({ turn, month, tick: turn * 3 + month, from, to, amount, kind, memo: `${kind} ${amount}`, balanceAfter, debtAfter: 0, ...(detail ? { detail } : {}) });

/** opening 100; +50 wage, -20 tax, -10 purchase; rolled-up older: +200 wages, -60 purchase. Closing = 100 + 20 + 140 = 260. */
const page: LedgerPage = {
  account: HH, openingBalance: 100, openingDebt: 0, closingBalance: 260, closingDebt: 0, reconciles: true, detailFromTurn: 9,
  entries: [
    e(9, 0, FIRM, HH, 50, 'wage', 290, 'Adult 1'),
    e(9, 1, HH, GOV, 20, 'tax', 270, 'income tax'),
    e(9, 2, HH, FIRM, 10, 'purchase', 260, 'goods'),
  ],
  summary: [
    { turn: 2, month: 0, counterparty: FIRM, kind: 'wage', inflow: 120, outflow: 0, count: 1, net: 120, debtNet: 0 },
    { turn: 3, month: 1, counterparty: FIRM, kind: 'wage', inflow: 80, outflow: 0, count: 1, net: 80, debtNet: 0 },
    { turn: 3, month: 1, counterparty: FIRM, kind: 'purchase', inflow: 0, outflow: 60, count: 2, net: -60, debtNet: 0 },
  ],
};
const snap = ok(createSimPort().init(1));

describe('ledger viewer: rows, filters, running balance', () => {
  it('shows entries newest first, signed from the open account, with the running balance of the entry', () => {
    const rows = ledgerRows(page, null);
    expect(rows.map((r) => r.kind)).toEqual(['purchase', 'tax', 'wage']);
    expect(rows.map((r) => r.amount)).toEqual([-10, -20, 50]);
    expect(rows.map((r) => r.amountText)).toEqual(['-$10.00', '-$20.00', '+$50.00']);
    expect(rows.map((r) => r.balanceAfter)).toEqual([260, 270, 290]);
    expect(rows[2]!.counterparty).toEqual(FIRM);
    expect(rows[1]!.counterparty).toEqual(GOV);
  });
  it('running balance chains: each balance is the one after it plus the change', () => {
    const rows = ledgerRows(page, null);
    for (let i = 0; i < rows.length - 1; i++) expect(rows[i + 1]!.balanceAfter + rows[i]!.amount).toBeCloseTo(rows[i]!.balanceAfter, 6);
    expect(rows[0]!.balanceAfter).toBe(page.closingBalance);
    expect(rows.at(-1)!.balanceAfter - rows.at(-1)!.amount).toBe(page.openingBalance + 0 + page.summary.reduce((a, r) => a + r.inflow - r.outflow, 0));
  });
  it('filters by kind, period and counterparty, and they combine', () => {
    expect(ledgerRows(page, null, { ...NO_FILTER, kind: 'tax' })).toHaveLength(1);
    expect(ledgerRows(page, null, { ...NO_FILTER, fromTurn: 10 })).toHaveLength(0);
    expect(ledgerRows(page, null, { ...NO_FILTER, toTurn: 8 })).toHaveLength(0);
    expect(ledgerRows(page, null, { ...NO_FILTER, counterparty: FIRM })).toHaveLength(2);
    expect(ledgerRows(page, null, { kind: 'wage', fromTurn: 9, toTurn: 9, counterparty: FIRM })).toHaveLength(1);
    expect(ledgerRows(page, null, { ...NO_FILTER, kind: 'wage', counterparty: GOV })).toHaveLength(0);
  });
  it('totals of the shown rows, and the filter option lists', () => {
    expect(rowTotals(ledgerRows(page, null))).toEqual({ inflow: 50, outflow: 30, net: 20, count: 3 });
    expect(kindsIn(page)).toEqual(['wage', 'tax', 'purchase']);
    expect(counterpartiesIn(page).map((a) => a.kind).sort()).toEqual(['firm', 'government']);
    expect(turnsIn(page)).toEqual([2, 3, 9]);
  });
  it('marks external counterparties as not selectable and names every account', () => {
    const ext: AccountRef = { kind: 'external', label: 'bank credit creation' };
    const p2: LedgerPage = { ...page, entries: [e(9, 0, ext, HH, 5, 'loan_new', 105)] };
    const [r] = ledgerRows(p2, snap);
    expect(r!.selectable).toBe(false);
    expect(r!.counterpartyLabel).toBe('Bank credit creation');
    expect(accountSelection(ext)).toBeNull();
    expect(accountSelection(GOV)).toEqual({ kind: 'government' });
    expect(accountSelection(BANK)).toEqual({ kind: 'bank' });
    expect(accountSelection(HH)).toEqual({ kind: 'household', id: 2 });
    expect(accountLabel(snap, FIRM)).toBe(snap.firms[0]!.name);
    expect(accountLabel(snap, HH)).toBe(snap.households.find((h) => h.id === 2)!.label);
  });
  it('dates read as month and year from the calendar', () => {
    expect(monthLabel(snap.game.firstTurn, 0, snap.game)).toBe(`Jan ${snap.game.startYear}`);
    expect(monthLabel(snap.game.firstTurn + 1, 2, snap.game)).toBe(`Jun ${snap.game.startYear}`);
    expect(monthLabel(snap.game.firstTurn + snap.game.turnsPerYear, 0, snap.game)).toBe(`Jan ${snap.game.startYear + 1}`);
  });
  it('formats exact money without a negative zero', () => {
    expect(exact(-1e-12)).toBe('$0.00');
    expect(exact(1234.5)).toBe('$1,234.50');
    expect(signedExact(-3)).toBe('-$3.00');
    expect(signedExact(0)).toBe('+$0.00');
  });
});

describe('ledger viewer: reconciliation', () => {
  it('opening + entries + roll-ups = closing is reconciled', () => {
    const r = reconcile(page);
    expect(r).toMatchObject({ ok: true, opening: 100, entries: 20, rolled: 140, closing: 260 });
    expect(Math.abs(r.diff)).toBeLessThan(1e-9);
  });
  it('is not reconciled (red) when the numbers disagree, even if the sim says true', () => {
    const r = reconcile({ ...page, closingBalance: 261 });
    expect(r.ok).toBe(false);
    expect(r.diff).toBeCloseTo(-1, 6);
  });
  it('is not reconciled when the sim says false, even if the numbers add up', () => {
    expect(reconcile({ ...page, reconciles: false }).ok).toBe(false);
  });
  it('ignores a self-transfer and counts only entries that move this account', () => {
    expect(reconcile({ ...page, entries: [...page.entries, e(9, 2, HH, HH, 1000, 'neutral', 260)] }).ok).toBe(true);
  });
});

describe('ledger viewer: roll-up display', () => {
  it('totals by counterparty and kind over the older periods, with the turns they cover', () => {
    const g = rollupGroups(page, null);
    expect(g).toHaveLength(2);
    const wage = g.find((x) => x.kind === 'wage')!;
    expect(wage).toMatchObject({ inflow: 200, outflow: 0, net: 200, count: 2, fromTurn: 2, toTurn: 3 });
    const buy = g.find((x) => x.kind === 'purchase')!;
    expect(buy).toMatchObject({ inflow: 0, outflow: 60, net: -60, count: 2 });
    expect(rollupTotals(g)).toEqual({ inflow: 200, outflow: 60, net: 140, debtNet: 0, count: 4 });
  });
  it('is sorted biggest movement first and honours the filters', () => {
    expect(rollupGroups(page, null)[0]!.kind).toBe('wage');
    expect(rollupGroups(page, null, { ...NO_FILTER, kind: 'purchase' })).toHaveLength(1);
    expect(rollupGroups(page, null, { ...NO_FILTER, fromTurn: 3 })).toHaveLength(2);
    expect(rollupGroups(page, null, { ...NO_FILTER, fromTurn: 4 })).toHaveLength(0);
  });
  it('the roll-up totals plus the detail window equal closing minus opening', () => {
    expect(rowTotals(ledgerRows(page, null)).net + rollupTotals(rollupGroups(page, null)).net).toBe(page.closingBalance - page.openingBalance);
  });
});

describe('ledger viewer: paging', () => {
  const rows = Array.from({ length: 53 }, (_, i) => i);
  it('slices pages and reports the shown range', () => {
    expect(paginate(rows, 0, 25)).toMatchObject({ pages: 3, page: 0, from: 1, to: 25, total: 53 });
    expect(paginate(rows, 2, 25)).toMatchObject({ page: 2, from: 51, to: 53 });
    expect(paginate(rows, 2, 25).items).toEqual([50, 51, 52]);
  });
  it('clamps the page when a filter shrinks the list, and handles an empty list', () => {
    expect(paginate(rows.slice(0, 10), 5, 25)).toMatchObject({ page: 0, pages: 1, from: 1, to: 10 });
    expect(paginate([], 3, 25)).toMatchObject({ page: 0, pages: 1, from: 0, to: 0, total: 0, items: [] });
    expect(paginate(rows, -4, 25).page).toBe(0);
  });
});

describe('selection and person views', () => {
  it('a home goes through its household; an empty firm has no ledger; government and bank have one', () => {
    const s = snap as Snapshot;
    const home = s.homes.find((h) => h.household !== null)!;
    expect(selectionAccount(s, { kind: 'home', id: home.id })).toEqual({ kind: 'household', id: home.household });
    expect(selectionAccount(s, { kind: 'government' })).toEqual(GOV);
    expect(selectionAccount(s, { kind: 'bank' })).toEqual(BANK);
    expect(selectionAccount(s, { kind: 'tile', x: 0, y: 0 })).toBeNull();
    expect(selectionAccount(s, null)).toBeNull();
    const dormant = s.firms.find((f) => !f.active);
    if (dormant) expect(selectionAccount(s, { kind: 'firm', id: dormant.id })).toBeNull();
  });
  it('a persons earnings are wages credited to the household, newest first', () => {
    const sim = createSimPort();
    ok(sim.init(2));
    ok(sim.submitTurn([]));
    const s = ok(sim.submitTurn([])).snapshot;
    const p = s.people.find((x) => x.job)!;
    const pl = ok(sim.personLedger(p.id));
    const rows = earningsRows(pl.earnings, s, pl.household);
    expect(rows.length).toBe(pl.earnings.length);
    expect(rows.every((r) => r.kind === 'wage' && r.amount > 0)).toBe(true);
    expect(rows[0]!.turn).toBeGreaterThanOrEqual(rows.at(-1)!.turn);
  });
});

describe('the real simulation passes the same reconciliation rule as the viewer', () => {
  it('after 10 turns every account page reconciles by the viewer, the independent rule and the sims flag', () => {
    const sim = createSimPort();
    const s0 = ok(sim.init(9));
    for (let t = 0; t < 10; t++) ok(sim.submitTurn([]));
    const accts: AccountRef[] = [...s0.households.map((h) => ({ kind: 'household' as const, id: h.id })), ...s0.firms.map((f) => ({ kind: 'firm' as const, id: f.id })), GOV, BANK];
    for (const a of accts) {
      const p = ok(sim.ledger(a));
      expect(reconcile(p).ok).toBe(true);
      expect(sameRule(p)).toBe(true);
      expect(p.reconciles).toBe(true);
    }
  });
});

describe('ledger viewer: debt, external accounts and the real posting rules', () => {
  const ext: AccountRef = { kind: 'external', label: 'bank credit creation' };
  const loan = (to: AccountRef): LedgerEntry => ({ turn: 1, month: 0, tick: 0, from: ext, to, amount: 100, kind: 'loan_new', detail: 'new loan', memo: '', balanceAfter: 0, debtAfter: 0 });
  it('a new loan raises the borrower\'s balance and debt, and only the bank\'s loans (it is a bystander)', () => {
    expect(entryEffect(loan(FIRM), FIRM)).toEqual({ balance: 100, debt: 100 });
    expect(entryEffect(loan(FIRM), BANK)).toEqual({ balance: 0, debt: 100 });
  });
  it('a repayment lowers balance and debt; a write-off lowers the borrower\'s debt and the bank\'s equity and loans; a bailout adds public debt', () => {
    const repay: LedgerEntry = { ...loan(FIRM), from: FIRM, to: { kind: 'external', label: 'bank credit destruction' }, kind: 'loan_repay' };
    expect(entryEffect(repay, FIRM)).toEqual({ balance: -100, debt: -100 });
    expect(entryEffect(repay, BANK)).toEqual({ balance: 0, debt: -100 });
    const wo: LedgerEntry = { ...loan(FIRM), from: BANK, to: FIRM, kind: 'loan_writeoff' };
    expect(entryEffect(wo, FIRM)).toEqual({ balance: 0, debt: -100 });
    expect(entryEffect(wo, BANK)).toEqual({ balance: -100, debt: -100 });
    const bail: LedgerEntry = { ...loan(FIRM), from: GOV, to: BANK, kind: 'bailout' };
    expect(entryEffect(bail, GOV)).toEqual({ balance: 0, debt: 100 });
    expect(entryEffect(bail, BANK)).toEqual({ balance: 100, debt: 100 });
  });
  it('a page whose debt does not add up is not reconciled even if the sim says so, and so is one whose balance chain breaks', () => {
    const base: LedgerPage = { account: FIRM, openingBalance: 10, openingDebt: 50, closingBalance: 110, closingDebt: 150, reconciles: true, detailFromTurn: 1, summary: [],
      entries: [{ ...loan(FIRM), balanceAfter: 110, debtAfter: 150 }] };
    expect(reconcile(base)).toMatchObject({ ok: true, brokenEntries: 0 });
    expect(reconcile({ ...base, closingDebt: 120 }).ok).toBe(false);
    expect(reconcile({ ...base, entries: [{ ...base.entries[0]!, debtAfter: 140 }] }).brokenEntries).toBe(1);
    expect(reconcile({ ...base, summary: [{ turn: 1, month: 0, counterparty: GOV, kind: 'purchase', inflow: 5, outflow: 0, count: 1, net: 5, debtNet: 0 }] }).ok).toBe(false);
  });
  it('rows show a loan as cash in plus debt up, and a bystander loan as no cash; external accounts have a plain-words hint and no ledger', () => {
    const page: LedgerPage = { account: BANK, openingBalance: 0, openingDebt: 0, closingBalance: 0, closingDebt: 100, reconciles: true, detailFromTurn: 1, summary: [], entries: [{ ...loan(FIRM), balanceAfter: 0, debtAfter: 100 }] };
    const [r] = ledgerRows(page, null);
    expect(r).toMatchObject({ amount: 0, amountText: 'no cash', debtDelta: 100, debtText: '+$100.00', counterparty: FIRM });
    expect(ledgerRows({ ...page, account: FIRM }, null)[0]).toMatchObject({ amount: 100, debtDelta: 100, selectable: false, counterparty: ext });
    expect(externalHint(ext)).toMatch(/creates/);
    expect(accountSelection(ext)).toBeNull();
  });
  it('dates an event by its turn', () => {
    const g = snap.game;
    expect(turnSpan(g.firstTurn, g)).toMatch(/^Jan to Mar \d{4}$/);
  });
});
