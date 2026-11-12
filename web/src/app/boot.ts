/** Boot shell: a loading state while the simulation starts, and a static error screen when startup fails (no framework, so it works even if the app never mounted). */

/** The slice of a DOM element the shell needs (a fake in tests). */
export interface BootTarget {
  innerHTML: string;
  querySelector(selector: string): { addEventListener(type: 'click', fn: () => void): void } | null;
}

const escapeHtml = (s: string) => s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);

const BOX = 'font:14px/1.4 var(--font);color:var(--text);background:var(--bg);min-height:100vh;display:grid;place-items:center;padding:16px;box-sizing:border-box';

export function loadingHtml(): string {
  return `<div style="${BOX}" role="status" aria-live="polite"><p>Loading the simulation...</p></div>`;
}

export function bootErrorHtml(message: string): string {
  return `<div style="${BOX}" role="alert"><div style="max-width:32rem"><h1 style="font-size:18px;margin:0 0 8px">The game could not start</h1>`
    + `<p>The simulation failed to load, so there is nothing to play yet. Reloading usually fixes it. Saves are files you download, so nothing you saved is lost.</p>`
    + `<p style="color:var(--text-dim);font-family:var(--font-mono);word-break:break-word">${escapeHtml(message)}</p>`
    + `<button type="button" data-reload style="font:inherit;padding:8px 16px;cursor:pointer">Reload</button></div></div>`;
}

/** Shows the loading state, runs `start`, and replaces the page with the error screen if it throws. Resolves true when it started. */
export async function boot(target: BootTarget, start: () => Promise<void>, reload: () => void = () => location.reload()): Promise<boolean> {
  target.innerHTML = loadingHtml();
  try {
    await start();
    return true;
  } catch (e) {
    console.error(e);
    target.innerHTML = bootErrorHtml(e instanceof Error ? e.message : String(e));
    target.querySelector('[data-reload]')?.addEventListener('click', reload);
    return false;
  }
}
