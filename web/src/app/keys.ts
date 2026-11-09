import type { Game } from '../store';

/** Global keyboard shortcuts: Q/E rotate, Escape leaves build mode or skips playback. */
export function attachKeys(game: Game): () => void {
  const onKey = (e: KeyboardEvent) => {
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'SELECT' || t.tagName === 'TEXTAREA')) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const k = e.key.toLowerCase();
    if (k === 'q') game.rotate(-1);
    else if (k === 'e') game.rotate(1);
    else if (k === 'escape') {
      const s = game.state.get();
      if (s.phase === 'playback') game.finishPlayback();
      else if (s.buildKind) game.setBuildKind(null);
    }
  };
  window.addEventListener('keydown', onKey);
  return () => window.removeEventListener('keydown', onKey);
}
