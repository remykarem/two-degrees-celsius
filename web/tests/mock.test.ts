import { describe, expect, it } from 'vitest';
import type { Command } from '../src/contract';
import { createSimPort } from '../src/sim-port';

const fresh = (seed = 7) => {
  const sim = createSimPort();
  sim.init(seed);
  return sim;
};
const carbon = (value: number): Command => ({ type: 'set_policy', lever: 'carbon_price', value });

describe('mock levers', () => {
  it('describes all 19 levers with sane metadata', () => {
    const ls = fresh().levers();
    expect(ls).toHaveLength(19);
    for (const l of ls) {
      expect(l.min).toBeLessThan(l.max);
      expect(l.step).toBeGreaterThan(0);
      expect(l.leadTime).toBeGreaterThanOrEqual(0);
      expect(l.availableIn.length).toBeGreaterThan(0);
    }
    expect(ls.some((l) => l.lobbied)).toBe(true);
  });
});

describe('political capital', () => {
  it('starts at 60 with cap 100', () => {
    const s = fresh().init(1);
    expect(s.pc.current).toBe(60);
    expect(s.pc.cap).toBe(100);
  });
  it('previews cost PC and lead time; submit charges the same amount', () => {
    const sim = fresh();
    const [p] = sim.preview([carbon(50)]);
    expect(p!.pc).toBeGreaterThan(0);
    expect(p!.effectiveTurn).toBe(1 + p!.leadTime);
    const r = sim.submitTurn([carbon(50)]);
    expect(r.accepted[0]!.pcCost).toBe(p!.pc);
    const regen = Math.round(8 * r.snapshot.government.legitimacy);
    expect(r.pcAfter).toBe(60 - p!.pc + regen);
  });
  it('caps PC at 100', () => {
    const sim = fresh();
    let r = sim.submitTurn([]);
    for (let i = 0; i < 20; i++) r = sim.submitTurn([]);
    expect(r.snapshot.pc.current).toBe(100);
  });
});

describe('lead times and activation', () => {
  it('activates a command on its effective turn, not before', () => {
    const sim = fresh();
    const [p] = sim.preview([carbon(50)]);
    const r1 = sim.submitTurn([carbon(50)]);
    expect(r1.snapshot.policy.carbon_price.nominal).toBe(0);
    expect(r1.snapshot.pending).toHaveLength(1);
    let activatedAt = 0;
    let r = r1;
    for (let t = 2; t <= 6 && !activatedAt; t++) {
      r = sim.submitTurn([]);
      if (r.activated.length) activatedAt = r.snapshot.turn;
    }
    expect(activatedAt).toBe(p!.effectiveTurn);
    expect(r.snapshot.policy.carbon_price.nominal).toBe(50);
    expect(r.snapshot.pending).toHaveLength(0);
  });
});

describe('cancelPending', () => {
  it('refunds 50% of the PC cost', () => {
    const sim = fresh();
    const r = sim.submitTurn([carbon(100)]);
    const item = r.accepted[0]!;
    const before = r.snapshot.pc.current;
    const out = sim.cancelPending(item.id);
    expect(out.refundPc).toBe(Math.floor(item.pcCost / 2));
    expect(out.snapshot.pc.current).toBe(before + out.refundPc);
    expect(out.snapshot.pending).toHaveLength(0);
  });
});

describe('block reasons', () => {
  it('reports validation reasons', () => {
    const sim = fresh();
    const reasons = sim
      .preview([
        { type: 'switch_system', to: 'market' },
        { type: 'remove', kind: 'public_housing', count: 99 },
        { type: 'set_policy', lever: 'campaign_cap', value: 1000 },
        { type: 'set_policy', lever: 'carbon_price', value: 9999 },
      ])
      .map((p) => p.blocked);
    expect(reasons).toEqual(['same_system', 'nothing_to_remove', 'lever_unavailable', 'invalid_command']);
  });
  it('blocks on insufficient PC', () => {
    const sim = fresh();
    const many: Command[] = Array.from({ length: 6 }, () => ({ type: 'switch_system', to: 'regulated' }));
    expect(sim.preview(many).some((p) => p.blocked === 'insufficient_political_capital')).toBe(true);
  });
  it('blocks on insufficient funds', () => {
    const sim = fresh();
    const p = sim.preview([{ type: 'set_policy', lever: 'bailout', value: 1000000 }]);
    expect(p[0]!.blocked).toBe('insufficient_funds');
  });
  it('blocks a second system switch while one is in progress', () => {
    const sim = fresh();
    let s = sim.submitTurn([{ type: 'switch_system', to: 'regulated' }]).snapshot;
    for (let i = 0; i < 6 && !s.system.transition; i++) s = sim.submitTurn([]).snapshot;
    expect(s.system.transition).not.toBeNull();
    expect(sim.preview([{ type: 'switch_system', to: 'coop' }])[0]!.blocked).toBe('transition_in_progress');
  });
  it('rejects blocked commands at submit', () => {
    const r = fresh().submitTurn([{ type: 'switch_system', to: 'market' }]);
    expect(r.rejected[0]!.reason).toBe('same_system');
    expect(r.accepted).toHaveLength(0);
  });
});

describe('lobbying', () => {
  it('weakens lobbied levers: effective below nominal once in force', () => {
    const sim = fresh();
    sim.submitTurn([carbon(100)]);
    let s = sim.submitTurn([]).snapshot;
    for (let i = 0; i < 4; i++) s = sim.submitTurn([]).snapshot;
    const e = s.policy.carbon_price;
    expect(e.nominal).toBe(100);
    expect(e.effective).not.toBeNull();
    expect(e.effective!).toBeLessThan(100);
    expect(s.policy.base_rate.effective).toBeNull();
  });
});

describe('turn result shape', () => {
  it('has three month frames and grows history', () => {
    const r = fresh().submitTurn([]);
    expect(r.months.map((m) => m.month)).toEqual([0, 1, 2]);
    expect(r.snapshot.history).toHaveLength(1);
    expect(r.snapshot.turn).toBe(2);
  });
});

describe('determinism and persistence', () => {
  it('same seed gives identical runs; different seeds differ', () => {
    const run = (seed: number) => {
      const sim = fresh(seed);
      let r = sim.submitTurn([carbon(40)]);
      for (let i = 0; i < 10; i++) r = sim.submitTurn([]);
      return JSON.stringify(r.snapshot);
    };
    expect(run(3)).toBe(run(3));
    expect(run(3)).not.toBe(run(4));
  });
  it('export and import round trip, then continue identically', () => {
    const a = fresh(5);
    a.submitTurn([carbon(30)]);
    const blob = a.exportState();
    const b = createSimPort();
    b.importState(blob);
    expect(JSON.stringify(b.submitTurn([]).snapshot)).toBe(JSON.stringify(a.submitTurn([]).snapshot));
    expect(() => b.importState(blob.replace('"schemaVersion":1', '"schemaVersion":99'))).toThrow();
  });
});

describe('full game', () => {
  it('doing nothing exhausts the carbon budget between turn 90 and 140', () => {
    const sim = fresh(11);
    let s = sim.init(11);
    while (!s.gameOver) s = sim.submitTurn([]).snapshot;
    expect(s.gameOver.outcome).toBe('budget_exhausted');
    expect(s.gameOver.turn).toBeGreaterThanOrEqual(90);
    expect(s.gameOver.turn).toBeLessThanOrEqual(140);
  });
  it('a decarbonising player reaches game over without throwing', () => {
    const sim = fresh(11);
    let s = sim.submitTurn([carbon(200), { type: 'set_policy', lever: 'green_subsidy', value: 0.8 }]).snapshot;
    let guard = 0;
    while (!s.gameOver && guard++ < 400) {
      const plant: Command[] = guard % 6 === 0 ? [{ type: 'place', kind: 'renewable_plant', count: 1 }] : [];
      s = sim.submitTurn(plant).snapshot;
    }
    expect(s.gameOver).not.toBeNull();
    expect(guard).toBeLessThan(400);
  });
  it('throws after the game is over', () => {
    const sim = fresh(11);
    let s = sim.init(11);
    while (!s.gameOver) s = sim.submitTurn([]).snapshot;
    expect(() => sim.submitTurn([])).toThrow();
  });
});
