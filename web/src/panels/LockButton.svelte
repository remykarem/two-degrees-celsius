<script lang="ts">
  import type { Game, Live } from '../store';
  import { Button } from '../ui-kit';

  let { game, live }: { game: Game; live: Live } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  // svelte-ignore state_referenced_locally
  const v = live.view;
  const locked = $derived(!!$gs.live?.locked);
  const next = $derived($v.planning ?? 'the next quarter');
</script>

<span class="wrap" data-coach="endturn">
  {#if locked}
    <span class="state" role="status"><span aria-hidden="true">🔒 </span>Plan locked for {next}</span>
    <Button onclick={() => live.unlock()} title="Unlock to change the plan again (L)">Unlock to edit</Button>
  {:else}
    <Button variant="primary" disabled={!$v.active || !!$gs.snapshot?.gameOver} onclick={() => live.lock()}
      title="Lock the plan so it cannot change. It is submitted when {next} begins, whether or not you lock it (L)">Lock plan · applies {next} ▸</Button>
  {/if}
</span>

<style>
  .wrap { display: inline-flex; gap: 8px; align-items: center; }
  .state { color: var(--good); font-weight: 600; }
</style>
