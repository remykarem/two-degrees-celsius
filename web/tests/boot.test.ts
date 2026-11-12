import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import type { Component } from 'svelte';
import { boot, bootErrorHtml, type BootTarget } from '../src/app/boot';
import { createReplayPort } from '../src/sim-port/replay';
import { RECORDING } from '../src/sim-port/replay/recording.generated';
import { createGame, isFatalError } from '../src/store';
import { ErrorScreen } from '../src/panels';

const fakeTarget = () => {
  const clicks: Array<() => void> = [];
  const t: BootTarget & { clicks: Array<() => void> } = {
    innerHTML: '', clicks,
    querySelector: (sel) => (t.innerHTML.includes('data-reload') && sel === '[data-reload]' ? { addEventListener: (_t, fn) => { clicks.push(fn); } } : null),
  };
  return t;
};

describe('boot shell', () => {
  it('shows loading, then leaves the page to the app when start succeeds', async () => {
    const t = fakeTarget();
    let during = '';
    expect(await boot(t, async () => { during = t.innerHTML; })).toBe(true);
    expect(during).toContain('Loading the simulation');
  });
  it('replaces the page with a static error screen and a working Reload when start fails', async () => {
    const t = fakeTarget();
    let reloaded = 0;
    expect(await boot(t, async () => { throw new Error('port <exploded>'); }, () => { reloaded++; })).toBe(false);
    expect(t.innerHTML).toContain('could not start');
    expect(t.innerHTML).toContain('port &lt;exploded&gt;');
    expect(t.innerHTML).toContain('Reload');
    t.clicks[0]!();
    expect(reloaded).toBe(1);
    expect(bootErrorHtml('x')).toContain('role="alert"');
  });
});

describe('simulation crash during play', () => {
  it('a wasm trap becomes a fatal error that asks for a reload', () => {
    const port = createReplayPort(RECORDING);
    const g = createGame({ ...port, submitTurn: () => ({ ok: false, error: { code: 'wasm_panic', message: 'The simulation stopped unexpectedly.' } }) } as typeof port);
    g.newGame(1);
    g.endTurn();
    const err = g.state.get().error!;
    expect(isFatalError(err)).toBe(true);
    const html = render(ErrorScreen as unknown as Component<Record<string, unknown>>, { props: { game: g, live: {} } }).body;
    expect(html).toContain('The simulation crashed');
    expect(html).toContain('Reload');
    expect(html).toContain('downloaded files');
  });
});
