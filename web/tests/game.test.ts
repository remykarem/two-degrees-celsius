import { describe, expect, it } from 'vitest';
import { createSimPort } from './port';
import { attachPlayback, type Scheduler } from '../src/app/playback';
import { createGame, layoutOf, paletteActions, quarterLabel, selectionAt, selectionTile, type Game } from '../src/store';

/** Empty tiles of the current snapshot, row-major. */
const empties = (g: Game) => g.state.get().snapshot!.map.tiles.filter((t) => t.kind === 'empty').map(({ x, y }) => ({ x, y }));

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
  it('places on free tiles only, with the sim\'s validTiles deciding', () => {
    const g = newGame();
    const [a] = empties(g);
    expect(g.tileError(a!)).toBeNull(); // not building yet: nothing to say
    g.setBuildKind('renewable_plant');
    expect(g.state.get().validTiles?.keys.size).toBe(empties(g).length);
    expect(g.clickTile({ x: 6, y: 5 })).toMatch(/already built/); // the bank
    expect(g.clickTile({ x: 0, y: 0 })).toMatch(/Nothing can be built/); // forest
    expect(g.clickTile({ x: 12, y: 0 })).toMatch(/Outside/);
    expect(g.clickTile(a!)).toBeNull();
    expect(g.clickTile(a!)).toMatch(/plan/);
    expect(g.state.get().draft).toHaveLength(1);
    expect(g.state.get().draft[0]!.command).toEqual({ type: 'place', kind: 'renewable_plant', tile: a });
    expect(g.state.get().draft[0]!.preview.tileValid).toBe(true);
  });
  it('runs the full loop; a queued build keeps its tile and shows on the map when it takes effect', () => {
    const g = newGame();
    const tile = empties(g)[5]!;
    g.setBuildKind('renewable_plant');
    g.clickTile(tile);
    g.endTurn();
    expect(g.state.get().phase).toBe('playback');
    expect(g.state.get().validTiles).toBeNull(); // build mode ends with the turn
    const snap = g.state.get().snapshot!;
    expect(snap.buildings.renewablePlant).toBe(0); // not built yet
    const id = snap.pending[0]!.id;
    expect(layoutOf(snap).pendingTiles[id]).toEqual(tile);
    g.setBuildKind('road'); // ignored in playback
    g.finishPlayback();
    expect(g.state.get().phase).toBe('results');
    g.continueFromResults();
    expect(g.state.get().phase).toBe('review');
    g.setBuildKind('road');
    expect(g.tileError(tile)).toMatch(/already built/); // the queued tile is no longer valid
    g.setBuildKind(null);
    for (let i = 0; i < 6 && g.state.get().snapshot!.buildings.renewablePlant === 0; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); }
    const s = g.state.get();
    expect(s.snapshot!.buildings.renewablePlant).toBe(1);
    expect(layoutOf(s.snapshot).placed.find((p) => p.kind === 'renewable_plant')).toMatchObject(tile);
    expect(layoutOf(s.snapshot).pendingTiles).toEqual({});
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
    expect(paletteActions(g.state.get().actions).map((p) => p.kind)).toEqual(['public_housing', 'fossil_plant', 'renewable_plant', 'road', 'fire_station']);
    const game = g.state.get().snapshot!.game;
    expect(quarterLabel(1, game)).toBe('Q1 2026');
    expect(quarterLabel(6, game)).toBe('Q2 2027');
  });
});

describe('v0.3 store behaviour', () => {
  it('the map comes from the snapshot: scenery, plants and roads are derived, deterministic and memoised', () => {
    const g = newGame();
    const snap = g.state.get().snapshot!;
    const l = layoutOf(snap);
    expect(l.scenery.filter((x) => x.kind === 'home')).toHaveLength(snap.homes.length);
    expect(l.scenery.filter((x) => x.kind === 'firm')).toHaveLength(snap.map.tiles.filter((t) => t.kind === 'firm').length); // a dormant slot has no tile
    expect(l.placed.filter((p) => p.kind === 'fossil_plant')).toHaveLength(2);
    expect(l.roads).toHaveLength(11); // 4 in the town and the estate's 7
    expect(layoutOf(snap)).toBe(l);
    expect(layoutOf(null).scenery).toEqual([]);
    const tiles = [...l.scenery, ...l.placed, ...l.roads].map((t) => `${t.x},${t.y}`);
    expect(new Set(tiles).size).toBe(tiles.length);
    expect(g.state.get()).not.toHaveProperty('layout');
  });
  it('place and remove drafts share a slot per tile; different tiles do not merge', () => {
    const g = newGame();
    const [a, b] = empties(g);
    g.draftCommand({ type: 'place', kind: 'renewable_plant', tile: a! });
    g.draftCommand({ type: 'place', kind: 'fossil_plant', tile: a! }); // same tile replaces
    g.draftCommand({ type: 'place', kind: 'road', tile: b! });
    expect(g.state.get().draft.map((d) => d.command)).toEqual([{ type: 'place', kind: 'fossil_plant', tile: a }, { type: 'place', kind: 'road', tile: b }]);
    const road = g.state.get().snapshot!.map.tiles.find((t) => t.kind === 'road')!;
    expect(g.removeAt(road)).toBeNull();
    expect(g.removeAt(a!)).toMatch(/nothing to remove/i);
    expect(g.state.get().draft).toHaveLength(3);
    const rm = g.state.get().draft[2]!;
    expect(rm.preview).toMatchObject({ blocked: null, tileValid: true, leadTime: 1 });
  });
  it('a blocked tile in the draft previews as invalid with the sim\'s reason', () => {
    const g = newGame();
    g.draftCommand({ type: 'place', kind: 'road', tile: { x: 3, y: 2 } }); // a home
    expect(g.state.get().draft[0]!.preview).toMatchObject({ blocked: 'tile_occupied', tileValid: false, tileReason: 'tile_occupied' });
    g.draftCommand({ type: 'place', kind: 'road', tile: { x: 0, y: 0 } }); // forest
    expect(g.state.get().draft[1]!.preview).toMatchObject({ blocked: 'tile_invalid', tileValid: false });
  });
  it('the selection model: person, household, home, firm and tile; clicking a tile picks the right one', () => {
    const g = newGame();
    const snap = g.state.get().snapshot!;
    const home = snap.homes[2]!;
    g.clickTile(home.tile);
    expect(g.state.get().selection).toEqual({ kind: 'home', id: home.id });
    expect(selectionTile(snap, g.state.get().selection)).toEqual(home.tile);
    const firm = snap.map.tiles.find((t) => t.kind === 'firm')!;
    g.clickTile(firm);
    expect(g.state.get().selection).toEqual({ kind: 'firm', id: firm.ref });
    g.clickTile({ x: 6, y: 5 });
    expect(g.state.get().selection).toEqual({ kind: 'bank' }); // the bank tile selects the bank (contract v1.4: it has a ledger)
    const person = snap.people.find((p) => p.job)!;
    g.select({ kind: 'person', id: person.id });
    const hh = snap.households[person.household]!;
    expect(selectionTile(snap, g.state.get().selection)).toEqual(snap.homes.find((h) => h.id === hh.home)!.tile);
    g.select({ kind: 'household', id: 7 });
    expect(selectionTile(snap, g.state.get().selection)).toEqual(snap.homes.find((h) => h.id === snap.households[7]!.home)?.tile ?? null);
    expect(selectionAt(snap, { x: 0, y: 0 })).toEqual({ kind: 'tile', x: 0, y: 0 });
    g.select(null);
    expect(g.state.get().selection).toBeNull();
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
  it('save and load round trip: sim state, last result and previous snapshot (no layout in the file)', () => {
    const g = newGame();
    g.setBuildKind('renewable_plant');
    g.clickTile(empties(g)[7]!);
    g.draftCommand({ type: 'set_policy', lever: 'tax_wage', value: 0.4 });
    g.endTurn(); g.finishPlayback(); g.continueFromResults();
    g.setLens('wealth');
    const text = g.exportSave()!;
    const file = JSON.parse(text);
    expect(file).toMatchObject({ format: 'ecotown-save', saveVersion: 2, schemaVersion: 6 });
    expect(Object.keys(file)).toEqual(expect.arrayContaining(['schemaVersion', 'simState', 'lastResult', 'previous', 'uiPrefs']));
    expect(file).not.toHaveProperty('layout');

    const h = createGame(createSimPort());
    h.newGame(99);
    expect(h.importSave(text)).toBeNull();
    const a = g.state.get(), b = h.state.get();
    expect(b.snapshot).toEqual(a.snapshot);
    const plain = <T,>(v: T): T => JSON.parse(JSON.stringify(v)) as T; // a save is JSON, which has no -0
    expect(plain(b.lastResult)).toEqual(plain(a.lastResult));
    expect(plain(b.previous)).toEqual(plain(a.previous));
    expect(b.lens).toBe('wealth');
    expect(b.phase).toBe('review');
    expect(b.selection).toBeNull();
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
    expect(g.importSave(JSON.stringify({ ...file, lastResult: 5 }))).toMatch(/damaged last result/);
    expect(g.importSave(JSON.stringify({ ...file, saveVersion: 1 }))).toMatch(/file version 1/); // an old save with a layout is refused
    expect(g.importSave(JSON.stringify({ ...file, simState: file.simState.replace(/"schemaVersion":\d+/, '"schemaVersion":99') }))).toMatch(/schemaVersion/);
    expect(g.state.get().snapshot!.turn).toBe(1); // a failed load changes nothing
  });
});
