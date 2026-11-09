import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { attachPlayback, type Scheduler } from '../src/app/playback';
import { createGame } from '../src/store';

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
  it('runs the full loop and keeps the layout in sync with accepted builds', () => {
    const g = newGame();
    g.setBuildKind('renewable_plant');
    g.clickTile({ x: 7, y: 9 });
    const before = g.state.get().layout.placed.length;
    g.endTurn();
    expect(g.state.get().phase).toBe('playback');
    expect(g.state.get().layout.placed).toHaveLength(before + 1);
    g.finishPlayback();
    expect(g.state.get().phase).toBe('results');
    g.continueFromResults();
    expect(g.state.get().phase).toBe('review');
    expect(g.state.get().snapshot!.turn).toBe(2);
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
