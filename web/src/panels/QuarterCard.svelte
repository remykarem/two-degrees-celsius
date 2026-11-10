<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { ARROW, describeCommand, TREND_WORD } from './format';
  import { blockText } from './policy';
  import { eventLines, metricDeltas } from './results';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const sum = $derived($gs.mode === 'pressure' ? $gs.live?.summary ?? null : null);
  const report = $derived($gs.mode === 'pressure' ? $gs.live?.report ?? null : null);
  const deltas = $derived(sum ? metricDeltas($gs.previous, sum.snapshot).slice(0, 5) : []);
  const events = $derived(sum ? eventLines(sum.months.flatMap((m) => m.events), sum.snapshot, 3) : { lines: [], more: 0 });
  const desc = (c: Parameters<typeof describeCommand>[0]) => describeCommand(c, $gs.levers, $gs.actions);
</script>

<!-- Non-blocking by design: no focus is taken, nothing is modal, the clock never waits for it. -->
{#if sum || report || $gs.live?.notice}
  <div class="stack" aria-label="Quarter report">
    {#if report}
      <section class="card warn" role="status" aria-label="Commands not carried out">
        <header><strong>Not carried out at the start of the quarter</strong><button class="x" aria-label="Dismiss" onclick={() => game.dismissReport()}>✕</button></header>
        <ul>{#each report.rejected as r (r.index)}<li>{desc(r.command)} <span class="dim">({blockText(r.reason)})</span></li>{/each}</ul>
      </section>
    {/if}
    {#if $gs.live?.notice}
      <section class="card warn" role="status"><header><span>{$gs.live.notice}</span><button class="x" aria-label="Dismiss" onclick={() => game.setLiveNotice(null)}>✕</button></header></section>
    {/if}
    {#if sum}
      <section class="card" aria-label="Last quarter summary">
        <header><strong>{quarterLabel(sum.turn, sum.snapshot.game)} finished</strong><button class="x" aria-label="Dismiss quarter summary" onclick={() => game.dismissSummary()}>✕</button></header>
        <table>
          <tbody>
            {#each deltas as d (d.def.id)}
              <tr><th scope="row">{d.def.label}</th><td class="num">{d.to}</td><td class="chg {d.change.tone}"><span aria-hidden="true">{ARROW[d.change.trend]}</span><span class="sr-only">{TREND_WORD[d.change.trend]}</span></td></tr>
            {/each}
          </tbody>
        </table>
        {#if events.lines.length}<ul class="ev">{#each events.lines as e, i (i)}<li class:big={e.importance >= 2}>{e.text}</li>{/each}</ul>{/if}
      </section>
    {/if}
  </div>
{/if}

<style>
  .stack { position: absolute; left: 12px; top: 12px; width: 290px; z-index: 14; display: flex; flex-direction: column; gap: 8px; pointer-events: none; max-height: calc(100% - 24px); overflow: hidden; }
  .card { pointer-events: auto; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow); padding: 6px 10px; font-size: 12px; opacity: 0.96; }
  .card.warn { border-color: var(--warn); }
  header { display: flex; justify-content: space-between; gap: 8px; align-items: baseline; }
  .x { border: 0; background: transparent; color: var(--text); cursor: pointer; }
  table { width: 100%; border-collapse: collapse; margin-top: 2px; }
  th { text-align: left; font-weight: 500; padding: 1px 0; }
  td { padding: 1px 4px; }
  .num { font-family: var(--font-mono); text-align: right; }
  .chg { width: 18px; text-align: center; } .chg.good { color: var(--good); } .chg.bad { color: var(--bad); }
  ul { margin: 4px 0 0; padding-left: 16px; } .ev { list-style: none; padding: 0; } li.big { font-weight: 600; }
  .dim { color: var(--text-dim); }
</style>
