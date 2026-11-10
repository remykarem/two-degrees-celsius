// Component smoke tests: server-render each panel against the mock game and check the key text is there.
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import { createSimPort } from '../src/sim-port';
import { createGame, type Game } from '../src/store';
import { BuildPalette, CrisisBanner, EndScreen, ErrorScreen, Inspector, MetricsPanel, PendingList, PlanTray, PolicyDrawer, ResultsScreen, TopBar } from '../src/panels';
import BarChart from '../src/panels/BarChart.svelte';
import Sparkline from '../src/panels/Sparkline.svelte';
import { setUi } from '../src/panels/ui';
import PolicyRow from '../src/panels/PolicyRow.svelte';
import PlaybackTicker from '../src/panels/PlaybackTicker.svelte';

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
    expect(html(BuildPalette, { game: g })).toContain('Build fossil power plant');
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
    g.select({ kind: 'tile', x: 6, y: 5 });
    expect(html(Inspector, { game: g })).toContain('Bank');
    g.select({ kind: 'person', id: 4 });
    const p = html(Inspector, { game: g });
    expect(p).toContain('Manager A');
    expect(p).toContain('Commute');
    g.select({ kind: 'household', id: 7 });
    expect(html(Inspector, { game: g })).toMatch(/unhoused/i);
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

describe('v0.3 component behaviour', () => {
  it('the build palette offers the four buildable kinds with built counts, and road is one of them', () => {
    const b = html(BuildPalette, { game: game() });
    for (const label of ['Build public housing', 'Build fossil power plant', 'Build renewable power plant', 'Build road']) expect(b).toContain(label);
    expect(b).toContain('Built 2'); // two fossil plants
    expect(b).toContain('Built 4'); // four roads
    expect(b).toContain('click it on the map');
  });
  it('shows cost estimates from the cost model, then the authoritative preview once drafted', () => {
    const g = game();
    const lever = g.state.get().levers.find((l) => l.key === 'carbon_price')!;
    const row = (props: Record<string, unknown>) => html(PolicyRow, { lever, nominal: 0, effective: 0, disabled: null, system: 'market', draftValue: undefined, preview: undefined, onset: () => {}, onreset: () => {}, ...props });
    expect(row({})).toContain('About 2 PC for a small change');
    expect(row({})).not.toMatch(/Base \d/);
    expect(row({})).toContain('If raised:');
    const down = row({ nominal: 100, draftValue: 20 });
    expect(down).toContain('Your change (down):');
  });
  it('greys the community commons option in Market with the reason', () => {
    const g = game();
    const lever = g.state.get().levers.find((l) => l.key === 'commons_access')!;
    const out = html(PolicyRow, { lever, nominal: 'open', effective: null, disabled: null, system: 'market', draftValue: undefined, preview: undefined, onset: () => {}, onreset: () => {} });
    expect(out).toContain('Needs Regulated capitalism or Worker cooperatives');
    const ok = html(PolicyRow, { lever, nominal: 'open', effective: null, disabled: null, system: 'regulated', draftValue: undefined, preview: undefined, onset: () => {}, onreset: () => {} });
    expect(ok).not.toContain('Needs Regulated capitalism');
  });
  it('the top bar flags mock data and shows the same turn as the end screen at game over', () => {
    const g = game();
    expect(html(TopBar, { game: g })).toContain('Mock data');
    for (let i = 0; i < 200 && g.state.get().phase !== 'ended'; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); }
    const turn = g.state.get().snapshot!.gameOver!.turn;
    expect(html(TopBar, { game: g })).toContain(`Turn ${turn}/40`);
    expect(html(EndScreen, { game: g })).toContain(`(turn ${turn})`);
  });
  it('the crisis region keeps role=status; only critical banners carry role=alert', () => {
    const g = game();
    expect(html(CrisisBanner, { game: g })).toContain('role="status"');
    expect(html(CrisisBanner, { game: g })).not.toContain('role="alert"');
  });
  it('the error screen appears for a failed sim call and not otherwise', () => {
    const base = createSimPort();
    const g = createGame({ ...base, submitTurn: () => ({ ok: false, error: { code: 'violation', message: 'money is not conserved' } }) });
    g.newGame(1);
    expect(html(ErrorScreen, { game: g })).not.toContain('Something went wrong');
    g.endTurn();
    const e = html(ErrorScreen, { game: g });
    expect(e).toContain('Something went wrong');
    expect(e).toContain('money is not conserved');
    expect(e).toContain('playing the turn');
  });
  it('the playback live region reads only notable events (importance 2 and 3)', () => {
    const g = game();
    g.endTurn();
    g.setPlaybackMonth(2);
    const t = html(PlaybackTicker, { game: g });
    expect(t).toMatch(/role="log"/);
    const live = t.slice(t.indexOf('role="log"'));
    const result = g.state.get().lastResult!;
    const minor = result.months.flatMap((m) => m.events).find((e) => e.importance === 1 && e.type === 'hired');
    if (minor) expect(live).not.toContain('hired');
  });
});

