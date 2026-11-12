// Sustainable-yield view (contract v2.6, docs/features/farm-from-nature.md): drawn generically from the wire's words.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { Snapshot } from '../src/contract';
import YieldPanel from '../src/panels/YieldPanel.svelte';
import { cardModel } from '../src/panels/markets';
import { chartOf, yieldModels } from '../src/panels/yields';
import { catalog } from '../src/store/catalog';

const repo = resolve(__dirname, '..', '..');
const fixture = <T>(f: string): T => JSON.parse(readFileSync(resolve(repo, 'docs/fixtures', f), 'utf8')) as T;
const snap = (): Snapshot => fixture<{ snapshot: Snapshot }>('turn-result.json').snapshot;
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
const html = (s: Snapshot): string => render(YieldPanel as unknown as Component<Record<string, unknown>>, { props: { snap: s } }).body;
const plain = (h: string): string => h.replace(/<[^>]*>/g, ' ').replace(/&[a-z#0-9]+;/g, ' ');

describe('the yield model', () => {
  it('has one model per wire stock with the wire\'s labels, in wire order', () => {
    const s = snap();
    const m = yieldModels(s);
    expect(m.map((x) => x.id)).toEqual(s.natureYield.map((x) => x.id));
    for (const x of m) {
      const w = s.natureYield.find((v) => v.id === x.id)!;
      expect(x.verdict).toBe(w.verdict);
      expect(x.headline).toBe(w.headline);
      for (const r of x.rows) expect(w.rows.find((q) => q.key === r.key)!.label).toBe(r.label);
    }
  });
  it('draws a made-up stock and a made-up row without knowing them by name', () => {
    const s = clone(snap());
    s.natureYield.push({ ...clone(s.natureYield[0]!), id: 'fish', title: 'Fish in the pond', rows: [{ key: 'bait', label: 'Bait used', text: '12 jars', unit: 'jars' }] });
    const h = plain(html(s));
    expect(h).toContain('Fish in the pond');
    expect(h).toContain('Bait used');
    expect(h).toContain('12 jars');
  });
  it('scales the two lines in the box and marks the best possible regrowth', () => {
    const s = clone(snap());
    const v = s.natureYield[0]!;
    v.series = [{ stock: 800, regrowth: 3, taken: 2 }, { stock: 700, regrowth: 3.5, taken: 5 }];
    const c = chartOf(v, 270, 70)!;
    expect(c.taken.split(' ')).toHaveLength(2);
    expect(c.msy).not.toBeNull();
    for (const p of [...c.taken.split(' '), ...c.regrowth.split(' ')]) {
      const [x, y] = p.split(',').map(Number);
      expect(x! >= 0 && x! <= 270 && y! >= 0 && y! <= 70).toBe(true);
    }
  });
});

describe('the panel', () => {
  it('shows the over-the-line flag and the months left in the wire\'s words, and no bare percentage', () => {
    const s = clone(snap());
    const v = s.natureYield[0]!;
    v.overLine = true;
    v.monthsToCollapse = 14;
    for (const r of v.rows) {
      if (r.key === 'over') { r.value = 1; r.text = 'Yes: more is taken than regrows'; }
      if (r.key === 'months') { r.value = 14; r.text = '14 months'; }
    }
    const out = html(s);
    expect(out).toContain('data-over="true"');
    const t = plain(out);
    expect(t).toContain('Yes: more is taken than regrows');
    expect(t).toContain('14 months');
    expect(t).not.toContain('%');
  });
  it('shows the access rule as a mechanism tag on the commons', () => {
    expect(html(snap())).toContain('data-testid="yield-tag"');
  });
});

describe('the Markets food card link', () => {
  it('carries the wire link to the Ecology view', () => {
    const s = snap();
    const food = s.markets.find((m) => m.id === 'food')!;
    const card = cardModel(food, catalog().markets.mechanisms);
    expect(card.link?.target).toBe('panel:ecology');
    expect(card.link?.label.length).toBeGreaterThan(0);
    expect(cardModel(s.markets.find((m) => m.id === 'goods')!, catalog().markets.mechanisms).link).toBeNull();
  });
});
