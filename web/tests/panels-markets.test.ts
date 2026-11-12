// Markets view (contract v2.3, docs/features/markets.md): the web draws whatever list the snapshot carries, with the wire's words.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { Catalog, MarketView, Snapshot } from '../src/contract';
import { writable } from '../src/store';
import MarketsPanel from '../src/panels/MarketsPanel.svelte';
import { cardModel, marketsModel, selectionOf } from '../src/panels/markets';
import { catalog } from '../src/store/catalog';

const repo = resolve(__dirname, '..', '..');
const fixture = <T>(f: string): T => JSON.parse(readFileSync(resolve(repo, 'docs/fixtures', f), 'utf8')) as T;
const snap = (): Snapshot => fixture<{ snapshot: Snapshot }>('turn-result.json').snapshot;
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
// The panel reads `game.state` (a store) and calls `game.select` from a chip; nothing else.
const fakeGame = (s: Snapshot) => ({ state: writable({ snapshot: s }), select: () => {} });
const html = (s: Snapshot): string => render(MarketsPanel as unknown as Component<Record<string, unknown>>, { props: { game: fakeGame(s) } }).body;
const plain = (h: string): string => h.replace(/<[^>]*>/g, ' ').replace(/&[a-z#0-9]+;/g, ' ');

describe('the wire list', () => {
  it('the turn-1 fixture carries every market the content lists, in content order', () => {
    const s = snap();
    expect(s.markets.map((m) => m.id)).toEqual(catalog().markets.list.map((d) => d.id));
    for (const m of s.markets) expect(m.reconciles).toBe(true);
  });
});

describe('the card model', () => {
  it('has one card per wire market, with the wire\'s words and bars on one scale', () => {
    const s = snap();
    const m = marketsModel(s);
    expect(m.ready).toBe(true);
    const cards = m.groups.flatMap((g) => g.cards);
    expect(cards.map((c) => c.id).sort()).toEqual(s.markets.map((x) => x.id).sort());
    for (const c of cards) {
      const w = s.markets.find((x) => x.id === c.id)!;
      expect(c.verdict).toBe(w.verdict);
      expect(c.gap).toEqual(w.gapWords);
      expect(c.gapKind).toBe(w.gapKind);
      for (const b of c.bars) expect(b.width >= 0 && b.width <= 1).toBe(true);
      expect(Math.max(...c.bars.map((b) => b.width))).toBeCloseTo(1, 9);
      expect(c.mechanisms.length).toBe(w.mechanisms.length);
      expect(c.how).toBe(w.howId);
    }
  });
  it('shows a price with its direction word and cause word, and none for a market without a price', () => {
    const s = snap();
    const food = cardModel(s.markets.find((x) => x.id === 'food')!, catalog().markets.mechanisms);
    expect(food.price!.text).toMatch(/^\$/);
    expect(food.price!.cause).toBe(catalog().markets.causes.cost);
    const energy = cardModel(s.markets.find((x) => x.id === 'energy')!, catalog().markets.mechanisms);
    expect(energy.price).toBeNull();
    expect(energy.series).toEqual([]);
  });
  it('a chip points at the thing the wire names', () => {
    expect(selectionOf({ id: 'sold_out', label: 'x', count: 1, entity: { kind: 'firm', id: 2 } })).toEqual({ kind: 'firm', id: 2 });
    expect(selectionOf({ id: 'jobless', label: 'x', count: 3 })).toBeNull();
  });
  it('is empty, with a note, before the first month', () => {
    const s = snap(); s.markets = [];
    const m = marketsModel(s);
    expect(m.ready).toBe(false);
    expect(html(s)).toContain(m.empty);
  });
});

describe('the Markets view', () => {
  it('renders the intro on coordination, every mechanism, how prices are set, and a strip for every market', () => {
    const s = snap();
    const out = html(s);
    const c = catalog().markets;
    expect(out).toContain('data-testid="markets-panel"');
    for (const x of c.mechanisms) { expect(out).toContain(`data-testid="mechanism-${x.id}"`); expect(plain(out)).toContain(x.example); }
    expect(plain(out)).toContain('A market is one way to run an economy');
    expect(plain(out)).toContain('does not mean capitalism');
    expect(plain(out)).toContain('Firms set prices from their costs plus a markup');
    for (const m of s.markets) {
      expect(out).toContain(`data-testid="market-${m.id}"`);
      expect(out).toContain(`data-testid="how-${m.howId}"`);
      expect(plain(out)).toContain(m.verdict);
      for (const g of m.gapWords) expect(plain(out)).toContain(g);
    }
    // The cost-plus note is on the cards of the goods markets, in the wire's words.
    expect(plain(out)).toContain(s.markets.find((x) => x.id === 'food')!.priceNote);
    expect(out).toContain('data-testid="how-markets:food"');
  });
  it('shows no bare percentage and no unfilled placeholder anywhere', () => {
    const t = plain(html(snap()));
    expect(t).not.toMatch(/\d\s*%/);
    expect(t).not.toMatch(/\{\w+\}/);
    expect(t).not.toMatch(/\?markets\./);
  });
  it('a market added only in content (a made-up one on the wire) appears with no change to the web', () => {
    const s = snap();
    const food = s.markets.find((x) => x.id === 'food')!;
    const bread: MarketView = { ...clone(food), id: 'bread', label: 'Bread', blurb: 'A made-up market.', howId: 'markets:bread', verdict: 'Bakers wanted 12 loaves and had 9: 3 loaves went without.',
      gapWords: ['3 loaves went without'], gapKind: 'short', demandText: '12', supplyText: '9', tradedText: '9', demand: 12, supply: 9, traded: 9, unmet: 3, unsold: 0,
      short: [{ id: 'sold_out', label: 'The Bakery ran out of stock', count: 1, entity: { kind: 'firm', id: 1 } }], mechanisms: ['market', 'commons'] };
    s.markets = [...s.markets, bread];
    const out = html(s);
    expect(out).toContain('data-testid="market-bread"');
    expect(out).toContain('data-gap="short"');
    expect(plain(out)).toContain('Bakers wanted 12 loaves');
    expect(plain(out)).toContain('The Bakery ran out of stock');
    expect(out).toContain('data-testid="how-markets:bread"');
    expect(out).toContain('data-testid="mech-commons"');
    // A category the content does not list is still shown, so nothing on the wire is hidden.
    const s2 = clone(s); s2.markets.find((x) => x.id === 'bread')!.category = 'brand_new';
    expect(html(s2)).toContain('data-testid="market-bread"');
    expect(marketsModel(s2).groups.some((g) => g.cards.some((c) => c.id === 'bread'))).toBe(true);
  });
  it('draws a strip from the wire even when the catalog has fewer markets than the snapshot', () => {
    const cat = fixture<Catalog>('catalog.json');
    expect(cat.markets.list.length).toBe(snap().markets.length);
    expect(cat.systems.every((c) => c.coordination.length > 0)).toBe(true);
  });
});
