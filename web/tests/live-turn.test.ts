import { describe, expect, it } from 'vitest';
import type { Command } from '../src/contract';
import { createSimPort } from './port';
import { ok } from './helpers';

const plans: Command[][] = [
  [],
  [{ type: 'set_policy', lever: 'carbon_price', value: 60 }],
  [{ type: 'set_policy', lever: 'carbon_price', value: 40 }, { type: 'set_policy', lever: 'wealth_tax', value: 0.02 }],
];

function runSplit(seed: number, plan: (i: number) => Command[], turns: number) {
  const sim = createSimPort();
  sim.init(seed);
  const out: { months: unknown[]; fin: unknown }[] = [];
  for (let i = 0; i < turns; i++) {
    ok(sim.beginTurn(plan(i)));
    const months = [0, 1, 2].map(() => ok(sim.advanceMonth()).frame);
    out.push({ months, fin: ok(sim.finishTurn()) });
  }
  return { out, state: ok(sim.exportState()) };
}
function runSubmit(seed: number, plan: (i: number) => Command[], turns: number) {
  const sim = createSimPort();
  sim.init(seed);
  const out: { months: unknown[] }[] = [];
  for (let i = 0; i < turns; i++) out.push(ok(sim.submitTurn(plan(i))));
  return { out, state: ok(sim.exportState()) };
}

describe('live turn on the real simulation (contract v1.1)', () => {
  it('begin + 3 x advance + finish equals submitTurn: result, frames and final state', () => {
    for (const seed of [1, 7, 42]) {
      const plan = (i: number) => plans[i % plans.length]!;
      const a = runSplit(seed, plan, 6);
      const b = runSubmit(seed, plan, 6);
      expect(a.state).toBe(b.state);
      a.out.forEach((x, i) => {
        expect(JSON.stringify(x.fin)).toBe(JSON.stringify(b.out[i]));
        expect(JSON.stringify(x.months)).toBe(JSON.stringify(b.out[i]!.months));
      });
    }
  });

  it('enforces the state machine with turn_state and leaves state unchanged', () => {
    const sim = createSimPort();
    const s0 = ok(sim.init(3));
    expect(s0.turnPhase).toEqual({ open: false, monthsDone: 0 });
    const code = (r: { ok: boolean; error?: { code: string } }) => (r.ok ? 'ok' : r.error!.code);
    expect(code(sim.advanceMonth())).toBe('turn_state');
    expect(code(sim.finishTurn())).toBe('turn_state');
    const before = ok(sim.exportState());
    expect(ok(sim.beginTurn([])).snapshot.turnPhase).toEqual({ open: true, monthsDone: 0 });
    expect(code(sim.beginTurn([]))).toBe('turn_state');
    expect(code(sim.submitTurn([]))).toBe('turn_state');
    expect(code(sim.exportState())).toBe('turn_state');
    expect(code(sim.finishTurn())).toBe('turn_state'); // 0 of 3 months
    expect(ok(sim.advanceMonth()).snapshot.turnPhase).toEqual({ open: true, monthsDone: 1 });
    ok(sim.advanceMonth());
    ok(sim.advanceMonth());
    expect(code(sim.advanceMonth())).toBe('turn_state'); // 3 of 3
    const fin = ok(sim.finishTurn());
    expect(fin.snapshot.turnPhase).toEqual({ open: false, monthsDone: 0 });
    expect(fin.months).toHaveLength(3);
    expect(ok(sim.exportState())).not.toBe(before);
  });

  it('importState closes any open turn; preview works while a turn is open', () => {
    const sim = createSimPort();
    sim.init(5);
    const blob = ok(sim.exportState());
    ok(sim.beginTurn([]));
    expect(ok(sim.preview([{ type: 'set_policy', lever: 'carbon_price', value: 50 }]))).toHaveLength(1);
    expect(ok(sim.importState(blob)).turnPhase).toEqual({ open: false, monthsDone: 0 });
    ok(sim.beginTurn([]));
  });

  it('reports rejected commands in TurnStart', () => {
    const sim = createSimPort();
    sim.init(5);
    const r = ok(sim.beginTurn([{ type: 'place', kind: 'road', tile: { x: 0, y: 0 } }]));
    expect(r.rejected).toHaveLength(1);
    expect(r.turn).toBe(1);
  });
});
