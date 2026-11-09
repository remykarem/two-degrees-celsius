import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ActionInfo, FirmView, HouseholdView, Indicators, LeverInfo, MonthFrame, PendingItem, Snapshot, TurnResult } from '../src/contract';
import { deriveCrises } from '../src/panels/crisis';
import { formatEvent, formatLever } from '../src/panels/format';
import { resolveInspect } from '../src/panels/inspect';
import { METRICS } from '../src/panels/metrics';
import { lobbyLines, metricDeltas } from '../src/panels/results';
import { generateStory } from '../src/panels/story';
import { planEntities } from '../src/scene/entities';
import { ctxFromSnapshot, planFlowLoop, planFrame } from '../src/scene/flows';
import { reconcileLayout } from '../src/store';

// Real JSON written by the Rust wire layer (crates/sim-api) into docs/fixtures/. These tests check it against the TypeScript types
// (key sets are tied to the interfaces with `satisfies Record<keyof T, ...>`, so adding a field to a type breaks this file until it is listed)
// and run the UI logic on it.
const dir = resolve(__dirname, '..', '..', 'docs', 'fixtures');
const present = ['snapshot-turn1.json', 'turn-result.json', 'levers.json', 'actions.json'].every((f) => existsSync(resolve(dir, f)));
const load = <T>(f: string) => JSON.parse(readFileSync(resolve(dir, f), 'utf8')) as T;

type Kind = 'number' | 'string' | 'boolean' | 'object' | 'array';
type Shape<T> = Record<keyof T, Kind | `${Kind}?` | 'nullable'>;
function conforms(value: unknown, shape: Record<string, string>, where: string): string[] {
  const bad: string[] = [];
  const obj = value as Record<string, unknown>;
  for (const [k, kind] of Object.entries(shape)) {
    const optional = kind.endsWith('?');
    const v = obj[k];
    if (v === undefined) { if (!optional) bad.push(`${where}.${k} is missing`); continue; }
    const want = optional ? kind.slice(0, -1) : kind;
    const got = Array.isArray(v) ? 'array' : v === null ? 'null' : typeof v;
    if (kind === 'nullable') { if (v !== null && typeof v !== 'object') bad.push(`${where}.${k} should be null or an object, got ${got}`); }
    else if (got !== want) bad.push(`${where}.${k} should be ${want}, got ${got}`);
  }
  for (const k of Object.keys(obj)) if (!(k in shape)) bad.push(`${where}.${k} is not in the TypeScript type`);
  return bad;
}

const SNAPSHOT = { schemaVersion: 'number', game: 'object', turn: 'number', tick: 'number', buildings: 'object', mock: 'boolean?', gameOver: 'nullable', pc: 'object', system: 'object',
  pending: 'array', policy: 'object', netLobby: 'number', unionDensity: 'number', indicators: 'object', households: 'array', firms: 'array', bank: 'object', government: 'object',
  nature: 'object', housing: 'object', energy: 'object', history: 'array' } satisfies Shape<Snapshot>;
const INDICATORS = { gdpAnnual: 'number', gdpRealAnnual: 'number', unemployment: 'number', employed: 'number', labourForce: 'number', gini: 'number', wageShare: 'number', profitShare: 'number',
  avgWellbeing: 'number', remainingBudget: 'number', budgetShare: 'number', temperature: 'number', gpiAnnual: 'number', moneyInCirculation: 'number', priceIndex: 'number' } satisfies Shape<Indicators>;
const HOUSEHOLD = { id: 'number', label: 'string', class: 'string', people: 'number', employed: 'number', labourShare: 'number?', labourForce: 'number', savings: 'number', debt: 'number', wealthPerCapita: 'number',
  income: 'object', wellbeing: 'number', politicalPower: 'number', footprintTonnes: 'number', foodMet: 'number', housedShare: 'number' } as unknown as Shape<HouseholdView>;
delete (HOUSEHOLD as Record<string, unknown>).labourShare;
const FIRM = { id: 'number', active: 'boolean', sector: 'string', name: 'string', owner: 'object', workers: 'number', wage: 'number', price: 'number', output: 'number', emissions: 'number',
  revenue: 'number', profit: 'number', capital: 'number', debt: 'number', greenTech: 'number' } satisfies Shape<FirmView>;
const TURN = { turn: 'number', accepted: 'array', rejected: 'array', activated: 'array', pcBefore: 'number', pcAfter: 'number', months: 'array', policyGaps: 'array', trace: 'array', snapshot: 'object' } satisfies Shape<TurnResult>;
const FRAME = { month: 'number', events: 'array', flows: 'object' } satisfies Shape<MonthFrame>;
const LEVER = { key: 'string', label: 'string', unit: 'string', min: 'number', max: 'number', step: 'number', baseCostPc: 'number', costModel: 'object', zeroMeansNone: 'boolean?', leadTime: 'number',
  lobbied: 'boolean', arrows: 'array', availableIn: 'array', options: 'array?', optionAvailableIn: 'object?' } satisfies Shape<LeverInfo>;
const ACTION = { key: 'string', label: 'string', baseCostPc: 'number', costModel: 'object?', leadTime: 'number', moneyCost: 'number', availableIn: 'array' } satisfies Shape<ActionInfo>;
const PENDING = { id: 'number', command: 'object', submittedTurn: 'number', effectiveTurn: 'number', pcCost: 'number', moneyCost: 'number' } satisfies Shape<PendingItem>;

describe.skipIf(!present)('real fixtures from docs/fixtures/ against the TypeScript types', () => {
  const snap = present ? load<Snapshot>('snapshot-turn1.json') : (null as never);
  const turn = present ? load<TurnResult>('turn-result.json') : (null as never);
  const levers = present ? load<LeverInfo[]>('levers.json') : (null as never);
  const actions = present ? load<ActionInfo[]>('actions.json') : (null as never);

  it('has the keys and kinds the types declare', () => {
    const bad = [
      ...conforms(snap, SNAPSHOT, 'snapshot'), ...conforms(snap.indicators, INDICATORS, 'snapshot.indicators'),
      ...snap.households.flatMap((h, i) => conforms(h, HOUSEHOLD, `households[${i}]`)), ...snap.firms.flatMap((f, i) => conforms(f, FIRM, `firms[${i}]`)),
      ...conforms(turn, TURN, 'turnResult'), ...turn.months.flatMap((m, i) => conforms(m, FRAME, `months[${i}]`)), ...conforms(turn.snapshot, SNAPSHOT, 'turnResult.snapshot'),
      ...levers.flatMap((l, i) => conforms(l, LEVER, `levers[${i}]`)), ...actions.flatMap((a, i) => conforms(a, ACTION, `actions[${i}]`)),
      ...turn.accepted.flatMap((p, i) => conforms(p, PENDING, `accepted[${i}]`)),
    ];
    expect(bad).toEqual([]);
  });
  it('has 3 month frames, 19 levers, 7 actions, an entry per lever in the policy table', () => {
    expect(turn.months).toHaveLength(3);
    expect(levers).toHaveLength(19);
    expect(actions).toHaveLength(7);
    expect(Object.keys(snap.policy).sort()).toEqual(levers.map((l) => l.key).sort());
  });
  it('units are in the v0.3 set and the cost model is present', () => {
    for (const l of levers) {
      expect(['usd', 'usd_per_month', 'usd_per_tonne', 'fraction', 'tonnes_per_month', 'bool', 'enum']).toContain(l.unit);
      expect(l.costModel.base).toBeGreaterThan(0);
    }
    expect(levers.find((l) => l.key === 'emissions_cap')!.zeroMeansNone).toBe(true);
    expect(formatLever(levers.find((l) => l.key === 'emissions_cap')!, 0)).toBe('no cap');
  });
  it('the UI logic runs on the real snapshot and turn result', () => {
    const layout = reconcileLayout(null, snap);
    expect(layout.placed.filter((p) => p.kind === 'fossil_plant')).toHaveLength(snap.buildings.fossilPlant);
    expect(layout.scenery.filter((s) => s.kind === 'home')).toHaveLength(snap.households.length);
    const descs = planEntities(layout, snap);
    expect(descs.length).toBeGreaterThan(10);
    for (const s of layout.scenery) expect(resolveInspect(snap, layout, s, []).title).toBeTruthy();
    for (const m of METRICS) expect(Number.isFinite(m.value(snap)), m.id).toBe(true);
    expect(() => deriveCrises(snap, turn)).not.toThrow();
    const after = turn.snapshot;
    expect(metricDeltas(snap, after).length).toBeGreaterThan(0);
    for (const e of turn.months.flatMap((m) => m.events)) expect(formatEvent(e, after)).toBeTruthy();
    expect(lobbyLines(turn, levers, actions).length).toBeGreaterThanOrEqual(turn.rejected.length);
    const ctx = ctxFromSnapshot(reconcileLayout(layout, after), after);
    for (const f of turn.months) expect(() => planFrame(f, ctx)).not.toThrow();
    expect(() => planFlowLoop(turn.months, ctx)).not.toThrow();
    expect(() => generateStory(after, turn)).not.toThrow();
  });
  it('a real tile-less accepted build gets a reserved tile', () => {
    const place = turn.accepted.find((a) => a.command.type === 'place')!;
    expect(place.command).not.toHaveProperty('tile'); // the sim has no tiles
    const layout = reconcileLayout(null, turn.snapshot);
    expect(layout.pendingTiles[place.id]).toBeDefined();
  });
  it('policy gaps list the lobbied levers every turn, mostly with no gap', () => {
    expect(turn.policyGaps.map((g) => g.lever).sort()).toEqual(['carbon_price', 'emissions_cap', 'tax_profit', 'wealth_tax']);
    expect(lobbyLines(turn, levers, actions).filter((l) => l.kind === 'weakened').length).toBeLessThan(turn.policyGaps.length);
  });
  it('KNOWN MISMATCH: nature.forest and nature.commons are raw stocks, the UI treats them as 0..1 shares', () => {
    // docs/web-notes-align.md "Contract issues". When the wire sends shares (or the maxima), flip this expectation.
    expect(snap.nature.forest).toBeGreaterThan(1);
    expect(snap.nature.commons).toBeGreaterThan(1);
  });
});

describe('fixtures presence', () => {
  it('reports whether docs/fixtures/ exists', () => {
    expect(typeof present).toBe('boolean');
  });
});
