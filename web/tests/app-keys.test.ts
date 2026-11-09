import { afterEach, describe, expect, it, vi } from 'vitest';
import { attachKeys } from '../src/app/keys';
import { focusTarget } from '../src/app/focus';
import { createSimPort } from '../src/sim-port';
import { createGame } from '../src/store';

// A tiny DOM stub: the key handler only needs window.addEventListener and document.querySelector.
function stubDom(modal: boolean) {
  let handler: ((e: unknown) => void) | null = null;
  vi.stubGlobal('window', { addEventListener: (_: string, f: (e: unknown) => void) => { handler = f; }, removeEventListener: () => {} });
  vi.stubGlobal('document', { querySelector: (q: string) => (modal && q.includes('aria-modal') ? {} : null) });
  return (key: string) => handler!({ key, target: { tagName: 'BODY' }, ctrlKey: false, metaKey: false, altKey: false });
}
afterEach(() => vi.unstubAllGlobals());

describe('global keys', () => {
  it('Q, E and Escape act when no modal is open', () => {
    const press = stubDom(false);
    const g = createGame(createSimPort());
    g.newGame(1);
    attachKeys(g);
    press('e');
    expect(g.state.get().camera).toBe(1);
    g.setBuildKind('fossil_plant');
    press('Escape');
    expect(g.state.get().buildKind).toBeNull();
  });
  it('Q, E and Escape do nothing while a modal is open', () => {
    const press = stubDom(true);
    const g = createGame(createSimPort());
    g.newGame(1);
    attachKeys(g);
    g.setBuildKind('fossil_plant');
    press('e'); press('q'); press('Escape');
    expect(g.state.get().camera).toBe(0);
    expect(g.state.get().buildKind).toBe('fossil_plant');
  });
});

describe('focus after phase changes', () => {
  it('goes to Skip during playback and to End turn after the results screen', () => {
    expect(focusTarget('review', 'playback')[0]).toContain('skip');
    expect(focusTarget('results', 'review')[0]).toContain('endturn');
    expect(focusTarget('review', 'plan')).toEqual([]);
  });
});
