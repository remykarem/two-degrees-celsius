import type { AccountRef, FinanceFlows, FinanceView, HistoryPoint, LedgerKind, Snapshot } from '../contract';
import { money, pct } from './format';
import { capitalBacking, debtBacking, legitimacyBacking, shareBacking } from './backing';
import type { BreakdownKey } from './metrics';

/** Pure derivations for the Finance tab (docs/town-v5.md section 3). Figures are dollars. Monthly = the latest tick; quarter = the trailing quarter. */

export type FinFmt = 'money' | 'pct' | 'index';
export interface FinRow {
  id: string; label: string; hint?: string;
  month: number; /** null when the wire has no trailing-quarter figure for it. */ quarter: number | null;
  fmt: FinFmt;
  /** Series from history for the sparkline (per turn). */
  spark?: number[];
  /** Where "see transactions" goes. */
  ledger?: { account: AccountRef; kind?: LedgerKind };
  /** A stock (balance sheet figure), not a flow: no quarter column. */
  stock?: boolean;
  /** The numbers behind a percent figure (no bare percentages). */
  backing?: string;
  /** The wire breakdown this row opens with "how?". */
  how?: BreakdownKey;
}
export interface FinSection { id: 'government' | 'bank' | 'households' | 'firms' | 'money' | 'prices'; title: string; rows: FinRow[]; ledgers?: { label: string; account: AccountRef }[] }

export const fmtFin = (v: number, f: FinFmt): string => (f === 'pct' ? pct(v, 1) : f === 'index' ? v.toFixed(2) : money(v));

const GOV: AccountRef = { kind: 'government' };
const BANK: AccountRef = { kind: 'bank' };
const sum = (o: Record<string, number>) => Object.values(o).reduce((a, b) => a + b, 0);
const hist = (h: HistoryPoint[], f: (p: HistoryPoint) => number | undefined): number[] => h.map(f).filter((v): v is number => typeof v === 'number' && Number.isFinite(v));

export const REVENUE_LABEL: Record<keyof FinanceFlows['govRevenue'], string> = { wageTax: 'Income tax', profitTax: 'Profit tax', capitalTax: 'Capital tax', wealthTax: 'Wealth tax', carbon: 'Carbon revenue', other: 'Other revenue' };
export const SPENDING_LABEL: Record<keyof FinanceFlows['govSpending'], string> = {
  benefits: 'Benefits', ubi: 'Basic income', publicSpending: 'Public purchases', jobGuarantee: 'Job guarantee', upkeep: 'Road and plant upkeep', subsidy: 'Subsidies', interest: 'Debt interest', builds: 'Building', other: 'Other spending',
};
const INCOME_LABEL: Record<keyof FinanceFlows['householdIncome'], string> = { wages: 'Wages', dividends: 'Dividends', rent: 'Rent received', interest: 'Interest', benefits: 'Benefits' };
const REVENUE_KIND: Partial<Record<keyof FinanceFlows['govRevenue'], LedgerKind>> = { wageTax: 'tax', profitTax: 'tax', capitalTax: 'tax', wealthTax: 'tax', carbon: 'tax' };
const SPENDING_KIND: Partial<Record<keyof FinanceFlows['govSpending'], LedgerKind>> = { benefits: 'benefit', ubi: 'benefit', publicSpending: 'purchase', upkeep: 'upkeep', subsidy: 'subsidy', interest: 'interest', builds: 'build' };

/** Rows with a zero month AND quarter are dropped from flow lists to keep the tab short, except for the first entries of a section (kept so the player sees what exists). */
const nonZero = (r: FinRow) => r.stock || Math.abs(r.month) > 1e-9 || Math.abs(r.quarter ?? 0) > 1e-9;

const TAX_HINT: Partial<Record<keyof FinanceFlows['govRevenue'], string>> = {
  wageTax: 'Income tax: charged on the wages households earn (the lever key is still tax_wage).',
  profitTax: 'Profit tax: charged on a firm\'s profit each month, before dividends are paid out to its owners.',
  capitalTax: 'Capital tax: charged on income from owning things (dividends, rent, interest) that households receive.',
};
const HOW: Record<string, BreakdownKey> = { 'gov-dgdp': 'debtToGdp', 'gov-leg': 'legitimacy', 'bank-cr': 'capitalRatio', 'p-wage': 'wageShare', 'p-profit': 'profitShare', 'rev-wageTax': 'taxes', 'rev-profitTax': 'taxes', 'rev-capitalTax': 'taxes', 'rev-wealthTax': 'taxes', 'rev-carbon': 'taxes' };

/** Adds the "how?" link and the backing numbers of percent rows. */
function decorate(snap: Snapshot, secs: FinSection[]): FinSection[] {
  const hh = snap.finance.households;
  const incomeTotal = hh.income.wages + hh.income.dividends + hh.income.rent + hh.income.interest + hh.income.benefits;
  const backing: Record<string, string> = {
    'gov-dgdp': debtBacking(snap), 'gov-leg': legitimacyBacking(snap), 'bank-cr': capitalBacking(snap), 'p-wage': shareBacking(snap, 'wage'), 'p-profit': shareBacking(snap, 'profit'),
    'bank-def': `on ${money(snap.finance.bank.loans)} of loans`, 'hh-save': `income ${money(incomeTotal)}, spending ${money(hh.spending)} a month`,
  };
  return secs.map((sec) => ({ ...sec, rows: sec.rows.map((r) => {
    const tax = r.id.startsWith('rev-') ? TAX_HINT[r.id.slice(4) as keyof FinanceFlows['govRevenue']] : undefined;
    return { ...r, ...(tax ? { hint: tax } : {}), ...(backing[r.id] ? { backing: backing[r.id]! } : {}), ...(HOW[r.id] ? { how: HOW[r.id]! } : {}) };
  }) }));
}

export function financeSections(snap: Snapshot): FinSection[] {
  return decorate(snap, rawSections(snap));
}
function rawSections(snap: Snapshot): FinSection[] {
  const f: FinanceView = snap.finance;
  const q = f.quarter ?? null;
  const h = snap.history;
  const gov = f.government, bank = f.bank, hh = f.households, fi = f.firms;
  const govRevenue: FinRow[] = (Object.keys(REVENUE_LABEL) as (keyof FinanceFlows['govRevenue'])[]).map((k) => ({
    id: `rev-${k}`, label: REVENUE_LABEL[k], month: gov.revenue[k], quarter: q ? q.govRevenue[k] : null, fmt: 'money', ledger: { account: GOV, ...(REVENUE_KIND[k] ? { kind: REVENUE_KIND[k]! } : {}) },
  }));
  const govSpend: FinRow[] = (Object.keys(SPENDING_LABEL) as (keyof FinanceFlows['govSpending'])[]).map((k) => ({
    id: `spend-${k}`, label: SPENDING_LABEL[k], month: gov.spending[k], quarter: q ? q.govSpending[k] : null, fmt: 'money', ledger: { account: GOV, ...(SPENDING_KIND[k] ? { kind: SPENDING_KIND[k]! } : {}) },
  }));
  const income: FinRow[] = (Object.keys(INCOME_LABEL) as (keyof FinanceFlows['householdIncome'])[]).map((k) => ({
    id: `inc-${k}`, label: INCOME_LABEL[k], month: hh.income[k], quarter: q ? q.householdIncome[k] : null, fmt: 'money',
  }));
  const qv = (get: (x: FinanceFlows) => number): number | null => (q ? get(q) : null);
  return [
    { id: 'government', title: 'Government', rows: [
      { id: 'gov-in', label: 'Revenue, total', month: sum(gov.revenue), quarter: q ? sum(q.govRevenue) : null, fmt: 'money', spark: hist(h, (p) => p.govRevenue), ledger: { account: GOV } },
      ...govRevenue.filter(nonZero),
      { id: 'gov-out', label: 'Spending, total', month: sum(gov.spending), quarter: q ? sum(q.govSpending) : null, fmt: 'money', spark: hist(h, (p) => p.govSpending), ledger: { account: GOV } },
      ...govSpend.filter(nonZero),
      { id: 'gov-bal', label: 'Balance (revenue minus spending)', month: gov.balance, quarter: q ? sum(q.govRevenue) - sum(q.govSpending) : null, fmt: 'money', ledger: { account: GOV } },
      { id: 'gov-debt', label: 'Public debt', month: gov.debt, quarter: null, fmt: 'money', stock: true, ledger: { account: GOV, kind: 'loan_new' } },
      { id: 'gov-dgdp', label: 'Debt / GDP', month: gov.debtToGdp, quarter: null, fmt: 'pct', stock: true, spark: hist(h, (p) => p.debtToGdp) },
      { id: 'gov-leg', label: 'Legitimacy', month: gov.legitimacy, quarter: null, fmt: 'pct', stock: true, spark: hist(h, (p) => p.legitimacy) },
    ] },
    { id: 'bank', title: 'Bank', rows: [
      { id: 'bank-eq', label: 'Equity', month: bank.equity, quarter: null, fmt: 'money', stock: true, ledger: { account: BANK } },
      { id: 'bank-loans', label: 'Loans', month: bank.loans, quarter: null, fmt: 'money', stock: true, ledger: { account: BANK, kind: 'loan_new' } },
      { id: 'bank-dep', label: 'Deposits', month: bank.deposits, quarter: null, fmt: 'money', stock: true },
      { id: 'bank-cr', label: 'Capital ratio', hint: 'Equity over loans. Must stay above the limit, or lending stops.', month: bank.capitalRatio, quarter: null, fmt: 'pct', stock: true, spark: hist(h, (p) => p.bankCapitalRatio) },
      { id: 'bank-head', label: 'Lending headroom', hint: 'How much more the bank can lend before it hits the capital limit.', month: bank.headroom, quarter: null, fmt: 'money', stock: true },
      { id: 'bank-def', label: 'Default rate', month: bank.defaultRate, quarter: null, fmt: 'pct', stock: true },
      { id: 'bank-int', label: 'Interest income', month: bank.interestIncome, quarter: qv((x) => x.bankInterestIncome), fmt: 'money', ledger: { account: BANK, kind: 'interest' } },
    ] },
    { id: 'households', title: 'Households (all 8 together)', ledgers: snap.households.map((x) => ({ label: x.label, account: { kind: 'household', id: x.id } as AccountRef })), rows: [
      ...income.filter(nonZero),
      { id: 'hh-tax', label: 'Taxes paid', month: hh.taxes, quarter: qv((x) => x.householdTaxes), fmt: 'money' },
      { id: 'hh-rent', label: 'Rent paid', month: hh.rentPaid, quarter: qv((x) => x.rentPaid), fmt: 'money' },
      { id: 'hh-loan', label: 'Loan payments', month: hh.loanPayments, quarter: qv((x) => x.loanPayments), fmt: 'money' },
      { id: 'hh-spend', label: 'Spending', month: hh.spending, quarter: qv((x) => x.householdSpending), fmt: 'money' },
      { id: 'hh-save', label: 'Saving rate', hint: 'Share of income left after tax, rent, loan payments and spending.', month: hh.savingRate, quarter: null, fmt: 'pct', stock: true },
      { id: 'hh-debt', label: 'Debt', month: hh.debt, quarter: null, fmt: 'money', stock: true },
      { id: 'hh-nw', label: 'Net worth', month: hh.netWorth, quarter: null, fmt: 'money', stock: true },
    ] },
    { id: 'firms', title: 'Firms (all together)', ledgers: snap.firms.filter((x) => x.active).map((x) => ({ label: x.name, account: { kind: 'firm', id: x.id } as AccountRef })), rows: [
      { id: 'f-rev', label: 'Revenue', hint: 'Money received for purchases, from the ledger. Up to about 10% below the model\'s own firm revenue: a firm buying from itself moves no money.', month: fi.revenue, quarter: qv((x) => x.firmRevenue), fmt: 'money' },
      { id: 'f-wage', label: 'Wage bill', month: fi.wageBill, quarter: qv((x) => x.wageBill), fmt: 'money' },
      { id: 'f-profit', label: 'Profit', month: fi.profit, quarter: qv((x) => x.firmProfit), fmt: 'money' },
      { id: 'f-inv', label: 'Investment', month: fi.investment, quarter: qv((x) => x.investment), fmt: 'money' },
      { id: 'f-debt', label: 'Debt', month: fi.debt, quarter: null, fmt: 'money', stock: true },
      { id: 'f-cash', label: 'Cash', month: fi.cash, quarter: null, fmt: 'money', stock: true },
    ] },
    { id: 'money', title: 'Money', rows: [
      { id: 'm-supply', label: 'Money supply', month: f.money.supply, quarter: null, fmt: 'money', stock: true },
      { id: 'm-created', label: 'Created this turn', hint: 'New loans create money.', month: f.money.createdThisTurn, quarter: null, fmt: 'money', stock: true },
      { id: 'm-destroyed', label: 'Destroyed this turn', hint: 'Repaid loans destroy money.', month: f.money.destroyedThisTurn, quarter: null, fmt: 'money', stock: true },
    ] },
    { id: 'prices', title: 'Prices and distribution', rows: [
      { id: 'p-index', label: 'Price index', month: f.priceIndex, quarter: null, fmt: 'index', stock: true },
      { id: 'p-wage', label: 'Wage share', month: f.wageShare, quarter: null, fmt: 'pct', stock: true },
      { id: 'p-profit', label: 'Profit share', month: f.profitShare, quarter: null, fmt: 'pct', stock: true },
      { id: 'p-gini', label: 'Inequality (Gini)', month: snap.indicators.gini, quarter: null, fmt: 'index', stock: true, spark: hist(h, (p) => p.gini) },
      { id: 'p-median', label: 'Median income', hint: 'Per person, per month.', month: snap.indicators.medianIncomePerPerson, quarter: null, fmt: 'money', stock: true, spark: hist(h, (p) => p.medianIncomePerPerson) },
      { id: 'p-avg', label: 'Average income', hint: 'Per person, per month.', month: snap.indicators.avgIncomePerPerson, quarter: null, fmt: 'money', stock: true, spark: hist(h, (p) => p.avgIncomePerPerson) },
    ] },
  ];
}

/** True when the monthly flows are all zero (the game has not played a month yet). */
export const financeEmpty = (f: FinanceView): boolean => sum(f.government.revenue) + sum(f.government.spending) + sum(f.households.income) === 0;

export interface CapitalGauge { /** 0..1 position of the ratio on the scale. */ at: number; /** 0..1 position of the limit. */ limitAt: number; status: 'ok' | 'tight' | 'breach'; text: string }
/** The capital ratio against its limit. The scale runs from 0 to twice the limit (or the ratio, if higher). */
export function capitalGauge(bank: FinanceView['bank']): CapitalGauge {
  const max = Math.max(bank.capitalRatioLimit * 2, bank.capitalRatio * 1.15, 0.01);
  const status = bank.capitalRatio < bank.capitalRatioLimit ? 'breach' : bank.capitalRatio < bank.capitalRatioLimit * 1.25 ? 'tight' : 'ok';
  return {
    at: Math.min(1, bank.capitalRatio / max), limitAt: Math.min(1, bank.capitalRatioLimit / max), status,
    text: status === 'breach' ? 'Below the limit: the bank cannot lend.' : status === 'tight' ? 'Close to the limit.' : 'Comfortably above the limit.',
  };
}

export interface FlowEdge { from: 'households' | 'firms' | 'government' | 'bank'; to: 'households' | 'firms' | 'government' | 'bank'; amount: number; label: string }
/** The circular flow over the last quarter: who paid whom. Uses the quarter totals; when the wire has none, three times the month (flagged). */
export function circularFlow(f: FinanceView): { edges: FlowEdge[]; approx: boolean } {
  const q = f.quarter ?? null;
  const k = q ? 1 : 3;
  const a = q ?? ({ govRevenue: f.government.revenue, govSpending: f.government.spending, householdIncome: f.households.income, householdTaxes: f.households.taxes, rentPaid: f.households.rentPaid, loanPayments: f.households.loanPayments, householdSpending: f.households.spending, firmRevenue: f.firms.revenue, wageBill: f.firms.wageBill, firmProfit: f.firms.profit, investment: f.firms.investment, bankInterestIncome: f.bank.interestIncome } as FinanceFlows);
  const edges: FlowEdge[] = [
    { from: 'households', to: 'firms', amount: a.householdSpending * k, label: 'spending' },
    { from: 'firms', to: 'households', amount: (a.householdIncome.wages + a.householdIncome.dividends) * k, label: 'wages and dividends' },
    { from: 'households', to: 'government', amount: a.householdTaxes * k, label: 'taxes' },
    { from: 'firms', to: 'government', amount: a.govRevenue.profitTax * k, label: 'profit tax' },
    { from: 'government', to: 'households', amount: (a.govSpending.benefits + a.govSpending.ubi) * k, label: 'benefits' },
    { from: 'government', to: 'firms', amount: a.govSpending.publicSpending * k, label: 'public purchases' },
    { from: 'households', to: 'bank', amount: a.loanPayments * k, label: 'loan payments' },
    { from: 'bank', to: 'households', amount: a.householdIncome.interest * k, label: 'interest' },
    { from: 'government', to: 'bank', amount: a.govSpending.interest * k, label: 'debt interest' },
  ];
  return { edges: edges.filter((e) => e.amount > 0.5), approx: !q };
}
