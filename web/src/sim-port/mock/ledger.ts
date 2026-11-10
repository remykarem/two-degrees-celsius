import type { AccountRef, FinanceFlows, LedgerEntry, LedgerKind, LedgerPage, LedgerQuery, LedgerSummaryRow, PersonLedger, Result, SimEvent } from '../../contract';
import { GEN } from './data';
import type { Economy, Income } from './people';
import { personWage } from './people';
import { buildingCounts, type MockState } from './state';

// MOCK ONLY. The mock's money is invented; this module books it as double-entry transactions so the UI can be built and tested before the real
// ledger (docs/town-v5.md section 4) exists. The rules are the real ones: every entry moves money between two accounts, each account's
// balance is opening + entries, and entries older than the detail window are rolled up into monthly summaries that keep the totals exact.

/** Account key: 'h3' household, 'f1' firm, 'gov', 'bank', 'x:label' external. */
export type AcctKey = string;
export const DETAIL_TURNS = 8;
const SPLIT = { food: 0.4, goods: 0.3, services: 0.3 } as const;

export interface Rec {
  turn: number; month: 0 | 1 | 2; tick: number; from: AcctKey; to: AcctKey; amount: number; kind: LedgerKind; detail: string; memo: string;
  fromAfter: number; toAfter: number; fromDebt: number; toDebt: number; person?: number;
  /** A new loan also moves the bank's loans (it is a bystander of the entry): the bank's position right after. */
  bank?: { balance: number; debt: number };
}
export interface MockLedger {
  opening: Record<AcctKey, number>;
  openingDebt: Record<AcctKey, number>;
  balance: Record<AcctKey, number>;
  debt: Record<AcctKey, number>;
  detail: Rec[];
  summary: Record<AcctKey, LedgerSummaryRow[]>;
  /** The last three months of flow totals (newest last), for the finance view. */
  months: FinanceFlows[];
  personEvents: { person: number; turn: number; event: SimEvent }[];
  lastTurn: number;
}

const r2 = (x: number) => Math.round(x * 100) / 100;
export const hKey = (id: number): AcctKey => `h${id}`;
export const fKey = (id: number): AcctKey => `f${id}`;
const EXT = (label: string): AcctKey => `x:${label}`;

export function keyOf(a: AccountRef): AcctKey {
  return a.kind === 'household' ? hKey(a.id) : a.kind === 'firm' ? fKey(a.id) : a.kind === 'government' ? 'gov' : a.kind === 'bank' ? 'bank' : EXT(a.label);
}
export function refOf(k: AcctKey): AccountRef {
  if (k === 'gov') return { kind: 'government' };
  if (k === 'bank') return { kind: 'bank' };
  if (k.startsWith('x:')) return { kind: 'external', label: k.slice(2) };
  return k[0] === 'h' ? { kind: 'household', id: Number(k.slice(1)) } : { kind: 'firm', id: Number(k.slice(1)) };
}

export function emptyFlows(): FinanceFlows {
  return {
    govRevenue: { wageTax: 0, profitTax: 0, capitalTax: 0, wealthTax: 0, carbon: 0, other: 0 },
    govSpending: { benefits: 0, ubi: 0, publicSpending: 0, jobGuarantee: 0, upkeep: 0, subsidy: 0, interest: 0, builds: 0, other: 0 },
    householdIncome: { wages: 0, dividends: 0, rent: 0, interest: 0, benefits: 0 },
    householdTaxes: 0, rentPaid: 0, loanPayments: 0, householdSpending: 0, firmRevenue: 0, wageBill: 0, firmProfit: 0, investment: 0, bankInterestIncome: 0,
  };
}

/** Opening balances at turn 0: household savings, firm cash, government cash, bank equity (as its cash). */
export function initialLedger(s: Pick<MockState, 'households' | 'cash' | 'debt' | 'bank'>): MockLedger {
  const opening: Record<AcctKey, number> = { gov: s.cash, bank: s.bank.equity };
  const openingDebt: Record<AcctKey, number> = { gov: s.debt, bank: s.bank.loans };
  for (const h of s.households) { opening[hKey(h.id)] = h.savings; openingDebt[hKey(h.id)] = h.debt; }
  for (const f of GEN.firms) { opening[fKey(f.id)] = f.cash; openingDebt[fKey(f.id)] = f.debt; }
  return { opening, openingDebt, balance: { ...opening }, debt: { ...openingDebt }, detail: [], summary: {}, months: [], personEvents: [], lastTurn: 0 };
}

function book(s: MockState, from: AcctKey, to: AcctKey, amount: number, kind: LedgerKind, detail: string, memo: string, person?: number): void {
  const L = s.ledger;
  const a = r2(amount);
  if (!(a > 0) || from === to) return;
  L.balance[from] = r2((L.balance[from] ?? 0) - a);
  L.balance[to] = r2((L.balance[to] ?? 0) + a);
  // a new loan raises the borrower's debt and the bank's loans (the posting `CreditCreate` of the real ledger); a transfer moves balances only
  const loan = kind === 'loan_new';
  if (loan) { L.debt[to] = r2((L.debt[to] ?? 0) + a); L.debt.bank = r2((L.debt.bank ?? 0) + a); }
  // advanceMonth books after stepMonth has already counted the tick, so this month's tick is tick - 1
  const tick = s.tick - 1;
  const m = (((tick % 3) + 3) % 3) as 0 | 1 | 2;
  L.detail.push({ turn: s.turn, month: m, tick, from, to, amount: a, kind, detail, memo, fromAfter: L.balance[from]!, toAfter: L.balance[to]!, fromDebt: L.debt[from] ?? 0, toDebt: L.debt[to] ?? 0, ...(loan ? { bank: { balance: L.balance.bank ?? 0, debt: L.debt.bank ?? 0 } } : {}), ...(person !== undefined ? { person } : {}) });
  L.lastTurn = s.turn;
}

/**
 * Books one month of the mock economy from `incs` (the households' income lines, computed from the state before this month's money moves)
 * and returns the month's flow totals. Household balances are the households' savings: the caller copies them back.
 */
export function recordMonth(s: MockState, e: Economy, incs: Map<number, Income>): void {
  const L = s.ledger;
  const start = L.detail.length;
  const name = (id: number) => GEN.firms[id]?.name ?? `Firm ${id}`;
  const active = GEN.firms.filter((f) => s.firmActive[f.id]);
  const taxProfit = Number(s.policy.tax_profit);
  const baseRate = Number(s.policy.base_rate);
  const ubi = Number(s.policy.ubi);

  // wages: firm -> household, one entry per working person
  for (const p of s.people) if (p.job !== null) book(s, fKey(p.job), hKey(p.household), personWage(s, p), 'wage', p.label, `wage, ${name(p.job)}, ${p.label}`, p.id);
  // dividends and the profit tax on them
  const grossDiv = active.length ? (e.profitFirms * 0.6) / active.length : 0;
  for (const f of active) {
    book(s, fKey(f.id), 'gov', grossDiv * taxProfit, 'tax', 'profit tax', `profit tax, ${name(f.id)}`);
    book(s, fKey(f.id), hKey(f.founder), grossDiv * (1 - taxProfit), 'dividend', name(f.id), `dividend, ${name(f.id)}, to the owners`);
  }
  for (const h of s.households) {
    const inc = incs.get(h.id)!;
    book(s, hKey(h.id), 'gov', inc.taxPaid, 'tax', 'income tax', 'income tax on wages');
    book(s, 'bank', hKey(h.id), inc.interest, 'interest', 'savings interest', 'interest on savings');
    const members = h.members.length;
    book(s, 'gov', hKey(h.id), members * ubi, 'benefit', 'ubi', 'universal basic income');
    book(s, 'gov', hKey(h.id), Math.max(0, inc.benefits - members * ubi), 'benefit', 'unemployment and pension benefit', 'benefit');
    const home = h.home === null ? undefined : s.homes.find((x) => x.id === h.home);
    if (home && inc.rentPaid > 0) {
      if (home.owner === 'public') book(s, hKey(h.id), 'gov', inc.rentPaid, 'rent', 'public housing', `rent, home ${home.id} (public)`);
      else if (typeof home.owner === 'number') book(s, hKey(h.id), hKey(home.owner), inc.rentPaid, 'rent', 'private rent', `rent, home ${home.id}`);
    }
    book(s, hKey(h.id), 'bank', inc.loanPayments, 'interest', 'loan service', 'loan interest and fees');
    // spending, split over the sectors the way the mock's purchase flows split it
    const weights = active.map((f) => SPLIT[f.sector] / active.filter((x) => x.sector === f.sector).length);
    const total = weights.reduce((a, b) => a + b, 0) || 1;
    active.forEach((f, i) => book(s, hKey(h.id), fKey(f.id), (inc.spending * weights[i]!) / total, 'purchase', f.sector, `${f.sector}, ${name(f.id)}`));
  }
  // government: public purchases from firms, road upkeep, interest on its debt
  const publicSpend = Number(s.policy.public_spending);
  active.forEach((f) => book(s, 'gov', fKey(f.id), publicSpend / Math.max(1, active.length), 'purchase', 'public spending', `public spending, ${name(f.id)}`));
  book(s, 'gov', EXT('public works'), buildingCounts(s).road * GEN.build.roadUpkeepUsd, 'upkeep', 'road upkeep', 'road maintenance');
  book(s, 'gov', 'bank', (s.debt * baseRate) / 12, 'interest', 'debt interest', 'interest on public debt');
  // borrowing: an account that would go below zero borrows the gap (money created by the bank, or the bond market for the government)
  for (const f of GEN.firms) {
    const b = L.balance[fKey(f.id)] ?? 0;
    if (b < 0) book(s, EXT('bank credit creation'), fKey(f.id), -b, 'loan_new', 'working loan', 'new loan to cover a shortfall');
  }
  const g = L.balance.gov ?? 0;
  if (g < 0) { book(s, EXT('bond market'), 'gov', -g, 'loan_new', 'borrowing', 'new public debt to cover the deficit'); s.debt += -g; }

  L.months.push(flowsOf(L.detail.slice(start)));
  while (L.months.length > 3) L.months.shift();
}

/** Household savings below zero become debt: a loan created by the bank (the caller adds it to the household's debt). Returns the amount. */
export function coverShortfall(s: MockState, hid: number): number {
  const b = s.ledger.balance[hKey(hid)] ?? 0;
  if (b >= 0) return 0;
  book(s, EXT('bank credit creation'), hKey(hid), -b, 'loan_new', 'household loan', 'new loan to cover a shortfall');
  return -b;
}

/** Flow totals of a batch of entries (one month), classified by kind and detail. */
export function flowsOf(recs: Rec[]): FinanceFlows {
  const f = emptyFlows();
  const isH = (k: AcctKey) => k[0] === 'h' && k !== 'h' && !k.startsWith('x:');
  const isF = (k: AcctKey) => k[0] === 'f';
  for (const r of recs) {
    const a = r.amount;
    if (r.to === 'gov' && r.kind === 'tax') { if (r.detail === 'income tax') f.govRevenue.wageTax += a; else if (r.detail === 'profit tax') f.govRevenue.profitTax += a; else f.govRevenue.other += a; }
    if (r.to === 'gov' && r.kind === 'rent') f.govRevenue.other += a;
    if (r.from === 'gov' && r.kind === 'benefit') { if (r.detail === 'ubi') f.govSpending.ubi += a; else f.govSpending.benefits += a; }
    if (r.from === 'gov' && r.kind === 'purchase') f.govSpending.publicSpending += a;
    if (r.from === 'gov' && r.kind === 'upkeep') f.govSpending.upkeep += a;
    if (r.from === 'gov' && r.kind === 'interest') f.govSpending.interest += a;
    if (isF(r.from) && isH(r.to)) { if (r.kind === 'wage') f.householdIncome.wages += a; else if (r.kind === 'dividend') f.householdIncome.dividends += a; }
    if (r.kind === 'wage') f.wageBill += a;
    if (isH(r.from) && isH(r.to) && r.kind === 'rent') { f.householdIncome.rent += a; f.rentPaid += a; }
    if (isH(r.from) && r.to === 'gov' && r.kind === 'rent') f.rentPaid += a;
    if (r.from === 'bank' && isH(r.to) && r.kind === 'interest') f.householdIncome.interest += a;
    if (r.from === 'gov' && isH(r.to) && r.kind === 'benefit') f.householdIncome.benefits += a;
    if (isH(r.from) && r.to === 'gov' && r.kind === 'tax') f.householdTaxes += a;
    if (isH(r.from) && r.to === 'bank') { f.loanPayments += a; f.bankInterestIncome += a; }
    if (r.from === 'gov' && r.to === 'bank') f.bankInterestIncome += a;
    if (isH(r.from) && isF(r.to) && r.kind === 'purchase') { f.householdSpending += a; f.firmRevenue += a; }
    if (r.from === 'gov' && isF(r.to) && r.kind === 'purchase') f.firmRevenue += a;
  }
  f.firmProfit = f.firmRevenue - f.wageBill - f.govRevenue.profitTax - f.householdIncome.dividends;
  const r = (o: Record<string, number>) => { for (const k of Object.keys(o)) o[k] = r2(o[k]!); };
  r(f.govRevenue); r(f.govSpending); r(f.householdIncome);
  for (const k of ['householdTaxes', 'rentPaid', 'loanPayments', 'householdSpending', 'firmRevenue', 'wageBill', 'firmProfit', 'bankInterestIncome'] as const) f[k] = r2(f[k]);
  return f;
}

export const sumFlows = (list: FinanceFlows[]): FinanceFlows => {
  const out = emptyFlows();
  for (const x of list) {
    for (const k of Object.keys(out.govRevenue) as (keyof FinanceFlows['govRevenue'])[]) out.govRevenue[k] += x.govRevenue[k];
    for (const k of Object.keys(out.govSpending) as (keyof FinanceFlows['govSpending'])[]) out.govSpending[k] += x.govSpending[k];
    for (const k of Object.keys(out.householdIncome) as (keyof FinanceFlows['householdIncome'])[]) out.householdIncome[k] += x.householdIncome[k];
    for (const k of ['householdTaxes', 'rentPaid', 'loanPayments', 'householdSpending', 'firmRevenue', 'wageBill', 'firmProfit', 'investment', 'bankInterestIncome'] as const) out[k] += x[k];
  }
  return out;
};

/** Money created this (latest) turn by loans, and destroyed by repayment, from the detail window. */
export function moneyMoved(s: MockState): { created: number; destroyed: number } {
  let created = 0, destroyed = 0;
  for (const r of s.ledger.detail) {
    if (r.turn !== s.ledger.lastTurn) continue;
    if (r.kind === 'loan_new' && r.from === EXT('bank credit creation')) created += r.amount;
    if (r.kind === 'loan_repay') destroyed += r.amount;
  }
  return { created: r2(created), destroyed: r2(destroyed) };
}

/** After a turn: entries older than the detail window become monthly summaries per (account, counterparty, kind). Totals are preserved exactly. */
export function rollUp(s: MockState): void {
  const L = s.ledger;
  const cutoff = s.turn - DETAIL_TURNS;
  const old = L.detail.filter((r) => r.turn < cutoff);
  if (!old.length) return;
  L.detail = L.detail.filter((r) => r.turn >= cutoff);
  const add = (acct: AcctKey, r: Rec, cp: AcctKey, inflow: number, outflow: number, net: number, debtNet: number) => {
    const rows = (L.summary[acct] ??= []);
    const row = rows.find((x) => x.turn === r.turn && x.month === r.month && x.kind === r.kind && keyOf(x.counterparty) === cp);
    if (row) { row.inflow = r2(row.inflow + inflow); row.outflow = r2(row.outflow + outflow); row.net = r2(row.net + net); row.debtNet = r2(row.debtNet + debtNet); row.count++; } else rows.push({ turn: r.turn, month: r.month, counterparty: refOf(cp), kind: r.kind, inflow, outflow, count: 1, net, debtNet });
  };
  for (const r of old) {
    const loan = r.kind === 'loan_new';
    add(r.to, r, r.from, r.amount, 0, r.amount, loan ? r.amount : 0);
    add(r.from, r, r.to, 0, r.amount, -r.amount, 0);
    if (loan) add('bank', r, r.to, 0, 0, 0, r.amount); // the bank is a bystander of a loan: only its loans rise
  }
}

const entryOf = (r: Rec, acct: AcctKey): LedgerEntry => {
  const mineSide = r.from === acct ? { balance: r.fromAfter, debt: r.fromDebt } : r.to === acct ? { balance: r.toAfter, debt: r.toDebt } : r.bank!;
  return {
    turn: r.turn, month: r.month, tick: r.tick, from: refOf(r.from), to: refOf(r.to), amount: r.amount, kind: r.kind, ...(r.detail ? { detail: r.detail } : {}), memo: r.memo,
    balanceAfter: mineSide.balance, debtAfter: mineSide.debt,
  };
};

const bad = (code: string, message: string): Result<never> => ({ ok: false, error: { code, message } });

export function ledgerPage(s: MockState, account: AccountRef, q?: LedgerQuery): Result<LedgerPage> {
  const k = keyOf(account);
  if (account.kind === 'external') return bad('no_ledger', 'External accounts have no ledger.');
  if (!(k in s.ledger.opening)) return bad('unknown_account', `There is no account ${k}.`);
  const L = s.ledger;
  const mine = L.detail.filter((r) => r.from === k || r.to === k || (k === 'bank' && r.bank !== undefined));
  const summary = L.summary[k] ?? [];
  const bal = (r: Rec) => (r.from === k ? -r.amount : r.to === k ? r.amount : 0);
  const dbt = (r: Rec) => (r.kind === 'loan_new' && (r.to === k || k === 'bank') ? r.amount : 0);
  const rolledBal = summary.reduce((a, x) => a + x.net, 0), rolledDebt = summary.reduce((a, x) => a + x.debtNet, 0);
  const closing = L.balance[k] ?? 0, closingDebt = L.debt[k] ?? 0;
  const ok = Math.abs(L.opening[k]! + rolledBal + mine.reduce((a, r) => a + bal(r), 0) - closing) < 0.005 && Math.abs((L.openingDebt[k] ?? 0) + rolledDebt + mine.reduce((a, r) => a + dbt(r), 0) - closingDebt) < 0.005;
  const inRange = (turn: number) => (q?.fromTurn === undefined || turn >= q.fromTurn) && (q?.toTurn === undefined || turn <= q.toTurn);
  return {
    ok: true,
    value: {
      account, openingBalance: L.opening[k]!, openingDebt: L.openingDebt[k] ?? 0, closingBalance: closing, closingDebt, reconciles: ok,
      entries: mine.filter((r) => inRange(r.turn) && (!q?.kind || r.kind === q.kind)).map((r) => entryOf(r, k)),
      summary: summary.filter((x) => inRange(x.turn) && (!q?.kind || x.kind === q.kind)).map((x) => ({ ...x, counterparty: { ...x.counterparty } })),
      detailFromTurn: Math.max(GEN.game.firstTurn, cutoffTurn(s)),
    },
  };
}
const cutoffTurn = (s: MockState) => s.turn - DETAIL_TURNS;

export function personLedger(s: MockState, person: number): Result<PersonLedger> {
  const p = s.people.find((x) => x.id === person);
  if (!p) return bad('unknown_person', `There is no person ${person}.`);
  const hk = hKey(p.household);
  return {
    ok: true,
    value: {
      person, household: p.household,
      earnings: s.ledger.detail.filter((r) => r.person === person && r.kind === 'wage').map((r) => entryOf(r, hk)),
      summary: [],
      events: s.ledger.personEvents.filter((x) => x.person === person).map((x) => structuredClone(x.event)),
      eventTurns: s.ledger.personEvents.filter((x) => x.person === person).map((x) => x.turn),
    },
  };
}
