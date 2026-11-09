<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { Button } from '../ui-kit';
  import { describeCommand, leadText, turnsUntil } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const canCancel = $derived($gs.phase === 'review' || $gs.phase === 'plan');
  const snap = $derived($gs.snapshot);
</script>

<section aria-label="Pending items">
  <h2>Pending</h2>
  {#if !snap?.pending.length}
    <span class="dim">Nothing scheduled. Items you queue take effect after their lead time and appear here.</span>
  {:else}
    <ul>
      {#each snap.pending as p (p.id)}
        <li>
          <span>{describeCommand(p.command, $gs.levers, $gs.actions)}</span>
          <span class="dim">{leadText(turnsUntil(p.effectiveTurn, snap.turn))} ({quarterLabel(p.effectiveTurn, snap.game)})</span>
          {#if canCancel}<Button variant="ghost" title="Cancel and refund half the political capital; no money was charged yet" onclick={() => game.cancelPending(p.id)}>Cancel (+{(p.pcCost / 2).toFixed(1)} PC)</Button>{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  section { display: flex; align-items: center; gap: 14px; padding: 4px 14px; background: var(--surface-2); border-top: 1px solid var(--border); min-height: 38px; overflow: auto; }
  ul { display: flex; gap: 14px; margin: 0; padding: 0; list-style: none; }
  li { display: flex; align-items: center; gap: 6px; white-space: nowrap; }
  .dim { color: var(--text-dim); }
</style>
