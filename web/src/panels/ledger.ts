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

/** "Jan to Mar 2027" for a whole turn (the date of an event that carries only a turn). */
export function turnSpan(turn: number, g: GameCalendar): string {
  const a = monthLabel(turn, 0, g), b = monthLabel(turn, 2, g);
  return a === b ? a : a.slice(0, 3) === b.slice(0, 3) ? a : `${a.slice(0, 3)} to ${b}`;
}

export interface LedgerFilter { kind: LedgerKind | 'all'; fromTurn: number | null; toTurn: number | null; counterparty: AccountRef | null }
export const NO_FILTER: LedgerFilter = { kind: 'all', fromTurn: null, toTurn: null, counterparty: null };

export interface LedgerRow {
  key: string; turn: number; month: 0 | 1 | 2; tick: number; date: string;
  counterparty: AccountRef; counterpartyLabel: string; selectable: boolean;
  kind: LedgerKind; kindLabel: string; detail: string; memo: string;
  /** Effect on the open account's balance (savings, cash, equity): money in is positive. 0 for an entry that only moves debt. */
  amount: number; amountText: string; balanceAfter: number; balanceText: string;
  /** Effect on the open account's debt (a new loan is positive, a repayment negative). */
  debtDelta: number; debtText: string; debtAfter: number;
}
export const signedMoney = (v: number): string => `${v >= 0 ? '+' : '-'}${money(Math.abs(v)).replace('-', '')}`;
const money2 = (v: number): string => { const c = Math.round(v * 100) / 100; return `${c < 0 ? '-' : ''}$${Math.abs(c).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`; };
export const exact = money2;
export const signedExact = (v: number): string => `${v >= 0 ? '+' : '-'}${money2(Math.abs(v)).replace('-', '')}`;
/** Debt column text: blank when the entry leaves debt alone. */
export const debtChangeText = (v: number): string => (Math.abs(v) < 0.005 ? '' : signedExact(v));

/** What the two positions are called for each account type (the bank's balance is its equity and its debt column is the loans it has made). */
export function positionNames(a: AccountRef): { balance: string; debt: string } {
  switch (a.kind) {
    case 'household': return { balance: 'Savings', debt: 'Debt' };
    case 'firm': return { balance: 'Cash', debt: 'Debt' };
    case 'government': return { balance: 'Cash', debt: 'Public debt' };
    case 'bank': return { balance: 'Equity', debt: 'Loans outstanding' };
    case 'external': return { balance: 'Balance', debt: 'Debt' };
  }
}
/** Plain words for the accounts that are not in the town (credit creation and destruction are the bank's lending, not a counterparty with a purse). */
export const EXTERNAL_HINT: Record<string, string> = {
  'bank credit creation': 'Money the bank creates when it lends: the borrower\'s balance and debt both rise.',
  'bank credit destruction': 'Money that disappears when a loan is repaid: the payer\'s balance and debt both fall.',
};
export const externalHint = (a: AccountRef): string => (a.kind === 'external' ? EXTERNAL_HINT[a.label] ?? 'An account outside the town (no ledger).' : '');

const UNPAID_INTEREST = 'unpaid interest added to debt';
export interface Effect { balance: number; debt: number }
/**
 * How one entry moves the open account's two positions, from the entry alone (the rule of docs/sim-api-notes.md "What balance means"):
 * a transfer moves the balance of both ends; a new loan raises the borrower's balance and debt (the bank, as a bystander, only its loans);
 * a repayment lowers both; a write-off lowers the borrower's debt and the bank's equity and loans; a bailout or unpaid interest added to the debt
 * raises the government's debt and the bank's equity and loans (no cash moves for the government).
 */
export function entryEffect(e: LedgerEntry, acct: AccountRef): Effect {
  const from = sameAccount(e.from, acct), to = sameAccount(e.to, acct), a = e.amount;
  if (e.kind === 'loan_new') return to ? { balance: a, debt: a } : { balance: 0, debt: a };
  if (e.kind === 'loan_repay') return from ? { balance: -a, debt: -a } : { balance: 0, debt: -a };
  if (e.kind === 'loan_writeoff') return from ? { balance: -a, debt: -a } : to ? { balance: 0, debt: -a } : { balance: 0, debt: 0 };
  if (e.kind === 'bailout' || e.detail === UNPAID_INTEREST) return from ? { balance: 0, debt: a } : to ? { balance: a, debt: a } : { balance: 0, debt: 0 };
  return { balance: to && !from ? a : from && !to ? -a : 0, debt: 0 };
}

const passes = (turn: number, kind: LedgerKind, cp: AccountRef, f: LedgerFilter): boolean =>
  (f.kind === 'all' || f.kind === kind) && (f.fromTurn === null || turn >= f.fromTurn) && (f.toTurn === null || turn <= f.toTurn) && (!f.counterparty || sameAccount(cp, f.counterparty));

/** The other end of an entry, as seen from the open account (for a bystander, such as the bank in a loan, the borrower). */
const counterpartyOf = (e: LedgerEntry, acct: AccountRef): AccountRef => {
  if (sameAccount(e.to, acct)) return e.from;
  if (sameAccount(e.from, acct)) return e.to;
  return e.from.kind === 'external' ? e.to : e.from;
};

/** Detail rows, newest first, filtered. */
export function ledgerRows(page: LedgerPage, snap: Snapshot | null, f: LedgerFilter = NO_FILTER): LedgerRow[] {
  const rows: LedgerRow[] = [];
  page.entries.forEach((e: LedgerEntry, i) => {
    const cp = counterpartyOf(e, page.account);
    if (!passes(e.turn, e.kind, cp, f)) return;
    const fx = entryEffect(e, page.account);
    rows.push({
      key: `${i}`, turn: e.turn, month: e.month, tick: e.tick, date: snap ? `${monthLabel(e.turn, e.month, snap.game)}` : `T${e.turn}`,
      counterparty: cp, counterpartyLabel: accountLabel(snap, cp), selectable: accountSelection(cp) !== null, kind: e.kind, kindLabel: KIND_LABEL[e.kind],
      detail: e.detail ?? '', memo: `${e.detail ?? KIND_LABEL[e.kind]}: ${accountLabel(snap, e.from)} -> ${accountLabel(snap, e.to)}`, amount: fx.balance, amountText: Math.abs(fx.balance) < 0.005 ? 'no cash' : signedExact(fx.balance), balanceAfter: e.balanceAfter, balanceText: exact(e.balanceAfter),
      debtDelta: fx.debt, debtText: debtChangeText(fx.debt), debtAfter: e.debtAfter,
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

export interface Reconciliation {
  /** True iff the sim's own flag agrees AND balance AND debt both add up AND every entry's running position is what the entry implies. */
  ok: boolean;
  opening: number; entries: number; rolled: number; closing: number; diff: number;
  openingDebt: number; debtEntries: number; debtRolled: number; closingDebt: number; debtDiff: number;
  /** Number of entries whose balanceAfter or debtAfter is not the previous position plus their own effect. */
  brokenEntries: number;
}
const CENT = 0.0051;
/**
 * The viewer's reconciliation rule, recomputed from the page (not just the sim's flag): for the balance AND for the debt,
 * opening + older periods (`net`, `debtNet`) + every entry's effect = closing, and each entry's `balanceAfter` / `debtAfter`
 * equals the running position before it plus its own effect. Red if the sim's flag or any of these says no.
 */
export function reconcile(page: LedgerPage): Reconciliation {
  const rolled = page.summary.reduce((a, r) => a + r.net, 0), debtRolled = page.summary.reduce((a, r) => a + r.debtNet, 0);
  let bal = page.openingBalance + rolled, debt = page.openingDebt + debtRolled, entries = 0, debtEntries = 0, broken = 0;
  for (const e of page.entries) {
    const fx = entryEffect(e, page.account);
    entries += fx.balance; debtEntries += fx.debt;
    if (Math.abs(bal + fx.balance - e.balanceAfter) > CENT || Math.abs(debt + fx.debt - e.debtAfter) > CENT) broken++;
    bal = e.balanceAfter; debt = e.debtAfter; // follow the page so one bad entry is reported once
  }
  const diff = page.openingBalance + rolled + entries - page.closingBalance;
  const debtDiff = page.openingDebt + debtRolled + debtEntries - page.closingDebt;
  const ok = page.reconciles && Math.abs(diff) < 0.01 && Math.abs(debtDiff) < 0.01 && broken === 0 && Math.abs(bal - page.closingBalance) < 0.01 && Math.abs(debt - page.closingDebt) < 0.01;
  return { ok, opening: page.openingBalance, entries, rolled, closing: page.closingBalance, diff, openingDebt: page.openingDebt, debtEntries, debtRolled, closingDebt: page.closingDebt, debtDiff, brokenEntries: broken };
}

export interface RollupGroup { counterparty: AccountRef; counterpartyLabel: string; kind: LedgerKind; kindLabel: string; inflow: number; outflow: number; net: number; debtNet: number; count: number; fromTurn: number; toTurn: number }
/** Totals by counterparty and kind over the rolled-up (older than the detail window) periods, biggest movement first. */
export function rollupGroups(page: LedgerPage, snap: Snapshot | null, f: LedgerFilter = NO_FILTER): RollupGroup[] {
  const m = new Map<string, RollupGroup>();
  for (const r of page.summary as LedgerSummaryRow[]) {
    if (!passes(r.turn, r.kind, r.counterparty, f)) continue;
    const key = `${accountKey(r.counterparty)}|${r.kind}`;
    const g = m.get(key) ?? { counterparty: r.counterparty, counterpartyLabel: accountLabel(snap, r.counterparty), kind: r.kind, kindLabel: KIND_LABEL[r.kind], inflow: 0, outflow: 0, net: 0, debtNet: 0, count: 0, fromTurn: r.turn, toTurn: r.turn };
    g.inflow += r.inflow; g.outflow += r.outflow; g.net += r.net; g.debtNet += r.debtNet; g.count += r.count; g.fromTurn = Math.min(g.fromTurn, r.turn); g.toTurn = Math.max(g.toTurn, r.turn);
    m.set(key, g);
  }
  return [...m.values()].sort((a, b) => Math.abs(b.inflow) + Math.abs(b.outflow) - (Math.abs(a.inflow) + Math.abs(a.outflow)));
}
export const rollupTotals = (g: RollupGroup[]): Totals & { debtNet: number } => {
  const inflow = g.reduce((a, x) => a + x.inflow, 0), outflow = g.reduce((a, x) => a + x.outflow, 0);
  return { inflow, outflow, net: g.reduce((a, x) => a + x.net, 0), debtNet: g.reduce((a, x) => a + x.debtNet, 0), count: g.reduce((a, x) => a + x.count, 0) };
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
  return ledgerRows({ account: { kind: 'household', id: household }, openingBalance: 0, openingDebt: 0, entries, summary: [], closingBalance: 0, closingDebt: 0, reconciles: true, detailFromTurn: 0 }, snap);
}

export const PEOPLE_NOTE = 'People have no purse in this model: money belongs to the household. A person\'s page lists the wages they earned (credited to their household) and their job events.';

/** Shown wherever a firm's ledger revenue and the model's own revenue figure can both be seen. Not an error: some sales move no money. */
export const FIRM_REVENUE_NOTE = 'Revenue on the ledger is money actually received. It can sit up to about 10% below the Revenue figure of the firm\'s books, because when the Factory buys capital goods from itself the model records a sale but no money moves.';
