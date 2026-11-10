import type { Game, Live } from '../store';

/** True while any modal dialog is open: the page behind it must not react to the keyboard. */
export const modalOpen = () => !!document.querySelector('[aria-modal="true"]');

/** Global keyboard shortcuts: Q/E rotate, Escape leaves build mode or skips playback. None of them act while a modal is open. */
export function attachKeys(game: Game, live?: Live): () => void {
  const onKey = (e: KeyboardEvent) => {
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'SELECT' || t.tagName === 'TEXTAREA')) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (modalOpen()) return;
    const k = e.key.toLowerCase();
    if (k === 'q') game.rotate(-1);
    else if (k === 'e') game.rotate(1);
    else if (live && game.state.get().mode === 'pressure' && (k === 'p' || k === 'l')) {
      if (k === 'p') live.togglePause();
      else if (game.state.get().live?.locked) live.unlock();
      else live.lock();
    } else if (k === 'escape') {
      const s = game.state.get();
      if (s.phase === 'playback') game.finishPlayback();
      else if (s.buildKind) game.setBuildKind(null);
    }
  };
  window.addEventListener('keydown', onKey);
  return () => window.removeEventListener('keydown', onKey);
}
