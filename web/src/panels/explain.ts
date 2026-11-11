import type { Breakdown, BreakdownInput, Snapshot } from '../contract';
import { compact, money, tonnes } from './format';
import type { BreakdownKey } from './metrics';

/** Pure logic of the breakdown popover (docs/snapshot-contract.md section 16). The numbers are the wire's; nothing here recomputes the model. */

/**
 * Value with its unit, in plain words. Units seen on the wire: "$", "$/year", "$/t CO2", "$/unit", "usd_per_tonne", "t CO2", "share", "ratio", "fraction", "0..1", "0 or 1", "index", "people", "units".
 * A share or ratio shows its decimal and, beside it, the percent (the inputs of the same breakdown are always listed next to it).
 */
export function fmtUnit(v: number, unit: string): string {
  const x = Math.abs(v) < 5e-5 ? 0 : v;
  const u = unit.toLowerCase().trim();
  if (u.startsWith('$') || u.startsWith('usd')) {
    const per = u === 'usd_per_tonne' ? ' per tonne' : unit.replace(/^\$|^usd/i, '').replace(/^[/_]/, ' per ').replace(/^ per per /, ' per ');
    return `${money(x)}${per && !per.startsWith(' per') ? ` ${per.trim()}` : per}`;
  }
  if (u === 'share' || u === 'ratio' || u === 'fraction') return `${x.toFixed(3)} (${(x * 100).toFixed(1)}%)`;
  if (u === '0..1' || u === 'index') return x.toFixed(3);
  if (u === '0 or 1') return String(x);
  if (u === 't co2' || u === 'tco2' || u === 't' || u === 'tonnes') return tonnes(x);
  const small = x !== 0 && Math.abs(x) < 0.1 ? String(Number(x.toPrecision(2))) : compact(x);
  return `${small} ${unit}`.trim();
}

export interface PopoverRow { label: string; text: string; note?: string; source?: string }
export interface SumCheck { sum: number; sumText: string; resultText: string; ok: boolean; wireFlag: boolean | null }
export interface PopoverModel { id: string; title: string; formula: string; inputs: PopoverRow[]; resultText: string; parts: PopoverRow[]; sums: SumCheck | null }

/** The sums-to check: the parts added up against the result, side by side with the wire's own flag. Only a CHECK: the shown result is always the wire's. */
export function sumCheck(b: Pick<Breakdown, 'parts' | 'result' | 'sumsToResult'>): SumCheck | null {
  if (!b.parts || !b.parts.length) return null;
  const sum = b.parts.reduce((a, p) => a + p.value, 0);
  const tol = 1e-6 * Math.max(1, Math.abs(b.result.value), Math.abs(sum));
  const ok = Math.abs(sum - b.result.value) <= tol;
  return { sum, sumText: fmtUnit(sum, b.result.unit), resultText: fmtUnit(b.result.value, b.result.unit), ok, wireFlag: b.sumsToResult ?? null };
}

const inputRow = (i: BreakdownInput): PopoverRow => ({ label: i.label, text: fmtUnit(i.value, i.unit), ...(i.source ? { source: i.source } : {}) });

export function popoverModel(b: Breakdown): PopoverModel {
  return {
    id: b.id, title: b.title, formula: b.formula, inputs: b.inputs.map(inputRow), resultText: fmtUnit(b.result.value, b.result.unit),
    parts: (b.parts ?? []).map((p) => ({ label: p.label, text: fmtUnit(p.value, p.unit), ...(p.note ? { note: p.note } : {}) })), sums: sumCheck(b),
  };
}

/** The wire breakdown for a key, or null (older package, or a key that is not a single Breakdown). */
export function breakdownOf(s: Snapshot | null, key: BreakdownKey): Breakdown | null {
  const b = s?.breakdowns;
  if (!b || key === 'emissions' || key === 'taxes') return null;
  return b[key] ?? null;
}

/** Pressure mode pauses the clock while a popover is open (a modal dialog; app/keys.ts modalOpen). */
export const PRESSURE_NOTE = 'In Pressure mode the clock is paused while this is open.';
