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
const prev = (sim: ReturnType<typeof fresh>, c: Command[]) => ok(sim.preview(c));
/** Edit the mock's exported state (a test hook: the blob is the mock's own JSON). */
function tweak(sim: ReturnType<typeof fresh>, f: (s: Record<string, any>) => void) {
  const s = JSON.parse(ok(sim.exportState()));
  f(s);
  return ok(sim.importState(JSON.stringify(s)));
}

describe('mock levers', () => {
  it('describes all 19 levers with sane metadata', () => {
    const ls = fresh().levers();
    expect(ls).toHaveLength(19);
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
    expect(s.policy.tax_wage.nominal).toBe(0.28);
    expect(s.policy.min_wage.nominal).toBe(1200);
    expect(s.policy.benefit.nominal).toBe(800);
    expect(s.policy.public_spending.nominal).toBe(450000);
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
    expect(prev(sim, [set('tax_wage', 0.38)])[0]!.pc).toBeCloseTo(12, 9); // 0.10 / 0.05 = 2 units x 6
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
    const [wealth, profitUp, profitDown, wage, cap] = prev(sim, [set('wealth_tax', 0.01), set('tax_profit', 0.3), set('tax_profit', 0.2), set('tax_wage', 0.33), set('emissions_cap', 300)]);
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
    const [tax, c, fossil, ren, sw] = prev(sim, [set('tax_wage', 0.3), carbon(50), { type: 'place', kind: 'fossil_plant', count: 1 }, { type: 'place', kind: 'renewable_plant', count: 1 }, { type: 'switch_system', to: 'regulated' }]);
    expect([tax!.leadTime, c!.leadTime, fossil!.leadTime, ren!.leadTime, sw!.leadTime]).toEqual([1, 2, 4, 4, 1]);
    expect([fossil!.pc, ren!.pc, sw!.pc]).toEqual([12, 2, 45]);
  });
});

describe('activation timing (Rust: activate_due runs at the START of the next submit)', () => {
  it('a lead-1 item has effectiveTurn == snapshot.turn after the turn and fires at the next submit', () => {
    const sim = fresh();
    const r1 = ok(sim.submitTurn([set('tax_wage', 0.33)]));
    expect(r1.snapshot.turn).toBe(2);
    expect(r1.snapshot.pending[0]!.effectiveTurn).toBe(2);
    expect(r1.snapshot.policy.tax_wage.nominal).toBe(0.28); // not yet
    expect(r1.activated).toHaveLength(0);
    const r2 = ok(sim.submitTurn([]));
    expect(r2.activated).toHaveLength(1);
    expect(r2.snapshot.policy.tax_wage.nominal).toBe(0.33);
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
      { type: 'remove', kind: 'public_housing', count: 1 },
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
    const cmds: Command[] = Array.from({ length: n + 1 }, () => ({ type: 'place', kind: 'renewable_plant', count: 1 }));
    const p = prev(sim, cmds);
    expect(p.slice(0, n).every((x) => x.blocked === null)).toBe(true);
    expect(p[n]!.blocked).toBe('insufficient_funds');
    expect(p[0]!.money).toBe(plant);
    expect(prev(sim, [{ type: 'place', kind: 'public_housing', count: 3 }])[0]!.money).toBe(3 * GEN.build.publicHousingCostUsd);
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

describe('buildings are counts derived from capacity', () => {
  it('starts with 5 fossil plants (100 / 20), no renewables, no public housing', () => {
    const s = ok(fresh().init(1));
    expect(s.buildings).toEqual({ publicHousing: 0, fossilPlant: 5, renewablePlant: 0 });
    expect(s.energy.fossilCap).toBe(100);
  });
  it('a renewable plant appears in the counts when it takes effect, and money is charged then, not when queued', () => {
    const sim = fresh();
    const cash0 = ok(sim.init(7)).government.cash;
    const cmd: Command = { type: 'place', kind: 'renewable_plant', count: 1 };
    const r1 = ok(sim.submitTurn([cmd]));
    expect(r1.accepted[0]!.moneyCost).toBe(GEN.build.plantCostUsd);
    expect(r1.snapshot.buildings.renewablePlant).toBe(0);
    expect(r1.snapshot.government.cash).toBeGreaterThan(cash0 - GEN.build.plantCostUsd / 2);
    let s = r1.snapshot;
    while (s.pending.length || s.buildings.renewablePlant === 0) s = ok(sim.submitTurn([])).snapshot;
    expect(s.buildings.renewablePlant).toBe(1);
    expect(s.energy.renewableCap).toBe(20);
    expect(s.government.cash - s.government.debt).toBeLessThan(cash0 - GEN.town.govDebt - GEN.build.plantCostUsd / 2);
  });
  it('removing needs a building, and lowers the count', () => {
    const sim = fresh();
    expect(prev(sim, [{ type: 'remove', kind: 'fossil_plant', count: 1 }])[0]!.blocked).toBeNull();
    ok(sim.submitTurn([{ type: 'remove', kind: 'fossil_plant', count: 1 }]));
    const s = ok(sim.submitTurn([])).snapshot;
    expect(s.buildings.fossilPlant).toBe(4);
  });
  it('cancel refunds PC only', () => {
    const sim = fresh();
    const r = ok(sim.submitTurn([{ type: 'place', kind: 'renewable_plant', count: 1 }]));
    const cash = r.snapshot.government.cash;
    const out = ok(sim.cancelPending(r.accepted[0]!.id));
    expect(out.refundPc).toBe(r.accepted[0]!.pcCost / 2);
    expect(out.snapshot.government.cash).toBe(cash);
  });
  it('actions() carries cost models from content', () => {
    const a = fresh().actions();
    expect(a.map((x) => x.key).sort()).toEqual([
      'place:fossil_plant', 'place:public_housing', 'place:renewable_plant',
      'remove:fossil_plant', 'remove:public_housing', 'remove:renewable_plant', 'switch_system',
    ]);
    const place = a.find((x) => x.key === 'place:renewable_plant')!;
    expect(place.costModel).toEqual({ base: 2, unit: 1, minSize: 1 });
    expect(place.leadTime).toBe(4);
    expect(place.moneyCost).toBe(GEN.build.plantCostUsd);
    expect(a.find((x) => x.key === 'switch_system')!.baseCostPc).toBe(45);
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
    expect(s.game).toEqual({ startYear: 2031, firstTurn: 1, lastTurn: 160, turnsPerYear: 4 });
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
  it('doing nothing exhausts the carbon budget between turn 90 and 140', () => {
    const sim = fresh(11);
    let s = ok(sim.init(11));
    while (!s.gameOver) s = ok(sim.submitTurn([])).snapshot;
    expect(s.gameOver.outcome).toBe('budget_exhausted');
    expect(s.gameOver.turn).toBeGreaterThanOrEqual(90);
    expect(s.gameOver.turn).toBeLessThanOrEqual(140);
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
  it('survived: turn is lastTurn and the snapshot has moved to 161', () => {
    const sim = fresh(11);
    tweak(sim, (s) => { s.turn = 160; });
    const r = ok(sim.submitTurn([]));
    expect(r.snapshot.gameOver).toEqual({ turn: 160, outcome: 'survived' });
    expect(r.snapshot.turn).toBe(161);
  });
});

describe('mock-only dynamics are labelled', () => {
  it('every snapshot says it is mock data', () => {
    expect(ok(fresh().init(1)).mock).toBe(true);
  });
});
