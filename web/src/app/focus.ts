import type { Phase } from '../store';

/** Selectors to try, in order, for where keyboard focus should go when the game phase changes (empty: leave it alone). Pure so it can be tested. */
export function focusTarget(from: Phase | null, to: Phase): string[] {
  if (to === 'playback') return ['[data-focus="skip"] button'];
  if (to === 'review' && from === 'results') return ['[data-coach="endturn"] button:not([disabled])', 'header button.primary:not([disabled])', '[data-focus="map"]'];
  return [];
}

export function attachFocus(subscribe: (fn: (phase: Phase) => void) => () => void): () => void {
  let prev: Phase | null = null;
  return subscribe((phase) => {
    const sel = prev !== phase ? focusTarget(prev, phase) : [];
    prev = phase;
    if (!sel.length) return;
    // Wait for Svelte to mount the new screen and for a closing modal to give its focus back, then take it.
    setTimeout(() => { for (const q of sel) { const el = document.querySelector<HTMLElement>(q); if (el) { el.focus(); if (document.activeElement === el) return; } } }, 60);
  });
}
