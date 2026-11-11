import type { Breakdowns, EmissionsLine, HistoryPoint } from '../contract';
import { emissionOf, sourceIds } from '../store/sources';
import { tonnes } from './format';

/** Pure view of snapshot.breakdowns.emissions (contract v1.6). Totals are the wire's; the reconciliation line shows the wire's numbers beside the flag. */
export interface EmissionsRow { id: string; label: string; month: string; turn: string; kind: 'source' | 'sink' | 'total' | 'net' }
export interface EmissionsView {
  rows: EmissionsRow[];
  cumulative: { used: string; budget: string; remaining: string; shareText: string; share: number };
  /** The visible reconciliation: "sources + sink = net", with the numbers, and whether it holds. */
  reconciliation: { text: string; ok: boolean; wireFlag: boolean; note: string };
}
const TOL = (a: number, b: number) => Math.abs(a - b) <= 1e-6 * Math.max(1, Math.abs(a), Math.abs(b));
const listed = (l: EmissionsLine) => l.sources.reduce((a, x) => a + x.tonnes, 0);
const lineOk = (l: EmissionsLine) => TOL(listed(l), l.grossTotal) && TOL(l.grossTotal + l.sink, l.net) && TOL(l.net, l.headline);

export function emissionsView(e: Breakdowns['emissions']): EmissionsView {
  const m = e.month, t = e.turn;
  const r = (id: string, label: string, a: number, b: number, kind: EmissionsRow['kind']): EmissionsRow => ({ id, label, month: tonnes(a), turn: tonnes(b), kind });
  const ok = lineOk(m) && lineOk(t) && e.reconciles;
  const c = e.cumulative;
  // One row per emission source on the wire (month's order, then any the turn adds); a source one line omits is zero there.
  const srcs = sourceIds(m, t);
  const parts = srcs.map((x) => tonnes(emissionOf(m, x.id)));
  return {
    rows: [
      ...srcs.map((x) => r(x.id, x.label, emissionOf(m, x.id), emissionOf(t, x.id), 'source')),
      r('gross', 'Gross total', m.grossTotal, t.grossTotal, 'total'), r('sink', 'Forest sink', m.sink, t.sink, 'sink'), r('net', 'Net emissions', m.net, t.net, 'net'),
    ],
    cumulative: { used: tonnes(c.tonnes), budget: tonnes(c.budget), remaining: tonnes(c.remaining), shareText: `${(c.share * 100).toFixed(1)}% of the budget used (${tonnes(c.tonnes)} of ${tonnes(c.budget)})`, share: c.share },
    reconciliation: {
      text: `Month: ${parts.join(' + ') || tonnes(0)} = ${tonnes(m.grossTotal)} gross; ${tonnes(m.grossTotal)} ${m.sink < 0 ? '-' : '+'} ${tonnes(Math.abs(m.sink))} sink = ${tonnes(m.net)} net, the headline ${tonnes(m.headline)}.`,
      ok, wireFlag: e.reconciles, note: e.note,
    },
  };
}

/** Cumulative tonnes at the end of each turn, for the chart against the budget line. Read from history (budget share left x the wire's budget). */
export function cumulativeSeries(history: HistoryPoint[], budget: number): { turn: number; tonnes: number }[] {
  return history.map((h) => ({ turn: h.turn, tonnes: (1 - h.budgetShare) * budget }));
}
