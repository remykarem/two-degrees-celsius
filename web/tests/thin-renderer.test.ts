import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ActionInfo, BlockReason, Catalog, CommandPreview, LeverInfo, SimEvent, Snapshot } from '../src/contract';
import { createMockSim } from '../src/sim-port/mock';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import { actionModel, estimatePc } from '../src/panels/costs';
import { REASON_TEXT } from '../src/panels/format';
import { ok } from './helpers';

// Town v6 (docs/town-v6.md): the web is a thin renderer. These tests keep it so: every catalog key the code names exists, every wire id the web words has words,
// the mock has the same shapes, labels and lever table as the real wire (docs/fixtures), and the cost fallback agrees with the simulation.

const repo = resolve(__dirname, '..', '..');
const fixture = <T>(name: string): T => JSON.parse(readFileSync(resolve(repo, 'docs', 'fixtures', name), 'utf8')) as T;
const files = (dir: string): string[] => readdirSync(dir).flatMap((f) => { const p = join(dir, f); return statSync(p).isDirectory() ? files(p) : [p]; });

describe('catalog text', () => {
  const cat = fixture<Catalog>('catalog.json');
  const src = files(resolve(__dirname, '..', 'src')).filter((f) => /\.(ts|svelte)$/.test(f));
  it('every text(group, key) the code names exists in content/ui.json', () => {
    const missing: string[] = [];
    let n = 0;
    for (const f of src) {
      for (const m of readFileSync(f, 'utf8').matchAll(/\btext\(\s*'(\w+)'\s*,\s*'([\w.-]+)'/g)) {
        n++;
        if (cat.text[m[1]!]?.[m[2]!] === undefined) missing.push(`${f.replace(repo, '')}: ${m[1]}.${m[2]}`);
      }
    }
    expect(n).toBeGreaterThan(30);
    expect(missing).toEqual([]);
  });
  it('every wire id the web labels has a label: ledger kinds, classes, infra kinds, owners, metrics', () => {
    const t = cat.text;
    expect(Object.keys(t.ledger_kind!).sort()).toEqual(['bailout', 'benefit', 'build', 'dividend', 'interest', 'loan_new', 'loan_repay', 'loan_writeoff', 'lobby', 'neutral', 'purchase', 'rent', 'subsidy', 'tax', 'upkeep', 'wage', 'wealth_consumption']);
    expect(Object.keys(t.class_label!).sort()).toEqual(['capitalist', 'dependent', 'manager', 'rentier', 'worker']);
    expect(Object.keys(t.infra_label!).sort()).toEqual(['fire_station', 'fossil_plant', 'public_housing', 'renewable_plant', 'road']);
    expect(Object.keys(t.owner_text!).sort()).toEqual(['bank', 'household', 'state', 'workers']);
  });
  it('every authored finance label and hint names a row the finance tab has', () => {
    for (const k of Object.keys(cat.text.finance_label!)) expect(k).toMatch(/^(revenue|spending|income)\./);
  });
});

describe('wire ids the web words (the web owns this wording, so every id must have some)', () => {
  it('every BlockReason has text', () => {
    const all: Record<BlockReason, true> = { transition_in_progress: true, nothing_to_remove: true, same_system: true, lever_unavailable: true, insufficient_political_capital: true, insufficient_funds: true, invalid_command: true, tile_occupied: true, tile_invalid: true };
    const rust = readFileSync(resolve(repo, 'crates/sim-api/src/keys.rs'), 'utf8');
    const body = rust.slice(rust.indexOf('pub enum BlockReason'), rust.indexOf('}', rust.indexOf('pub enum BlockReason')));
    const wire = [...body.matchAll(/^\s{4}([A-Z]\w+),/gm)].map((m) => m[1]!.replace(/([a-z])([A-Z])/g, '$1_$2').toLowerCase()).sort();
    expect(wire).toEqual(Object.keys(all).sort());
    for (const r of Object.keys(all)) expect(REASON_TEXT[r], r).toBeTruthy();
  });
  it('every SimEvent kind has a case in formatEvent', () => {
    const all: Record<SimEvent['type'], true> = {
      commuted: true, person_hired: true, person_fired: true, household_moved: true, household_evicted: true, built: true, removed: true, electricity_rationed: true, purchase: true, hired: true, fired: true,
      loan_defaulted: true, firm_founded: true, firm_closed: true, forest_shrank: true, commons_collapsed: true, commons_enclosed: true, policy_blocked: true, command_activated: true, bank_failed: true,
      rent_shortfall: true, spending_cut: true, bank_bailout: true, class_changed: true, system_switch_started: true, system_switch_completed: true, budget_exhausted: true, collapse: true,
      fire_started: true, fire_spread: true, fire_out: true, forest_burned: true, drought_started: true, drought_ended: true, home_damaged: true,
      private_jet_flight: true, pilot_let_go: true, pilot_hired: true,
    };
    const rust = readFileSync(resolve(repo, 'crates/sim-api/src/events.rs'), 'utf8');
    const start = rust.indexOf('pub enum SimEventKind');
    const body = rust.slice(start, rust.indexOf('\n}\n', start));
    const wire = [...body.matchAll(/^ {4}([A-Z]\w+)(?: \{|,)/gm)].map((m) => m[1]!.replace(/([a-z])([A-Z])/g, '$1_$2').toLowerCase()).sort();
    // `hired` and `fired` are cohort-era kinds the wire no longer sends (docs/known-issues.md); the web still words them.
    expect(wire).toEqual(Object.keys(all).filter((k) => k !== 'hired' && k !== 'fired').sort());
    const fmt = readFileSync(resolve(__dirname, '..', 'src/panels/format.ts'), 'utf8');
    for (const k of Object.keys(all)) expect(fmt, k).toContain(`case '${k}'`);
  });
});

/** The set of keys of a value, nested; arrays use their first element. Optional fields appear on the real side only when a fixture has them. */
function shape(v: unknown, path = ''): string[] {
  if (Array.isArray(v)) return v.length ? shape(v[0], `${path}[]`) : [];
  if (v && typeof v === 'object') return Object.entries(v).flatMap(([k, x]) => [`${path}.${k}`, ...shape(x, `${path}.${k}`)]);
  return [];
}

/** Optional detail the mock's transparency data (breakdowns, policies, industry) does not carry. Listed, so a NEW difference fails the test; see docs/known-issues.md. */
const KNOWN_MOCK_GAPS = [
  '.breakdowns.wageShare.parts', '.breakdowns.wageShare.sumsToResult', '.breakdowns.wageShare.inputs[].source', '.breakdowns.profitShare.parts', '.breakdowns.profitShare.sumsToResult',
  '.breakdowns.debtToGdp.inputs[].source', '.breakdowns.legitimacy.inputs[].source', '.breakdowns.emissions.monthLines', '.breakdowns.emissions.turnNumber', '.breakdowns.emissions.turnMonths',
  '.breakdowns.taxes[]', '.policies.cancelRefund', '.industry.firms[].active', '.industry.infrastructure[].upkeepEachMonthly',
];

describe('mock parity with the real wire (docs/fixtures)', () => {
  it('levers(): same keys, labels, groups, units, ranges, arrows and availability as content/levers.json', () => {
    const real = fixture<LeverInfo[]>('levers.json');
    const mock = createMockSim().levers();
    expect(mock.map((l) => l.key)).toEqual(real.map((l) => l.key));
    for (const r of real) {
      const m = mock.find((l) => l.key === r.key)!;
      const pick = (l: LeverInfo) => ({ label: l.label, group: l.group, unit: l.unit, min: l.min, max: l.max, step: l.step, lobbied: l.lobbied, arrows: l.arrows, availableIn: l.availableIn, options: l.options, optionAvailableIn: l.optionAvailableIn, zeroMeansNone: l.zeroMeansNone });
      expect(pick(m), r.key).toEqual(pick(r));
      expect(Object.keys(m).sort(), r.key).toEqual(Object.keys(r).sort());
      expect(m.baseCostPc).toBe(r.baseCostPc); expect(m.leadTime).toBe(r.leadTime); expect(m.costModel).toEqual(r.costModel);
    }
  });
  it('actions(): same keys, labels, costs and availability', () => {
    const real = fixture<ActionInfo[]>('actions.json');
    const mock = createMockSim().actions();
    expect(mock.map((a) => a.key)).toEqual(real.map((a) => a.key));
    for (const r of real) expect(mock.find((a) => a.key === r.key), r.key).toEqual(r);
  });
  it('catalog(): the mock serves the real reply', () => {
    expect(ok(createMockSim().catalog())).toEqual(fixture<Catalog>('catalog.json'));
  });
  it('snapshot: the mock has every key the real snapshot has (and none the wire lacks, bar its mock flag)', () => {
    const real = fixture<Snapshot>('snapshot-turn1.json');
    const mock = ok(createMockSim().init(1));
    const r = new Set(shape(real)), m = new Set(shape(mock));
    // fields that exist only when something happened (an event, a nullable record) and so may be absent from either side in one sample
    const sample = /\.(gameOver|turnPhase|history\[\]\.\w+|weather\.(drought|fires)\b.*|pending\b.*|tiles\[\]\.(ref|emissionsMonthly|state|fireRisk)|map\.tiles\[\]\.(ref|emissionsMonthly|state|fireRisk))$/;
    const onlyReal = [...r].filter((k) => !m.has(k) && !sample.test(k) && !KNOWN_MOCK_GAPS.some((g) => k.startsWith(g)));
    const onlyMock = [...m].filter((k) => !r.has(k) && k !== '.mock' && !sample.test(k) && !KNOWN_MOCK_GAPS.some((g) => k.startsWith(g)));
    expect({ onlyReal, onlyMock }).toEqual({ onlyReal: [], onlyMock: [] });
  });
  it('finance and energy views have the same key sets', () => {
    const real = fixture<Snapshot>('snapshot-turn1.json');
    const mock = ok(createMockSim().submitTurn([])).snapshot;
    for (const k of ['finance', 'energy'] as const) expect(new Set(shape(mock[k])), k).toEqual(new Set(shape(real[k])));
  });
});

const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
describe.skipIf(!existsSync(resolve(pkg, 'sim_wasm_bg.wasm')))('cost fallback vs the real preview()', async () => {
  const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  const port = createWasmPort(mod);
  ok(port.init(3));
  const levers = port.levers(), actions = port.actions();
  it('estimatePc (the labelled fallback in panels/costs.ts) equals the sim\'s preview().pc when opposition is 0', () => {
    const changes: [LeverInfo['key'], number][] = [['tax_wage', 0.4], ['tax_capital', 0.5], ['benefit', 1500], ['min_wage', 1500], ['public_spending', 12000], ['base_rate', 0.08], ['carbon_price', 50]];
    let checked = 0;
    for (const [key, to] of changes) {
      const l = levers.find((x) => x.key === key)!;
      const pv = ok(port.preview([{ type: 'set_policy', lever: key, value: to }]))[0] as CommandPreview;
      if (pv.blocked || pv.opposition !== 0) continue;
      const cur = ok(port.init(3)).policy[key]!.nominal as number;
      expect(pv.pc, key).toBeCloseTo(estimatePc(l.costModel, Math.abs(to - cur) / l.costModel.unit), 9);
      checked++;
    }
    expect(checked).toBeGreaterThanOrEqual(5);
    const sw = actions.find((a) => a.key === 'switch_system')!;
    const pv = ok(port.preview([{ type: 'switch_system', to: 'regulated' }]))[0] as CommandPreview;
    expect(pv.pc).toBeCloseTo(estimatePc(actionModel(sw), 1), 9);
  });
});
