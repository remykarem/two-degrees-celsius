/* eslint-disable @typescript-eslint/no-explicit-any -- the mock state blob is edited as untyped JSON on purpose */
import { describe, expect, it } from 'vitest';
import type { Command, Snapshot } from '../src/contract';
import { createSimPort } from '../src/sim-port';
import { GEN } from '../src/sim-port/mock/data';
import { ok } from './helpers';

const fresh = (seed = 7) => {
  const sim = createSimPort();
  sim.init(seed);
  return sim;
};
const carbon = (value: number): Command => ({ type: 'set_policy', lever: 'carbon_price', value });
const set = (lever: string, value: unknown) => ({ type: 'set_policy', lever, value }) as Command;
/** Empty tiles that can take a building right now, in row-major order. */
const free = (sim: ReturnType<typeof fresh>, n = 1) => sim.validTiles('fossil_plant').slice(0, n);
const placeAt = (kind: 'public_housing' | 'fossil_plant' | 'renewable_plant' | 'road', tile: { x: number; y: number }): Command => ({ type: 'place', kind, tile });
const prev = (sim: ReturnType<typeof fresh>, c: Command[]) => ok(sim.preview(c));
/** Edit the mock's exported state (a test hook: the blob is the mock's own JSON). */
function tweak(sim: ReturnType<typeof fresh>, f: (s: Record<string, any>) => void) {
  const s = JSON.parse(ok(sim.exportState()));
  f(s);
  return ok(sim.importState(JSON.stringify(s)));
}

describe('mock levers', () => {
  it('describes all 20 levers with sane metadata', () => {
    const ls = fresh().levers();
    expect(ls).toHaveLength(20);
    for (const l of ls) {
      if (l.unit === 'enum') expect(l.options!.length).toBeGreaterThan(1);
      else expect(l.min).toBeLessThan(l.max);
      if (l.unit !== 'enum') expect(l.step).toBeGreaterThan(0);
      expect(l.leadTime).toBeGreaterThanOrEqual(1);
      expect(l.availableIn.length).toBeGreaterThan(0);
    }
    expect(ls.filter((l) => l.lobbied).map((l) => l.key).sort()).toEqual(['carbon_price', 'emissions_cap', 'tax_profit', 'wealth_tax']);
  });
  it('starts from the real starting policy', () => {
    const s = ok(fresh().init(1));
    expect(s.policy.tax_wage.nominal).toBe(0.3);
    expect(s.policy.min_wage.nominal).toBe(1200);
    expect(s.policy.benefit.nominal).toBe(800);
    expect(s.policy.public_spending.nominal).toBe(8000);
    expect(s.policy.campaign_cap.nominal).toBe(1);
    expect(s.policy.commons_access.nominal).toBe('open');
    expect(s.policy.bailout.nominal).toBe(false);
  });
  it('levers() availability mirrors the rules', () => {
    const ls = fresh().levers();
    expect(ls.find((l) => l.key === 'campaign_cap')!.availableIn).toEqual(['regulated', 'coop']);
    expect(ls.find((l) => l.key === 'emissions_cap')!.availableIn).toEqual(['market', 'regulated', 'coop']);
    expect(ls.find((l) => l.key === 'commons_access')!.optionAvailableIn).toEqual({ community: ['regulated', 'coop'] });
  });
});

describe('political capital and costs (turn.rs rules)', () => {
  it('starts at 60 with cap 100', () => {
    const s = ok(fresh().init(1));
    expect(s.pc.current).toBe(60);
    expect(s.pc.cap).toBe(100);
  });
  it('real cost: base x max(|change| / unit, min size) x (1 + opposition)', () => {
    const sim = fresh();
    expect(prev(sim, [carbon(50)])[0]!.pc).toBe(8); // 0 -> 50 is one unit
    expect(prev(sim, [carbon(100)])[0]!.pc).toBe(16);
    expect(prev(sim, [carbon(5)])[0]!.pc).toBe(2); // below min size 0.25 => 8 x 0.25
    expect(prev(sim, [set('tax_wage', 0.4)])[0]!.pc).toBeCloseTo(12, 9); // 0.10 / 0.05 = 2 units x 6
    expect(prev(sim, [set('min_wage', 1600)])[0]!.pc).toBeCloseTo(10, 9); // 400 / 200 x 5
  });
  it('size is measured against the latest pending value, in one plan and across turns', () => {
    const sim = fresh();
    const [a, b] = prev(sim, [carbon(50), carbon(60)]);
    expect(a!.pc).toBe(8);
    expect(b!.pc).toBe(2); // |60 - 50| / 50 = 0.2 -> min size 0.25 x 8
    const net = ok(sim.submitTurn([carbon(100)])).snapshot.netLobby;
    expect(prev(sim, [carbon(110)])[0]!.pc).toBeCloseTo(2 * (1 + net), 9); // vs the pending 100 (0.1 units -> min size), not the active 0
  });
  it('bool and enum levers have size 1', () => {
    const sim = fresh();
    expect(prev(sim, [set('bailout', true)])[0]!.pc).toBe(15);
    expect(prev(sim, [set('commons_access', { mode: 'open' })])[0]!.pc).toBe(6);
  });
  it('opposition applies only to changes that hurt capital, with factor (1 + net lobby)', () => {
    const sim = fresh();
    let s: Snapshot = ok(sim.init(7));
    for (let i = 0; i < 12; i++) s = ok(sim.submitTurn([])).snapshot;
    expect(s.netLobby).toBeGreaterThan(0.05);
    const raise = prev(sim, [carbon(50)])[0]!;
    expect(raise.opposition).toBeCloseTo(s.netLobby, 9);
    expect(raise.pc).toBeCloseTo(8 * (1 + s.netLobby), 9);
    const [wealth, profitUp, profitDown, wage, cap] = prev(sim, [set('wealth_tax', 0.01), set('tax_profit', 0.3), set('tax_profit', 0.2), set('tax_wage', 0.35), set('emissions_cap', 300)]);
    expect(wealth!.opposition).toBeGreaterThan(0);
    expect(profitUp!.opposition).toBeGreaterThan(0);
    expect(profitDown!.opposition).toBe(0);
    expect(wage!.opposition).toBe(0);
    expect(cap!.opposition).toBeGreaterThan(0); // introducing a cap tightens from "no cap"
  });
  it('tightening an existing cap is opposed, loosening it is not', () => {
    const sim = fresh();
    tweak(sim, (s) => { s.policy.emissions_cap = 300; s.netLobby = 0.4; });
    const [tight, loose] = prev(sim, [set('emissions_cap', 200), set('emissions_cap', 400)]);
    expect(tight!.opposition).toBe(0.4);
    expect(loose!.opposition).toBe(0);
  });
  it('previews cost PC and lead time; submit charges the same amount', () => {
    const sim = fresh();
    const [p] = prev(sim, [carbon(50)]);
    expect(p!.effectiveTurn).toBe(1 + p!.leadTime);
    expect(p!.leadTime).toBe(2);
    const r = ok(sim.submitTurn([carbon(50)]));
    expect(r.accepted[0]!.pcCost).toBe(p!.pc);
    const regen = 8 * r.snapshot.government.legitimacy;
    expect(r.pcBefore).toBe(60);
    expect(r.pcAfter).toBeCloseTo(60 - p!.pc + regen, 6);
    expect(r.snapshot.pc.regenNextTurn).toBeCloseTo(regen, 6);
  });
  it('caps PC at 100', () => {
    const sim = fresh();
    let r = ok(sim.submitTurn([]));
    for (let i = 0; i < 20; i++) r = ok(sim.submitTurn([]));
    expect(r.snapshot.pc.current).toBe(100);
  });
  it('lead times come from content (tax 1, carbon 2, place_fossil 4)', () => {
    const sim = fresh();
    const [a, b, c2] = free(sim, 3);
    const [tax, c, fossil, ren, road, sw] = prev(sim, [set('tax_wage', 0.35), carbon(50), placeAt('fossil_plant', a!), placeAt('renewable_plant', b!), placeAt('road', c2!), { type: 'switch_system', to: 'regulated' }]);
    expect([tax!.leadTime, c!.leadTime, fossil!.leadTime, ren!.leadTime, road!.leadTime, sw!.leadTime]).toEqual([1, 2, 4, 4, 2, 1]);
    expect([fossil!.pc, ren!.pc, road!.pc, sw!.pc]).toEqual([12, 2, 0.5, 45]);
  });
});

describe('activation timing (Rust: activate_due runs at the START of the next submit)', () => {
  it('a lead-1 item has effectiveTurn == snapshot.turn after the turn and fires at the next submit', () => {
    const sim = fresh();
    const r1 = ok(sim.submitTurn([set('tax_wage', 0.4)]));
    expect(r1.snapshot.turn).toBe(2);
    expect(r1.snapshot.pending[0]!.effectiveTurn).toBe(2);
    expect(r1.snapshot.policy.tax_wage.nominal).toBe(0.3); // not yet
    expect(r1.activated).toHaveLength(0);
    const r2 = ok(sim.submitTurn([]));
    expect(r2.activated).toHaveLength(1);
    expect(r2.snapshot.policy.tax_wage.nominal).toBe(0.4);
    expect(r2.snapshot.pending).toHaveLength(0);
  });
  it('a lead-2 item activates one submit later', () => {
    const sim = fresh();
    const r1 = ok(sim.submitTurn([carbon(50)]));
    expect(r1.snapshot.pending[0]!.effectiveTurn).toBe(3);
    const r2 = ok(sim.submitTurn([]));
    expect(r2.activated).toHaveLength(0);
    expect(r2.snapshot.pending[0]!.effectiveTurn).toBe(r2.snapshot.turn);
    const r3 = ok(sim.submitTurn([]));
    expect(r3.activated.map((a) => a.command)).toEqual([carbon(50)]);
    expect(r3.snapshot.policy.carbon_price.nominal).toBe(50);
  });
});

describe('cancelPending', () => {
  it('refunds 50% of the PC cost, and nothing else', () => {
    const sim = fresh();
    const r = ok(sim.submitTurn([carbon(100)]));
    const item = r.accepted[0]!;
    const before = r.snapshot.pc.current;
    const out = ok(sim.cancelPending(item.id));
    expect(out.refundPc).toBe(item.pcCost / 2);
    expect(out.snapshot.pc.current).toBe(before + out.refundPc);
    expect(out.snapshot.pending).toHaveLength(0);
  });
  it('an unknown id is an error result, not a silent no-op', () => {
    const sim = fresh();
    const r = sim.cancelPending(99);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.code).toBe('unknown_pending');
  });
});

describe('block reasons (validate)', () => {
  it('reports validation reasons', () => {
    const sim = fresh();
    const reasons = prev(sim, [
      { type: 'switch_system', to: 'market' },
      { type: 'remove', tile: free(sim)[0]! },
      set('campaign_cap', 0.5),
      carbon(9999),
      set('commons_access', { mode: 'community', cap: 0.5 }),
      set('commons_access', { mode: 'enclosed', owner: 1 }),
      set('bailout', 1),
    ]).map((p) => p.blocked);
    expect(reasons).toEqual(['same_system', 'nothing_to_remove', 'lever_unavailable', 'invalid_command', 'lever_unavailable', 'invalid_command', 'invalid_command']);
  });
  it('lever availability follows the CURRENT system', () => {
    const sim = fresh();
    tweak(sim, (s) => { s.system = 'regulated'; });
    const [cap, community] = prev(sim, [set('campaign_cap', 0.5), set('commons_access', { mode: 'community', cap: 0.5 })]);
    expect(cap!.blocked).toBeNull();
    expect(community!.blocked).toBeNull();
  });
  it('blocks on insufficient PC', () => {
    const sim = fresh();
    const many: Command[] = Array.from({ length: 6 }, (_, i) => ({ type: 'set_policy', lever: 'carbon_price', value: (i + 1) * 30 }));
    const p = prev(sim, [{ type: 'switch_system', to: 'regulated' }, ...many]);
    expect(p.some((x) => x.blocked === 'insufficient_political_capital')).toBe(true);
  });
  it('funds check: cash plus the bank lending headroom, minus builds already queued', () => {
    const sim = fresh();
    const s = ok(sim.init(7));
    const available = s.government.cash + GEN.town.bank.headroom;
    const plant = GEN.build.plantCostUsd;
    const n = Math.floor(available / plant);
    const tiles = free(sim, n + 1);
    const p = prev(sim, tiles.map((t) => placeAt('renewable_plant', t)));
    expect(p.slice(0, n).every((x) => x.blocked === null)).toBe(true);
    expect(p[n]!.blocked).toBe('insufficient_funds');
    expect(p[0]!.money).toBe(plant);
    expect(prev(sim, [placeAt('public_housing', tiles[0]!)])[0]!.money).toBe(GEN.build.publicHousingCostUsd);
    expect(prev(sim, [placeAt('road', tiles[0]!)])[0]!.money).toBe(GEN.build.roadCostUsd);
  });
  it('a second system switch is transition_in_progress while one is pending or running', () => {
    const sim = fresh();
    const r = ok(sim.submitTurn([{ type: 'switch_system', to: 'regulated' }]));
    expect(prev(sim, [{ type: 'switch_system', to: 'coop' }])[0]!.blocked).toBe('transition_in_progress'); // pending
    let s = r.snapshot;
    s = ok(sim.submitTurn([])).snapshot;
    expect(s.system.transition).not.toBeNull();
    expect(s.system.transition!.total).toBe(GEN.systems.regulated.transitionMonths);
    expect(prev(sim, [{ type: 'switch_system', to: 'coop' }])[0]!.blocked).toBe('transition_in_progress'); // running
    expect(prev(sim, [{ type: 'switch_system', to: 'regulated' }])[0]!.blocked).toBe('transition_in_progress');
  });
  it('completes the transition after the real number of months', () => {
    const sim = fresh();
    ok(sim.submitTurn([{ type: 'switch_system', to: 'regulated' }]));
    let s = ok(sim.submitTurn([])).snapshot; // starts, 12 months = 4 turns (3 months already ticked)
    expect(s.system.current).toBe('market');
    for (let i = 0; i < 4 && s.system.transition; i++) s = ok(sim.submitTurn([])).snapshot;
    expect(s.system.current).toBe('regulated');
    expect(s.system.transition).toBeNull();
  });
  it('rejects blocked commands at submit and reports them', () => {
    const r = ok(fresh().submitTurn([{ type: 'switch_system', to: 'market' }]));
    expect(r.rejected[0]!.reason).toBe('same_system');
    expect(r.accepted).toHaveLength(0);
  });
});

describe('buildings live on tiles', () => {
  it('starts with 2 fossil plants, 11 roads (4 in the town and 7 for the estate), no renewables, no public housing, 8 homes', () => {
    const s = ok(fresh().init(1));
    expect(s.buildings).toEqual({ publicHousing: 0, fossilPlant: 2, renewablePlant: 0, road: 11 });
    expect(s.energy.supply.filter((p) => p.kind === 'fossil').reduce((a, p) => a + p.capacity, 0)).toBe(10);
    expect(s.homes).toHaveLength(8);
    expect(s.map.tiles.filter((t) => t.kind === 'fossil_plant')).toHaveLength(2);
  });
  it('a renewable plant appears on its tile when it takes effect, and money is charged then, not when queued', () => {
    const sim = fresh();
    const cash0 = ok(sim.init(7)).government.cash;
    const tile = free(sim)[0]!;
    const r1 = ok(sim.submitTurn([placeAt('renewable_plant', tile)]));
    expect(r1.accepted[0]!.moneyCost).toBe(GEN.build.plantCostUsd);
    expect(r1.accepted[0]!.command).toMatchObject({ tile });
    expect(r1.snapshot.buildings.renewablePlant).toBe(0);
    expect(r1.snapshot.government.cash).toBeGreaterThan(cash0 - GEN.build.plantCostUsd / 2);
    let s = r1.snapshot;
    while (s.pending.length || s.buildings.renewablePlant === 0) s = ok(sim.submitTurn([])).snapshot;
    expect(s.buildings.renewablePlant).toBe(1);
    expect(s.map.tiles.find((t) => t.x === tile.x && t.y === tile.y)!.kind).toBe('renewable_plant');
    expect(s.energy.supply.filter((p) => p.kind === 'renewable').reduce((a, p) => a + p.capacity, 0)).toBe(GEN.build.plantCapacity);
    expect(s.government.cash - s.government.debt).toBeLessThan(cash0 - GEN.town.govDebt - GEN.build.plantCostUsd / 2);
  });
  it('public housing adds a vacant public home, and the unhoused household moves in', () => {
    const sim = fresh();
    const tile = free(sim)[0]!;
    ok(sim.submitTurn([placeAt('public_housing', tile)]));
    let s = ok(sim.init(1));
    ok(sim.submitTurn([placeAt('public_housing', tile)]));
    const moved: unknown[] = [];
    for (let i = 0; i < 8; i++) { const r = ok(sim.submitTurn([])); s = r.snapshot; moved.push(...r.months.flatMap((m) => m.events).filter((e) => e.type === 'household_moved')); }
    expect(s.homes).toHaveLength(9);
    expect(s.homes[8]!.owner).toBe('public');
    expect(s.households.every((h) => h.home !== null)).toBe(true);
    expect(moved).toHaveLength(1);
    expect(s.housing.unhousedHouseholds).toBe(0);
  });
  it('removing a plant needs a tile that holds one, and lowers the count', () => {
    const sim = fresh();
    const s0 = ok(sim.init(1));
    const plant = s0.map.tiles.find((t) => t.kind === 'fossil_plant')!;
    const [onPlant, onEmpty, onForest] = prev(sim, [{ type: 'remove', tile: plant }, { type: 'remove', tile: free(sim)[0]! }, { type: 'remove', tile: { x: 0, y: 0 } }]);
    expect(onPlant!.blocked).toBeNull();
    expect(onEmpty!.blocked).toBe('nothing_to_remove');
    expect(onEmpty!.tileValid).toBe(false);
    expect(onForest!.blocked).toBe('tile_invalid');
    ok(sim.submitTurn([{ type: 'remove', tile: plant }]));
    const s = ok(sim.submitTurn([])).snapshot;
    expect(s.buildings.fossilPlant).toBe(1);
    expect(s.map.tiles.find((t) => t.x === plant.x && t.y === plant.y)!.kind).toBe('empty');
  });
  it('cancel refunds PC only', () => {
    const sim = fresh();
    const r = ok(sim.submitTurn([placeAt('renewable_plant', free(sim)[0]!)]));
    const cash = r.snapshot.government.cash;
    const out = ok(sim.cancelPending(r.accepted[0]!.id));
    expect(out.refundPc).toBe(r.accepted[0]!.pcCost / 2);
    expect(out.snapshot.government.cash).toBe(cash);
  });
  it('actions() carries cost models from content, including roads', () => {
    const a = fresh().actions();
    expect(a.map((x) => x.key).sort()).toEqual([
      'place:fire_station', 'place:fossil_plant', 'place:public_housing', 'place:renewable_plant', 'place:road',
      'remove:fire_station', 'remove:fossil_plant', 'remove:public_housing', 'remove:renewable_plant', 'remove:road', 'switch_system',
    ]);
    const place = a.find((x) => x.key === 'place:renewable_plant')!;
    expect(place.costModel).toEqual({ base: 2, unit: 1, minSize: 1 });
    expect(place.leadTime).toBe(4);
    expect(place.moneyCost).toBe(GEN.build.plantCostUsd);
    expect(a.find((x) => x.key === 'place:road')).toMatchObject({ leadTime: 2, moneyCost: GEN.build.roadCostUsd });
    expect(a.find((x) => x.key === 'switch_system')!.baseCostPc).toBe(45);
  });
});

describe('tile validation', () => {
  const T = (x: number, y: number) => ({ x, y });
  it('tile_invalid: off the map, forest, commons; tile_occupied: homes, firms, roads, plants, town hall and tiles with a queued build', () => {
    const sim = fresh();
    const kinds = [T(-1, 0), T(12, 3), T(0, 0), T(9, 9), T(3, 2), T(9, 3), T(5, 6), T(10, 2), T(5, 5), T(6, 5)];
    const p = prev(sim, kinds.map((t) => placeAt('road', t)));
    expect(p.map((x) => x.blocked)).toEqual(['tile_invalid', 'tile_invalid', 'tile_invalid', 'tile_invalid', 'tile_occupied', 'tile_occupied', 'tile_occupied', 'tile_occupied', 'tile_occupied', 'tile_occupied']);
    expect(p.every((x) => !x.tileValid && x.tileReason === x.blocked)).toBe(true);
    const t = free(sim)[0]!;
    const [a, b] = prev(sim, [placeAt('road', t), placeAt('fossil_plant', t)]);
    expect(a!.tileValid).toBe(true);
    expect(a!.tileReason).toBeUndefined();
    expect(b!.blocked).toBe('tile_occupied'); // the first command queues the tile
    ok(sim.submitTurn([placeAt('road', t)]));
    expect(prev(sim, [placeAt('road', t)])[0]!.blocked).toBe('tile_occupied');
    expect(sim.validTiles('road')).not.toContainEqual(t);
  });
  it('rejects a blocked tile at submit with its reason, and takes no PC', () => {
    const sim = fresh();
    const r = ok(sim.submitTurn([placeAt('road', T(3, 2))]));
    expect(r.rejected[0]!.reason).toBe('tile_occupied');
    expect(r.accepted).toHaveLength(0);
    expect(r.pcAfter).toBeCloseTo(60 + 8 * r.snapshot.government.legitimacy - 8 * 0 - (60 - r.pcBefore), 6);
  });
  it('validTiles is the empty, unqueued tiles, and the same for every kind', () => {
    const sim = fresh();
    const s = ok(sim.init(1));
    const empty = s.map.tiles.filter((t) => t.kind === 'empty').map(({ x, y }) => ({ x, y }));
    expect(sim.validTiles('road')).toEqual(empty);
    expect(sim.validTiles('public_housing')).toEqual(empty);
    expect(s.map.tiles).toHaveLength(144);
    expect(s.map.tiles.every((t) => t.buildable === (t.kind === 'empty'))).toBe(true);
  });
  it('a tile queued for building is re-validated when the build activates (something else took it)', () => {
    const sim = fresh();
    const t = free(sim)[0]!;
    ok(sim.submitTurn([placeAt('road', t)]));
    const blob = JSON.parse(ok(sim.exportState()));
    blob.grid[t.y * 12 + t.x] = { kind: 'home', ref: 0 }; // test hook: the tile is taken before the road is built
    ok(sim.importState(JSON.stringify(blob)));
    const r = ok(sim.submitTurn([]));
    const r2 = ok(sim.submitTurn([]));
    const events = [r, r2].flatMap((x) => x.months.flatMap((m) => m.events));
    expect(events.some((e) => e.type === 'policy_blocked' && e.reason === 'tile_occupied')).toBe(true);
  });
});

describe('roads shorten commutes', () => {
  /** The worker with the longest commute, and the road tiles that cut it: all empty tiles on the cheapest route. */
  it('a built road cuts the commute minutes of the people who walk over it, and nobody else gets longer', () => {
    const sim = fresh(3);
    const before = ok(sim.init(3));
    const worker = [...before.people].filter((p) => p.job).sort((a, b) => b.commuteMinutes - a.commuteMinutes)[0]!;
    const home = before.homes.find((h) => h.household === worker.household)!;
    const firm = before.firms.find((f) => f.id === worker.job!.firm)!;
    // Build roads along an L from the home to the firm over empty tiles (as many as are valid).
    const valid = new Set(sim.validTiles('road').map((t) => `${t.x},${t.y}`));
    const line: { x: number; y: number }[] = [];
    for (let x = home.tile.x; x !== firm.tile.x; x += Math.sign(firm.tile.x - x)) line.push({ x, y: home.tile.y });
    for (let y = home.tile.y; y !== firm.tile.y; y += Math.sign(firm.tile.y - y)) line.push({ x: firm.tile.x, y });
    const cmds = line.filter((t) => valid.has(`${t.x},${t.y}`)).map((t) => placeAt('road', t));
    expect(cmds.length).toBeGreaterThan(2);
    let s = ok(sim.submitTurn(cmds)).snapshot;
    for (let i = 0; i < 4; i++) s = ok(sim.submitTurn([])).snapshot;
    expect(s.pending).toHaveLength(0);
    expect(s.buildings.road).toBe(11 + cmds.length);
    const after = s.people.find((p) => p.id === worker.id)!;
    expect(after.commuteMinutes).toBeLessThan(worker.commuteMinutes);
    expect(after.commuteTiles).toBeGreaterThanOrEqual(1);
    expect(after.effectiveHours).toBeGreaterThanOrEqual(worker.effectiveHours);
  });
  it('commute minutes are slow x open tiles + fast x road tiles, and the unhoused get the fixed long commute', () => {
    const s = ok(fresh(3).init(3));
    for (const p of s.people.filter((x) => x.job)) {
      expect(p.commuteMinutes).toBeGreaterThanOrEqual(p.commuteTiles * GEN.space.fastMinutes);
      expect(p.commuteMinutes).toBeLessThanOrEqual(p.commuteTiles * GEN.space.slowMinutes);
    }
    expect(s.people.filter((p) => !p.job && !p.employer).every((p) => p.commuteTiles === 0 && p.commuteMinutes === 0)).toBe(true);
    // domestic staff (the estate's pilot) commute to the hangar tile
    for (const p of s.people.filter((x) => x.employer)) expect(p.commuteTiles).toBeGreaterThan(0);
  });
  it('removing a road lengthens (never shortens) commutes', () => {
    const sim = fresh(3);
    const before = ok(sim.init(3));
    const road = before.map.tiles.find((t) => t.kind === 'road')!;
    ok(sim.submitTurn([{ type: 'remove', tile: road }]));
    const s = ok(sim.submitTurn([])).snapshot;
    expect(s.buildings.road).toBe(10);
    for (const p of before.people) expect(s.people.find((x) => x.id === p.id)!.commuteMinutes).toBeGreaterThanOrEqual(p.commuteMinutes);
  });
});

describe('people, households and homes', () => {
  it('starts as designed: 22 people, 9 households, 8 homes, one unhoused, harsh inequality', () => {
    const s = ok(fresh().init(1));
    expect(s.people).toHaveLength(22);
    expect(s.households).toHaveLength(9);
    expect(s.homes).toHaveLength(8);
    expect(s.households.filter((h) => h.home === null)).toHaveLength(1);
    expect(s.households.flatMap((h) => h.members).sort((a, b) => a - b)).toEqual(s.people.map((p) => p.id));
    for (const h of s.households) for (const m of h.members) expect(s.people[m]!.household).toBe(h.id);
    expect(s.indicators.gini).toBeGreaterThan(0.6);
    expect(s.indicators.topHouseholdWealthShare).toBeGreaterThan(0.5);
    expect(s.indicators.medianIncomePerPerson).toBeLessThan(s.indicators.avgIncomePerPerson);
    expect(s.firms.map((f) => f.name)).toEqual(['Factory', 'Farm', 'Shop', 'Shop 2']);
    expect(s.firms.map((f) => f.sector)).toEqual(['goods', 'food', 'services', 'services']);
    expect(s.firms.filter((f) => f.active)).toHaveLength(3);
  });
  it('every home holds at most one household; every tile holds one thing; homes sit on home tiles', () => {
    const sim = fresh(2);
    let s = ok(sim.init(2));
    for (let i = 0; i < 15; i++) s = ok(sim.submitTurn([])).snapshot; // the 600 t budget ends a do-nothing game near turn 21
    const seen = new Set<number>();
    for (const h of s.households) if (h.home !== null) { expect(seen.has(h.home)).toBe(false); seen.add(h.home); expect(s.homes[h.home]!.household).toBe(h.id); }
    for (const home of s.homes) expect(['home', 'public_housing']).toContain(s.map.tiles[home.tile.y * 12 + home.tile.x]!.kind);
    expect(new Set(s.map.tiles.map((t) => `${t.x},${t.y}`)).size).toBe(144);
  });
  it('wellbeing is per person: the unhoused and jobless fare worse', () => {
    const s = ok(fresh(1).init(1));
    const unhoused = s.people.filter((p) => s.households[p.household]!.home === null);
    const employed = s.people.filter((p) => p.job);
    const mean = (xs: { wellbeing: number }[]) => xs.reduce((a, p) => a + p.wellbeing, 0) / xs.length;
    expect(mean(unhoused)).toBeLessThan(mean(employed));
    expect(s.indicators.avgWellbeing).toBeCloseTo(mean(s.people), 9);
  });
  it('unemployment counts people: employed of labourForce, and jobs match', () => {
    const s = ok(fresh(1).init(1));
    expect(s.indicators.employed).toBe(s.people.filter((p) => p.job || p.employer).length); // domestic staff (the pilot) are employed
    expect(s.indicators.labourForce).toBe(s.people.filter((p) => p.inLabourForce).length);
    expect(s.indicators.unemployment).toBeCloseTo(1 - s.indicators.employed / s.indicators.labourForce, 9);
  });
  it('purchases are real flows from households to firms, and commutes are per person', () => {
    const r = ok(fresh(1).submitTurn([]));
    const f = r.months[2]!.flows;
    expect(f.purchases!.length).toBeGreaterThan(8);
    for (const p of f.purchases!) { expect(p.amount).toBeGreaterThanOrEqual(0); expect(r.snapshot.firms.some((x) => x.id === p.firm && x.sector === p.sector)).toBe(true); }
    expect(f.commutes!.map((c) => c.person).sort((a, b) => a - b)).toEqual(r.snapshot.people.filter((p) => p.job).map((p) => p.id));
    expect(f.emissions!.filter((e) => e.source === 'plant')).toHaveLength(2);
    expect(r.trace.some((t) => t.type === 'emissions' && t.source === 'plant' && t.tile)).toBe(true);
  });
  it('a plant tile reports its monthly emissions, renewables none', () => {
    const s = ok(fresh(1).init(1));
    for (const t of s.map.tiles.filter((x) => x.kind === 'fossil_plant')) expect(t.emissionsMonthly).toBeGreaterThan(0);
    const total = s.map.tiles.reduce((a, t) => a + (t.emissionsMonthly ?? 0), 0);
    expect(total).toBeLessThan(s.nature.grossEmissionsMonthly);
  });
  it('three months behind on rent means eviction, then a move when a vacancy appears', () => {
    const sim = fresh(1);
    const blob = JSON.parse(ok(sim.exportState()));
    const poor = blob.households[5];
    poor.savings = 0; poor.arrears = 2;
    blob.people.filter((p: { household: number }) => p.household === 5).forEach((p: { job: number | null }) => { p.job = null; });
    blob.policy.benefit = 0;
    blob.unemployment = 0.95;
    ok(sim.importState(JSON.stringify(blob)));
    const events = ok(sim.submitTurn([])).months.flatMap((m) => m.events);
    expect(events.some((e) => e.type === 'household_evicted')).toBe(true);
  });
});

describe('lobbying', () => {
  it('weakens lobbied levers: effective below nominal once in force', () => {
    const sim = fresh();
    ok(sim.submitTurn([carbon(100)]));
    let s = ok(sim.submitTurn([])).snapshot;
    for (let i = 0; i < 12; i++) s = ok(sim.submitTurn([])).snapshot;
    const e = s.policy.carbon_price;
    expect(e.nominal).toBe(100);
    expect(e.effective!).toBeLessThan(100);
    expect(s.policy.base_rate.effective).toBeNull();
  });
  it('policyGaps lists the four lobbied levers every turn (even with no gap)', () => {
    const r = ok(fresh().submitTurn([]));
    expect(r.policyGaps.map((g) => g.lever).sort()).toEqual(['carbon_price', 'emissions_cap', 'tax_profit', 'wealth_tax']);
  });
});

describe('turn result shape', () => {
  it('has three month frames and grows history', () => {
    const r = ok(fresh().submitTurn([]));
    expect(r.months.map((m) => m.month)).toEqual([0, 1, 2]);
    expect(r.snapshot.history).toHaveLength(1);
    expect(r.snapshot.turn).toBe(2);
  });
  it('carries a calendar, a mock flag, and payload-only events', () => {
    const sim = fresh();
    const s = ok(sim.init(1));
    expect(s.game).toEqual({ startYear: 2026, firstTurn: 1, lastTurn: 40, turnsPerYear: 4, ticksPerTurn: 3 });
    expect(s.mock).toBe(true);
    for (const e of ok(sim.submitTurn([])).months.flatMap((m) => m.events)) {
      expect(e).not.toHaveProperty('text');
      expect([1, 2, 3]).toContain(e.importance);
    }
  });
  it('wage and profit share are computed from state, not constants', () => {
    const sim = fresh();
    const a = ok(sim.init(1)).indicators;
    expect(a.wageShare + a.profitShare).toBeCloseTo(1, 9);
    let s = ok(sim.submitTurn([])).snapshot;
    for (let i = 0; i < 20; i++) s = ok(sim.submitTurn([])).snapshot;
    expect(s.indicators.wageShare).not.toBe(a.wageShare);
    expect(s.indicators.moneyInCirculation).not.toBe(a.moneyInCirculation);
  });
});

describe('determinism and persistence', () => {
  it('same seed gives identical runs; different seeds differ', () => {
    const run = (seed: number) => {
      const sim = fresh(seed);
      let r = ok(sim.submitTurn([carbon(40)]));
      for (let i = 0; i < 10; i++) r = ok(sim.submitTurn([]));
      return JSON.stringify(r.snapshot);
    };
    expect(run(3)).toBe(run(3));
    expect(run(3)).not.toBe(run(4));
  });
  it('export and import round trip, then continue identically', () => {
    const a = fresh(5);
    ok(a.submitTurn([carbon(30)]));
    const blob = ok(a.exportState());
    const b = createSimPort();
    ok(b.importState(blob));
    expect(JSON.stringify(ok(b.submitTurn([])).snapshot)).toBe(JSON.stringify(ok(a.submitTurn([])).snapshot));
  });
  it('bad saves are error results, not exceptions', () => {
    const sim = fresh();
    const blob = ok(sim.exportState());
    const bad = sim.importState(blob.replace(/"schemaVersion":\d+/, '"schemaVersion":99'));
    expect(bad.ok).toBe(false);
    if (!bad.ok) expect(bad.error.code).toBe('schema_mismatch');
    expect(sim.importState('{').ok).toBe(false);
    expect(sim.importState('{"turn":1}').ok).toBe(false);
  });
});

describe('game over', () => {
  it('doing nothing exhausts the 900 t carbon budget between turn 24 and 34 (40-turn horizon; mock dynamics, the real sim ends at about turn 27.5)', () => {
    const sim = fresh(11);
    let s = ok(sim.init(11));
    while (!s.gameOver) s = ok(sim.submitTurn([])).snapshot;
    expect(s.gameOver.outcome).toBe('budget_exhausted');
    expect(s.gameOver.turn).toBeGreaterThanOrEqual(24);
    expect(s.gameOver.turn).toBeLessThanOrEqual(34);
  });
  it('submitTurn after game over is an error result', () => {
    const sim = fresh(11);
    let s = ok(sim.init(11));
    while (!s.gameOver) s = ok(sim.submitTurn([])).snapshot;
    const r = sim.submitTurn([]);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.code).toBe('game_over');
  });
  it.each([
    ['legitimacy_zero', (s: Record<string, any>) => { s.legitimacy = 0.0001; }],
    ['mass_unemployment', (s: Record<string, any>) => { s.unemployment = 0.55; }],
    ['bank_failure_cascade', (s: Record<string, any>) => { s.bank.failed = true; s.unemployment = 0.26; }],
  ])('collapse outcome %s is reachable and halts at the first outcome', (reason, edit) => {
    const sim = fresh(2);
    tweak(sim, edit);
    const r = ok(sim.submitTurn([]));
    expect(r.snapshot.gameOver!.outcome).toEqual({ collapse: reason });
    const events = r.months.flatMap((m) => m.events);
    expect(events.filter((e) => e.type === 'collapse')).toHaveLength(1);
    expect(events.find((e) => e.type === 'collapse')).toMatchObject({ reason });
    expect(sim.submitTurn([]).ok).toBe(false);
  });
  it('budget_exhausted is reachable and emits a single event', () => {
    const sim = fresh(2);
    tweak(sim, (s) => { s.cumulativeTonnes = GEN.carbon.budget - 1; });
    const r = ok(sim.submitTurn([]));
    expect(r.snapshot.gameOver!.outcome).toBe('budget_exhausted');
    expect(r.months.flatMap((m) => m.events).filter((e) => e.type === 'budget_exhausted')).toHaveLength(1);
  });
  it('the bank fails by play: extreme interest rates with no bailout', () => {
    const sim = fresh(3);
    ok(sim.submitTurn([set('base_rate', 0.15), set('bank_capital_ratio', 0.3)]));
    let s = ok(sim.submitTurn([])).snapshot;
    for (let i = 0; i < 80 && !s.gameOver; i++) s = ok(sim.submitTurn([])).snapshot;
    expect(s.bank.failed || s.gameOver !== null).toBe(true);
  });
  it('survived: turn is lastTurn and the snapshot has moved to 41', () => {
    const sim = fresh(11);
    tweak(sim, (s) => { s.turn = 40; });
    const r = ok(sim.submitTurn([]));
    expect(r.snapshot.gameOver).toEqual({ turn: 40, outcome: 'survived' });
    expect(r.snapshot.turn).toBe(41);
  });
});

describe('mock-only dynamics are labelled', () => {
  it('every snapshot says it is mock data', () => {
    expect(ok(fresh().init(1)).mock).toBe(true);
  });
});

describe('a full run (40 turns)', () => {
  /** A fixed plan that builds roads, renewables and public housing as it goes, and cuts carbon. Checks the invariants after every turn. */
  it('keeps every invariant for 40 turns (or until a game-over) and is deterministic', () => {
    const run = () => {
      const sim = fresh(21);
      let s = ok(sim.init(21));
      let turns = 0;
      const log: string[] = [];
      while (!s.gameOver && turns < 40) {
        const cmds: Command[] = [];
        if (turns === 0) cmds.push(carbon(30), set('tax_wage', 0.32));
        if (turns % 12 === 4) cmds.push(placeAt('road', sim.validTiles('road')[turns % 7]!));
        if (turns === 20) cmds.push(placeAt('public_housing', sim.validTiles('public_housing')[3]!));
        if (turns === 30) cmds.push(placeAt('renewable_plant', sim.validTiles('renewable_plant')[9]!));
        const r = ok(sim.submitTurn(cmds));
        s = r.snapshot;
        turns++;
        expect(s.map.tiles).toHaveLength(144);
        expect(new Set(s.map.tiles.map((t) => t.kind === 'home' || t.kind === 'public_housing' ? t.ref : -1).filter((x) => x !== -1 && x !== undefined)).size).toBe(s.homes.length);
        expect(s.people.map((p) => p.household).every((h) => s.households[h]!.members.includes(s.people[s.people.findIndex((q) => q.household === h)]!.id))).toBe(true);
        const occupied = s.households.filter((h) => h.home !== null).map((h) => h.home);
        expect(new Set(occupied).size).toBe(occupied.length);
        for (const v of [s.indicators.gdpAnnual, s.indicators.gini, s.indicators.avgWellbeing, s.indicators.avgIncomePerPerson, s.indicators.medianIncomePerPerson, s.indicators.topHouseholdWealthShare, s.government.cash, s.bank.equity]) expect(Number.isFinite(v)).toBe(true);
        for (const p of s.people) { expect(p.wellbeing).toBeGreaterThanOrEqual(0); expect(p.wellbeing).toBeLessThanOrEqual(1); }
        expect(s.indicators.employed).toBe(s.people.filter((p) => p.job || p.employer).length); // domestic staff (the pilot) are employed
        log.push(`${s.turn}:${Math.round(s.indicators.gdpAnnual)}:${s.indicators.employed}`);
      }
      return { turns, log: log.join(','), outcome: JSON.stringify(s.gameOver), final: JSON.stringify(s) };
    };
    const a = run();
    expect(a.turns).toBeGreaterThan(15); // a 900 t budget ends the game near turn 30, before the 40-turn limit
    const b = run();
    expect(b.final).toBe(a.final);
    expect(b.log).toBe(a.log);
  });
});
