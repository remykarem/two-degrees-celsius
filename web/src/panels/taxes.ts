import type { Snapshot, TaxRow } from '../contract';
import { money } from './format';

/** Why GST/VAT is absent: shown in the Taxes table (docs/town-v5.md section 11). */
export const GST_REASON = 'Households buy goods and services untaxed: the model taxes income, profit, capital, wealth and carbon, but a sales tax would need a price-and-purchase layer it does not have. Government revenue therefore has no GST line.';

/** Taxes as one table of display lines: rates (nominal and in force, with the lobbying gap) and the dollars collected. */
export interface TaxLine { id: TaxRow['id']; label: string; appliesTo: string; nominal: string; effective: string; month: string; quarter: string; modelled: boolean }
const rate = (v: number | null, id: TaxRow['id']): string => (v === null ? 'n/a' : id === 'carbon' ? `${money(v)}/t` : `${(v * 100).toFixed(1)}%`);

/** Taxes from the wire, or a synthesised GST-only table when an older package sends no breakdowns. */
export function taxLines(s: Snapshot): TaxLine[] {
  const rows = s.breakdowns?.taxes;
  if (!rows) return [];
  return rows.map((r) => ({
    id: r.id, label: r.label, appliesTo: r.appliesTo, nominal: r.modelled ? rate(r.rateNominal, r.id) : 'not modelled', effective: r.modelled ? (r.rateEffective === null ? 'same as set' : rate(r.rateEffective, r.id)) : 'none',
    month: r.modelled ? money(r.revenueMonth) : '$0 (none)', quarter: r.modelled ? money(r.revenueQuarter) : '$0 (none)', modelled: r.modelled,
  }));
}
