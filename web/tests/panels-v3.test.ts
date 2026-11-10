// Town v3 panels, server-rendered against the mock: Pressure controls appear only in Pressure mode, Relaxed is unchanged.
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import { createSimPort } from '../src/sim-port';
import { createGame, createLive, DEFAULT_CONFIG } from '../src/store';
import { ClockStrip, LockButton, PlanTray, QuarterCard, StartScreen, TopBar } from '../src/panels';
import { setUi } from '../src/panels/ui';

const html = (c: unknown, props: Record<string, unknown>) => render(c as Component<Record<string, unknown>>, { props }).body;
function pressure() {
  const game = createGame(createSimPort());
  const live = createLive(game, { ...DEFAULT_CONFIG, mode: 'pressure' });
  live.newGame(5, 'pressure');
  return { game, live };
}

describe('Pressure panels', () => {
  it('shows the clock, pause, speeds and slow pace, and a Lock button that says it applies next quarter', () => {
    const { game, live } = pressure();
    const top = html(TopBar, { game, live });
    expect(top).toContain('Game clock');
    expect(top).toContain('Pause');
    expect(top).toContain('Slow pace');
    expect(top).toContain('4x');
    expect(top).not.toContain('End turn');
    expect(top).toContain('1 Jan 2031');
    expect(html(LockButton, { game, live })).toContain('Lock plan · applies Q2 2031');
    const tray = html(PlanTray, { game, live });
    expect(tray).toContain('Applies when the next quarter begins');
    expect(tray).toContain('Plan for Q2 2031');
  });
  it('shows a locked plan and a non-blocking summary card after a quarter', () => {
    const { game, live } = pressure();
    live.lock();
    expect(html(LockButton, { game, live })).toContain('Unlock to edit');
    live.unlock();
    for (let i = 0; i < 3; i++) game.liveMonth();
    game.liveFinish();
    const card = html(QuarterCard, { game });
    expect(card).toContain('Q1 2031 finished');
    expect(card).not.toContain('aria-modal');
  });
  it('does not render the clock in Relaxed mode', () => {
    const game = createGame(createSimPort());
    game.newGame(5);
    const live = createLive(game, DEFAULT_CONFIG);
    expect(html(ClockStrip, { game, live })).not.toContain('Game clock');
    expect(html(TopBar, { game, live })).toContain('End turn');
    expect(html(QuarterCard, { game })).not.toContain('finished');
  });
  it('the New game screen preselects Relaxed and offers Pressure', () => {
    const game = createGame(createSimPort());
    game.newGame(5);
    const live = createLive(game, DEFAULT_CONFIG);
    setUi({ startOpen: true });
    const s = html(StartScreen, { game, live, seed: 1 });
    expect(s).toMatch(/value="relaxed"[^>]*checked/);
    expect(s).toContain('about 13 seconds');
    setUi({ startOpen: false });
  });
});
