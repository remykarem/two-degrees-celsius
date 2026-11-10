import type { AccountRef, GameCalendar, LedgerEntry, LedgerKind, LedgerPage, LedgerSummaryRow, Snapshot } from '../contract';
import type { Selection } from '../store';
import { money } from './format';

/** Pure logic of the transaction viewer (docs/town-v5.md section 4). Signs are always from the point of view of the account whose ledger is open. */

export const KIND_LABEL: Record<LedgerKind, string> = {
  wage: 'Wage', dividend: 'Dividend', rent: 'Rent', benefit: 'Benefit', tax: 'Tax', purchase: 'Purchase', lobby: 'Lobbying', interest: 'Interest', loan_new: 'New loan',
  loan_repay: 'Loan repayment', loan_writeoff: 'Loan write-off', build: 'Building', upkeep: 'Upkeep', subsidy: 'Subsidy', bailout: 'Bailout', wealth_consumption: 'Wealth spent', neutral: 'Other',
};
export const KIND_ORDER = Object.keys(KIND_LABEL) as LedgerKind[];

export const sameAccount = (a: AccountRef, b: AccountRef): boolean =>
  a.kind === b.kind && (a.kind === 'household' || a.kind === 'firm' ? a.id === (b as typeof a).id : a.kind === 'external' ? a.label === (b as typeof a).label : true);
export const accountKey = (a: AccountRef): string => (a.kind === 'household' || a.kind === 'firm' ? `${a.kind}:${a.id}` : a.kind === 'external' ? `external:${a.label}` : a.kind);

export function accountLabel(snap: Snapshot | null, a: AccountRef): string {
  switch (a.kind) {
    case 'household': return snap?.households.find((h) => h.id === a.id)?.label ?? `Household ${a.id}`;
    case 'firm': return snap?.firms.find((f) => f.id === a.id)?.name ?? `Firm ${a.id}`;
    case 'government': return 'Government';
    case 'bank': return 'Bank';
    case 'external': return a.label.charAt(0).toUpperCase() + a.label.slice(1);
  }
}
/** What clicking a counterparty selects, or null for an external account (it has no ledger). */
export function accountSelection(a: AccountRef): Selection | null {
  return a.kind === 'household' ? { kind: 'household', id: a.id } : a.kind === 'firm' ? { kind: 'firm', id: a.id } : a.kind === 'government' ? { kind: 'government' } : a.kind === 'bank' ? { kind: 'bank' } : null;
}
/** The account a selection has a ledger for (a home goes through its household), or null. */
export function selectionAccount(snap: Snapshot | null, sel: Selection | null): AccountRef | null {
  if (!sel) return null;
  switch (sel.kind) {
    case 'household': return { kind: 'household', id: sel.id };
    case 'home': { const h = snap?.homes.find((x) => x.id === sel.id)?.household; return h == null ? null : { kind: 'household', id: h }; }
    case 'firm': return snap?.firms.find((f) => f.id === sel.id)?.active ? { kind: 'firm', id: sel.id } : null;
    case 'government': return { kind: 'government' };
    case 'bank': return { kind: 'bank' };
    default: return null;
  }
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
/** "Mar 2027" for a turn and month-in-turn (a quarter is 3 months; nothing hard-coded beyond the 12 month names). */
export function monthLabel(turn: number, month: number, g: GameCalendar): string {
  const i = turn - g.firstTurn;
  const m = ((i % g.turnsPerYear) * 3 + month) % 12;
  return `${MONTHS[m]} ${g.startYear + Math.floor(i / g.turnsPerYear)}`;
}

export interface LedgerFilter { kind: LedgerKind | 'all'; fromTurn: number | null; toTurn: number | null; counterparty: AccountRef | null }
export const NO_FILTER: LedgerFilter = { kind: 'all', fromTurn: null, toTurn: null, counterparty: null };

export interface LedgerRow {
  key: string; turn: number; month: 0 | 1 | 2; tick: number; date: string;
  counterparty: AccountRef; counterpartyLabel: string; selectable: boolean;
  kind: LedgerKind; kindLabel: string; detail: string; memo: string;
  /** Signed from the open account: money in is positive. */
  amount: number; amountText: string; balanceAfter: number; balanceText: string;
}
export const signedMoney = (v: number): string => `${v >= 0 ? '+' : '-'}${money(Math.abs(v)).replace('-', '')}`;
const money2 = (v: number): string => { const c = Math.round(v * 100) / 100; return `${c < 0 ? '-' : ''}$${Math.abs(c).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`; };
export const exact = money2;
export const signedExact = (v: number): string => `${v >= 0 ? '+' : '-'}${money2(Math.abs(v)).replace('-', '')}`;

const passes = (turn: number, kind: LedgerKind, cp: AccountRef, f: LedgerFilter): boolean =>
  (f.kind === 'all' || f.kind === kind) && (f.fromTurn === null || turn >= f.fromTurn) && (f.toTurn === null || turn <= f.toTurn) && (!f.counterparty || sameAccount(cp, f.counterparty));

/** Detail rows, newest first, filtered. */
export function ledgerRows(page: LedgerPage, snap: Snapshot | null, f: LedgerFilter = NO_FILTER): LedgerRow[] {
  const rows: LedgerRow[] = [];
  page.entries.forEach((e: LedgerEntry, i) => {
    const incoming = sameAccount(e.to, page.account) && !sameAccount(e.from, page.account);
    const cp = incoming ? e.from : e.to;
    if (!passes(e.turn, e.kind, cp, f)) return;
    const amount = incoming ? e.amount : -e.amount;
    rows.push({
      key: `${i}`, turn: e.turn, month: e.month, tick: e.tick, date: snap ? `${monthLabel(e.turn, e.month, snap.game)}` : `T${e.turn}`,
      counterparty: cp, counterpartyLabel: accountLabel(snap, cp), selectable: accountSelection(cp) !== null, kind: e.kind, kindLabel: KIND_LABEL[e.kind],
      detail: e.detail ?? '', memo: e.memo, amount, amountText: signedExact(amount), balanceAfter: e.balanceAfter, balanceText: exact(e.balanceAfter),
    });
  });
  return rows.reverse();
}

export interface Totals { inflow: number; outflow: number; net: number; count: number }
export const rowTotals = (rows: LedgerRow[]): Totals => {
  let inflow = 0, outflow = 0;
  for (const r of rows) { if (r.amount >= 0) inflow += r.amount; else outflow -= r.amount; }
  return { inflow, outflow, net: inflow - outflow, count: rows.length };
};

export interface Reconciliation { ok: boolean; opening: number; entries: number; rolled: number; closing: number; diff: number }
/** opening + detail entries + rolled-up periods = closing, recomputed here from the page (not just the sim's own flag). Red if either says no. */
export function reconcile(page: LedgerPage): Reconciliation {
  const entries = page.entries.reduce((a, e) => a + (sameAccount(e.to, page.account) && !sameAccount(e.from, page.account) ? e.amount : sameAccount(e.from, page.account) && !sameAccount(e.to, page.account) ? -e.amount : 0), 0);
  const rolled = page.summary.reduce((a, r) => a + r.inflow - r.outflow, 0);
  const diff = page.openingBalance + entries + rolled - page.closingBalance;
  return { ok: page.reconciles && Math.abs(diff) < 0.01, opening: page.openingBalance, entries, rolled, closing: page.closingBalance, diff };
}

export interface RollupGroup { counterparty: AccountRef; counterpartyLabel: string; kind: LedgerKind; kindLabel: string; inflow: number; outflow: number; net: number; count: number; fromTurn: number; toTurn: number }
/** Totals by counterparty and kind over the rolled-up (older than the detail window) periods, biggest movement first. */
export function rollupGroups(page: LedgerPage, snap: Snapshot | null, f: LedgerFilter = NO_FILTER): RollupGroup[] {
  const m = new Map<string, RollupGroup>();
  for (const r of page.summary as LedgerSummaryRow[]) {
    if (!passes(r.turn, r.kind, r.counterparty, f)) continue;
    const key = `${accountKey(r.counterparty)}|${r.kind}`;
    const g = m.get(key) ?? { counterparty: r.counterparty, counterpartyLabel: accountLabel(snap, r.counterparty), kind: r.kind, kindLabel: KIND_LABEL[r.kind], inflow: 0, outflow: 0, net: 0, count: 0, fromTurn: r.turn, toTurn: r.turn };
    g.inflow += r.inflow; g.outflow += r.outflow; g.net = g.inflow - g.outflow; g.count += r.count; g.fromTurn = Math.min(g.fromTurn, r.turn); g.toTurn = Math.max(g.toTurn, r.turn);
    m.set(key, g);
  }
  return [...m.values()].sort((a, b) => Math.abs(b.inflow) + Math.abs(b.outflow) - (Math.abs(a.inflow) + Math.abs(a.outflow)));
}
export const rollupTotals = (g: RollupGroup[]): Totals => {
  const inflow = g.reduce((a, x) => a + x.inflow, 0), outflow = g.reduce((a, x) => a + x.outflow, 0);
  return { inflow, outflow, net: inflow - outflow, count: g.reduce((a, x) => a + x.count, 0) };
};

export interface PageSlice<T> { items: T[]; page: number; pages: number; from: number; to: number; total: number }
/** Paging: `page` is clamped into range, so a filter that shrinks the list never leaves an empty page. */
export function paginate<T>(rows: T[], page: number, size: number): PageSlice<T> {
  const pages = Math.max(1, Math.ceil(rows.length / size));
  const p = Math.min(Math.max(0, page), pages - 1);
  const start = p * size;
  return { items: rows.slice(start, start + size), page: p, pages, from: rows.length ? start + 1 : 0, to: Math.min(rows.length, start + size), total: rows.length };
}

/** The kinds that occur in a page (detail and roll-up), in the fixed kind order, for the filter list. */
export const kindsIn = (page: LedgerPage): LedgerKind[] => {
  const seen = new Set<LedgerKind>([...page.entries.map((e) => e.kind), ...page.summary.map((r) => r.kind)]);
  return KIND_ORDER.filter((k) => seen.has(k));
};
/** Distinct counterparties (detail and roll-up), for the filter list. */
export function counterpartiesIn(page: LedgerPage): AccountRef[] {
  const out: AccountRef[] = [];
  const add = (a: AccountRef) => { if (!sameAccount(a, page.account) && !out.some((x) => sameAccount(x, a))) out.push(a); };
  for (const e of page.entries) { add(e.from); add(e.to); }
  for (const r of page.summary) add(r.counterparty);
  return out;
}
/** Turns present in the detail window and the roll-up, ascending. */
export const turnsIn = (page: LedgerPage): number[] => [...new Set([...page.entries.map((e) => e.turn), ...page.summary.map((r) => r.turn)])].sort((a, b) => a - b);

/** A person's earnings: wages credited to the household, newest first. */
export function earningsRows(entries: LedgerEntry[], snap: Snapshot | null, household: number): LedgerRow[] {
  return ledgerRows({ account: { kind: 'household', id: household }, openingBalance: 0, entries, summary: [], closingBalance: 0, reconciles: true, detailFromTurn: 0 }, snap);
}

export const PEOPLE_NOTE = 'People have no purse in this model: money belongs to the household. A person\'s page lists the wages they earned (credited to their household) and their job events.';
