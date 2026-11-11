import type { Snapshot } from '../contract';
import { compact, money, pct, tonnes } from './format';

/**
 * "No bare percentages": every percent figure is shown with the numbers behind it. These helpers produce the backing text from values the wire gives
 * (stocks, dollars, tonnes, counts). They never compute model results; at most they place two given numbers side by side.
 */

/** "62%" with its backing: `pctWith(0.62, '$62 of $100')` -> "62% ($62 of $100)". */
export const pctWith = (v: number, backing: string, d = 0): string => `${pct(v, d)} (${backing})`;

/** True when the text holds a percent figure with no other number beside it (the rule the lint-style test enforces). */
export function isBarePercent(text: string): boolean {
  if (!text.includes('%')) return false;
  const rest = text.replace(/-?\d+(\.\d+)?\s*%/g, '');
  return !/\d/.test(rest);
}

/** Stock out of its maximum, e.g. "12.4 of 20 units": the max is the stock divided by its share (both given by the wire). */
export function stockOf(stock: number | undefined, share: number): string | null {
  if (stock === undefined || !(share > 0)) return null;
  return `${compact(stock)} of ${compact(stock / share)}`;
}

export function forestTiles(s: Snapshot): string {
  const f = s.map.tiles.filter((t) => t.kind === 'forest');
  const standing = f.filter((t) => (t.state ?? 'standing') === 'standing').length;
  return `${standing} of ${f.length} forest tiles standing`;
}

export const budgetBacking = (s: Snapshot): string => `${tonnes(s.nature.cumulativeTonnes)} used, ${tonnes(s.indicators.remainingBudget)} left`;
export const budgetOf = (s: Snapshot): string => {
  const total = s.breakdowns?.emissions.cumulative.budget;
  return total !== undefined ? `${budgetBacking(s)} of ${tonnes(total)}` : budgetBacking(s);
};

export function debtBacking(s: Snapshot): string { return `${money(s.government.debt)} debt / ${money(s.indicators.gdpAnnual)} GDP`; }

/** Wage and profit dollars: the wire's own breakdown inputs when present, else the finance block's monthly wage bill and profit. */
export function shareBacking(s: Snapshot, which: 'wage' | 'profit'): string {
  const b = s.breakdowns?.[which === 'wage' ? 'wageShare' : 'profitShare'];
  if (b) return b.inputs.map((i) => `${i.label} ${i.unit.startsWith('USD') ? money(i.value) : compact(i.value)}`).join(', ');
  const f = s.finance.firms;
  return which === 'wage' ? `${money(f.wageBill)} wages, ${money(f.profit)} profit a month` : `${money(f.profit)} profit, ${money(f.wageBill)} wages a month`;
}

export function legitimacyBacking(s: Snapshot): string {
  const l = s.breakdowns?.legitimacy;
  if (!l) return `level ${s.government.legitimacy.toFixed(3)} of 1`;
  const top = [...l.terms].sort((a, b) => Math.abs(b.contribution) - Math.abs(a.contribution)).slice(0, 2);
  return `trend ${l.trend >= 0 ? '+' : ''}${l.trend.toFixed(3)} a month; ${top.map((t) => `${t.label.toLowerCase()} ${t.contribution >= 0 ? '+' : ''}${t.contribution.toFixed(3)}`).join(', ')}`;
}

export function richestBacking(s: Snapshot): string {
  const top = s.households.reduce((a, h) => Math.max(a, h.netWorth), 0);
  return `${money(top)} of ${money(s.finance.households.netWorth)} net worth`;
}

export function capitalBacking(s: Snapshot): string {
  const b = s.finance.bank;
  return `equity ${money(b.equity)} / loans ${money(b.loans)}; limit ${pct(b.capitalRatioLimit, 1)} needs ${money(b.capitalRatioLimit > 0 ? b.equity / b.capitalRatioLimit : 0)} equity-backed loans, headroom ${money(b.headroom)}`;
}
