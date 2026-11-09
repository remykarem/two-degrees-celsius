<script lang="ts">
  import { PALETTE, type Game } from '../store';
  import { Button } from '../ui-kit';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const locked = $derived($gs.phase === 'playback' || $gs.phase === 'results' || $gs.phase === 'ended');
</script>

<aside aria-label="Build palette">
  <h2>Build</h2>
  {#each PALETTE as item (item.kind)}
    <Button pressed={$gs.buildKind === item.kind} disabled={locked} title={item.hint}
      onclick={() => game.setBuildKind($gs.buildKind === item.kind ? null : item.kind)}>{item.label}</Button>
  {/each}
  <p class="hint">
    {#if $gs.buildKind}Click a tile to place. Esc to stop.{:else}Pick a building, then click the map.{/if}
  </p>
  <p class="hint">Firms, homes and forest are fixed. Tile position is cosmetic: it does not change the economy.</p>
</aside>

<style>
  aside { display: flex; flex-direction: column; gap: 6px; padding: 10px; background: var(--surface); border-right: 1px solid var(--border); overflow: auto; }
  .hint { color: var(--text-dim); font-size: 12px; margin: 4px 0; }
</style>
