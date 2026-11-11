import { describe, expect, expectTypeOf, it } from 'vitest';
import type { Command, LeverKey, Result, Snapshot, TurnResult } from '../src/contract';
import { createSimPort } from './port';
import { ok } from './helpers';

// Checked by `npm run typecheck` (tsc/svelte-check include tests/).
describe('contract types', () => {
  it('port outputs conform to the contract', () => {
    const sim = createSimPort();
    const snap = ok(sim.init(1));
    expectTypeOf(snap).toEqualTypeOf<Snapshot>();
    expectTypeOf(sim.submitTurn([])).toEqualTypeOf<Result<TurnResult>>();
    const cmd: Command = { type: 'place', kind: 'road', tile: { x: 1, y: 2 } };
    const rm: Command = { type: 'remove', tile: { x: 1, y: 2 } };
    expect(rm.type).toBe('remove');
    expect(cmd.type).toBe('place');
  });
  it('snapshot has an entry for every lever and levers() agrees', () => {
    const sim = createSimPort();
    const snap = ok(sim.init(1));
    const keys = sim.levers().map((l) => l.key).sort();
    expect(Object.keys(snap.policy).sort()).toEqual(keys);
    expectTypeOf<keyof Snapshot['policy']>().toEqualTypeOf<LeverKey>();
  });
  it('v1.0 fields: buildings, map, costModel, units, zeroMeansNone', () => {
    const sim = createSimPort();
    const snap = ok(sim.init(1));
    expect(snap.buildings).toEqual({ publicHousing: 0, fossilPlant: 2, renewablePlant: 0, road: 11 });
    expect(snap.map.tiles).toHaveLength(snap.map.width * snap.map.height);
    const ls = sim.levers();
    for (const l of ls) {
      expect(l.costModel.base).toBeGreaterThan(0);
      expect(['usd', 'usd_per_month', 'usd_per_tonne', 'fraction', 'tonnes_per_month', 'bool', 'enum']).toContain(l.unit);
    }
    expect(ls.find((l) => l.key === 'emissions_cap')!.zeroMeansNone).toBe(true);
    expect(ls.filter((l) => l.zeroMeansNone)).toHaveLength(1);
    expect(ls.find((l) => l.key === 'min_wage')!.unit).toBe('usd_per_month');
    expect(ls.find((l) => l.key === 'public_spending')!.unit).toBe('usd_per_month');
    expect(ls.find((l) => l.key === 'job_guarantee')!.unit).toBe('fraction');
  });
  it('schemaVersion is 3 (Town v4 weather layout) and there are 3 month frames per turn', () => {
    const sim = createSimPort();
    expect(ok(sim.init(1)).schemaVersion).toBe(6);
    expect(ok(sim.submitTurn([])).months).toHaveLength(3);
  });
  it('a failing call is a result, never a thrown exception', () => {
    const sim = createSimPort();
    sim.init(1);
    const r = sim.cancelPending(12345);
    expect(r.ok).toBe(false);
    expect(sim.importState('not json').ok).toBe(false);
    expect(() => sim.preview(null as unknown as Command[])).not.toThrow();
  });
});
