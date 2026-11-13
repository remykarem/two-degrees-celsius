import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { AgentsCatalog, DecisionInfo, MechanismInfo } from '../src/contract';
import { isBarePercent } from '../src/panels/backing';
import { fmtUnit } from '../src/panels/explain';
import { agentsAnchorFor, causalLines, decisionLine, effectLines, loopLines, loopSummary, mechanismLine, mechanismsOfKind, nodeLines, LOOP_MARK, decisionsOfKind, decisionTag, isAgentsAnchor, kindForAnchor, liveLookup, searchAgents } from '../src/panels/explorer';
import { createSimPort } from './port';
import { ok } from './helpers';

const sim = createSimPort();

// Contract v1.7: the agents catalogue from the real simulation (wasm), checked against the fixture.
const cat = ok(sim.agents());
const fixturePath = resolve(__dirname, '..', '..', 'docs', 'fixtures', 'agents.json');
const fixture = existsSync(fixturePath) ? (JSON.parse(readFileSync(fixturePath, 'utf8')) as AgentsCatalog) : null;

describe('agents catalogue (the real simulation)', () => {
  it('has every kind, the five classes and a randomness table with live parameters', () => {
    expect(cat.kinds.map((k) => k.id)).toEqual(['households', 'firms', 'bank', 'government', 'lobbyists', 'nature', 'weather', 'estate']);
    expect(cat.kinds[0]!.variants.map((v) => v.id)).toEqual(['capitalist', 'rentier', 'manager', 'worker', 'dependent']);
    expect(cat.randomness.length).toBeGreaterThanOrEqual(8);
    for (const k of cat.kinds) expect(k.params.every((p) => p.value !== undefined), k.id).toBe(true);
  });
  it('no parameter value is shown as a bare percentage', () => {
    const all = [...cat.kinds.flatMap((k) => [...k.params, ...k.classParams]), ...cat.randomness.flatMap((r) => r.params)];
    for (const p of all) if (typeof p.value === 'number') expect(isBarePercent(fmtUnit(p.value, p.unit)), p.path).toBe(false);
  });
  it('searches kinds and draws', () => {
    expect(searchAgents(cat, 'drought').draws.map((d) => d.id)).toContain('drought_onset');
    expect(searchAgents(cat, 'lobby').kinds.map((k) => k.id)).toContain('lobbyists');
    expect(searchAgents(cat, 'zzzznothing')).toEqual({ kinds: [], draws: [] });
    expect(searchAgents(cat, '').kinds).toHaveLength(cat.kinds.length);
  });
  it('resolves links: a class opens Households, a bare anchor opens the first kind', () => {
    expect(isAgentsAnchor(agentsAnchorFor('worker'))).toBe(true);
    expect(isAgentsAnchor('emissions')).toBe(false);
    expect(kindForAnchor(cat, 'agents:worker')!.id).toBe('households');
    expect(kindForAnchor(cat, 'agents:bank')!.id).toBe('bank');
    expect(kindForAnchor(cat, 'agents')!.id).toBe('households');
  });
  it('gives a person the decision tag of their household class', () => {
    const snap = ok(sim.init(1));
    const h = snap.households[0]!;
    const tag = decisionTag(cat, snap, h.members[0]!);
    expect(tag!.cls).toBe(h.class);
    expect(tag!.variant.decision.length).toBeGreaterThan(10);
    expect(decisionTag(null, snap, 0)).toBeNull();
    expect(decisionTag(cat, snap, 9999)).toBeNull();
  });
});

// Contract v2.7: decisions come from the registry in the Rust model and are rendered generically.
describe('decisions (generated from the registry)', () => {
  const catalog = ok(sim.catalog());
  it('the firm labour decisions are on the wire with labelled inputs, outputs and live parameters', () => {
    const ids = catalog.decisions.map((d) => d.id);
    for (const id of ['firm_hire', 'firm_fire', 'firm_wage', 'firm_price', 'firm_capex', 'household_found_firm']) expect(ids).toContain(id);
    for (const d of catalog.decisions) {
      expect(['firms', 'households', 'bank', 'government', 'estate', 'nature']).toContain(d.agentKind);
      expect(d.inputs.length).toBeGreaterThan(0);
      expect(d.params.every((p) => typeof p.value === 'number')).toBe(true);
    }
    expect(catalog.quantities.length).toBeGreaterThanOrEqual(10);
  });
  it('Who decides is generated: every decision is a decides line of its kind and its parameters are listed', () => {
    for (const d of catalog.decisions) {
      const kind = cat.kinds.find((k) => k.id === d.agentKind)!;
      expect(kind.decides).toContain(d.decides);
      for (const p of d.params) expect(kind.params.map((x) => x.path)).toContain(p.path);
    }
  });
  it('groups decisions by agent kind and draws the inputs -> decision -> outputs line from wire labels', () => {
    const mine = decisionsOfKind(catalog.decisions, 'firms');
    expect(mine.length).toBeGreaterThanOrEqual(7);
    expect(mine.every((d) => d.agentKind === 'firms')).toBe(true);
    expect(decisionsOfKind(catalog.decisions, 'weather')).toEqual([]);
    const l = decisionLine(mine.find((d) => d.id === 'firm_wage')!);
    expect(l.title).toBe('Wage');
    expect(l.inputs).toContain('Unemployment');
    expect(l.outputs).toBe('Base wage');
  });
  it('a decision added only in the catalogue appears with no web change', () => {
    const made: DecisionInfo = { ...catalog.decisions[0]!, id: 'weather_made_up', title: 'Made up', agentKind: 'weather', inputs: [{ id: 'q1', label: 'Alpha', unit: 'x' }], outputs: [{ id: 'q2', label: 'Beta', unit: 'y' }] };
    const all = [...catalog.decisions, made];
    expect(decisionsOfKind(all, 'weather').map((d) => d.id)).toEqual(['weather_made_up']);
    expect(decisionLine(decisionsOfKind(all, 'weather')[0]!)).toEqual({ inputs: 'Alpha', title: 'Made up', outputs: 'Beta' });
  });
});

describe('causal edges (contract v2.8)', () => {
  const g = ok(sim.catalog()).causal;
  it('the real graph reads as plain lines: starting points, end points, loops and upstream causes', () => {
    const wage = causalLines(g, 'firm_wage');
    expect(wage.inputs).toContain('Minimum wage (Starting point)');
    // Unemployment is written by a mechanism now (stage 1), so it is no longer a starting point.
    expect(wage.inputs).not.toContain('Unemployment (Starting point)');
    const leg = causalLines(g, 'government_legitimacy');
    expect(leg.inputs).toContain('Legitimacy');
    expect(leg.inputs).toContain(LOOP_MARK);
    expect(leg.outputs).toContain(`Legitimacy (End point) ${LOOP_MARK}`);
    expect(leg.upstream.map((u) => u.label)).toContain('Legitimacy');
    expect(leg.upstream[0]!.causes.length).toBeGreaterThan(0);
  });
  it('a made-up decision and edges added only in the catalogue appear with no web change', () => {
    const node = (id: string, kind: 'quantity' | 'decision', role: string, label: string, upstream: string[] = []) => ({ id, kind, role, refId: id.split(':')[1]!, label, owner: 'firm', selfLoop: false, component: 99, upstream, inLoops: [] as string[] });
    const made = {
      ...g,
      nodes: [...g.nodes, node('quantity:qa', 'quantity', 'source', 'Alpha'), node('quantity:qb', 'quantity', 'sink', 'Beta', ['quantity:qa']), node('decision:made_up', 'decision', 'decision', 'Made up')],
      edges: [...g.edges, { from: 'quantity:qa', to: 'decision:made_up', kind: 'reads', via: null, selfLoop: false }, { from: 'decision:made_up', to: 'quantity:qb', kind: 'writes', via: null, selfLoop: false }],
    };
    expect(causalLines(made, 'made_up')).toEqual({ inputs: 'Alpha (Starting point)', outputs: 'Beta (End point)', upstream: [{ label: 'Beta', causes: 'Alpha' }] });
  });
});

describe('mechanisms (contract v2.10)', () => {
  const c = ok(sim.catalog());
  it('the plain-code mechanisms are on the wire, grouped by Agents kind, and read as text lines', () => {
    expect(c.mechanisms.length).toBeGreaterThan(30);
    const kinds = new Set(cat.kinds.map((k) => k.id));
    for (const m of c.mechanisms) {
      expect(kinds.has(m.agentKind), m.id).toBe(true);
      expect(m.inputs.length).toBeGreaterThan(0);
      expect(c.decisions.some((d) => d.id === m.id)).toBe(false);
    }
    // Weather, Nature and Firms each list some.
    const weather = mechanismsOfKind(c.mechanisms, 'weather');
    expect(weather.map((m) => m.id)).toEqual(expect.arrayContaining(['climate_stress_curve', 'drought_onset', 'fire_ignition']));
    expect(mechanismsOfKind(c.mechanisms, 'nature').map((m) => m.id)).toContain('emissions_accounting');
    expect(mechanismsOfKind(c.mechanisms, 'firms').map((m) => m.id)).toContain('production_output');
    const line = mechanismLine(weather.find((m) => m.id === 'drought_onset')!);
    expect(line.inputs).toContain('Climate stress');
    expect(line.outputs).toBe('Drought under way');
  });
  it('the chain emissions -> weather -> output reads off the mechanism lines with their edge marks', () => {
    const g = c.causal;
    const out = nodeLines(g, 'mechanism:drought_loss');
    expect(out.inputs).toContain('Drought under way');
    expect(out.outputs).toContain('Weather and climate effect on output');
    // The mechanism's own output is read by the hiring decision, so it is not an end point.
    expect(out.outputs).not.toContain('End point');
    const stress = nodeLines(g, 'mechanism:climate_stress_curve');
    expect(stress.inputs).toContain('Net emissions so far');
  });
  it('a mechanism added only in the catalogue appears with no web change', () => {
    const node = (id: string, kind: string, role: string, label: string, upstream: string[] = []) => ({ id, kind, role, refId: id.split(':')[1]!, label, owner: 'weather', selfLoop: false, component: 99, upstream, inLoops: [] as string[] });
    const g = c.causal;
    const made = {
      ...g,
      nodes: [...g.nodes, node('quantity:qa', 'quantity', 'source', 'Alpha'), node('quantity:qb', 'quantity', 'sink', 'Beta', ['quantity:qa']), node('mechanism:made_up', 'mechanism', 'mechanism', 'Made up')],
      edges: [...g.edges, { from: 'quantity:qa', to: 'mechanism:made_up', kind: 'reads', via: null, selfLoop: false }, { from: 'mechanism:made_up', to: 'quantity:qb', kind: 'writes', via: null, selfLoop: false }],
    };
    const m: MechanismInfo = { ...c.mechanisms[0]!, id: 'made_up', title: 'Made up', agentKind: 'weather', inputs: [{ id: 'qa', label: 'Alpha', unit: 'x' }], outputs: [{ id: 'qb', label: 'Beta', unit: 'y' }] };
    const all = [...c.mechanisms, m];
    expect(mechanismsOfKind(all, 'weather').map((x) => x.id)).toContain('made_up');
    expect(mechanismLine(m)).toEqual({ inputs: 'Alpha', title: 'Made up', outputs: 'Beta' });
    expect(nodeLines(made, 'mechanism:made_up')).toEqual({ inputs: 'Alpha (Starting point)', outputs: 'Beta (End point)', upstream: [{ label: 'Beta', causes: 'Alpha' }] });
  });
});

describe('live values on causes (contract v2.9)', () => {
  const c = ok(sim.catalog());
  const played = createSimPort(7);
  ok(played.submitTurn([]));
  const snap = ok(played.importState(ok(played.exportState())));
  it('every bound quantity has a value or a reason: bound ones read as a phrase, unbound ones are null', () => {
    const live = liveLookup(c, snap);
    const bound = c.quantities.filter((q) => q.live.bound);
    const unbound = c.quantities.filter((q) => !q.live.bound);
    expect(bound.length).toBeGreaterThan(20);
    expect(unbound.length).toBeGreaterThan(20);
    for (const q of unbound) {
      expect(c.causal.live.unbound.map((r) => r.id), q.id).toContain(q.live.unbound);
      expect(live(q.id), q.id).toBeNull();
    }
    expect(live('unemployment')).toMatch(/\d/);
  });
  it('every quantity id the snapshot links (breakdown inputs, market price and drivers) is in the catalogue', () => {
    const ids = new Set(c.quantities.map((q) => q.id));
    const linked: string[] = [];
    const walk = (v: unknown): void => {
      if (Array.isArray(v)) v.forEach(walk);
      else if (v && typeof v === 'object') for (const [k, x] of Object.entries(v)) { if ((k === 'quantityId' || k === 'priceQuantity') && typeof x === 'string') linked.push(x); else walk(x); }
    };
    walk(snap.breakdowns);
    walk(snap.markets);
    expect(linked.length).toBeGreaterThan(5);
    for (const id of linked) expect(ids.has(id), id).toBe(true);
  });
  it('a made-up binding and value added only in the wire shows beside its cause with no web change', () => {
    const g = c.causal;
    const node = (id: string, kind: 'quantity' | 'decision', role: string, label: string, upstream: string[] = []) => ({ id, kind, role, refId: id.split(':')[1]!, label, owner: 'firm', selfLoop: false, component: 99, upstream, inLoops: [] as string[] });
    const made = {
      ...g,
      nodes: [...g.nodes, node('quantity:qa', 'quantity', 'source', 'Alpha'), node('quantity:qb', 'quantity', 'sink', 'Beta', ['quantity:qa']), node('decision:made_up', 'decision', 'decision', 'Made up')],
      edges: [...g.edges, { from: 'quantity:qa', to: 'decision:made_up', kind: 'reads', via: null, selfLoop: false }, { from: 'decision:made_up', to: 'quantity:qb', kind: 'writes', via: null, selfLoop: false }],
    };
    const cat2 = { ...c, quantities: [...c.quantities, { id: 'qa', label: 'Alpha', unit: 'widgets', owner: 'firm', summary: '', live: { bound: true, path: 'x', aggregate: 'town', hasTrend: true } }] };
    const snap2 = { ...snap, quantities: [...snap.quantities, { id: 'qa', value: 3, previous: 2, trend: 'rising' }] };
    const out = causalLines(made, 'made_up', liveLookup(cat2, snap2));
    expect(out.upstream).toEqual([{ label: 'Beta', causes: `Alpha (3 widgets, ${c.markets.directions.rising})` }]);
    // No lookup, no value: the old text.
    expect(causalLines(made, 'made_up').upstream[0]!.causes).toBe('Alpha');
  });
  it('the "what changed" labels are catalogue text, not literals in the web', () => {
    for (const k of ['what_changed_title', 'what_changed_button', 'what_changed_short', 'what_changed_aria', 'what_changed_empty']) expect(c.text.misc?.[k], k).toBeTruthy();
    expect(c.text.misc!.what_changed_title).toBe('What changed this turn');
  });
});

describe.skipIf(!fixture)('agents catalogue (real fixture docs/fixtures/agents.json)', () => {
  it('equals the catalogue the wasm simulation serves', () => {
    const fx = ((fixture as unknown as { value?: AgentsCatalog }).value ?? fixture)!;
    expect(JSON.parse(JSON.stringify(cat.kinds))).toEqual(fx.kinds);
    expect(JSON.parse(JSON.stringify(cat.randomness))).toEqual(fx.randomness);
    expect(cat.deterministic).toEqual(fx.deterministic);
  });
});

describe('signed edges (contract v2.11)', () => {
  const c = ok(sim.catalog());
  const g = c.causal;
  it('every drives edge carries a sign, a lag and a sentence, and reads off as a line', () => {
    const drives = g.edges.filter((e) => e.kind === 'drives');
    expect(drives.length).toBe(331);
    for (const e of drives) {
      expect(e.effect, `${e.from} -> ${e.to} via ${e.via}`).toBeTruthy();
      expect(['up', 'down', 'varies', 'none']).toContain(e.effect!.sign);
      expect(e.effect!.sentence.length).toBeGreaterThan(20);
    }
    expect(g.edges.filter((e) => e.kind !== 'drives').every((e) => !e.effect)).toBe(true);
    const lines = effectLines(g, 'mechanism:unit_cost_buildup');
    expect(lines.map((l) => l.text)).toContain('Base wage (+) -> Unit cost, the same month');
    const w = lines.find((l) => l.text.startsWith('Base wage'))!;
    expect(w.sentence).toMatch(/^When base wage goes up, unit cost goes up \(the same month\), because /);
  });
  it('a sign added only in the catalogue renders with no web change', () => {
    const node = (id: string, kind: string, role: string, label: string) => ({ id, kind, role, refId: id.split(':')[1]!, label, owner: 'firm', selfLoop: false, component: 99, upstream: [] as string[], inLoops: [] as string[] });
    const fx = (sign: string, lagLabel: string) => ({ sign, lagMonths: 3, basis: 'perturbed', lagLabel, sentence: `When alpha goes up, beta goes ${sign}.`, checked: 'Checked.' });
    const made = {
      ...g,
      nodes: [...g.nodes, node('quantity:qa', 'quantity', 'source', 'Alpha'), node('quantity:qb', 'quantity', 'sink', 'Beta'), node('mechanism:made_up', 'mechanism', 'mechanism', 'Made up')],
      edges: [...g.edges, { from: 'quantity:qa', to: 'quantity:qb', kind: 'drives', via: 'mechanism:made_up', selfLoop: false, effect: fx('down', '3 months later') }],
    };
    const [line] = effectLines(made, 'mechanism:made_up');
    expect(line).toMatchObject({ text: 'Alpha (-) -> Beta, 3 months later', sign: 'down', sentence: 'When alpha goes up, beta goes down.' });
  });
});

describe('feedback loops (contract v2.12)', () => {
  const c = ok(sim.catalog());
  const g = c.causal;
  it('the wire serves loops with kinds, sentences and inLoops, and they read off as lines', () => {
    expect(g.loops.length).toBeGreaterThan(100);
    expect(g.loopKinds.map((k) => k.id)).toEqual(['R', 'B', 'depends']);
    const lines = loopLines(g, ['mechanism:climate_stress_curve']);
    expect(lines.length).toBeGreaterThan(0);
    for (const l of lines) {
      expect(['R', 'B', '?']).toContain(l.badge);
      expect(l.sentence).toMatch(/^This is an? (reinforcing|balancing|mixed) loop: /);
      expect(l.chain.split(' -> ').length).toBeGreaterThan(1);
    }
    const firstSingle = lines.findIndex((l) => !l.chain.slice(0, -1).includes(' -> '));
    if (firstSingle >= 0) expect(lines.slice(firstSingle).every((l) => !l.chain.slice(0, -1).includes(' -> '))).toBe(true);
    expect(lines.some((l) => l.title === 'Overshoot bites' && l.kind === 'B')).toBe(true);
    expect(loopSummary(g, 'mechanism:climate_stress_curve')).toMatch(/^In \d+ feedback loops?: /);
    expect(loopSummary(g, 'quantity:nonexistent')).toBe('');
  });
  it('a loop added only in the catalogue renders with no web change', () => {
    const node = (id: string, label: string, inLoops: string[]) => ({ id, kind: 'quantity', role: 'derived', refId: id.split(':')[1]!, label, owner: 'firm', selfLoop: false, component: 99, upstream: [] as string[], inLoops });
    const made = {
      ...g,
      nodes: [...g.nodes, node('quantity:qa', 'Alpha', ['loop:qa+qb']), node('quantity:qb', 'Beta', ['loop:qa+qb']), { ...node('mechanism:made_up', 'Made up', ['loop:qa+qb']), kind: 'mechanism', role: 'mechanism' }],
      loops: [...g.loops, { id: 'loop:qa+qb', kind: 'B', members: ['quantity:qa', 'quantity:qb'], edges: [{ via: 'mechanism:made_up', sign: 'up', lagMonths: 1 }, { via: 'mechanism:made_up', sign: 'down', lagMonths: 2 }], delayMonths: 3, delayLabel: 'About 3 months to come round the loop.', sentence: 'This is a balancing loop: when alpha goes up, beta goes up, then alpha goes down again.', incomplete: true, incompleteReasons: ['lever'], scc: 0 }],
    };
    const [l] = loopLines(made, ['mechanism:made_up']);
    expect(l).toMatchObject({ id: 'loop:qa+qb', kind: 'B', badge: 'B', kindLabel: 'Balancing', title: 'Alpha -> Beta -> Alpha', delay: 'About 3 months to come round the loop.' });
    expect(l!.incomplete).toEqual([g.loopIncomplete.find((r) => r.id === 'lever')!.label]);
    expect(loopSummary(made, 'mechanism:made_up')).toBe('In 1 feedback loop: 1 balancing');
  });
});
