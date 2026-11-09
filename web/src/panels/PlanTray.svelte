<script lang="ts">
  import { totalMoney, totalPc, type Game } from '../store';
  import { Button } from '../ui-kit';
  import EndTurn from './EndTurn.svelte';
  import { describeCommand, leadText, money } from './format';
  import { blockText } from './policy';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const pcNow = $derived($gs.snapshot?.pc.current ?? 0);
  const r1 = (v: number) => Math.round(v * 10) / 10;
  const spend = $derived(totalPc($gs.draft));
  const left = $derived(pcNow - spend);
  const blocked = $derived($gs.draft.filter((d) => d.preview.blocked).length);
  const leads = $derived($gs.draft.filter((d) => !d.preview.blocked).map((d) => d.preview.leadTime));
  const leadRange = $derived(leads.length ? (Math.min(...leads) === Math.max(...leads) ? leadText(leads[0]!) : `${leadText(Math.min(...leads))} to ${leadText(Math.max(...leads))}`) : '');
</script>

<section aria-label="Plan this turn" data-coach="plan">
  <div class="head">
    <h2>Plan this turn</h2>
    <span class="sum num" class:neg={left < 0} title="Political capital now, after this plan">PC {r1(pcNow)} → {r1(left)}</span>
    <span class="sum num" title="Money is charged when each item takes effect, not now">Money −{money(totalMoney($gs.draft))} on activation</span>
    {#if leadRange}<span class="sum dim">Takes effect {leadRange}</span>{/if}
    <span class="spacer"></span>
    {#if $gs.draft.length}<Button variant="ghost" onclick={() => game.clearDraft()}>Clear plan</Button>{/if}
    <EndTurn {game} coach />
  </div>
  {#if blocked}<p class="err" role="alert">{blocked} blocked item{blocked === 1 ? '' : 's'} will be skipped. Remove or change {blocked === 1 ? 'it' : 'them'}.</p>{/if}
  {#if $gs.draft.length === 0}
    <p class="dim">Nothing queued. Open Policies and move a slider, or pick a building and click the map. Ending an empty turn is allowed.</p>
  {:else}
    <ol>
      {#each $gs.draft as d, i (d.id)}
        {@const text = describeCommand(d.command, $gs.levers, $gs.actions)}
        <li class:bad={d.preview.blocked}>
          <span class="n">{i + 1}</span>
          <span class="what">{text}</span>
          {#if d.preview.blocked}
            <span class="why"><span aria-hidden="true">⚠ </span>Blocked: {blockText(d.preview.blocked)}</span>
          {:else}
            <span class="dim">{leadText(d.preview.leadTime)}</span>
            <span class="num">−{d.preview.pc} PC</span>
            {#if d.preview.money > 0}<span class="num">−{money(d.preview.money)}</span>{/if}
          {/if}
          <button class="x" aria-label="Remove from plan: {text}" onclick={() => game.removeDraft(d.id)}>✕</button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  section { padding: 6px 14px; background: var(--surface); border-top: 1px solid var(--border); max-height: 150px; overflow: auto; }
  .head { display: flex; align-items: center; gap: 6px 14px; flex-wrap: wrap; }
  .spacer { flex: 1; }
  .sum { font-size: 13px; }
  .neg { color: var(--bad); font-weight: 700; }
  ol { margin: 6px 0 0; padding: 0; list-style: none; display: flex; flex-wrap: wrap; gap: 6px 12px; }
  li { display: flex; gap: 8px; align-items: center; padding: 2px 8px; border: 1px solid var(--border); border-radius: var(--radius); font-size: 13px; }
  li.bad { border-color: var(--bad); background: var(--bad-bg); }
  .n { font-weight: 700; }
  .why { color: var(--bad); font-weight: 600; }
  .err { color: var(--bad); margin: 4px 0 0; font-size: 12px; font-weight: 600; }
  .dim { color: var(--text-dim); }
  .num { font-family: var(--font-mono); }
  .x { border: 0; background: transparent; cursor: pointer; color: var(--text); font-size: 14px; padding: 2px 6px; }
  p { margin: 4px 0 0; font-size: 13px; }
</style>
