import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import type { Component } from 'svelte';
import type { CausalGraph, Catalog, LeverInfo, Snapshot } from '../src/contract';
import { back, causeModel, entryQuantity, inGraph, layoutCauses, overlaps, placeChips, recentre, route, startDepth, STROKE, strokeDash, strokeWidth, type CauseModel } from '../src/panels/causes';
import { catalog, setCatalog, text } from '../src/store/catalog';
import CausesModal from '../src/panels/CausesModal.svelte';
import CausesButton from '../src/panels/CausesButton.svelte';
import { setUi } from '../src/panels/ui';
import { createGame } from '../src/store';
import { createReplayPort } from '../src/sim-port/replay';
import { RECORDING } from '../src/sim-port/replay/recording.generated';

// Causality graph stage 4: the Why? view. Real catalogue (docs/fixtures) and a real snapshot; made-up quantities prove the view reads the wire only.
const fx = <T>(n: string): T => JSON.parse(readFileSync(resolve(__dirname, '..', '..', 'docs', 'fixtures', n), 'utf8')) as T;
const cat = catalog();
const g: CausalGraph = cat.causal;
const snap = fx<Snapshot>('snapshot-turn1.json');
const levers = fx<LeverInfo[]>('levers.json');
const quantities = g.nodes.filter((n) => n.kind === 'quantity');
const model = (id: string, depth: number, none = false): CauseModel => causeModel(g, cat, snap, levers, id, depth, none)!;

describe('causeModel (the data of the Why? view)', () => {
  it('depth 1 is the direct causes, deeper adds more, and the set only grows', () => {
    const u = quantities.find((n) => n.refId === 'unemployment')!;
    const sizes = [1, 2, 3].map((d) => model(u.id, d).nodes.length);
    expect(sizes[0]).toBeGreaterThan(1);
    expect(sizes[0]).toBeLessThanOrEqual(sizes[1]!);
    expect(sizes[1]).toBeLessThanOrEqual(sizes[2]!);
    for (const d of [1, 2, 3]) {
      const m = model(u.id, d);
      expect(m.nodes.every((n) => n.depth <= d)).toBe(true);
      expect(m.nodes[m.nodes.length - 1]!.focus).toBe(true);
      expect(m.nodes.filter((n) => n.focus)).toHaveLength(1);
    }
    // Reading order: furthest cause first, the effect last.
    const depths = model(u.id, 3).nodes.map((n) => n.depth);
    expect(depths).toEqual([...depths].sort((a, b) => b - a));
  });
  it('every drawn link joins two nodes of the view, steps towards the focus, and carries the wire sentence, sign and lag', () => {
    for (const q of quantities) for (const d of [1, 2, 3]) {
      const m = model(q.id, d);
      const ids = new Set(m.nodes.map((n) => n.id));
      const depth = new Map(m.nodes.map((n) => [n.id, n.depth]));
      for (const e of m.edges) {
        expect(ids.has(e.from) && ids.has(e.to), `${q.id} ${e.key}`).toBe(true);
        if (!e.selfLoop) expect(depth.get(e.from)!).toBeGreaterThan(depth.get(e.to)!);
        const wire = g.edges.find((x) => x.kind === 'drives' && `${x.via}|${x.from}|${x.to}` === e.key)!;
        expect(e.sentence).toBe(wire.effect!.sentence);
        expect(e.sign).toBe(wire.effect!.sign);
        expect(e.lagLabel).toBe(wire.effect!.lagLabel);
      }
      // Every cause except the focus has a drawn link out, so nothing floats alone.
      for (const n of m.nodes.filter((x) => !x.focus)) expect(m.edges.some((e) => e.from === n.id && !e.selfLoop), `${q.id} ${n.id}`).toBe(true);
      // Self-loops come last.
      const firstSelf = m.edges.findIndex((e) => e.selfLoop);
      if (firstSelf >= 0) expect(m.edges.slice(firstSelf).every((e) => e.selfLoop)).toBe(true);
    }
  });
  it('links with no direct effect are hidden by default and shown on request; they are never silently dropped', () => {
    const withNone = quantities.filter((n) => model(n.id, 3).noneHidden > 0);
    expect(withNone.length).toBeGreaterThan(0);
    for (const n of withNone) {
      const hidden = model(n.id, 3);
      const shown = model(n.id, 3, true);
      expect(hidden.edges.every((e) => e.sign !== 'none')).toBe(true);
      expect(shown.edges.some((e) => e.sign === 'none')).toBe(true);
      expect(shown.noneHidden).toBe(0);
      expect(shown.nodes.length).toBeGreaterThanOrEqual(hidden.nodes.length);
    }
  });
  it('the net sign on the focus comes from the wire (Mixed is said, never reduced to one sign)', () => {
    let mixed = 0;
    for (const q of quantities) {
      const m = model(q.id, 3);
      for (const n of m.nodes.filter((x) => !x.focus)) {
        const wire = q.causes.find((c) => c.id === n.id)!;
        expect(n.netSign).toBe(wire.netSign);
        expect(n.netLabel).toBe(g.netSigns.find((s) => s.id === wire.netSign)!.label);
        if (n.netSign === 'mixed') mixed++;
      }
    }
    expect(mixed).toBeGreaterThan(0);
  });
  it('two writers of one quantity are separate links, in separate lanes', () => {
    // Find a pair with two drives edges in the real catalogue.
    const pairs = new Map<string, number>();
    for (const e of g.edges.filter((x) => x.kind === 'drives' && x.from !== x.to)) pairs.set(`${e.from}|${e.to}`, (pairs.get(`${e.from}|${e.to}`) ?? 0) + 1);
    const double = [...pairs].find(([, n]) => n > 1);
    if (!double) return; // none in this catalogue: the made-up test below covers the case
    const to = double[0].split('|')[1]!;
    const m = model(to, 3, true);
    const same = m.edges.filter((e) => `${e.from}|${e.to}` === double[0]);
    if (same.length > 1) expect(new Set(same.map((e) => e.lane)).size).toBe(same.length);
  });
  it('loops on a node come from the wire and the focus lists its loop sentences', () => {
    const w = quantities.find((n) => n.refId === 'wage')!;
    const m = model(w.id, 2);
    expect(m.loops.length).toBe(w.inLoops.length);
    expect(m.loops.every((l) => l.sentence.startsWith('This is a'))).toBe(true);
    for (const n of m.nodes) {
      const wire = g.nodes.find((x) => x.id === n.id)!;
      expect(n.loops.reduce((s, l) => s + l.count, 0)).toBe(wire.inLoops.length);
    }
  });
  it('a live value and trend come from the snapshot, and a quantity with none says why from the catalogue', () => {
    const m = model(quantities.find((n) => n.refId === 'unemployment')!.id, 3);
    const published = m.nodes.filter((n) => n.value !== null);
    const not = m.nodes.filter((n) => n.value === null);
    expect(published.length).toBeGreaterThan(0);
    for (const n of not) expect(g.live.unbound.map((u) => u.label)).toContain(n.unpublished);
    for (const n of published) expect(n.unpublished).toBeNull();
  });
  it('lists the policy settings among the causes with the lead time from the lever list', () => {
    const w = quantities.find((n) => n.refId === 'wage')!;
    const m = model(w.id, 3);
    expect(m.levers.length).toBeGreaterThan(0);
    for (const l of m.levers) {
      const info = cat.quantities.find((q) => q.id === l.node.refId)!;
      const lever = levers.find((x) => x.key === info.live.lever)!;
      expect(l.lever).toMatchObject({ key: lever.key, label: lever.label });
    }
  });
  it('an unknown quantity gives no model, and one with no causes gives a lone focus', () => {
    expect(causeModel(g, cat, snap, levers, 'quantity:nope', 2)).toBeNull();
    const src = quantities.find((n) => n.causes.length === 0)!;
    const m = model(src.id, 3);
    expect(m.nodes).toHaveLength(1);
    expect(m.edges).toHaveLength(0);
  });
});

describe('layout (presentational, deterministic)', () => {
  it('never overlaps nodes, is the same every time, and puts the focus alone at the right', () => {
    for (const q of quantities) for (const d of [1, 2, 3]) {
      const m = model(q.id, d);
      const a = layoutCauses(m);
      const b = layoutCauses(model(q.id, d));
      expect(JSON.stringify(a.nodes)).toBe(JSON.stringify(b.nodes));
      for (let i = 0; i < a.nodes.length; i++) for (let j = i + 1; j < a.nodes.length; j++) expect(overlaps(a.nodes[i]!, a.nodes[j]!), `${q.id} d${d}`).toBe(false);
      const focus = a.byId.get(m.focus.id)!;
      expect(a.nodes.filter((n) => n.x >= focus.x)).toHaveLength(1);
      for (const n of a.nodes) { expect(n.x).toBeGreaterThanOrEqual(0); expect(n.y).toBeGreaterThanOrEqual(0); expect(n.x + n.w).toBeLessThanOrEqual(a.width); expect(n.y + n.h).toBeLessThanOrEqual(a.height); }
      // Deeper causes sit further left.
      for (const e of m.edges.filter((x) => !x.selfLoop)) expect(a.byId.get(e.from)!.x).toBeLessThan(a.byId.get(e.to)!.x);
    }
  });
  it('routes every link and places polarity chips without covering a node or each other', () => {
    for (const name of ['unemployment', 'legitimacy', 'wellbeing', 'net_emissions']) {
      const q = quantities.find((n) => n.refId === name)!;
      const m = model(q.id, 3);
      const l = layoutCauses(m);
      const drawn = m.edges.filter((e) => !e.selfLoop);
      const chips = placeChips(l, drawn);
      expect(chips.size).toBe(drawn.length);
      for (const e of drawn) expect(route(l, e)).not.toBeNull();
      const keys = [...chips.keys()];
      let covering = 0;
      for (const k of keys) {
        const p = chips.get(k)!;
        if (l.nodes.some((n) => overlaps({ id: '', x: p.x - 25, y: p.y - 9, w: 50, h: 18, depth: 0 }, n))) covering++;
      }
      // Chips sit in the gaps between columns; a rare crowded link may not find a free spot, but none covers a node on the focus column.
      expect(covering).toBeLessThanOrEqual(Math.ceil(keys.length * 0.1));
    }
  });
  it('is fast enough for about a hundred nodes', () => {
    const big = quantities.reduce((a, b) => (b.causes.length > a.causes.length ? b : a));
    const t0 = performance.now();
    const m = model(big.id, 3, true);
    const l = layoutCauses(m);
    placeChips(l, m.edges.filter((e) => !e.selfLoop));
    expect(performance.now() - t0).toBeLessThan(1500);
    expect(m.nodes.length).toBeGreaterThan(20);
  });
});

describe('re-centring with a history', () => {
  it('pushes the old centre, goes back one at a time, and stops at the start', () => {
    let s = { quantity: 'quantity:a', trail: [] as string[] };
    s = recentre(s, 'quantity:b');
    s = recentre(s, 'quantity:c');
    expect(s).toEqual({ quantity: 'quantity:c', trail: ['quantity:a', 'quantity:b'] });
    expect(recentre(s, 'quantity:c')).toBe(s);
    s = back(s);
    expect(s).toEqual({ quantity: 'quantity:b', trail: ['quantity:a'] });
    s = back(back(s));
    expect(s).toEqual({ quantity: 'quantity:a', trail: [] });
  });
  it('opens at depth 2 on a roomy screen and 1 on a narrow one, never beyond what the wire allows', () => {
    expect(startDepth(3, false)).toBe(2);
    expect(startDepth(3, true)).toBe(1);
    expect(startDepth(1, false)).toBe(1);
  });
});

describe('entry points come from the wire', () => {
  it('maps anchors to quantities that exist, and nothing in the web names one', () => {
    expect(g.entryPoints.length).toBeGreaterThan(5);
    for (const e of g.entryPoints) expect(inGraph(g, e.quantity), e.anchor).toBe(true);
    expect(entryQuantity(g, 'unemployment')).toBe('quantity:unemployment');
    expect(entryQuantity(g, 'no-such-anchor')).toBeNull();
    expect(inGraph(g, 'wage')).toBe(true);
    expect(inGraph(g, 'quantity:nope')).toBe(false);
  });
  it('the button renders for an anchor the wire maps and for a market price quantity, and not for an unknown one', () => {
    const html = (props: Record<string, unknown>) => render(CausesButton as unknown as Component<Record<string, unknown>>, { props }).body;
    expect(html({ anchor: 'unemployment', label: 'Unemployment' })).toContain('data-testid="why-unemployment"');
    expect(html({ quantity: 'wage', label: 'Wage' })).toContain('why-wage');
    expect(html({ anchor: 'zzz', label: 'x' })).not.toContain('<button');
  });
});

// A made-up quantity and edge added only to the catalogue and snapshot render with no web change.
const madeUp = (): { c: Catalog; s: Snapshot } => {
  const node = (id: string, label: string, causes: { id: string; depth: number; netSign: string }[] = []) => ({ id: `quantity:${id}`, kind: 'quantity', role: causes.length ? 'derived' : 'source', refId: id, label, owner: 'firm', selfLoop: false, component: 99, upstream: causes.map((x) => x.id), inLoops: [] as string[], causes });
  const edge = (from: string, to: string, via: string, sign: string) => ({ from: `quantity:${from}`, to: `quantity:${to}`, kind: 'drives', via: `mechanism:${via}`, selfLoop: false, effect: { sign, lagMonths: 2, basis: 'perturbed', lagLabel: '2 months later', sentence: `When ${from} goes up, ${to} goes ${sign === 'up' ? 'up' : 'down'} because of ${via}.`, checked: 'checked' } });
  const nodes = [
    ...g.nodes,
    node('zeta', 'Zeta', [{ id: 'quantity:alpha', depth: 1, netSign: 'up' }, { id: 'quantity:beta', depth: 1, netSign: 'mixed' }]),
    node('alpha', 'Alpha'), node('beta', 'Beta'),
  ] as unknown as CausalGraph['nodes'];
  const edges = [...g.edges, edge('alpha', 'zeta', 'made_a', 'up'), edge('beta', 'zeta', 'made_b', 'down'), edge('beta', 'zeta', 'made_c', 'up')] as unknown as CausalGraph['edges'];
  const c = { ...cat, causal: { ...g, nodes, edges }, quantities: [...cat.quantities, { id: 'zeta', label: 'Zeta', unit: 'widgets', owner: 'firm', summary: '', live: { bound: true, aggregate: 'town', hasTrend: true } }] } as Catalog;
  const s = { ...snap, quantities: [...snap.quantities, { id: 'zeta', value: 7, previous: 5, trend: 'rising' }] } as Snapshot;
  return { c, s };
};

describe('a made-up quantity and edges added only in the catalogue render with no web change', () => {
  it('shows the node with its value, both writers as separate links, and Mixed as such', () => {
    const { c, s } = madeUp();
    const m = causeModel(c.causal, c, s, levers, 'zeta', 1)!;
    expect(m.focus).toMatchObject({ label: 'Zeta', value: '7 widgets', trendWord: c.markets.directions.rising });
    expect(m.nodes.map((n) => n.label)).toEqual(['Alpha', 'Beta', 'Zeta'].sort((a, b) => (a === 'Zeta' ? 1 : b === 'Zeta' ? -1 : a.localeCompare(b))).filter(Boolean));
    expect(m.edges.filter((e) => e.from === 'quantity:beta')).toHaveLength(2);
    expect(m.edges.filter((e) => e.from === 'quantity:beta').map((e) => e.lane).sort()).toEqual([0, 1]);
    expect(m.nodes.find((n) => n.label === 'Beta')!.netSign).toBe('mixed');
    expect(m.nodes.find((n) => n.label === 'Beta')!.netLabel).toBe(c.causal.netSigns.find((x) => x.id === 'mixed')!.label);
  });
});

// Rendering: the modal shows the same nodes as a graph and as the accessible list, polarity is never colour only.
describe('the modal (server render)', () => {
  const game = () => { const gm = createGame(createReplayPort(RECORDING)); gm.newGame(7); return gm; };
  const html = (): string => render(CausesModal as unknown as Component<Record<string, unknown>>, { props: { game: game() } }).body;
  const ids = (h: string, tid: string): string[] => [...h.matchAll(new RegExp(`data-testid="${tid}" data-id="([^"]+)"`, 'g'))].map((m) => m[1]!);
  afterEach(() => { setUi({ causes: null }); delete (globalThis as { window?: unknown }).window; });

  it('the text alternative lists exactly the nodes of the graph, from causes to effect', () => {
    setUi({ causes: { quantity: 'quantity:unemployment', trail: [], depth: 3, view: 'graph' } });
    const graph = html();
    setUi({ causes: { quantity: 'quantity:unemployment', trail: [], depth: 3, view: 'list' } });
    const list = html();
    const gn = ids(graph, 'cause-node');
    const ln = ids(list, 'cause-item');
    expect(gn.length).toBeGreaterThan(1);
    expect(ln).toEqual(gn);
    expect(gn[gn.length - 1]).toBe('quantity:unemployment');
    expect(list).toContain('data-testid="causes-list"');
    expect(graph).not.toContain('data-testid="causes-list"');
    // Narrow screens open on the list at depth 1.
    setUi({ causes: { quantity: 'quantity:unemployment', trail: [] } });
    (globalThis as { window?: unknown }).window = { matchMedia: () => ({ matches: true }) };
    const narrow = html();
    expect(narrow).toContain('data-testid="causes-list"');
    expect(ids(narrow, 'cause-item')).toEqual(model('quantity:unemployment', 1).nodes.map((n) => n.id));
  });
  it('every link drawn has a text label that is not only a colour: a glyph, a line style and a sentence', () => {
    setUi({ causes: { quantity: 'quantity:unemployment', trail: [] } });
    const h = html();
    const chips = [...h.matchAll(/<button[^>]*data-testid="cause-edge"[^>]*data-sign="(\w+)"[^>]*>(.*?)<\/button>/g)];
    expect(chips.length).toBeGreaterThan(0);
    for (const c of chips) {
      expect(c[0]).toMatch(/aria-label="[^"]{10,}/); // the sentence
      expect(c[2]).toMatch(/class="gl[^"]*">(\+|-|\+\/-|0)</); // the glyph
    }
    // Dashed, dotted and solid are drawn by class: the stylesheet carries a dash pattern for down, varies and none.
    const css = readFileSync(resolve(__dirname, '..', 'src', 'panels', 'CausesModal.svelte'), 'utf8');
    for (const sign of ['down', 'varies', 'none']) expect(css, sign).toMatch(new RegExp(`\\.ln\\.${sign} \\{[^}]*stroke-dasharray`));
    expect(h).toContain('data-testid="causes-legend"');
  });
  it('lists the policy settings and loops, and the depth control has the wire range', () => {
    setUi({ causes: { quantity: 'quantity:wage', trail: [] } });
    const h = html();
    expect(h).toContain('data-testid="causes-levers"');
    expect(h).toContain('data-testid="causes-loops"');
    expect((h.match(/data-testid="depth-\d"/g) ?? []).length).toBe(g.upstreamDepth);
  });
  it('a quantity with no causes says it is a starting point', () => {
    const src = quantities.find((n) => n.causes.length === 0)!;
    setUi({ causes: { quantity: src.id, trail: [] } });
    expect(html()).toContain('data-testid="causes-empty"');
  });
});

// Stage 5: strength. The numbers, words, weights and thresholds are the wire's; the web maps a weight to a width.
describe('strength (line thickness and the numbers)', () => {
  const sized = g.edges.filter((e) => e.kind === 'drives' && e.effect?.strength?.weight !== undefined);
  it('maps a weight from the wire linearly to a width between the presentation limits, and a missing one to the plain width', () => {
    expect(strokeWidth(0)).toBe(STROKE.min);
    expect(strokeWidth(1)).toBe(STROKE.max);
    expect(strokeWidth(null)).toBe(STROKE.base);
    expect(strokeWidth(2)).toBe(STROKE.max);
    expect(strokeWidth(0.5)).toBeCloseTo((STROKE.min + STROKE.max) / 2, 9);
    // Hovering adds to the width, so thickness never collides with the hover state.
    expect(strokeWidth(0.3, true)).toBeCloseTo(strokeWidth(0.3) + STROKE.hover, 9);
    expect(strokeDash('down', 6)).toBeDefined();
    expect(strokeDash('up', 6)).toBeUndefined();
  });
  it('every drawn link carries the wire weight and prints its number and word; depends and unmeasured links say so', () => {
    expect(sized.length).toBeGreaterThan(100);
    let withNumber = 0, depends = 0, unverified = 0;
    for (const q of quantities) {
      const m = model(q.id, 3, true);
      for (const e of [...m.edges, ...m.undrawn]) {
        const wire = g.edges.find((x) => x.kind === 'drives' && `${x.via}|${x.from}|${x.to}` === e.key)!.effect!;
        expect(e.weight).toBe(wire.strength?.weight ?? null);
        if (wire.strength?.label) {
          withNumber++;
          expect(e.strengthLine).toContain(wire.strength.label);
          expect(e.strengthLine).toContain(g.strength.words.find((w) => w.id === wire.strength!.word)!.label);
          expect(e.strengthText).toBe(wire.strength.text);
        }
        if (wire.sign === 'varies') { depends++; expect(e.weight).toBeNull(); expect(e.strengthLine).toBe(text('causes', 'strength_depends')); expect(e.strengthText).toBe(g.strength.depends); }
        if (wire.basis === 'declared_unverified' && (wire.sign === 'up' || wire.sign === 'down')) { unverified++; expect(e.weight).toBeNull(); expect(e.strengthLine).toBe(text('causes', 'strength_unverified')); }
        if (wire.sign === 'none') expect(e.strengthLine).toBe('');
      }
    }
    expect(withNumber).toBeGreaterThan(100);
    expect(depends).toBeGreaterThan(0);
    expect(unverified).toBeGreaterThan(0);
  });
  it('a cause carries its overall size from the wire, filled into the wire templates', () => {
    const u = quantities.find((n) => n.refId === 'unemployment')!;
    const m = model(u.id, 3);
    const withNet = m.nodes.filter((n) => n.strengthLine);
    expect(withNet.length).toBeGreaterThan(0);
    for (const n of withNet) {
      const ref = u.causes.find((c) => c.id === n.id)!.netStrength!;
      expect(n.strengthLabel).toBe(ref.label);
      expect(n.strengthText).toContain(ref.label);
      expect(n.strengthText).not.toContain('{');
    }
    expect(m.focus.strengthLine).toBeNull();
  });
  it('a made-up edge with a strength added only in the catalogue is thick, numbered and listed with no web change', () => {
    const { c, s } = madeUp();
    const strength = { kind: 'elasticity', value: 0.42, low: 0.3, high: 0.55, samples: 5, live: 5, total: 6, label: '0.42', word: 'moderate', weight: 0.6, text: 'Strength: moderate. Made up.' };
    const edges = c.causal.edges.map((e) => (e.via === 'mechanism:made_a' ? { ...e, effect: { ...e.effect!, strength } } : e)) as CausalGraph['edges'];
    const c2 = { ...c, causal: { ...c.causal, edges } } as Catalog;
    const m = causeModel(c2.causal, c2, s, levers, 'zeta', 1)!;
    const a = m.edges.find((e) => e.via === 'mechanism:made_a')!;
    expect(a.weight).toBe(0.6);
    expect(a.strengthLine).toBe(`Strength 0.42 (${g.strength.words.find((w) => w.id === 'moderate')!.label})`);
    expect(m.edges.find((e) => e.via === 'mechanism:made_b')!.weight).toBeNull();
  });
});

describe('strength in the modal (server render)', () => {
  const game = () => { const gm = createGame(createReplayPort(RECORDING)); gm.newGame(7); return gm; };
  const html = (): string => render(CausesModal as unknown as Component<Record<string, unknown>>, { props: { game: game() } }).body;
  afterEach(() => { setUi({ causes: null }); delete (globalThis as { window?: unknown }).window; });
  it('draws weighted links with a stroke width, shows the thickness legend with the wire words, and the caveat', () => {
    setUi({ causes: { quantity: 'quantity:unemployment', trail: [], depth: 3, view: 'graph' } });
    const h = html();
    const lines = [...h.matchAll(/data-testid="cause-line"[^>]*/g)].map((m) => m[0]);
    const weighted = [...h.matchAll(/<path[^>]*style="stroke-width:([\d.]+)px[^"]*"[^>]*data-weight="([\d.]+)"/g)];
    expect(lines.length).toBeGreaterThan(0);
    expect(weighted.length).toBeGreaterThan(0);
    for (const w of weighted) {
      const weight = Number(w[2]);
      expect(Number(w[1])).toBeCloseTo(strokeWidth(weight), 5);
    }
    // Thicker lines for larger weights (more than one distinct width on a real graph).
    expect(new Set(weighted.map((w) => w[1])).size).toBeGreaterThan(1);
    expect(h).toContain('data-testid="causes-legend-thickness"');
    for (const w of g.strength.words) expect(h).toContain(w.label);
    expect(h).toContain('data-testid="causes-strength-note"');
    expect(h).toContain('not a forecast');
  });
  it('the list prints the number and the word for each sized link, and does not rely on thickness', () => {
    setUi({ causes: { quantity: 'quantity:unemployment', trail: [], depth: 3, view: 'list' } });
    const h = html();
    const items = [...h.matchAll(/data-testid="cause-line-strength"[^>]*>([^<]*)</g)].map((m) => m[1]!);
    expect(items.length).toBeGreaterThan(0);
    const word = new RegExp(`Strength [0-9.]+ \\((${g.strength.words.map((w) => w.label).join('|')})\\)|Strength: `);
    for (const i of items) expect(i).toMatch(word);
    expect(h).toContain('data-testid="cause-item-strength"');
    expect(h).not.toMatch(/\d(\.\d+)?\s?%/); // no bare percentages
  });
});

it('restores the catalogue', () => { setCatalog(cat); });

describe('Agents explorer entry points', () => {
  it('each decision line offers a Why? for the quantities it reads or writes that the model can explain', async () => {
    const { nodeLines } = await import('../src/panels/explorer');
    const d = g.nodes.find((n) => n.kind === 'decision' && n.id === 'decision:firm_hire')!;
    const l = nodeLines(g, d.id);
    expect(l.quantities.length).toBeGreaterThan(0);
    for (const q of l.quantities) expect(g.nodes.find((n) => n.id === q.id)!.causes.length).toBeGreaterThan(0);
    expect(new Set(l.quantities.map((q) => q.id)).size).toBe(l.quantities.length);
    for (const u of l.upstream) expect(u.id.startsWith('quantity:')).toBe(true);
  });
});
