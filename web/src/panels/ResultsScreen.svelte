<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { ARROW, describeCommand, TREND_WORD } from './format';
  import type { WhyKind } from './metrics';
  import { comingNextTurn, eventLines, lobbyLines, metricDeltas } from './results';
  import WhyView from './WhyView.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const r = $derived($gs.lastResult);
  const desc = (c: Parameters<typeof describeCommand>[0]) => describeCommand(c, $gs.levers, $gs.actions);
  let why = $state<WhyKind | null>(null);

  const deltas = $derived(metricDeltas($gs.previous, r?.snapshot ?? null));
  const events = $derived(eventLines(r?.months.flatMap((m) => m.events) ?? [], r?.snapshot ?? null));
  const lobby = $derived(r ? lobbyLines(r, $gs.levers, $gs.actions) : []);
  const soon = $derived(r ? comingNextTurn(r.snapshot) : { next: [], later: 0 });
  const over = $derived(!!r?.snapshot.gameOver);
  const toggle = (k: WhyKind | null) => { if (k) why = why === k ? null : k; };
  const IMP = ['', 'minor', 'notable', 'major'];
  const next = () => { why = null; game.continueFromResults(); };
</script>

{#if $gs.phase === 'results' && r}
  <Modal title="Results · {quarterLabel(r.turn, r.snapshot.game)}" size="lg" dismissible={false}>
    <div class="grid">
      <div class="col">
        <h3>What changed</h3>
        {#if deltas.length}
          <table>
            <thead><tr><th scope="col">Metric</th><th scope="col">Before</th><th scope="col">After</th><th scope="col"><span class="sr-only">Change</span></th><th scope="col"><span class="sr-only">Why</span></th></tr></thead>
            <tbody>
              {#each deltas as d (d.def.id)}
                <tr>
                  <th scope="row">{d.def.label}</th>
                  <td class="num dim">{d.from}</td>
                  <td class="num"><b>{d.to}</b></td>
                  <td class="chg {d.change.tone}"><span aria-hidden="true">{ARROW[d.change.trend]}</span><span class="sr-only">{TREND_WORD[d.change.trend]}</span></td>
                  <td>{#if d.def.why}<button class="link" aria-expanded={why === d.def.why} aria-label="Why did {d.def.label} change?" onclick={() => toggle(d.def.why)}>why?</button>{/if}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}<p class="dim">No earlier turn to compare with.</p>{/if}

        {#if why}
          <div class="whybox">
            <WhyView kind={why} trace={r.trace} snap={r.snapshot} levers={$gs.levers} />
            <Button variant="ghost" onclick={() => (why = null)}>Hide</Button>
          </div>
        {:else}
          <p class="dim hint">Pick <b>why?</b> on a row, or look at: <button class="link" onclick={() => (why = 'emissions')}>emissions by source</button>,
            <button class="link" onclick={() => (why = 'income')}>income by type</button>, <button class="link" onclick={() => (why = 'lobbying')}>lobbying gap</button>.</p>
        {/if}
      </div>

      <div class="col">
        <h3>What happened</h3>
        <ul class="ev">
          {#each events.lines as e, i (i)}
            <li class="i{e.importance}"><span class="imp" title="Importance: {IMP[e.importance]}"><span aria-hidden="true">{'●'.repeat(e.importance)}</span><span class="sr-only">{IMP[e.importance]}:</span></span> {e.text}</li>
          {:else}<li class="dim">A quiet quarter. Nothing notable.</li>{/each}
        </ul>
        {#if events.more}<p class="dim">and {events.more} more minor events</p>{/if}

        {#if lobby.length}
          <h3>Weakened or blocked by lobbying</h3>
          <ul>{#each lobby as l, i (i)}<li><span class="tagl">{l.kind === 'blocked' ? 'Blocked' : 'Weakened'}</span> {l.text}</li>{/each}</ul>
        {/if}
        {#if r.activated.length}
          <h3>Now in force</h3>
          <ul>{#each r.activated as a (a.id)}<li>{desc(a.command)}</li>{/each}</ul>
        {/if}
        <h3>Coming next turn</h3>
        {#if soon.next.length}
          <ul>{#each soon.next as a (a.id)}<li>{desc(a.command)}</li>{/each}</ul>
        {:else}<p class="dim">Nothing is scheduled for next turn.</p>{/if}
        {#if soon.later}<p class="dim">{soon.later} more item{soon.later === 1 ? '' : 's'} scheduled for later.</p>{/if}
      </div>
    </div>
    {#snippet footer()}
      <Button variant="primary" onclick={next}>{over ? 'See the ending ▸' : 'Continue ▸'}</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .grid { display: grid; grid-template-columns: 1.1fr 1fr; gap: 20px; }
  @media (max-width: 860px) { .grid { grid-template-columns: 1fr; } }
  h3 { margin: 10px 0 4px; } h3:first-child { margin-top: 0; }
  table { border-collapse: collapse; width: 100%; font-size: 13px; }
  th, td { padding: 3px 6px; text-align: left; border-bottom: 1px solid var(--border); }
  thead th { font-size: 11px; color: var(--text-dim); text-transform: uppercase; }
  tbody th { font-weight: 500; }
  .num { font-family: var(--font-mono); text-align: right; } .dim { color: var(--text-dim); }
  .chg { text-align: center; width: 24px; } .chg.good { color: var(--good); } .chg.bad { color: var(--bad); }
  .link { border: 0; background: transparent; color: var(--accent); text-decoration: underline; cursor: pointer; padding: 0 2px; font-size: inherit; }
  .whybox { margin-top: 10px; padding: 10px; background: var(--surface-2); border-radius: var(--radius); }
  .hint { font-size: 12px; }
  ul { margin: 2px 0; padding-left: 18px; } .ev { list-style: none; padding: 0; }
  .ev li { padding: 2px 0; } .i3 { font-weight: 700; } .i2 { font-weight: 500; }
  .imp { color: var(--text-dim); font-size: 10px; letter-spacing: -1px; margin-right: 4px; display: inline-block; min-width: 24px; }
  .tagl { font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--warn); }
</style>
