import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { AgentsCatalog } from '../src/contract';
import { isBarePercent } from '../src/panels/backing';
import { fmtUnit } from '../src/panels/explain';
import { agentsAnchorFor, decisionTag, isAgentsAnchor, kindForAnchor, searchAgents } from '../src/panels/explorer';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import { createMockSim } from '../src/sim-port/mock';
import { ok } from './helpers';

const sim = createMockSim();

// Contract v1.7: the agents catalogue (mock generated from content/, real fixture, real wasm when built).
const mock = ok(sim.agents());
const fixturePath = resolve(__dirname, '..', '..', 'docs', 'fixtures', 'agents.json');
const fixture = existsSync(fixturePath) ? (JSON.parse(readFileSync(fixturePath, 'utf8')) as AgentsCatalog) : null;

describe('agents catalogue (mock)', () => {
  it('has every kind, the five classes and a randomness table with live parameters', () => {
    expect(mock.kinds.map((k) => k.id)).toEqual(['households', 'firms', 'bank', 'government', 'lobbyists', 'nature', 'weather', 'estate']);
    expect(mock.kinds[0]!.variants.map((v) => v.id)).toEqual(['capitalist', 'rentier', 'manager', 'worker', 'dependent']);
    expect(mock.randomness.length).toBeGreaterThanOrEqual(8);
    for (const k of mock.kinds) expect(k.params.every((p) => p.value !== undefined), k.id).toBe(true);
  });
  it('no parameter value is shown as a bare percentage', () => {
    const all = [...mock.kinds.flatMap((k) => [...k.params, ...k.classParams]), ...mock.randomness.flatMap((r) => r.params)];
    for (const p of all) if (typeof p.value === 'number') expect(isBarePercent(fmtUnit(p.value, p.unit)), p.path).toBe(false);
  });
  it('searches kinds and draws', () => {
    expect(searchAgents(mock, 'drought').draws.map((d) => d.id)).toContain('drought_onset');
    expect(searchAgents(mock, 'lobby').kinds.map((k) => k.id)).toContain('lobbyists');
    expect(searchAgents(mock, 'zzzznothing')).toEqual({ kinds: [], draws: [] });
    expect(searchAgents(mock, '').kinds).toHaveLength(mock.kinds.length);
  });
  it('resolves links: a class opens Households, a bare anchor opens the first kind', () => {
    expect(isAgentsAnchor(agentsAnchorFor('worker'))).toBe(true);
    expect(isAgentsAnchor('emissions')).toBe(false);
    expect(kindForAnchor(mock, 'agents:worker')!.id).toBe('households');
    expect(kindForAnchor(mock, 'agents:bank')!.id).toBe('bank');
    expect(kindForAnchor(mock, 'agents')!.id).toBe('households');
  });
  it('gives a person the decision tag of their household class', () => {
    const snap = ok(sim.init(1));
    const h = snap.households[0]!;
    const tag = decisionTag(mock, snap, h.members[0]!);
    expect(tag!.cls).toBe(h.class);
    expect(tag!.variant.decision.length).toBeGreaterThan(10);
    expect(decisionTag(null, snap, 0)).toBeNull();
    expect(decisionTag(mock, snap, 9999)).toBeNull();
  });
});

describe.skipIf(!fixture)('agents catalogue (real fixture docs/fixtures/agents.json)', () => {
  it('matches the mock text and parameter values (the mock only appends its own note)', () => {
    const real = ((fixture as unknown as { value?: AgentsCatalog }).value ?? fixture)!;
    expect(mock.kinds).toEqual(real.kinds);
    expect(mock.randomness).toEqual(real.randomness);
    expect(mock.deterministic.slice(0, real.deterministic.length)).toEqual(real.deterministic);
  });
});

const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
describe.skipIf(!existsSync(resolve(pkg, 'sim_wasm_bg.wasm')))('agents catalogue on the real wasm sim', async () => {
  const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  const port = createWasmPort(mod);
  it('returns the same catalogue as the fixture', () => {
    ok(port.init(1));
    const r = port.agents();
    if (!r.ok) { expect(r.error.code).toBe('unsupported'); return; }
    expect(r.value.kinds).toEqual(mock.kinds);
    expect(r.value.randomness).toEqual(mock.randomness);
  });
});
