// The recorded demo (platform cleanup phase 2): a replay of a real run behind the SimPort interface. These tests need no wasm package.
// That the recording is current is checked on the Rust side (crates/sim-api/tests/recording.rs, in `cargo test --workspace`; `npm run check:recording`);
// that it matches the generated contract is checked by `npm run typecheck` (the file is annotated `: Recording`) and by the shape tests here.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { Snapshot } from '../src/contract';
import { createReplayPort, DEMO_UNAVAILABLE } from '../src/sim-port/replay';
import { RECORDING } from '../src/sim-port/replay/recording.generated';
import { createGame } from '../src/store';
import { ok } from './helpers';

const fixture = <T>(name: string): T => JSON.parse(readFileSync(resolve(__dirname, '..', '..', 'docs', 'fixtures', name), 'utf8')) as T;
const keysOf = (o: object): string[] => Object.keys(o).sort();

describe('the recording', () => {
  it('is a run of the current wire: schema version, and the same snapshot keys as the checked-in fixture', () => {
    expect(RECORDING.schemaVersion).toBe(6); // the version the wasm loader (sim-port/wasm) also demands
    const fx = fixture<Snapshot>('snapshot-turn1.json');
    expect(keysOf(RECORDING.initial)).toEqual(keysOf(fx));
    expect(keysOf(RECORDING.turns[0]!.snapshot)).toEqual(keysOf(fx));
    expect(RECORDING.catalog.entities.length).toBeGreaterThan(10);
  });
  it('is a real, consistent sequence: turns advance one at a time and the script is what was played', () => {
    expect(RECORDING.turns.length).toBeGreaterThanOrEqual(10);
    expect(RECORDING.script).toHaveLength(RECORDING.turns.length);
    RECORDING.turns.forEach((t, i) => {
      expect(t.turn).toBe(i + 1);
      expect(t.snapshot.turn).toBe(i + 2);
      expect(t.months).toHaveLength(3);
      expect(t.accepted.length + t.rejected.length).toBe(RECORDING.script[i]!.length);
    });
    expect(RECORDING.initial.turn).toBe(1);
  });
});

describe('the replay port', () => {
  const port = () => createReplayPort(RECORDING);
  it('serves the recorded replies in order and starts over on init', () => {
    const p = port();
    expect(ok(p.init(123))).toBe(RECORDING.initial); // the seed is the recording's, whatever is asked
    for (const t of RECORDING.turns.slice(0, 3)) expect(ok(p.submitTurn([]))).toBe(t);
    expect(ok(p.init(1))).toBe(RECORDING.initial);
    expect(ok(p.submitTurn([]))).toBe(RECORDING.turns[0]);
  });
  it('says what it is, and refuses what would change the outcome, with an error result and no state change', () => {
    const p = port();
    expect(p.demo).toMatchObject({ label: 'Recorded demo', seed: RECORDING.seed, lastTurn: RECORDING.turns.length });
    p.init(1);
    const refused = [p.preview([]), p.submitTurn([{ type: 'set_policy', lever: 'carbon_price', value: 99 }]), p.cancelPending(1), p.ledger({ kind: 'bank' }), p.personLedger(0), p.exportState(), p.importState('{}'), p.beginTurn([]), p.advanceMonth(), p.finishTurn()];
    for (const r of refused) expect(!r.ok && r.error.code).toBe(DEMO_UNAVAILABLE);
    expect(p.validTiles('road')).toEqual([]);
    expect(ok(p.submitTurn([]))).toBe(RECORDING.turns[0]); // still the first recorded turn
  });
  it('ends with a plain message after the last recorded turn', () => {
    const p = port();
    p.init(1);
    for (let i = 0; i < RECORDING.turns.length; i++) ok(p.submitTurn([]));
    const r = p.submitTurn([]);
    expect(!r.ok && r.error.code).toBe('demo_end');
  });
  it('serves the catalog, levers, actions and explorer data from the recording', () => {
    const p = port();
    expect(ok(p.catalog())).toBe(RECORDING.catalog);
    expect(p.levers()).toBe(RECORDING.levers);
    expect(p.actions()).toBe(RECORDING.actions);
    expect(ok(p.assumptions())).toBe(RECORDING.assumptions);
    expect(ok(p.agents())).toBe(RECORDING.agents);
    expect(p.liveTurn).toBe(false);
  });
});

describe('the game store on the recorded demo', () => {
  it('plays recorded turns, cannot draft policies or buildings, and stops at the end', () => {
    const g = createGame(createReplayPort(RECORDING));
    g.newGame(42);
    expect(g.state.get().demo?.label).toBe('Recorded demo');
    expect(g.state.get().liveAvailable).toBe(false);
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 99 });
    expect(g.state.get().draft).toEqual([]);
    g.setBuildKind('road');
    expect(g.state.get().buildKind).toBeNull();
    expect(g.removeAt({ x: 0, y: 0 })).toMatch(/recording/i);
    g.endTurn(); g.finishPlayback(); g.continueFromResults();
    expect(g.state.get().snapshot).toBe(RECORDING.turns[0]!.snapshot);
    expect(g.state.get().error).toBeNull();
  });
  it('cannot be saved or loaded, and a real game has no demo flag', () => {
    const g = createGame(createReplayPort(RECORDING));
    g.newGame(1);
    expect(g.exportSave()).toBeNull();
    expect(g.state.get().error?.code).toBe(DEMO_UNAVAILABLE);
  });
});
