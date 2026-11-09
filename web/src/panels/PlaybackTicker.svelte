<script lang="ts">
  import type { Game } from '../store';
  import { Button } from '../ui-kit';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const lines = $derived(($gs.lastResult?.months ?? []).filter((m) => m.month <= $gs.playback.month)
    .flatMap((m) => m.events.map((e) => ({ month: m.month, text: e.text ?? e.type, importance: e.importance }))));
</script>

{#if $gs.phase === 'playback'}
  <section class="ticker" aria-label="Quarter playback" aria-live="polite">
    <div class="head">
      <strong>{$gs.playback.month < 0 ? 'Starting…' : `Month ${$gs.playback.month + 1} of 3`}</strong>
      <Button onclick={() => game.finishPlayback()}>Skip ▸▸</Button>
    </div>
    <ul>{#each lines as l, i (i)}<li class:big={l.importance >= 2}><span class="m">M{l.month + 1}</span> {l.text}</li>{/each}</ul>
  </section>
{/if}

<style>
  .ticker { position: absolute; left: 12px; top: 12px; width: 380px; max-height: 45%; overflow: auto; z-index: 15; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow); padding: 8px 12px; }
  .head { display: flex; justify-content: space-between; align-items: center; }
  ul { list-style: none; margin: 6px 0 0; padding: 0; font-size: 13px; }
  li.big { font-weight: 600; }
  .m { font-family: var(--font-mono); color: var(--text-dim); margin-right: 4px; }
</style>
