import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { attachPlayback, type Scheduler } from '../src/app/playback';
import { createGame, paletteActions, quarterLabel, reconcileLayout } from '../src/store';
import { ok } from './helpers';

const newGame = () => {
  const g = createGame(createSimPort());
  g.newGame(1);
  return g;
};

describe('game store', () => {
  it('drafts a policy with a preview, replaces it in the same slot, and removes it', () => {
    const g = newGame();
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 50 });
    expect(g.state.get().phase).toBe('plan');
    expect(g.state.get().draft[0]!.preview.pc).toBeGreaterThan(0);
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 80 });
    expect(g.state.get().draft).toHaveLength(1);
    g.removeDraft(g.state.get().draft[0]!.id);
    expect(g.state.get().draft).toHaveLength(0);
  });
  it('places on free tiles only', () => {
    const g = newGame();
    g.setBuildKind('renewable_plant');
    expect(g.clickTile({ x: 6, y: 4 })).toMatch(/Bank/);
    expect(g.clickTile({ x: 7, y: 9 })).toBeNull();
    expect(g.clickTile({ x: 7, y: 9 })).toMatch(/plan/);
    expect(g.state.get().draft).toHaveLength(1);
  });
  it('runs the full loop; a queued build reserves its tile and becomes a finished tile when it takes effect', () => {
    const g = newGame();
    g.setBuildKind('renewable_plant');
    g.clickTile({ x: 7, y: 9 });
    const before = g.state.get().layout.placed.length;
    g.endTurn();
    expect(g.state.get().phase).toBe('playback');
    expect(g.state.get().layout.placed).toHaveLength(before); // not built yet
    const id = g.state.get().snapshot!.pending[0]!.id;
    expect(g.state.get().layout.pendingTiles[id]).toEqual({ x: 7, y: 9 });
    expect(g.tileError({ x: 7, y: 9 })).toMatch(/under construction/);
    g.finishPlayback();
    expect(g.state.get().phase).toBe('results');
    g.continueFromResults();
    expect(g.state.get().phase).toBe('review');
    expect(g.state.get().snapshot!.turn).toBe(2);
    for (let i = 0; i < 6 && g.state.get().snapshot!.buildings.renewablePlant === 0; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); }
    const s = g.state.get();
    expect(s.snapshot!.buildings.renewablePlant).toBe(1);
    expect(s.layout.placed).toHaveLength(before + 1);
    expect(s.layout.placed.find((p) => p.kind === 'renewable_plant')).toMatchObject({ x: 7, y: 9 });
    expect(s.layout.pendingTiles).toEqual({});
  });
  it('rotates the camera through 4 snaps', () => {
    const g = newGame();
    g.rotate(1); g.rotate(1); g.rotate(1); g.rotate(1);
    expect(g.state.get().camera).toBe(0);
    g.rotate(-1);
    expect(g.state.get().camera).toBe(3);
  });
  it('ignores draft commands during playback', () => {
    const g = newGame();
    g.endTurn();
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 50 });
    expect(g.state.get().draft).toHaveLength(0);
  });
  it('plays a whole game to the end screen', () => {
    const g = newGame();
    let guard = 0;
    while (g.state.get().phase !== 'ended' && guard++ < 400) {
      g.endTurn(); g.finishPlayback(); g.continueFromResults();
    }
    expect(g.state.get().phase).toBe('ended');
    expect(g.state.get().snapshot!.gameOver).not.toBeNull();
  });
});

describe('playback driver', () => {
  it('advances month by month then finishes, and can be skipped', () => {
    const queue: { fn: () => void; ms: number; live: boolean }[] = [];
    const sched: Scheduler = {
      set: (fn, ms) => { const h = { fn, ms, live: true }; queue.push(h); return h; },
      clear: (h) => { (h as { live: boolean }).live = false; },
    };
    const g = newGame();
    const off = attachPlayback(g, sched, { leadInMs: 1, monthMs: 10, tailMs: 1 });
    g.endTurn();
    expect(queue).toHaveLength(4);
    queue.slice().sort((a, b) => a.ms - b.ms).forEach((h) => h.live && h.fn());
    expect(g.state.get().playback.month).toBe(2);
    expect(g.state.get().phase).toBe('results');
    g.continueFromResults();
    queue.length = 0;
    g.endTurn();
    expect(queue).toHaveLength(4);
    g.finishPlayback();
    expect(queue.every((h) => !h.live)).toBe(true);
    off();
  });
});

describe('v0.2 store behaviour', () => {
  it('builds the palette from actions() and the calendar label from the snapshot', () => {
    const g = newGame();
    expect(paletteActions(g.state.get().actions).map((p) => p.kind)).toEqual(['public_housing', 'fossil_plant', 'renewable_plant']);
    const game = g.state.get().snapshot!.game;
    expect(quarterLabel(1, game)).toBe('Q1 2031');
    expect(quarterLabel(6, game)).toBe('Q2 2032');
  });
});

describe('v0.3 store behaviour', () => {
  it('derives the layout from households, firms and building counts, deterministically', () => {
    const g = newGame();
    const s = g.state.get();
    const homes = s.layout.scenery.filter((x) => x.kind === 'home');
    const firms = s.layout.scenery.filter((x) => x.kind === 'firm');
    expect(homes).toHaveLength(s.snapshot!.households.length);
    expect(firms).toHaveLength(s.snapshot!.firms.length);
    expect(s.layout.placed.filter((p) => p.kind === 'fossil_plant')).toHaveLength(5);
    const again = reconcileLayout(null, s.snapshot!);
    expect(again).toEqual(s.layout);
    expect(reconcileLayout(s.layout, s.snapshot!)).toBe(s.layout); // unchanged -> same object
    const tiles = [...s.layout.scenery, ...s.layout.placed].map((t) => `${t.x},${t.y}`);
    expect(new Set(tiles).size).toBe(tiles.length);
  });
  it('adds and removes tiles when the counts change (last placed goes first)', () => {
    const snap = ok(createSimPort().init(1));
    const l0 = reconcileLayout(null, snap);
    const more = reconcileLayout(l0, { ...snap, buildings: { ...snap.buildings, fossilPlant: 7, publicHousing: 2 } });
    expect(more.placed.filter((p) => p.kind === 'fossil_plant')).toHaveLength(7);
    expect(more.placed.filter((p) => p.kind === 'public_housing')).toHaveLength(2);
    expect(more.placed.filter((p) => p.kind === 'fossil_plant').slice(0, 5)).toEqual(l0.placed.filter((p) => p.kind === 'fossil_plant'));
    const fewer = reconcileLayout(more, { ...snap, buildings: { ...snap.buildings, fossilPlant: 3 } });
    expect(fewer.placed.filter((p) => p.kind === 'fossil_plant')).toEqual(more.placed.filter((p) => p.kind === 'fossil_plant').slice(0, 3));
    expect(fewer.placed.some((p) => p.kind === 'public_housing')).toBe(false);
    const tiles = fewer.placed.map((t) => `${t.x},${t.y}`);
    expect(new Set(tiles).size).toBe(tiles.length);
  });
  it('tile-less place drafts do not merge into one draft', () => {
    const g = newGame();
    g.draftCommand({ type: 'place', kind: 'renewable_plant', count: 1 });
    g.draftCommand({ type: 'place', kind: 'renewable_plant', count: 1 });
    g.draftCommand({ type: 'place', kind: 'fossil_plant', count: 1 });
    expect(g.state.get().draft).toHaveLength(3);
    g.draftCommand({ type: 'place', kind: 'renewable_plant', count: 1, tile: { x: 1, y: 7 } });
    g.draftCommand({ type: 'place', kind: 'renewable_plant', count: 1, tile: { x: 1, y: 7 } });
    expect(g.state.get().draft).toHaveLength(4); // same tile replaces
  });
  it('endTurn makes ONE store update and does not throw when the sim refuses', () => {
    const g = newGame();
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 50 });
    let updates = 0;
    const off = g.state.subscribe(() => updates++);
    updates = 0;
    g.endTurn();
    off();
    expect(updates).toBe(1);

    const sim = createSimPort();
    const bad = createGame({ ...sim, submitTurn: () => ({ ok: false, error: { code: 'violation', message: 'money is not conserved' } }) });
    bad.newGame(1);
    expect(() => bad.endTurn()).not.toThrow();
    expect(bad.state.get().error).toMatchObject({ code: 'violation', where: 'end_turn' });
    expect(bad.state.get().phase).toBe('review');
    bad.dismissError();
    expect(bad.state.get().error).toBeNull();

    const throwing = createGame({ ...sim, submitTurn: () => { throw new Error('boom'); } });
    throwing.newGame(1);
    expect(() => throwing.endTurn()).not.toThrow();
    expect(throwing.state.get().error?.code).toBe('internal');
  });
  it('ending a turn after game over is an error, not a crash', () => {
    const g = newGame();
    let guard = 0;
    while (g.state.get().phase !== 'ended' && guard++ < 400) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); }
    expect(g.endTurn()).toBeNull(); // phase 'ended' ignores the call
    expect(g.state.get().error).toBeNull();
  });
  it('dismissed crisis warnings reset on New game', () => {
    const g = newGame();
    g.dismissCrisis('low_budget');
    expect(g.state.get().dismissedCrises).toEqual({ low_budget: 1 });
    g.newGame(2);
    expect(g.state.get().dismissedCrises).toEqual({});
  });
  it('save and load round trip: sim state, layout, last result and previous snapshot', () => {
    const g = newGame();
    g.setBuildKind('renewable_plant');
    g.clickTile({ x: 7, y: 9 });
    g.draftCommand({ type: 'set_policy', lever: 'tax_wage', value: 0.3 });
    g.endTurn(); g.finishPlayback(); g.continueFromResults();
    g.setLens('wealth');
    const text = g.exportSave()!;
    const file = JSON.parse(text);
    expect(file).toMatchObject({ format: 'ecotown-save', saveVersion: 1 });
    expect(Object.keys(file)).toEqual(expect.arrayContaining(['schemaVersion', 'simState', 'layout', 'lastResult', 'previous', 'uiPrefs']));

    const h = createGame(createSimPort());
    h.newGame(99);
    expect(h.importSave(text)).toBeNull();
    const a = g.state.get(), b = h.state.get();
    expect(b.snapshot).toEqual(a.snapshot);
    expect(b.layout).toEqual(a.layout);
    expect(b.lastResult).toEqual(a.lastResult);
    expect(b.previous).toEqual(a.previous);
    expect(b.lens).toBe('wealth');
    expect(b.phase).toBe('review');
    // and the loaded game continues exactly like the original
    g.endTurn(); h.endTurn();
    expect(h.state.get().snapshot).toEqual(g.state.get().snapshot);
  });
  it('load gives friendly messages for bad files', () => {
    const g = newGame();
    expect(g.importSave('not json')).toMatch(/not a saved game/);
    expect(g.importSave('{"hello":1}')).toMatch(/not a saved game from this app/);
    const file = JSON.parse(g.exportSave()!);
    expect(g.importSave(JSON.stringify({ ...file, saveVersion: 7 }))).toMatch(/file version 7/);
    expect(g.importSave(JSON.stringify({ ...file, layout: null }))).toMatch(/damaged map layout/);
    expect(g.importSave(JSON.stringify({ ...file, simState: file.simState.replace(/"schemaVersion":\d+/, '"schemaVersion":99') }))).toMatch(/different version/);
    expect(g.state.get().snapshot!.turn).toBe(1); // a failed load changes nothing
  });
});
