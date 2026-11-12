// Market trades (contract v2.4, docs/features/market-trades.md): the list behind each market. The web only sorts and draws what the wire sends.
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { MarketTrades, Snapshot } from '../src/contract';
import { writable } from '../src/store';
import TradesTable from '../src/panels/TradesTable.svelte';
import MarketStrip from '../src/panels/MarketStrip.svelte';
import { cardModel } from '../src/panels/markets';
import { entitySelection, nextSort, periodFromValue, periodValue, reconcileLine, rowSummary, sortRows } from '../src/panels/trades';
import { catalog } from '../src/store/catalog';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import { createReplayPort } from '../src/sim-port/replay';
import { RECORDING } from '../src/sim-port/replay/recording.generated';
import { ok } from './helpers';

const repo = resolve(__dirname, '..', '..');
const fixture = <T>(f: string): T => JSON.parse(readFileSync(resolve(repo, 'docs/fixtures', f), 'utf8')) as T;
const plain = (h: string): string => h.replace(/<[^>]*>/g, ' ').replace(/&[a-z#0-9]+;/g, ' ').replace(/\s+/g, ' ');
const draw = (c: unknown, props: Record<string, unknown>): string => render(c as Component<Record<string, unknown>>, { props }).body;

/** A market the content and the Rust code have never heard of, as the wire would send it. */
const zoning = (): MarketTrades => ({
  market: 'zoning', label: 'Zoning permits', intro: 'Each row is one permit the council granted.', empty: 'No permits this month.', uncounted: 'Refused permits are not counted.',
  period: { turn: 3, month: 1, rolled: false, label: 'Turn 3, month 2' },
  periods: [{ turn: 3, month: 1, rolled: false, label: 'Turn 3, month 2' }, { turn: 2, rolled: true, label: 'Turn 2, all three months (older, rolled up)' }],
  columns: [
    { id: 'applicant', label: 'Applicant', help: 'Who asked.', numeric: false },
    { id: 'plots', label: 'Plots', help: 'How many plots were granted.', numeric: true },
    { id: 'fee', label: 'Fee', help: 'What the council charged.', numeric: true },
  ],
  rows: [
    { counted: true, cells: [{ text: 'Mira', entity: { kind: 'household', id: 3 } }, { text: '2', sort: 2 }, { text: '$40.00', sort: 40 }] },
    { counted: true, cells: [{ text: 'Brickworks', entity: { kind: 'firm', id: 1 } }, { text: '10', sort: 10 }, { text: '$25.00', sort: 25 }] },
    { counted: false, cells: [{ text: 'Council', entity: { kind: 'government', id: 0 } }, { text: '0', sort: 0 }, { text: '$0.00', sort: 0 }] },
  ],
  totals: { count: 2, units: 12, money: 65, text: '12 plots granted for $65.00 this month.', cells: [{ text: '2' }, { text: '12', sort: 12 }, { text: '$65.00', sort: 65 }] },
  aggregate: { traded: 12, text: 'Granted: 12' }, reconciles: true,
});
const fakeGame = (m: MarketTrades) => ({ state: writable({ snapshot: null as Snapshot | null }), select: () => {}, marketTrades: () => ({ ok: true as const, value: m }) });

describe('sorting, client side only', () => {
  const rows = zoning().rows;
  it('cycles ascending, descending, back to the wire order', () => {
    const a = nextSort(null, 1); expect(a).toEqual({ column: 1, dir: 1 });
    const d = nextSort(a, 1); expect(d).toEqual({ column: 1, dir: -1 });
    expect(nextSort(d, 1)).toBeNull();
    expect(nextSort(d, 2)).toEqual({ column: 2, dir: 1 });
  });
  it('sorts numbers by the cell\'s number (10 after 2, not before) and text by text, stably', () => {
    expect(sortRows(rows, { column: 1, dir: 1 }).map((r) => r.cells[1].text)).toEqual(['0', '2', '10']);
    expect(sortRows(rows, { column: 1, dir: -1 }).map((r) => r.cells[1].text)).toEqual(['10', '2', '0']);
    expect(sortRows(rows, { column: 0, dir: 1 }).map((r) => r.cells[0].text)).toEqual(['Brickworks', 'Council', 'Mira']);
    expect(sortRows(rows, null)).toBe(rows);
    expect(rows.map((r) => r.cells[0].text)).toEqual(['Mira', 'Brickworks', 'Council']); // the input is not changed
  });
});

describe('the pure helpers', () => {
  it('an entity reference selects what the inspector knows; government and bank carry no id', () => {
    expect(entitySelection({ kind: 'household', id: 4 })).toEqual({ kind: 'household', id: 4 });
    expect(entitySelection({ kind: 'person', id: 1 })).toEqual({ kind: 'person', id: 1 });
    expect(entitySelection({ kind: 'home', id: 2 })).toEqual({ kind: 'home', id: 2 });
    expect(entitySelection({ kind: 'government', id: 0 })).toEqual({ kind: 'government' });
    expect(entitySelection({ kind: 'bank', id: 0 })).toEqual({ kind: 'bank' });
  });
  it('periods round-trip through the select value', () => {
    for (const p of zoning().periods) expect(periodFromValue(periodValue(p))).toEqual({ turn: p.turn, ...(p.month === undefined ? {} : { month: p.month }) });
  });
  it('the reconcile line says it adds up, or that it does not', () => {
    const m = zoning();
    expect(reconcileLine(m)).toEqual({ ok: true, text: expect.stringContaining('Granted: 12') });
    expect(reconcileLine({ ...m, reconciles: false })!.ok).toBe(false);
    expect(reconcileLine({ ...m, aggregate: undefined, reconciles: undefined })).toBeNull();
  });
  it('an inspector line is the row\'s cells without the month', () => {
    const m = { ...zoning(), columns: [...zoning().columns.slice(0, 2), { id: 'month', label: 'When', help: '', numeric: false }] };
    expect(rowSummary(m, m.rows[0])).toBe('Mira · 2');
  });
});

describe('a made-up market draws with no change to the web', () => {
  it('shows its columns, help, rows as entity links, totals, the reconcile line and the periods', () => {
    const h = draw(TradesTable, { game: fakeGame(zoning()), market: 'zoning' });
    const t = plain(h);
    for (const w of ['Applicant', 'Plots', 'Fee', 'Mira', 'Brickworks', 'Council', '$40.00', 'Each row is one permit', 'Refused permits are not counted', '12 plots granted for $65.00', 'Granted: 12', 'Who asked.', 'How many plots were granted.', 'Turn 2, all three months']) expect(t).toContain(w);
    expect(h).toContain('aria-sort="none"');
    expect((h.match(/data-testid="trade-row"/g) ?? []).length).toBe(3);
    expect(h).toMatch(/class="lnk[ "]/); // entity cells are links
    expect(h).toContain('uncounted'); // the refused row is muted
    expect(h).not.toMatch(/\{\w+\}|NaN|undefined/);
  });
  it('a market with no columns says why instead of drawing an empty table', () => {
    const m = { ...zoning(), columns: [], rows: [], intro: 'No per-agent trades.', empty: 'Counted for the whole town.' };
    const h = draw(TradesTable, { game: fakeGame(m), market: 'zoning' });
    expect(h).not.toContain('<table');
    expect(plain(h)).toContain('Counted for the whole town.');
  });
  it('a refusal (the recorded demo, an old package) reads as plain words', () => {
    const game = { state: writable({ snapshot: null }), select: () => {}, marketTrades: () => ({ ok: false as const, error: { code: 'unsupported', message: 'x' } }) };
    expect(plain(draw(TradesTable, { game, market: 'food' }))).toContain(catalog().text.markets.trades_unavailable);
  });
  it('every market card has the See all trades button', () => {
    const s = fixture<{ snapshot: Snapshot }>('turn-result.json').snapshot;
    const card = cardModel(s.markets[0], catalog().markets.mechanisms);
    const h = draw(MarketStrip, { card, game: { select: () => {}, openTrades: () => {} }, noSeries: '' });
    expect(plain(h)).toContain(catalog().text.markets.trades_button);
  });
});

describe('the real fixtures from the Rust wire', () => {
  for (const f of ['market-trades-food.json', 'market-trades-labour.json']) {
    it(`${f} draws, reconciles and is consistent`, () => {
      const m = fixture<MarketTrades>(f);
      expect(m.reconciles).toBe(true);
      expect(m.rows.length).toBeGreaterThan(0);
      for (const r of m.rows) expect(r.cells.length).toBe(m.columns.length);
      const h = draw(TradesTable, { game: fakeGame(m), market: m.market });
      expect(plain(h)).toContain(m.totals.text);
      expect(h).not.toMatch(/\{\w+\}|NaN|undefined|%/);
    });
  }
});

describe('the recorded demo has no trade list', () => {
  it('refuses plainly', () => {
    const r = createReplayPort(RECORDING).marketTrades('food');
    expect(r.ok).toBe(false);
  });
});

// The real simulation, through wasm: skipped when the package is not built.
const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
describe.skipIf(!existsSync(resolve(pkg, 'sim_wasm_bg.wasm')))('the real sim through wasm', async () => {
  const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  const port = createWasmPort(mod);
  ok(port.init(42));
  for (let i = 0; i < 12; i++) ok(port.submitTurn([]));
  const ids = ['food', 'goods', 'services', 'labour', 'housing', 'credit'];
  it('every list reconciles to its card and costs little to fetch', () => {
    let bytes = 0;
    const t0 = performance.now();
    for (const id of ids) {
      const m = ok(port.marketTrades(id));
      expect(m.reconciles, id).toBe(true);
      expect(m.rows.length, id).toBeGreaterThan(0);
      bytes += JSON.stringify(m).length;
    }
    const ms = (performance.now() - t0) / ids.length;
    console.info(`[market-trades] wasm: ${ids.length} markets, ${bytes} bytes of JSON, ${ms.toFixed(2)} ms per call (incl. JSON parse)`);
    expect(ms).toBeLessThan(50);
  });
  it('recent turns are kept month by month, older turns only as sums', () => {
    const recent = ok(port.marketTrades('food', { turn: 10, month: 1 }));
    expect(recent.period).toMatchObject({ turn: 10, month: 1, rolled: false });
    const old = ok(port.marketTrades('food', { turn: 2 }));
    expect(old.period).toMatchObject({ turn: 2, rolled: true });
    expect(old.rows.length).toBeGreaterThan(0);
    expect(old.periods.some((p) => p.turn === 1 && p.rolled)).toBe(true);
  });
});
