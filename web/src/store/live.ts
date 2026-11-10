import type { Game } from './game';
import { createClock, gameDate, type Clock, type ClockConfig, type ClockEvent, type ClockState, type GameDate } from './clock';
import { writable, type Readable } from './readable';
import { liveClock } from './scene-live';
import { quarterLabel } from './config';

/** What the panels need to draw the clock. */
export interface LiveView {
  /** Pressure mode is on and the loop is running. */
  active: boolean;
  /** Pressure mode is chosen but the loop has not started or has stopped (game over, error). */
  stopped: boolean;
  clock: ClockState;
  date: GameDate | null;
  /** The quarter that is playing live now, e.g. "Q1 2031" (null before start). */
  playing: string | null;
  /** The quarter the player is planning. */
  planning: string | null;
  /** Month markers: where in the quarter (0..1) each month begins. */
  markers: number[];
}

export interface Live {
  view: Readable<LiveView>;
  clock: Clock;
  config: ClockConfig;
  /** Begins quarter 1 (beginTurn with the current drafts, month 1) and starts the clock. No-op unless the game is in Pressure mode. */
  start(): void;
  stop(): void;
  /** Call every 50-100 ms (the app) or after moving a fake time source (tests). */
  tick(): void;
  lock(): void;
  unlock(): void;
  togglePause(): void;
  setSpeed(n: number): void;
  toggleSlow(): void;
  /** True while a blocking modal is open (pauses the clock). */
  setBlocking(on: boolean): void;
  /** True while the tab is hidden (pauses the clock when `pauseWhenHidden`). */
  setHidden(on: boolean): void;
}

const MAX_FAILS = 3;

/**
 * The Pressure-mode game loop: turns clock events into beginTurn / advanceMonth / finishTurn calls through the Game store.
 * It never waits for the player: the boundary arrives, the plan is submitted (or not, per `onTimeout`) and the next quarter begins.
 * Errors never deadlock: a turn_state error is resynced; anything else is reported and, after repeated failures, the loop stops.
 */
export function createLive(game: Game, config: ClockConfig, now: () => number = () => Date.now()): Live {
  const turnsPerYear = () => game.state.get().snapshot?.game.turnsPerYear ?? 4;
  const clock = createClock(config, now, turnsPerYear());
  let running = false;
  let started = false;
  let fails = 0;
  const view = writable<LiveView>(compute());

  function compute(): LiveView {
    const g = game.state.get();
    const c = clock.state();
    const snap = g.snapshot;
    const playingTurn = snap && g.live?.open ? snap.turn : null;
    return {
      active: running, stopped: g.mode === 'pressure' && started && !running, clock: c,
      date: snap && started ? gameDate(snap.turn + c.frac, snap.game) : null,
      playing: snap && playingTurn !== null ? quarterLabel(playingTurn, snap.game) : null,
      planning: snap && started ? quarterLabel(Math.min(snap.game.lastTurn, (playingTurn ?? snap.turn) + 1), snap.game) : null,
      markers: [1 / 3, 2 / 3],
    };
  }
  const refresh = () => {
    view.set(compute());
    const c = clock.state();
    liveClock.set({ active: running, monthT: Math.min(1, Math.max(0, c.frac * 3 - c.month)), paused: c.phase === 'paused' });
  };
  clock.subscribe(refresh);
  game.state.subscribe(refresh);

  function halt() {
    running = false;
    game.setLocked(false);
    refresh();
  }

  /** Runs one live call; handles the failure modes. Returns true when the call succeeded. */
  function step(call: () => { ok: boolean; error?: { code: string; message: string } }): boolean {
    const r = call();
    if (r.ok) { fails = 0; return true; }
    const err = r.error!;
    if (err.code === 'game_over') { halt(); return false; }
    if (err.code === 'turn_state') {
      game.setLiveNotice('The simulation and the clock disagreed about the turn, so the quarter was resynchronised.');
      if (!game.liveRecover()) fail(err);
      return false;
    }
    fail(err);
    return false;
  }
  function fail(err: { code: string; message: string }) {
    fails++;
    game.setError('live', err);
    if (fails >= MAX_FAILS) { game.setLiveNotice('The live clock stopped after repeated errors. Save your game or start a new one.'); halt(); }
  }

  /** After a game over inside a month: the sim still needs its remaining months and the finish to close the turn. */
  function closeAfterGameOver() {
    for (let i = 0; i < 3 && game.state.get().live?.monthsDone !== 3; i++) game.liveMonth();
    game.liveFinish();
    halt();
  }
  const overNow = () => !!game.state.get().snapshot?.gameOver;

  function beginAndPlay(action: 'submit' | 'discard' | 'skip', month: number): boolean {
    if (!step(() => game.liveBegin(action))) {
      // A failed begin after a resync: try once more with the same action so the clock and the sim are aligned again.
      if (!running || game.state.get().live?.open || !step(() => game.liveBegin(action))) return false;
    }
    for (let m = 0; m <= month; m++) {
      if (!step(() => game.liveMonth())) return false;
      if (overNow()) { closeAfterGameOver(); return false; }
    }
    return true;
  }

  function onEvent(e: ClockEvent) {
    if (!running) return;
    if (e.type === 'month') {
      if (step(() => game.liveMonth()) && overNow()) closeAfterGameOver();
      else if (!game.state.get().live?.open && running) beginAndPlay('submit', e.month); // a resync closed the turn: rejoin the current quarter
      return;
    }
    // Quarter boundary: close the quarter that just played, start the next with the plan.
    if (game.state.get().live?.open && !step(() => game.liveFinish())) { if (running) beginAndPlay('skip', 0); return; }
    if (!running) return;
    if (overNow()) { halt(); return; }
    beginAndPlay(e.action, 0);
  }
  clock.onEvent(onEvent);

  game.setSaveExtras(() => ({ speed: clock.state().speed, slow: clock.state().slow }));

  return {
    view: { get: view.get, subscribe: view.subscribe },
    clock, config,
    start() {
      const g = game.state.get();
      if (g.mode !== 'pressure' || !g.live || g.snapshot?.gameOver) return;
      clock.reset();
      if (g.savedClock) { clock.setSpeed(g.savedClock.speed); clock.setSlow(g.savedClock.slow); }
      fails = 0;
      running = true;
      started = true;
      if (!beginAndPlay('submit', 0) && !game.state.get().live?.open) { /* error already reported; the loop may have stopped */ }
      refresh();
    },
    stop() { running = false; started = false; game.setLocked(false); refresh(); },
    tick() { if (running) clock.tick(); },
    lock() { if (running) { game.setLocked(true); clock.lock(); } },
    unlock() { game.setLocked(false); clock.unlock(); },
    togglePause() { if (clock.state().pausedBy.includes('manual')) clock.resume('manual'); else clock.pause('manual'); },
    setSpeed: (n) => clock.setSpeed(n),
    toggleSlow() { clock.setSlow(!clock.state().slow); },
    setBlocking(on) { if (on) clock.pause('modal'); else clock.resume('modal'); },
    setHidden(on) { if (on) clock.pause('hidden'); else clock.resume('hidden'); },
  };
}
