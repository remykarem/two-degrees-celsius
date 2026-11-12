import type { Snapshot, YieldView } from '../contract';
import { text } from '../store/catalog';

/**
 * Pure view of the sustainable-yield view (contract v2.6, docs/features/farm-from-nature.md). Every row label, figure, sentence and
 * flag is the wire's (`snapshot.natureYield`, authored in `content/ui.json` `nature_yield`); this only scales the two lines of the
 * "taken against regrown" chart. It names no stock and no row: whatever the wire lists is drawn the same way.
 */
export interface YieldRowModel { key: string; label: string; text: string; flag: boolean }
export interface YieldChart {
  /** Points as `x,y` pairs in a 0..w by 0..h box, y up is more. */
  taken: string; regrowth: string; msy: string | null;
  last: { taken: number; regrowth: number } | null;
}
export interface YieldModel {
  id: string; title: string; blurb: string; formula: string; headline: string; verdict: string; over: boolean;
  rows: YieldRowModel[]; stock: number[]; chart: YieldChart | null;
  tag: { label: string; mechanism: string } | null; how: string; reconciles: boolean;
}

const MSY_ROW = 'msy';

export function chartOf(v: YieldView, w: number, h: number): YieldChart | null {
  const s = v.series;
  if (s.length < 1) return null;
  const msy = v.rows.find((r) => r.key === MSY_ROW)?.value ?? null;
  const top = Math.max(1e-9, ...s.map((p) => Math.max(p.taken, p.regrowth)), msy ?? 0);
  const x = (i: number) => (s.length === 1 ? w / 2 : 2 + (i / (s.length - 1)) * (w - 4));
  const y = (val: number) => h - 2 - (val / top) * (h - 4);
  const line = (pick: (p: (typeof s)[number]) => number) => s.map((p, i) => `${x(i).toFixed(1)},${y(pick(p)).toFixed(1)}`).join(' ');
  const l = s[s.length - 1]!;
  return { taken: line((p) => p.taken), regrowth: line((p) => p.regrowth), msy: msy === null ? null : y(msy).toFixed(1), last: { taken: l.taken, regrowth: l.regrowth } };
}

export function yieldModels(snap: Snapshot | null, w = 270, h = 70): YieldModel[] {
  return (snap?.natureYield ?? []).map((v) => ({
    id: v.id, title: v.title, blurb: v.blurb, formula: v.formula, headline: v.headline, verdict: v.verdict, over: v.overLine,
    rows: v.rows.map((r) => ({ key: r.key, label: r.label, text: r.text, flag: r.key === 'over' && r.value === 1 })).filter((r) => r.text !== ''),
    stock: v.series.map((p) => p.stock), chart: chartOf(v, w, h),
    tag: v.access ? { label: v.access.label, mechanism: v.access.mechanismLabel } : null,
    how: v.howId, reconciles: v.reconciles,
  }));
}

/** The intro sentence above the view (authored text in `ui.json`). */
export const yieldIntro = (snap: Snapshot | null): string => (snap?.natureYield?.length ? text('ecology', 'yield_intro') : '');
