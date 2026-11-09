<script lang="ts">
  import { PALETTE, quarterLabel, type Game } from '../store';
  import { Button } from '../ui-kit';
  import { describeCommand, pct } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const r = $derived($gs.lastResult);
  const infraLabel = (k: string) => PALETTE.find((p) => p.kind === k)?.label ?? k;
  const desc = (c: Parameters<typeof describeCommand>[0]) => describeCommand(c, $gs.levers, infraLabel);

  const deltas = $derived.by(() => {
    const a = r?.snapshot, b = $gs.previous;
    if (!a || !b) return [];
    const row = (label: string, now: number, prev: number, goodUp: boolean, fmt: (v: number) => string) => ({
      label, now: fmt(now), arrow: Math.abs(now - prev) < 1e-9 ? '■' : now > prev ? '▲' : '▼', good: (now - prev) * (goodUp ? 1 : -1) >= 0,
    });
    return [
      row('GDP', a.indicators.gdpRealAnnual, b.indicators.gdpRealAnnual, true, (v) => `$${(v / 1e6).toFixed(2)}M`),
      row('Unemployment', a.indicators.unemployment, b.indicators.unemployment, false, (v) => pct(v, 1)),
      row('Inequality', a.indicators.gini, b.indicators.gini, false, (v) => v.toFixed(3)),
      row('Emissions / month', a.nature.grossEmissionsMonthly, b.nature.grossEmissionsMonthly, false, (v) => v.toFixed(0) + ' t'),
      row('Legitimacy', a.government.legitimacy, b.government.legitimacy, true, (v) => pct(v, 1)),
      row('Wellbeing', a.indicators.avgWellbeing, b.indicators.avgWellbeing, true, (v) => v.toFixed(3)),
    ];
  });
  const events = $derived((r?.months.flatMap((m) => m.events) ?? []).filter((e) => e.text).sort((a, b) => b.importance - a.importance).slice(0, 6));
  const next = $derived((r?.snapshot.pending ?? []).filter((p) => p.effectiveTurn === (r?.snapshot.turn ?? 0) + 1));
</script>

{#if $gs.phase === 'results' && r}
  <div class="backdrop">
    <div class="panel" role="dialog" aria-modal="true" aria-labelledby="rt">
      <h2 id="rt">Results · {quarterLabel(r.turn)}</h2>
      {#if deltas.length}
        <h3>What changed</h3>
        <ul class="deltas">
          {#each deltas as d (d.label)}
            <li><span class:good={d.good} class:bad={!d.good}>{d.arrow}</span> {d.label} <b>{d.now}</b>
              <details><summary>why?</summary>
                <ul class="trace">{#each r.trace.slice(0, 4) as t, i (i)}<li>{JSON.stringify(t)}</li>{:else}<li>No causal trace in the mock.</li>{/each}</ul>
              </details></li>
          {/each}
        </ul>
      {/if}
      <h3>Events</h3>
      <ul>{#each events as e, i (i)}<li>{'●'.repeat(e.importance)} {e.text}</li>{:else}<li>Nothing notable.</li>{/each}</ul>
      {#if r.rejected.length || r.policyGaps.length}
        <h3>Blocked or weakened by lobbying</h3>
        <ul>
          {#each r.rejected as x, i (i)}<li>Blocked: {desc(x.command)} ({x.reason})</li>{/each}
          {#each r.policyGaps as g (g.lever)}<li>{g.lever}: set {g.nominal}, about {g.effective.toFixed(2)} in force</li>{/each}
        </ul>
      {/if}
      {#if r.activated.length}<h3>Now in force</h3><ul>{#each r.activated as a (a.id)}<li>{desc(a.command)}</li>{/each}</ul>{/if}
      {#if next.length}<h3>Coming next turn</h3><ul>{#each next as a (a.id)}<li>{desc(a.command)}</li>{/each}</ul>{/if}
      <div class="row"><Button variant="primary" onclick={() => game.continueFromResults()}>{r.snapshot.gameOver ? 'See the ending ▸' : 'Continue ▸'}</Button></div>
    </div>
  </div>
{/if}

<style>
  .backdrop { position: absolute; inset: 0; background: rgba(0, 0, 0, 0.3); display: grid; place-items: center; z-index: 30; }
  .panel { background: var(--surface); border-radius: var(--radius); box-shadow: var(--shadow); padding: 14px 18px; width: min(560px, 92%); max-height: 92%; overflow: auto; }
  h3 { margin-top: 10px; }
  ul { margin: 4px 0; padding-left: 18px; }
  .deltas { list-style: none; padding: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 2px 16px; }
  .good { color: var(--good); } .bad { color: var(--bad); }
  details { display: inline; }
  .trace { font-size: 11px; font-family: var(--font-mono); }
  .row { display: flex; justify-content: flex-end; margin-top: 12px; }
</style>
