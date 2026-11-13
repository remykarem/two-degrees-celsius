import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { AgentsCatalog, DecisionInfo } from '../src/contract';
import { isBarePercent } from '../src/panels/backing';
import { fmtUnit } from '../src/panels/explain';
import { agentsAnchorFor, decisionLine, decisionsOfKind, decisionTag, isAgentsAnchor, kindForAnchor, searchAgents } from '../src/panels/explorer';
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
      expect(['firms', 'households']).toContain(d.agentKind);
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
    expect(decisionsOfKind(catalog.decisions, 'bank')).toEqual([]);
    const l = decisionLine(mine.find((d) => d.id === 'firm_wage')!);
    expect(l.title).toBe('Wage');
    expect(l.inputs).toContain('Unemployment');
    expect(l.outputs).toBe('Base wage');
  });
  it('a decision added only in the catalogue appears with no web change', () => {
    const made: DecisionInfo = { ...catalog.decisions[0]!, id: 'bank_made_up', title: 'Made up', agentKind: 'bank', inputs: [{ id: 'q1', label: 'Alpha', unit: 'x' }], outputs: [{ id: 'q2', label: 'Beta', unit: 'y' }] };
    const all = [...catalog.decisions, made];
    expect(decisionsOfKind(all, 'bank').map((d) => d.id)).toEqual(['bank_made_up']);
    expect(decisionLine(decisionsOfKind(all, 'bank')[0]!)).toEqual({ inputs: 'Alpha', title: 'Made up', outputs: 'Beta' });
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
