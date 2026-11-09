import { describe, expect, expectTypeOf, it } from 'vitest';
import type { Command, LeverKey, Snapshot, TurnResult } from '../src/contract';
import { createSimPort } from '../src/sim-port';

// Checked by `npm run typecheck` (tsc/svelte-check include tests/).
describe('contract types', () => {
  it('port outputs conform to the contract', () => {
    const sim = createSimPort();
    const snap = sim.init(1);
    expectTypeOf(snap).toEqualTypeOf<Snapshot>();
    expectTypeOf(sim.submitTurn([])).toEqualTypeOf<TurnResult>();
    const cmd: Command = { type: 'place', kind: 'fossil_plant', count: 1, tile: { x: 1, y: 2 } };
    expect(cmd.type).toBe('place');
  });
  it('snapshot has an entry for every lever and levers() agrees', () => {
    const sim = createSimPort();
    const snap = sim.init(1);
    const keys = sim.levers().map((l) => l.key).sort();
    expect(Object.keys(snap.policy).sort()).toEqual(keys);
    expectTypeOf<keyof Snapshot['policy']>().toEqualTypeOf<LeverKey>();
  });
  it('schemaVersion is present and 3 month frames per turn', () => {
    const sim = createSimPort();
    expect(sim.init(1).schemaVersion).toBeGreaterThan(0);
    expect(sim.submitTurn([]).months).toHaveLength(3);
  });
});
