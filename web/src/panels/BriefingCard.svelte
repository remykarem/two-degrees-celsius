<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { briefingLines } from './format';
  import { Button } from '../ui-kit';
  import { ui } from './ui';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let dismissedTurn = $state(0);
  const turn = $derived($gs.snapshot?.turn ?? 0);
  const show = $derived($gs.phase === 'review' && dismissedTurn !== turn && !$ui.drawerOpen);
  const lines = $derived(briefingLines($gs.lastResult, $gs.levers, $gs.actions));
</script>

{#if show}
  <section class="card" aria-label="Briefing">
    <h2>Briefing · {#if $gs.snapshot}{quarterLabel(turn, $gs.snapshot.game)}{/if}</h2>
    <ul>{#each lines as line, i (i)}<li>{line}</li>{/each}</ul>
    <div class="row">
      <Button onclick={() => (dismissedTurn = turn)}>Dismiss</Button>
      <Button variant="primary" onclick={() => { dismissedTurn = turn; game.enterPlan(); }}>Start planning</Button>
    </div>
  </section>
{/if}

<style>
  .card { position: absolute; left: 12px; top: 12px; width: 330px; z-index: 15; background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius); box-shadow: var(--shadow); padding: 10px 12px; }
  ul { margin: 6px 0; padding-left: 18px; }
  .row { display: flex; gap: 8px; justify-content: flex-end; }
</style>
