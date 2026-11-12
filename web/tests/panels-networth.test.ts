// Town v8 (contract v2.2): the net worth statement, the town's holders, and the panels that show them. The web reads every line from the wire.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { Catalog, NetWorthLine, Snapshot } from '../src/contract';
import { createSimPort } from './port';
import { createGame, layoutOf } from '../src/store';
import EstateCard from '../src/panels/EstateCard.svelte';
import GiniView from '../src/panels/GiniView.svelte';
import NetWorthStatement from '../src/panels/NetWorthStatement.svelte';
import WhoHoldsWhat from '../src/panels/WhoHoldsWhat.svelte';
import { statementModel, townModel } from '../src/panels/networth';
import { resolveSelection } from '../src/panels/inspect';
import { isBarePercent } from '../src/panels/backing';
import { ok } from './helpers';

const repo = resolve(__dirname, '..', '..');
const real = (): Snapshot => JSON.parse(readFileSync(resolve(repo, 'docs/fixtures/snapshot-turn1.json'), 'utf8')) as Snapshot;
const html = (c: unknown, props: Record<string, unknown>) => render(c as Component<Record<string, unknown>>, { props }).body;
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

describe('the statement model', () => {
  it('lists the wire lines as assets and liabilities, and reads the reconcile flag from the wire', () => {
    const s = real();
    for (const w of s.breakdowns!.gini.byHousehold) {
      const m = statementModel(s, w.household)!;
      expect(m.reconciles).toBe(true);
      expect(m.groups.map((g) => g.rows.length).reduce((a, b) => a + b, 0)).toBe(w.statement.lines.length);
      for (const g of m.groups) for (const r of g.rows) expect(r.how).toMatch(/^networth:/);
    }
    const bad = clone(s);
    bad.breakdowns!.gini.byHousehold[0]!.statement.reconciles = false;
    expect(statementModel(bad, bad.breakdowns!.gini.byHousehold[0]!.household)!.reconcileText).toMatch(/Does not add up/);
  });
  it('a person card shows each line per person, with the equal-share sentence', () => {
    const s = real();
    const w = s.breakdowns!.gini.byHousehold.find((x) => x.people > 1)!;
    const m = statementModel(s, w.household, { name: 'Alex' })!;
    expect(m.note).toContain(`${w.people} in all`);
    expect(m.netLabel).toMatch(/per person/);
    const line = w.statement.lines.find((l) => l.category === 'asset')!;
    expect(m.groups[0]!.rows.find((r) => r.id === line.id)!.amount).toBeCloseTo(line.value / w.people, 6);
  });
  it('counts homes and firms as numbers', () => {
    const s = real();
    const all = s.breakdowns!.gini.byHousehold.flatMap((w) => statementModel(s, w.household)!.groups.flatMap((g) => g.rows));
    expect(all.some((r) => r.count && /^\d+ (homes?|firms?)$/.test(r.count))).toBe(true);
  });
  it('words no bare percentages', () => {
    const s = real();
    const cats = s.breakdowns!.gini.byHousehold.flatMap((w) => { const m = statementModel(s, w.household, { name: 'A' })!; return [m.note, m.reconcileText, m.unpaidRentNote]; });
    for (const x of cats) expect(isBarePercent(x)).toBe(false);
  });
});

describe('who holds what', () => {
  it('follows the wire: groups with counts, a memo row, and the reconcile flag', () => {
    const s = real();
    const m = townModel(s)!;
    expect(m.holders.map((h) => h.id)).toEqual(['households', 'firm_owners', 'bank', 'estate', 'government']);
    expect(m.holders.find((h) => h.id === 'government')!.memo).toBe(true);
    expect(m.holders.find((h) => h.id === 'estate')!.counts).toMatch(/1 of 9 households, 1 of 22 people/);
    expect(m.reconciles).toBe(true);
    const out = html(WhoHoldsWhat, { snap: s });
    expect(out).toContain('who-holds-what');
    expect(out).toContain('data-reconciles="true"');
  });
});

describe('panels', () => {
  it('the inspector shows the statement for the estate owner, a worker household and a person', () => {
    const s = real();
    const layout = layoutOf(s);
    const eh = s.estate!.household;
    expect(resolveSelection(s, layout, { kind: 'household', id: eh }, [])!.statement!.groups[0]!.rows.map((r) => r.id)).toContain('jet');
    const worker = s.households.find((h) => h.class === 'worker')!;
    expect(resolveSelection(s, layout, { kind: 'household', id: worker.id }, [])!.statement).toBeTruthy();
    const p = s.people.find((x) => x.household === worker.id)!;
    const pm = resolveSelection(s, layout, { kind: 'person', id: p.id }, [])!;
    expect(pm.statement!.note).toContain('equal share');
    expect(pm.sections.map((x) => x.heading)).not.toContain('Wealth');
  });
  it('the Gini view, the estate card and the statement component render it', () => {
    const s = real();
    expect(html(GiniView, { snap: s })).toContain('gini-statement-');
    const game = createGame(createSimPort());
    expect(html(EstateCard, { game, snap: s })).toContain('networth-statement');
    expect(html(NetWorthStatement, { model: statementModel(s, s.estate!.household)! })).toContain('networth-line-jet');
  });
});

describe('an asset added only in content and the wire', () => {
  it('shows in every view with no web change', () => {
    const s = clone(real());
    const eh = s.estate!.household;
    const w = s.breakdowns!.gini.byHousehold.find((x) => x.household === eh)!;
    const crypto: NetWorthLine = { id: 'crypto', label: 'Crypto wallet', category: 'asset', value: 5000, perPerson: 5000 / w.people, howId: 'networth:crypto' };
    w.statement.lines.push(crypto);
    w.statement.assets += 5000; w.statement.netWorth += 5000; w.statement.headline += 5000;
    const cat = clone(JSON.parse(readFileSync(resolve(repo, 'docs/fixtures/catalog.json'), 'utf8')) as Catalog);
    expect(cat.netWorthLines.map((l) => l.id)).not.toContain('crypto');
    expect(html(NetWorthStatement, { model: statementModel(s, eh)! })).toContain('Crypto wallet');
    expect(html(GiniView, { snap: s })).toContain('Crypto wallet');
    expect(html(EstateCard, { game: createGame(createSimPort()), snap: s })).toContain('Crypto wallet');
    expect(resolveSelection(s, layoutOf(s), { kind: 'household', id: eh }, [])!.statement!.groups[0]!.rows.map((r) => r.label)).toContain('Crypto wallet');
  });
  it('a made-up holder appears in the town view', () => {
    const s = clone(real());
    const g = s.breakdowns!.gini;
    g.town.holders.push({ id: 'trust', label: 'The trust', note: 'x', inHeadline: true, households: 0, people: 0, assets: 10, liabilities: 0, netWorth: 10 });
    expect(html(WhoHoldsWhat, { snap: s })).toContain('The trust');
  });
  it('every networth text key the code names exists in content/ui.json', () => {
    const src = readFileSync(resolve(repo, 'web/src/panels/networth.ts'), 'utf8');
    const cat = JSON.parse(readFileSync(resolve(repo, 'docs/fixtures/catalog.json'), 'utf8')) as Catalog;
    const keys = [...src.matchAll(/\bt\('(\w+)'/g)].map((m) => m[1]!);
    expect(keys.length).toBeGreaterThan(10);
    for (const k of keys) expect(cat.text.networth?.[k], k).toBeDefined();
  });
});

describe('the real simulation', () => {
  it('every household and the town reconcile over several turns', () => {
    const port = createSimPort(7);
    let snap = ok(port.init(7));
    for (let i = 0; i < 6; i++) {
      for (const w of snap.breakdowns!.gini.byHousehold) expect(statementModel(snap, w.household)!.reconciles).toBe(true);
      expect(townModel(snap)!.reconciles).toBe(true);
      snap = ok(port.submitTurn([])).snapshot;
    }
  });
});
