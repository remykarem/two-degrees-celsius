<script lang="ts">
  import type { Game } from '../store';
  import { Button } from '../ui-kit';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let input: HTMLInputElement;
  let message = $state('');
  let bad = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** The message fades after a while so it does not keep the top bar tall. */
  function say(text: string, isBad: boolean) {
    message = text;
    bad = isBad;
    clearTimeout(timer);
    timer = setTimeout(() => (message = ''), isBad ? 9000 : 4000);
  }

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
    const s = game.state.get();
    say(s.mode === 'pressure' && s.live?.open
      ? 'Saved the start of the quarter now playing, with your current drafts. Progress inside a quarter is not saved.' : 'Game saved to a file.', false);
  }
  async function load(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = '';
    if (!f) return;
    const err = game.importSave(await f.text());
    say(err ? `Could not load that file. ${err}` : 'Saved game loaded.', !!err);
  }
</script>

<span class="sl">
  <Button onclick={save} disabled={!!$gs.demo} title={$gs.demo ? 'A recorded demo cannot be saved' : 'Download this game as a file'}>Save</Button>
  <Button onclick={() => input.click()} disabled={!!$gs.demo} title={$gs.demo ? 'A recorded demo cannot load a save' : 'Load a game from a saved file'}>Load</Button>
  <input bind:this={input} type="file" accept="application/json,.json" class="hide" aria-label="Choose a saved game file" onchange={load} tabindex="-1" />
  <span class="msg" class:bad role="status">{message}</span>
</span>

<style>
  .sl { display: inline-flex; gap: 6px; align-items: center; }
  .hide { display: none; }
  /* A toast floating below the top bar, so a message never makes the header wrap. */
  .msg { position: fixed; top: 54px; right: 14px; z-index: 600; font-size: 13px; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 6px 12px; box-shadow: var(--shadow); max-width: 320px; }
  .msg.bad { color: var(--bad); border-color: var(--bad); font-weight: 600; }
  .msg:empty { display: none; }
</style>
