import { describe, expect, it } from 'vitest';
import { configFromSearch, createClock, DEFAULT_CONFIG, gameDate, secondsPerQuarter, type ClockConfig, type ClockEvent } from '../src/store';

/** The calendar the wire sends for the default content (4 turns a year, 3 monthly ticks a turn). */
const CAL = () => ({ turnsPerYear: 4, ticksPerTurn: 3 });

/** A fake time source: `advance(s)` moves time forward by s seconds in 100 ms ticks, like the app's interval. */
function rig(over: Partial<ClockConfig> = {}) {
  let t = 1_000_000;
  const cfg: ClockConfig = { ...DEFAULT_CONFIG, realSecondsPerGameDay: 0.2, ...over }; // quarter = 18.25 s
  const c = createClock(cfg, () => t, CAL);
  const events: ClockEvent[] = [];
  c.onEvent((e) => events.push(e));
  const advance = (s: number) => { for (let i = 0; i < Math.round(s * 10); i++) { t += 100; c.tick(); } };
  return { c, events, advance, cfg, spq: secondsPerQuarter(cfg) };
}

describe('clock pace', () => {
  it('the default pace is about 13 s per quarter (1 s = 7 game days); 3 hours per second is not the default', () => {
    expect(DEFAULT_CONFIG.mode).toBe('relaxed');
    expect(DEFAULT_CONFIG.realSecondsPerGameDay).toBe(0.143);
    expect(secondsPerQuarter(DEFAULT_CONFIG)).toBeCloseTo(13.05, 1);
    const gameHoursPerSecond = 24 / DEFAULT_CONFIG.realSecondsPerGameDay;
    expect(gameHoursPerSecond).toBeGreaterThan(160); // about 168 h (a week) per second
    expect(gameHoursPerSecond).not.toBeCloseTo(3, 0);
    // at 3 h per second a quarter would take 12 minutes: the conflict the design doc explains
    expect(2190 / 3).toBe(730);
  });
  it('URL overrides change the pace and mode', () => {
    const c = configFromSearch('?mode=pressure&pace=0.1&timeout=discard&budget=20&pause=0');
    expect(c).toMatchObject({ mode: 'pressure', realSecondsPerGameDay: 0.1, onTimeout: 'discard', pauseBudgetSeconds: 20, allowPause: false });
    expect(configFromSearch('?pace=-3&mode=x').realSecondsPerGameDay).toBe(0.143);
  });
  it('derives game dates from turns', () => {
    const cal = { startYear: 2030, firstTurn: 1, turnsPerYear: 4 };
    expect(gameDate(1, cal).label).toBe('1 Jan 2030, 00:00');
    expect(gameDate(2, cal).month).toBe(3); // 91.25 days in: April
    expect(gameDate(5, cal).year).toBe(2031);
    expect(gameDate(1.5, cal).day).toBeGreaterThan(1);
  });
});

describe('clock state machine', () => {
  it('announces months and the quarter boundary at the right times', () => {
    const { c, events, advance, spq } = rig();
    advance(spq / 3 - 0.2);
    expect(events).toEqual([]);
    advance(0.4);
    expect(events).toEqual([{ type: 'month', month: 1, quarter: 0 }]);
    advance(spq / 3);
    expect(events.map((e) => e.type)).toEqual(['month', 'month']);
    advance(spq / 3);
    expect(events[2]).toEqual({ type: 'quarter', quarter: 1, locked: false, action: 'submit' });
    expect(c.state().quarter).toBe(1);
    expect(c.state().frac).toBeLessThan(0.1);
  });
  it('lock holds until the boundary, then resets; timeout policies follow the config', () => {
    for (const policy of ['submit', 'discard', 'skip'] as const) {
      const { c, events, advance, spq } = rig({ onTimeout: policy });
      expect(c.state().phase).toBe('planning');
      advance(1);
      c.lock();
      expect(c.state().phase).toBe('locked');
      advance(spq);
      expect(events.at(-1)).toMatchObject({ type: 'quarter', locked: true, action: 'submit' }); // a locked plan is always submitted
      expect(c.state().phase).toBe('planning');
      advance(spq);
      expect(events.at(-1)).toMatchObject({ type: 'quarter', locked: false, action: policy });
    }
  });
  it('unlock cancels a lock', () => {
    const { c, events, advance, spq } = rig({ onTimeout: 'discard' });
    c.lock(); c.unlock();
    advance(spq + 0.2);
    expect(events.at(-1)).toMatchObject({ locked: false, action: 'discard' });
  });
  it('pause freezes time; resume continues; the manual pause respects allowPause', () => {
    const { c, advance, spq } = rig();
    advance(2);
    const f = c.state().frac;
    expect(c.pause()).toBe(true);
    expect(c.state().phase).toBe('paused');
    advance(30);
    expect(c.state().frac).toBeCloseTo(f, 5);
    c.resume();
    advance(2);
    expect(c.state().frac).toBeCloseTo(f + 2 / spq, 2);
    const off = rig({ allowPause: false });
    expect(off.c.pause()).toBe(false);
    expect(off.c.state().phase).toBe('planning');
  });
  it('hidden tab and a blocking modal pause independently of the manual pause', () => {
    const { c, advance } = rig();
    c.pause('hidden'); c.pause('modal'); c.pause('manual');
    c.resume('hidden'); c.resume('manual');
    expect(c.state().pausedBy).toEqual(['modal']);
    advance(5);
    expect(c.state().frac).toBe(0);
    c.resume('modal');
    advance(1);
    expect(c.state().frac).toBeGreaterThan(0);
    const keep = rig({ pauseWhenHidden: false });
    expect(keep.c.pause('hidden')).toBe(false);
  });
  it('a pause budget runs out: the clock resumes by itself and refuses further pauses', () => {
    const { c, advance } = rig({ pauseBudgetSeconds: 2 });
    c.pause();
    advance(1);
    expect(c.state().pauseBudgetLeft).toBeCloseTo(1, 1);
    advance(1.5);
    expect(c.state().phase).toBe('planning');
    expect(c.state().pauseBudgetLeft).toBe(0);
    expect(c.pause()).toBe(false);
  });
  it('speeds scale the rate (only configured speeds); the slow-pace toggle halves it', () => {
    const a = rig(); const b = rig(); const s = rig();
    a.advance(2);
    b.c.setSpeed(4); b.advance(2);
    expect(b.c.state().frac).toBeCloseTo(a.c.state().frac * 4, 3);
    b.c.setSpeed(3);
    expect(b.c.state().speed).toBe(4);
    s.c.setSlow(true); s.advance(2);
    expect(s.c.state().frac).toBeCloseTo(a.c.state().frac * 0.5, 3);
  });
  it('a 4x speed can cross several boundaries without losing events', () => {
    const { events, advance, c } = rig();
    c.setSpeed(4);
    advance(40);
    const quarters = events.filter((e) => e.type === 'quarter').length;
    expect(quarters).toBe(Math.floor((40 * 4) / 18.25));
    expect(events.filter((e) => e.type === 'month')).toHaveLength(quarters * 2 + 2);
  });
  it('one very long gap (sleep) is capped instead of skipping quarters', () => {
    let t = 0;
    const c = createClock({ ...DEFAULT_CONFIG, realSecondsPerGameDay: 0.2 }, () => t, CAL);
    const ev: ClockEvent[] = [];
    c.onEvent((e) => ev.push(e));
    t += 3_600_000;
    c.tick();
    expect(ev.filter((e) => e.type === 'quarter')).toHaveLength(0);
  });
  it('reset goes back to the start', () => {
    const { c, advance, spq } = rig();
    advance(spq + 1);
    c.reset();
    expect(c.state()).toMatchObject({ quarter: 0, frac: 0, locked: false, phase: 'planning' });
  });
});
