<script lang="ts">
  import type { Game } from '../store';
  import { Button } from '../ui-kit';

  let { game }: { game: Game } = $props();
  let input: HTMLInputElement;
  let message = $state('');
  let bad = $state(false);

  function save() {
    const text = game.exportSave();
    if (text === null) return;
    const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `ecotown-turn-${game.state.get().snapshot?.turn ?? 0}.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    bad = false;
    message = 'Game saved to a file.';
  }
  async function load(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = '';
    if (!f) return;
    const err = game.importSave(await f.text());
    bad = !!err;
    message = err ? `Could not load that file. ${err}` : 'Saved game loaded.';
  }
</script>

<span class="sl">
  <Button onclick={save} title="Download this game as a file">Save</Button>
  <Button onclick={() => input.click()} title="Load a game from a saved file">Load</Button>
  <input bind:this={input} type="file" accept="application/json,.json" class="hide" aria-label="Choose a saved game file" onchange={load} tabindex="-1" />
  <span class="msg" class:bad role="status">{message}</span>
</span>

<style>
  .sl { display: inline-flex; gap: 6px; align-items: center; }
  .hide { display: none; }
  .msg { font-size: 12px; color: var(--text-dim); max-width: 280px; }
  .msg.bad { color: var(--bad); font-weight: 600; }
  .msg:empty { display: none; }
</style>
