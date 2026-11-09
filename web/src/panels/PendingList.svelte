<script lang="ts">
  import { PALETTE, type Game } from '../store';
  import { Button } from '../ui-kit';
  import { describeCommand } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const infraLabel = (k: string) => PALETTE.find((p) => p.kind === k)?.label ?? k;
  const canCancel = $derived($gs.phase === 'review' || $gs.phase === 'plan');
</script>

<section aria-label="Pending items and event log">
  <h2>Pending</h2>
  {#if !$gs.snapshot?.pending.length}
    <span class="dim">Nothing scheduled.</span>
  {:else}
    <ul>
      {#each $gs.snapshot.pending as p (p.id)}
        <li>
          {describeCommand(p.command, $gs.levers, infraLabel)} · effective turn {p.effectiveTurn}
          {#if canCancel}<Button variant="ghost" title="Cancel and refund 50% of the PC" onclick={() => game.cancelPending(p.id)}>Cancel (refund {Math.floor(p.pcCost / 2)} PC)</Button>{/if}
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
