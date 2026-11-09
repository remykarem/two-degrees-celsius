<script lang="ts">
  import { totalMoney, totalPc, type Game } from '../store';
  import { Button } from '../ui-kit';
  import EndTurn from './EndTurn.svelte';
  import { describeCommand, leadText, money, REASON_TEXT } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const pcNow = $derived(Math.round($gs.snapshot?.pc.current ?? 0));
  const spend = $derived(totalPc($gs.draft));
</script>

<section aria-label="Plan this turn">
  <div class="head">
    <h2>Plan this turn</h2>
    <span class="num">PC {pcNow} → {Math.round(($gs.snapshot?.pc.current ?? 0) - spend)} · Money {money(-totalMoney($gs.draft))}</span>
    <span class="spacer"></span>
    {#if $gs.draft.length}<Button variant="ghost" onclick={() => game.clearDraft()}>Clear</Button>{/if}
    <EndTurn {game} />
  </div>
  {#if $gs.draft.length === 0}
    <p class="dim">Nothing queued. Move a policy slider or pick a building. Ending an empty turn is allowed.</p>
  {:else}
    <ol>
      {#each $gs.draft as d, i (d.id)}
        <li class:bad={d.preview.blocked}>
          <span class="n">{i + 1}</span>
          <span class="what">{describeCommand(d.command, $gs.levers, $gs.actions)}</span>
          {#if d.preview.blocked}
            <span class="why">Blocked: {REASON_TEXT[d.preview.blocked]}</span>
          {:else}
            <span class="dim">{leadText(d.preview.leadTime)}</span>
            <span class="num">−{d.preview.pc} PC</span>
            {#if d.preview.money > 0}<span class="num">−{money(d.preview.money)}</span>{/if}
          {/if}
          <button class="x" aria-label="Cancel: {describeCommand(d.command, $gs.levers, $gs.actions)}" onclick={() => game.removeDraft(d.id)}>✕</button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  section { padding: 8px 14px; background: var(--surface); border-top: 1px solid var(--border); max-height: 170px; overflow: auto; }
  .head { display: flex; align-items: center; gap: 12px; }
  .spacer { flex: 1; }
  ol { margin: 6px 0 0; padding: 0; list-style: none; display: flex; flex-wrap: wrap; gap: 6px 16px; }
  li { display: flex; gap: 8px; align-items: center; padding: 2px 8px; border: 1px solid var(--border); border-radius: var(--radius); }
  li.bad { border-color: var(--bad); }
  .n { font-weight: 700; }
  .why { color: var(--bad); font-weight: 600; }
  .dim { color: var(--text-dim); }
  .num { font-family: var(--font-mono); }
  .x { border: 0; background: transparent; cursor: pointer; color: var(--text); font-size: 14px; padding: 2px 6px; }
  p { margin: 4px 0 0; }
</style>
