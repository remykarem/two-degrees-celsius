import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ActionInfo, FirmView, HomeView, HouseholdView, Indicators, LeverInfo, MonthFrame, PendingItem, PersonView, PlantView, Snapshot, Tile, TurnResult } from '../src/contract';
import { deriveCrises } from '../src/panels/crisis';
import { formatEvent } from '../src/panels/format';
import { resolveSelection, resolveTile } from '../src/panels/inspect';
import { METRICS } from '../src/panels/metrics';
import { lobbyLines, metricDeltas } from '../src/panels/results';
import { generateStory } from '../src/panels/story';
import { planEntities } from '../src/scene/entities';
import { ctxFromSnapshot, planFlowLoop, planFrame } from '../src/scene/flows';
import { forecastView, healthMeters, layoutOf, weatherLook } from '../src/store';

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
    if (kind === 'nullable') { if (v !== null && typeof v !== 'object' && typeof v !== 'number') bad.push(`${where}.${k} should be null, a number or an object, got ${got}`); }
    else if (got !== want) bad.push(`${where}.${k} should be ${want}, got ${got}`);
  }
  for (const k of Object.keys(obj)) if (!(k in shape)) bad.push(`${where}.${k} is not in the TypeScript type`);
  return bad;
}

const SNAPSHOT = { schemaVersion: 'number', game: 'object', turn: 'number', tick: 'number', map: 'object', buildings: 'object', mock: 'boolean?', gameOver: 'nullable', pc: 'object', system: 'object',
  pending: 'array', policy: 'object', netLobby: 'number', unionDensity: 'number', indicators: 'object', people: 'array', households: 'array', homes: 'array', plants: 'array', firms: 'array',
  bank: 'object', government: 'object', nature: 'object', housing: 'object', energy: 'object', history: 'array', turnPhase: 'object?', weather: 'object?' } satisfies Shape<Snapshot>; // weather: contract v1.2, optional until docs/fixtures carry it
const INDICATORS = { gdpAnnual: 'number', gdpRealAnnual: 'number', unemployment: 'number', employed: 'number', labourForce: 'number', gini: 'number', wageShare: 'number', profitShare: 'number',
  avgWellbeing: 'number', remainingBudget: 'number', budgetShare: 'number', temperature: 'number', gpiAnnual: 'number', moneyInCirculation: 'number', priceIndex: 'number',
  avgIncomePerPerson: 'number', medianIncomePerPerson: 'number', topHouseholdWealthShare: 'number' } satisfies Shape<Indicators>;
const PERSON = { id: 'number', label: 'string', household: 'number', ageBand: 'string', skill: 'number', job: 'nullable', wage: 'number', commuteTiles: 'number', commuteMinutes: 'number', wellbeing: 'number',
  inLabourForce: 'boolean', jobGuarantee: 'boolean', effectiveHours: 'number' } satisfies Shape<PersonView>;
const HOUSEHOLD = { id: 'number', label: 'string', members: 'array', home: 'nullable', class: 'string', savings: 'number', debt: 'number', netWorth: 'number', income: 'object', politicalPower: 'number',
  arrearsMonths: 'number', wellbeing: 'number', footprintTonnes: 'number', foodMet: 'number' } satisfies Shape<HouseholdView>;
const HOME = { id: 'number', tile: 'object', owner: 'string', household: 'nullable', rent: 'number', value: 'number' } as unknown as Shape<HomeView>; // owner is a number or "public"
(HOME as Record<string, unknown>).owner = 'nullable';
const PLANT = { id: 'number', kind: 'string', tile: 'object', capacity: 'number', output: 'number', emissionsMonthly: 'number' } satisfies Shape<PlantView>;
const TILE = { x: 'number', y: 'number', kind: 'string', ref: 'number?', buildable: 'boolean', emissionsMonthly: 'number?', state: 'string?' } satisfies Shape<Tile>;
const FIRM = { id: 'number', active: 'boolean', sector: 'string', name: 'string', tile: 'object', owner: 'object', workers: 'number', wage: 'number', price: 'number', output: 'number', emissions: 'number',
  revenue: 'number', profit: 'number', capital: 'number', debt: 'number', greenTech: 'number' } satisfies Shape<FirmView>;
const TURN = { turn: 'number', accepted: 'array', rejected: 'array', activated: 'array', pcBefore: 'number', pcAfter: 'number', months: 'array', policyGaps: 'array', trace: 'array', snapshot: 'object' } satisfies Shape<TurnResult>;
const FRAME = { month: 'number', events: 'array', flows: 'object' } satisfies Shape<MonthFrame>;
const LEVER = { key: 'string', label: 'string', unit: 'string', min: 'number', max: 'number', step: 'number', baseCostPc: 'number', costModel: 'object', zeroMeansNone: 'boolean?', leadTime: 'number',
  lobbied: 'boolean', arrows: 'array', availableIn: 'array', options: 'array?', optionAvailableIn: 'object?' } satisfies Shape<LeverInfo>;
const ACTION = { key: 'string', label: 'string', baseCostPc: 'number', costModel: 'object?', leadTime: 'number', moneyCost: 'number', availableIn: 'array', needsTile: 'boolean?', tileRule: 'string?' } satisfies Shape<ActionInfo>;
const PENDING = { id: 'number', command: 'object', submittedTurn: 'number', effectiveTurn: 'number', pcCost: 'number', moneyCost: 'number' } satisfies Shape<PendingItem>;

// Contract v1.0 to v1.2 (schemaVersion 2, or 3 since the weather State layout). Skipped, with this message, when the fixtures are still in the old cohort format.
const current = present && [2, 3].includes(load<{ schemaVersion: number; map?: unknown }>('snapshot-turn1.json').schemaVersion) && !!load<{ map?: unknown }>('snapshot-turn1.json').map;
if (present && !current) console.warn('TODO: docs/fixtures/*.json are still in the old (cohort, schemaVersion 1) format, so the fixtures tests are skipped. Re-run them once the wire layer has regenerated the fixtures.');

describe.skipIf(!current)('real fixtures from docs/fixtures/ against the TypeScript types (contract v1.0, v1.2 shapes optional)', () => {
  const snap = current ? load<Snapshot>('snapshot-turn1.json') : (null as never);
  const turn = current ? load<TurnResult>('turn-result.json') : (null as never);
  const levers = current ? load<LeverInfo[]>('levers.json') : (null as never);
  const actions = current ? load<ActionInfo[]>('actions.json') : (null as never);
  const tiles = current ? load<Record<string, { x: number; y: number }[]>>('tiles.json') : (null as never);

  it('has the keys and kinds the types declare', () => {
    const bad = [
      ...conforms(snap, SNAPSHOT, 'snapshot'), ...conforms(snap.indicators, INDICATORS, 'snapshot.indicators'),
      ...snap.people.flatMap((p, i) => conforms(p, PERSON, `people[${i}]`)), ...snap.households.flatMap((h, i) => conforms(h, HOUSEHOLD, `households[${i}]`)),
      ...snap.homes.flatMap((h, i) => conforms(h, HOME, `homes[${i}]`)), ...snap.plants.flatMap((p, i) => conforms(p, PLANT, `plants[${i}]`)),
      ...snap.map.tiles.flatMap((t, i) => conforms(t, TILE, `map.tiles[${i}]`)), ...snap.firms.flatMap((f, i) => conforms(f, FIRM, `firms[${i}]`)),
      ...conforms(turn, TURN, 'turnResult'), ...turn.months.flatMap((m, i) => conforms(m, FRAME, `months[${i}]`)), ...conforms(turn.snapshot, SNAPSHOT, 'turnResult.snapshot'),
      ...levers.flatMap((l, i) => conforms(l, LEVER, `levers[${i}]`)), ...actions.flatMap((a, i) => conforms(a, ACTION, `actions[${i}]`)),
      ...turn.accepted.flatMap((p, i) => conforms(p, PENDING, `accepted[${i}]`)),
    ];
    expect(bad).toEqual([]);
  });
  it('is schemaVersion 2 or 3 with a 12 x 12 map, 20 people, 8 households, 7 homes', () => {
    expect([2, 3]).toContain(snap.schemaVersion);
    expect(snap.map).toMatchObject({ width: 12, height: 12 });
    expect(snap.map.tiles).toHaveLength(144);
    snap.map.tiles.forEach((t, i) => expect([t.x, t.y]).toEqual([i % 12, Math.floor(i / 12)]));
    expect(snap.people).toHaveLength(20);
    expect(snap.households).toHaveLength(8);
    expect(snap.homes).toHaveLength(7);
    expect(snap.households.filter((h) => h.home === null)).toHaveLength(1);
    expect(snap.buildings).toEqual({ publicHousing: 0, fossilPlant: 2, renewablePlant: 0, road: 4 });
  });
  it('has 3 month frames, 20 levers, 11 actions, an entry per lever in the policy table', () => {
    expect(turn.months).toHaveLength(3);
    expect(levers).toHaveLength(20);
    expect(actions).toHaveLength(11);
    expect(Object.keys(snap.policy).sort()).toEqual(levers.map((l) => l.key).sort());
  });
  it('tile commands carry their tile; previews are not part of the fixtures, but tiles.json agrees with the map', () => {
    for (const a of turn.accepted) if (a.command.type === 'place') expect(a.command.tile).toBeDefined();
    const empty = snap.map.tiles.filter((t) => t.kind === 'empty').map(({ x, y }) => ({ x, y }));
    for (const kind of ['public_housing', 'fossil_plant', 'renewable_plant', 'road']) expect(tiles[kind]).toEqual(empty.filter((t) => tiles[kind]!.some((v) => v.x === t.x && v.y === t.y)));
    expect(tiles.road!.length).toBe(empty.length);
  });
  it('the UI logic runs on the real snapshot and turn result', () => {
    const layout = layoutOf(snap);
    expect(layout.placed.filter((p) => p.kind === 'fossil_plant')).toHaveLength(snap.buildings.fossilPlant);
    expect(layout.scenery.filter((s) => s.kind === 'home')).toHaveLength(snap.homes.length);
    expect(layout.roads).toHaveLength(snap.buildings.road);
    const descs = planEntities(layout, snap);
    expect(descs.length).toBeGreaterThan(10);
    for (const s of layout.scenery) expect(resolveTile(snap, layout, s, []).title).toBeTruthy();
    for (const sel of [{ kind: 'person', id: 3 }, { kind: 'household', id: 7 }, { kind: 'home', id: 2 }, { kind: 'firm', id: 0 }] as const) expect(resolveSelection(snap, layout, sel, [])?.title).toBeTruthy();
    for (const m of METRICS) expect(Number.isFinite(m.value(snap)), m.id).toBe(true);
    expect(() => deriveCrises(snap, turn)).not.toThrow();
    const after = turn.snapshot;
    expect(metricDeltas(snap, after).length).toBeGreaterThan(0);
    for (const e of turn.months.flatMap((m) => m.events)) expect(formatEvent(e, after)).toBeTruthy();
    expect(lobbyLines(turn, levers, actions).length).toBeGreaterThanOrEqual(turn.rejected.length);
    const ctx = ctxFromSnapshot(layoutOf(after), after);
    for (const f of turn.months) expect(() => planFrame(f, ctx)).not.toThrow();
    expect(() => planFlowLoop(turn.months, ctx)).not.toThrow();
    expect(() => generateStory(after, turn)).not.toThrow();
  });
  it('a real accepted build keeps its tile, and the layout shows it as queued', () => {
    const place = turn.accepted.find((a) => a.command.type === 'place')!;
    expect(place.command).toHaveProperty('tile');
    expect(layoutOf(turn.snapshot).pendingTiles[place.id]).toEqual((place.command as { tile: unknown }).tile);
  });
  it('policy gaps list the lobbied levers every turn, mostly with no gap', () => {
    expect(turn.policyGaps.map((g) => g.lever).sort()).toEqual(['carbon_price', 'emissions_cap', 'tax_profit', 'wealth_tax']);
    expect(lobbyLines(turn, levers, actions).filter((l) => l.kind === 'weakened').length).toBeLessThan(turn.policyGaps.length);
  });
  it('nature.forest and nature.commons are 0..1 shares, with the raw stocks beside them', () => {
    for (const n of [snap.nature, turn.snapshot.nature]) {
      expect(n.forest).toBeGreaterThanOrEqual(0);
      expect(n.forest).toBeLessThanOrEqual(1);
      expect(n.commons).toBeGreaterThanOrEqual(0);
      expect(n.commons).toBeLessThanOrEqual(1);
      expect(n.forestStock).toBeGreaterThan(1);
      expect(n.commonsStock).toBeGreaterThan(1);
    }
  });
  it('plant tiles report emissions and the trace names the plant', () => {
    for (const t of snap.map.tiles.filter((x) => x.kind === 'fossil_plant')) expect(t.emissionsMonthly).toBeGreaterThan(0);
    expect(turn.trace.some((t) => t.type === 'emissions' && t.source === 'plant' && t.tile)).toBe(true);
    expect(turn.months[0]!.flows.purchases!.length).toBeGreaterThan(0);
    expect(turn.months[0]!.flows.commutes!.length).toBeGreaterThan(0);
  });
  it('the real levers and actions carry what the UI needs', () => {
    expect(levers.find((l) => l.key === 'commons_access')!.optionAvailableIn).toEqual({ community: ['regulated', 'coop'] });
    for (const a of actions) expect(a.costModel!.base).toBe(a.baseCostPc);
    expect(actions.find((a) => a.key === 'place:road')).toMatchObject({ needsTile: true, leadTime: 2 });
  });
});

// Contract v1.2 (weather). docs/fixtures/ are written by the Rust wire layer; until they carry a weather block these checks are skipped (and say so).
const v12 = current && load<{ weather?: unknown }>('snapshot-turn1.json').weather !== undefined;
if (current && !v12) console.warn('TODO: docs/fixtures/*.json do not carry the contract v1.2 weather block yet, so the v1.2 fixture checks are skipped.');
describe.skipIf(!v12)('real fixtures against contract v1.2 (weather)', () => {
  const snap = v12 ? load<Snapshot>('snapshot-turn1.json') : (null as never);
  const turn = v12 ? load<TurnResult>('turn-result.json') : (null as never);
  it('has the weather block, health fields, tile state and history fields', () => {
    expect(conforms(snap.weather, { stress: 'number', drought: 'nullable', fires: 'array', forecast: 'object' }, 'weather')).toEqual([]);
    expect(conforms(snap.weather.forecast, { fireRisk: 'string', droughtRisk: 'string' }, 'weather.forecast')).toEqual([]);
    for (const k of ['forestHealth', 'commonsHealth', 'farmHealth'] as const) expect(typeof snap.nature[k]).toBe('number');
    for (const t of snap.map.tiles) if (t.kind === 'forest') expect(['standing', 'burning', 'regrowing']).toContain(t.state);
    for (const h of turn.snapshot.history) for (const k of ['climateStress', 'forestHealth', 'commonsHealth', 'farmHealth'] as const) expect(typeof h[k]).toBe('number');
  });
  it('the weather UI logic runs on it', () => {
    expect(forecastView(snap)).toBeTruthy();
    expect(healthMeters(snap)).toHaveLength(3);
    expect(() => deriveCrises(snap, turn)).not.toThrow();
    for (const e of turn.months.flatMap((m) => m.events)) expect(formatEvent(e, turn.snapshot)).toBeTruthy();
    expect(() => weatherLook(snap)).not.toThrow();
  });
});

describe('fixtures presence', () => {
  it('reports whether docs/fixtures/ exists', () => {
    expect(typeof present).toBe('boolean');
  });
});
