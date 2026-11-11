import type { GameCalendar } from '../contract';
import rawConfig from './clock-config.json';

/**
 * The Pressure-mode clock (docs/town-v3.md). Pure: no DOM, no timers of its own. The caller injects a time source (`now`, in milliseconds)
 * and calls `tick()` often; tests move a fake time and call `tick()`. The simulation never sees this clock.
 */

export type GameMode = 'relaxed' | 'pressure';
export type TimeoutPolicy = 'submit' | 'discard' | 'skip';
export type ClockPhase = 'planning' | 'locked' | 'paused';
export type PauseReason = 'manual' | 'hidden' | 'modal';

export interface ClockConfig {
  mode: GameMode;
  /** Real seconds per game day (default 0.143: 1 s is about 7 game days, a quarter about 13 s). */
  realSecondsPerGameDay: number;
  speeds: number[];
  onTimeout: TimeoutPolicy;
  allowPause: boolean;
  /** Total real seconds of manual pause allowed, or null for unlimited. */
  pauseBudgetSeconds: number | null;
  pauseWhenHidden: boolean;
  /** Speed multiplier of the slow-pace accessibility toggle (0.5 = half speed). */
  slowPace: number;
}

export const DEFAULT_CONFIG: ClockConfig = rawConfig as ClockConfig;

/** Game days in one quarter for a calendar (365 / turnsPerYear). */
export const daysPerTurn = (turnsPerYear = 4) => 365 / turnsPerYear;
/** Real seconds one quarter lasts at 1x: the number the UI shows ("about 13 s"). */
export const secondsPerQuarter = (cfg: Pick<ClockConfig, 'realSecondsPerGameDay'>, turnsPerYear = 4) => cfg.realSecondsPerGameDay * daysPerTurn(turnsPerYear);

/** The config with development overrides from the URL: `?mode=pressure&pace=0.1&timeout=discard&budget=20&pause=0`. `pace` is real seconds per game day. */
export function configFromSearch(search: string, base: ClockConfig = DEFAULT_CONFIG): ClockConfig {
  const q = new URLSearchParams(search);
  const cfg: ClockConfig = { ...base, speeds: [...base.speeds] };
  const mode = q.get('mode');
  if (mode === 'pressure' || mode === 'relaxed') cfg.mode = mode;
  const pace = Number(q.get('pace'));
  if (q.has('pace') && Number.isFinite(pace) && pace > 0) cfg.realSecondsPerGameDay = pace;
  const to = q.get('timeout');
  if (to === 'submit' || to === 'discard' || to === 'skip') cfg.onTimeout = to;
  const budget = Number(q.get('budget'));
  if (q.has('budget') && Number.isFinite(budget) && budget >= 0) cfg.pauseBudgetSeconds = budget;
  if (q.get('pause') === '0') cfg.allowPause = false;
  return cfg;
}

export interface GameDate { year: number; month: number; day: number; hour: number; label: string }
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** The in-game date at a (fractional) turn number: the calendar's start year plus `(turn - firstTurn) * 365/turnsPerYear` days. */
export function gameDate(turn: number, cal: Pick<GameCalendar, 'startYear' | 'firstTurn' | 'turnsPerYear'>): GameDate {
  const days = Math.max(0, turn - cal.firstTurn) * daysPerTurn(cal.turnsPerYear);
  const d = new Date(Date.UTC(cal.startYear, 0, 1) + Math.floor(days * 24) * 3600_000);
  const hour = d.getUTCHours();
  return {
    year: d.getUTCFullYear(), month: d.getUTCMonth(), day: d.getUTCDate(), hour,
    label: `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}, ${String(hour).padStart(2, '0')}:00`,
  };
}

export type ClockEvent =
  /** Month 2 or 3 of the quarter begins (month 1 begins together with the quarter). */
  | { type: 'month'; month: number; quarter: number }
  /** The quarter boundary. `action` says what to do with the drafts: locked plans are always submitted. `quarter` is the index of the quarter that begins. */
  | { type: 'quarter'; quarter: number; locked: boolean; action: TimeoutPolicy };

export interface ClockState {
  phase: ClockPhase;
  locked: boolean;
  pausedBy: PauseReason[];
  speed: number;
  slow: boolean;
  /** Quarters completed since start (0-based index of the quarter now playing). */
  quarter: number;
  /** 0..1 progress through the quarter now playing. */
  frac: number;
  /** Months of this quarter already begun, 1..3 once started. */
  month: number;
  secondsPerQuarter: number;
  /** Real seconds until the boundary at the current rate (Infinity while paused). */
  secondsLeft: number;
  pauseBudgetLeft: number | null;
}

/** Longest real gap one tick may account for: after a sleeping laptop or a throttled tab the clock does not jump whole quarters. */
const MAX_STEP_SECONDS = 1;

export interface Clock {
  state(): ClockState;
  subscribe(fn: (s: ClockState) => void): () => void;
  onEvent(fn: (e: ClockEvent) => void): () => void;
  /** Advance by the real time since the last call. Cheap; call it every 50-100 ms. */
  tick(): void;
  lock(): void;
  unlock(): void;
  pause(why?: PauseReason): boolean;
  resume(why?: PauseReason): void;
  setSpeed(n: number): void;
  setSlow(on: boolean): void;
  /** Back to the start of quarter 0, unlocked, speed kept, pauses cleared (budget refilled). */
  reset(): void;
  readonly config: ClockConfig;
}

/** `calendar` is read on every use: the wire's game.turnsPerYear and game.ticksPerTurn (months per turn, params run.ticks_per_turn) of the game in play. */
export function createClock(config: ClockConfig, now: () => number, calendar: () => Pick<GameCalendar, 'turnsPerYear' | 'ticksPerTurn'>): Clock {
  const cfg = config;
  const spqNow = () => secondsPerQuarter(cfg, calendar().turnsPerYear);
  let last = now();
  let elapsed = 0; // seconds into the quarter, in 1x time
  let monthsBegun = 0; // months after the first that have been announced (0 .. monthsPerTurn - 1)
  let quarter = 0;
  let locked = false;
  let speed = cfg.speeds[0] ?? 1;
  let slow = false;
  const pausedBy = new Set<PauseReason>();
  let budget = cfg.pauseBudgetSeconds;
  const subs = new Set<(s: ClockState) => void>();
  const listeners = new Set<(e: ClockEvent) => void>();

  const rate = () => speed * (slow ? cfg.slowPace : 1);
  const compute = (): ClockState => {
    const paused = pausedBy.size > 0;
    return {
      phase: paused ? 'paused' : locked ? 'locked' : 'planning', locked, pausedBy: [...pausedBy], speed, slow, quarter,
      frac: Math.min(1, elapsed / spqNow()), month: monthsBegun, secondsPerQuarter: spqNow(),
      secondsLeft: paused ? Infinity : (spqNow() - elapsed) / rate(), pauseBudgetLeft: budget,
    };
  };
  let snapshot = compute();
  const publish = () => { snapshot = compute(); subs.forEach((f) => f(snapshot)); };
  const emit = (e: ClockEvent) => listeners.forEach((f) => f(e));

  function tick() {
    const t = now();
    const dt = Math.min(MAX_STEP_SECONDS, Math.max(0, (t - last) / 1000));
    last = t;
    if (pausedBy.has('manual') && budget !== null) {
      budget = Math.max(0, budget - dt);
      if (budget === 0) pausedBy.delete('manual'); // the budget is spent: time runs again
    }
    if (pausedBy.size === 0 && dt > 0) {
      elapsed += dt * rate();
      for (let guard = 0; guard < 16; guard++) {
        const spq = spqNow(), monthsPerTurn = calendar().ticksPerTurn;
        const edge = ((monthsBegun + 1) * spq) / monthsPerTurn;
        if (elapsed < edge - 1e-9) break;
        if (monthsBegun < monthsPerTurn - 1) {
          monthsBegun++;
          emit({ type: 'month', month: monthsBegun, quarter });
        } else {
          elapsed -= spq;
          monthsBegun = 0;
          quarter++;
          const wasLocked = locked;
          locked = false;
          emit({ type: 'quarter', quarter, locked: wasLocked, action: wasLocked ? 'submit' : cfg.onTimeout });
        }
      }
    }
    publish();
  }

  return {
    config: cfg,
    state: () => snapshot,
    subscribe(fn) { subs.add(fn); fn(snapshot); return () => subs.delete(fn); },
    onEvent(fn) { listeners.add(fn); return () => listeners.delete(fn); },
    tick,
    lock() { if (!locked) { locked = true; publish(); } },
    unlock() { if (locked) { locked = false; publish(); } },
    pause(why = 'manual') {
      if (why === 'manual' && (!cfg.allowPause || budget === 0)) return false;
      if (why === 'hidden' && !cfg.pauseWhenHidden) return false;
      tick(); // settle the time up to now first
      pausedBy.add(why);
      publish();
      return true;
    },
    resume(why = 'manual') {
      if (!pausedBy.has(why)) return;
      tick();
      pausedBy.delete(why);
      last = now();
      publish();
    },
    setSpeed(n) {
      if (!cfg.speeds.includes(n)) return;
      tick();
      speed = n;
      publish();
    },
    setSlow(on) { tick(); slow = on; publish(); },
    reset() {
      last = now();
      elapsed = 0; monthsBegun = 0; quarter = 0; locked = false; pausedBy.clear(); budget = cfg.pauseBudgetSeconds;
      publish();
    },
  };
}
