import type { Game } from '../store';
import { TIMING } from './timing';

export interface Scheduler { set(fn: () => void, ms: number): unknown; clear(h: unknown): void }
const realScheduler: Scheduler = { set: (fn, ms) => setTimeout(fn, ms), clear: (h) => clearTimeout(h as number) };

/**
 * Drives the skippable timed sequence from the 3 MonthFrames: advances the playback month
 * on a timer and finishes by itself. Skipping (game.finishPlayback) leaves the phase and stops the timers.
 * Returns a disposer.
 */
export function attachPlayback(game: Game, sched: Scheduler = realScheduler, t: { leadInMs: number; monthMs: number; tailMs: number } = TIMING): () => void {
  let handles: unknown[] = [];
  const stop = () => { handles.forEach((h) => sched.clear(h)); handles = []; };

  const unsub = game.state.subscribe((s) => {
    if (s.phase === 'playback' && handles.length === 0 && s.playback.month === -1) {
      handles.push(
        sched.set(() => game.setPlaybackMonth(0), t.leadInMs),
        sched.set(() => game.setPlaybackMonth(1), t.leadInMs + t.monthMs),
        sched.set(() => game.setPlaybackMonth(2), t.leadInMs + 2 * t.monthMs),
        sched.set(() => { handles = []; game.finishPlayback(); }, t.leadInMs + 3 * t.monthMs + t.tailMs),
      );
    } else if (s.phase !== 'playback' && handles.length) {
      stop();
    }
  });
  return () => { stop(); unsub(); };
}
