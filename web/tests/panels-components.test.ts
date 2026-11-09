// Component smoke tests: server-render each panel against the mock game and check the key text is there.
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import { createSimPort } from '../src/sim-port';
import { createGame, type Game } from '../src/store';
import { BuildPalette, CrisisBanner, EndScreen, Inspector, MetricsPanel, PendingList, PlanTray, PolicyDrawer, ResultsScreen, TopBar } from '../src/panels';
import BarChart from '../src/panels/BarChart.svelte';
import Sparkline from '../src/panels/Sparkline.svelte';
import { setUi } from '../src/panels/ui';

const html = (c: unknown, props: Record<string, unknown>) => render(c as Component<Record<string, unknown>>, { props }).body;
const game = (): Game => { const g = createGame(createSimPort()); g.newGame(5); return g; };

describe('component smoke (server render)', () => {
  it('renders the top bar, metrics and palette for a fresh game', () => {
    const g = game();
    expect(html(TopBar, { game: g })).toContain('Market capitalism');
    const m = html(MetricsPanel, { game: g });
    expect(m).toContain('GDP');
    expect(m).toContain('GPI');
    expect(m).toContain('Unemployment');
    expect(html(BuildPalette, { game: g })).toContain('Fossil plant');
    expect(html(PendingList, { game: g })).toContain('Nothing scheduled');
    expect(html(PlanTray, { game: g })).toContain('Nothing queued');
  });
  it('plan tray lists drafts with cost and the drawer groups the 19 levers', () => {
    const g = game();
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 50 });
    expect(html(PlanTray, { game: g })).toContain('Carbon price → $50/t');
    setUi({ drawerOpen: true });
    const d = html(PolicyDrawer, { game: g });
    for (const label of ['Taxes', 'Money and credit', 'Labour and welfare', 'Nature and carbon', 'Government spending']) expect(d).toContain(label);
    expect(d).toContain('Draft: costs');
    setUi({ drawerOpen: false });
  });
  it('shows an inspector for a selected tile, and nothing for no selection', () => {
    const g = game();
    expect(html(Inspector, { game: g })).not.toContain('Read-only');
    g.select({ x: 6, y: 4 });
    expect(html(Inspector, { game: g })).toContain('Bank');
  });
  it('results screen only renders in the results phase', () => {
    const g = game();
    expect(html(ResultsScreen, { game: g })).not.toContain('Results');
    g.endTurn();
    g.finishPlayback();
    const r = html(ResultsScreen, { game: g });
    expect(r).toContain('Results');
    expect(r).toContain('Coming next turn');
    expect(html(CrisisBanner, { game: g })).not.toContain('Crisis');
  });
  it('end screen shows the verdict and a story', () => {
    const g = game();
    for (let i = 0; i < 200 && g.state.get().phase !== 'ended'; i++) {
      g.endTurn(); g.finishPlayback(); g.continueFromResults();
    }
    expect(g.state.get().phase).toBe('ended');
    const e = html(EndScreen, { game: g });
    expect(e).toMatch(/You (won|lost)/);
    expect(e).toContain('What happened, and why');
  });
  it('charts render labels without raw JSON', () => {
    expect(html(BarChart, { bars: [{ label: 'Factory', value: 5, text: '5 t' }] })).toContain('Factory');
    expect(html(Sparkline, { values: [1, 2, 3], label: 'X' })).toContain('polyline');
  });
});
