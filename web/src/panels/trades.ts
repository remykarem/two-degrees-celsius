import type { MarketEntity, MarketTrades, TradeCell, TradePeriod, TradeRow } from '../contract';
import type { Selection } from '../store';
import { fill, text } from '../store/catalog';

/**
 * Pure views of a market's trade list (contract v2.4, docs/features/market-trades.md). Every label, help text, cell, total and period
 * name is the wire's; this file only sorts, picks the period and turns an entity reference into an inspector selection. It knows no
 * market by name: a market that sends columns and rows is drawn like every other.
 */
const t = (key: string): string => text('markets', key);

/** What clicking an entity reference selects in the inspector. */
export function entitySelection(e: MarketEntity): Selection {
  switch (e.kind) {
    case 'government': return { kind: 'government' };
    case 'bank': return { kind: 'bank' };
    default: return { kind: e.kind, id: e.id };
  }
}

export interface SortState { column: number; dir: 1 | -1 }

/** Next sort when a heading is clicked: first ascending, then descending, then the wire's own order. */
export function nextSort(cur: SortState | null, column: number): SortState | null {
  if (!cur || cur.column !== column) return { column, dir: 1 };
  return cur.dir === 1 ? { column, dir: -1 } : null;
}

const key = (c: TradeCell | undefined): number | string => c?.sort ?? c?.text ?? '';

/** Client-side sort only: by the cell's `sort` number when it has one, else by its text. Stable; the wire's order breaks ties. */
export function sortRows(rows: TradeRow[], sort: SortState | null): TradeRow[] {
  if (!sort) return rows;
  const idx = rows.map((r, i) => ({ r, i }));
  idx.sort((a, b) => {
    const x = key(a.r.cells[sort.column]), y = key(b.r.cells[sort.column]);
    const c = typeof x === 'number' && typeof y === 'number' ? x - y : String(x).localeCompare(String(y), undefined, { numeric: true });
    return c !== 0 ? c * sort.dir : a.i - b.i;
  });
  return idx.map((x) => x.r);
}

export const periodValue = (p: TradePeriod): string => `${p.turn}:${p.month ?? 'all'}`;
export function periodFromValue(v: string): { turn: number; month?: number } {
  const [turn, m] = v.split(':');
  return m === 'all' ? { turn: Number(turn) } : { turn: Number(turn), month: Number(m) };
}

export interface ReconcileLine { ok: boolean; text: string }
/** The line under the table: the rows add up to the card's own figure (only for the latest month with nothing filtered out). */
export function reconcileLine(m: MarketTrades): ReconcileLine | null {
  if (!m.aggregate || m.reconciles === undefined) return null;
  return { ok: m.reconciles, text: fill(t(m.reconciles ? 'trades_reconciles' : 'trades_not_reconciles'), { traded: m.aggregate.text }) };
}

/** One line for an inspector list: the row's cells, written out, without the month. */
export const rowSummary = (m: MarketTrades, r: TradeRow): string =>
  r.cells.filter((c, i) => c.text && m.columns[i]?.id !== 'month').map((c) => c.text).join(' · ');
