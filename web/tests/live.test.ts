import { describe, expect, it } from 'vitest';
import type { Command } from '../src/contract';
import { createSimPort, type SimPort } from '../src/sim-port';
import { createGame, createLive, DEFAULT_CONFIG, parseSave, type ClockConfig } from '../src/store';

function rig(over: Partial<ClockConfig> = {}, port: SimPort = createSimPort()) {
  let t = 5_000;
  const cfg: ClockConfig = { ...DEFAULT_CONFIG, mode: 'pressure', realSecondsPerGameDay: 0.2, ...over };
  const game = createGame(port);
  game.setMode('pressure');
  game.newGame(7);
  const live = createLive(game, cfg, () => t);
  const advance = (s: number) => { for (let i = 0; i < Math.round(s * 10); i++) { t += 100; live.tick(); } };
  return { game, live, advance, spq: 0.2 * 91.25, port, cfg };
}
const carbon = (value: number): Command => ({ type: 'set_policy', lever: 'carbon_price', value });

describe('Pressure game loop on the mock', () => {
  it('starts a quarter with beginTurn + month 1, then plays months and finishes the quarter at the boundary', () => {
    const { game, live, advance, spq } = rig();
    live.start();
    let s = game.state.get();
    expect(s.live).toMatchObject({ open: true, monthsDone: 1 });
    expect(s.live!.frame!.month).toBe(0);
    expect(s.snapshot!.turn).toBe(1);
    advance(spq / 3 + 0.2);
    expect(game.state.get().live!.monthsDone).toBe(2);
    advance(spq / 3);
    expect(game.state.get().live!.monthsDone).toBe(3);
    advance(spq / 3);
    s = game.state.get();
    expect(s.snapshot!.turn).toBe(2); // quarter 1 finished, quarter 2 began
    expect(s.live).toMatchObject({ open: true, monthsDone: 1 });
    expect(s.lastResult!.turn).toBe(1);
    expect(s.live!.summary!.turn).toBe(1);
    expect(s.phase === 'review' || s.phase === 'plan').toBe(true);
  });

  it('auto-submits the drafts at the boundary (onTimeout submit), reports a rejected command, and keeps going over several quarters', () => {
    const { game, live, advance, spq } = rig();
    live.start();
    game.draftCommand(carbon(55));
    game.draftCommand({ type: 'place', kind: 'road', tile: { x: 6, y: 5 } }); // the bank is there: refused when the quarter begins
    expect(game.state.get().draft).toHaveLength(2);
    advance(spq + 0.3);
    let s = game.state.get();
    expect(s.snapshot!.turn).toBe(2);
    expect(s.draft).toHaveLength(0);
    expect(s.live!.report!.rejected.length).toBeGreaterThanOrEqual(1);
    expect(s.snapshot!.pending.length + s.lastResult!.accepted.length).toBeGreaterThan(0);
    game.dismissReport();
    expect(game.state.get().live!.report).toBeNull();
    advance(spq * 3);
    s = game.state.get();
    expect(s.snapshot!.turn).toBe(5);
    expect(s.live!.open).toBe(true);
  });

  it('lock refuses edits, is submitted, and clears at the boundary', () => {
    const { game, live, advance, spq } = rig({ onTimeout: 'discard' });
    live.start();
    game.draftCommand(carbon(30));
    live.lock();
    expect(game.state.get().live!.locked).toBe(true);
    game.draftCommand(carbon(90));
    expect(game.state.get().draft).toHaveLength(1);
    expect((game.state.get().draft[0]!.command as { value: number }).value).toBe(30);
    advance(spq + 0.3);
    expect(game.state.get().live!.locked).toBe(false);
    expect(game.state.get().lastResult!.turn).toBe(1);
    expect(game.state.get().snapshot!.pending.length).toBeGreaterThan(0); // the locked plan went in
  });

  it('timeout policies: discard drops the drafts, skip keeps them without submitting', () => {
    for (const policy of ['discard', 'skip'] as const) {
      const { game, live, advance, spq } = rig({ onTimeout: policy });
      live.start();
      game.draftCommand(carbon(55));
      advance(spq + 0.3);
      const s = game.state.get();
      expect(s.snapshot!.pending).toHaveLength(0);
      expect(s.draft).toHaveLength(policy === 'skip' ? 1 : 0);
    }
  });

  it('pause, modal and hidden stop the loop; fast-forward speeds it', () => {
    const { game, live, advance, spq } = rig();
    live.start();
    live.togglePause();
    advance(spq * 2);
    expect(game.state.get().snapshot!.turn).toBe(1);
    expect(game.state.get().live!.monthsDone).toBe(1);
    live.togglePause();
    live.setBlocking(true);
    advance(spq);
    expect(game.state.get().snapshot!.turn).toBe(1);
    live.setBlocking(false);
    live.setHidden(true);
    advance(spq);
    expect(game.state.get().snapshot!.turn).toBe(1);
    live.setHidden(false);
    live.setSpeed(4);
    advance(spq / 4 + 0.3);
    expect(game.state.get().snapshot!.turn).toBe(2);
  });

  it('survives a sim that disagrees about the turn (turn_state) without deadlocking', () => {
    const { game, live, advance, spq, port } = rig();
    live.start();
    port.advanceMonth(); // someone else moved the sim: the loop is now out of step
    port.advanceMonth();
    advance(spq / 3 + 0.2); // clock wants a month: sim says turn_state (3 months done)
    advance(spq * 2);
    const s = game.state.get();
    expect(s.snapshot!.turn).toBeGreaterThanOrEqual(2);
    expect(s.live!.open).toBe(true);
    expect(live.view.get().active).toBe(true);
  });

  it('plays to the end of the game when the sim reports game over, and stops the loop', () => {
    const { game, live, advance, spq } = rig();
    live.start();
    // fast-forward a long way: 40 quarters at 4x
    live.setSpeed(4);
    for (let i = 0; i < 40 && live.view.get().active; i++) advance(spq / 4 + 0.1);
    expect(game.state.get().snapshot!.gameOver).not.toBeNull();
    expect(game.state.get().phase).toBe('ended');
    expect(live.view.get().active).toBe(false);
    advance(5); // nothing explodes after the end
  });

  it('is unavailable (stays Relaxed) when the port has no live-turn calls', () => {
    const base = createSimPort();
    const old: SimPort = { ...base, liveTurn: false };
    const g = createGame(old);
    expect(g.setMode('pressure')).toBe('relaxed');
    expect(g.state.get().liveAvailable).toBe(false);
  });
});

describe('saves in Pressure mode', () => {
  it('saves the last boundary with mode, clock prefs and drafts, and loads back at a boundary', () => {
    const { game, live, advance, spq } = rig();
    live.start();
    advance(spq + 0.3); // now quarter 2 is open
    live.setSpeed(2);
    live.toggleSlow();
    game.draftCommand(carbon(45));
    const text = game.exportSave()!;
    expect(text).toBeTruthy();
    const f = parseSave(text);
    expect(f.ok).toBe(true);
    if (!f.ok) return;
    expect(f.value.mode).toBe('pressure');
    expect(f.value.clock).toEqual({ speed: 2, slow: true });
    expect(f.value.drafts).toHaveLength(1);

    const fresh = rig();
    expect(fresh.game.importSave(text)).toBeNull();
    const s = fresh.game.state.get();
    expect(s.mode).toBe('pressure');
    expect(s.snapshot!.turnPhase).toEqual({ open: false, monthsDone: 0 });
    expect(s.snapshot!.turn).toBe(2); // the boundary before quarter 2 began
    expect(s.draft).toHaveLength(1);
    fresh.live.start();
    expect(fresh.live.clock.state().speed).toBe(2);
    expect(fresh.game.state.get().live!.open).toBe(true);
  });
  it('a Relaxed save has no turn in progress and loads in Relaxed mode', () => {
    const g = createGame(createSimPort());
    g.newGame(3);
    const text = g.exportSave()!;
    expect(parseSave(text).ok).toBe(true);
    const h = createGame(createSimPort());
    h.setMode('pressure');
    expect(h.importSave(text)).toBeNull();
    expect(h.state.get().mode).toBe('relaxed'); // the save says relaxed (mode is stored in the save)
  });
});

describe('Relaxed regression', () => {
  it('does not touch the live state and plays turns with submitTurn exactly as before', () => {
    const g = createGame(createSimPort());
    g.newGame(1);
    expect(g.state.get()).toMatchObject({ mode: 'relaxed', live: null });
    g.draftCommand(carbon(50));
    const r = g.endTurn()!;
    expect(r.turn).toBe(1);
    expect(g.state.get().phase).toBe('playback');
    expect(g.state.get().live).toBeNull();
  });
});
