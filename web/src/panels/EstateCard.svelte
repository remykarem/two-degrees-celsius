<script lang="ts">
  import type { Snapshot } from '../contract';
  import type { Game } from '../store';
  import { estateCard } from './estate';

  let { game, snap, compact = false }: { game: Game; snap: Snapshot; compact?: boolean } = $props();
  const c = $derived(estateCard(snap));
  const maxPart = $derived(Math.max(1, ...(c?.wealthParts ?? []).map((p) => Math.abs(p.value))));
  const hid = $derived(snap.estate?.household ?? null);
</script>

{#if c}
  <section class="estate" aria-labelledby="estate-h" data-testid="estate-card">
    <h3 id="estate-h">{c.title}: <span class="tag">{c.tag}</span>{#if c.band} <span class="band">{c.band}</span>{/if}</h3>
    {#if c.topLine}<p class="top" data-testid="estate-top-line">{c.topLine}</p>{/if}
    <p class="nw">Net worth <b>{c.netWorth}</b>
      {#if hid !== null}<button type="button" class="lnk" onclick={() => game.select({ kind: 'household', id: hid })}>show on the map</button>{/if}</p>

    {#if !compact && c.wealthParts.length}
      <h4>What it owns</h4>
      <ul class="bars">{#each c.wealthParts as p (p.label)}
        <li><span>{p.label}</span><span class="track" aria-hidden="true"><span class="bar" class:neg={p.value < 0} style:width="{(Math.abs(p.value) / maxPart) * 100}%"></span></span><span class="v">{p.text}</span></li>{/each}</ul>
    {/if}

    <h4>Income, per month ({c.incomeTotal})</h4>
    <ul class="bars">{#each c.income as p (p.label)}<li class="i"><span>{p.label}</span><span class="v">{p.text}</span></li>{:else}<li class="i dim">none</li>{/each}</ul>

    <h4>Private jet</h4>
    <p class="fl" data-testid="estate-flights">{c.flights}</p>
    <dl>{#each c.jet as r (r.label)}<dt>{r.label}</dt><dd>{r.value}</dd>{/each}</dl>

    {#if c.pilot}
      <h4>{c.pilot.name} (the jet's pilot)</h4>
      <dl>{#each c.pilot.rows as r (r.label)}<dt>{r.label}</dt><dd>{r.value}</dd>{/each}</dl>
      {#if c.pilot.warning}<p class="warn" role="alert" data-testid="pilot-warning">{c.pilot.warning}</p>{/if}
      {#if c.pilot.grounded}<p class="warn" role="status" data-testid="pilot-grounded">{c.pilot.grounded}</p>{/if}
    {/if}

    <h4>Estate money</h4>
    <table>
      <thead><tr><th scope="col"><span class="sr-only">Item</span></th><th scope="col" class="n">Last month</th><th scope="col" class="n">Last {c.moneyMonths} {c.moneyMonths === 1 ? 'month' : 'months'}</th></tr></thead>
      <tbody>{#each c.money as r (r.label)}<tr><th scope="row">{r.label}</th><td class="n">{r.month}</td><td class="n">{r.last12}</td></tr>{/each}</tbody>
    </table>
    {#if c.shortfall}<p class="warn" role="status" data-testid="wealth-tax-short">{c.shortfall}</p>{/if}
  </section>
{/if}

<style>
  .estate { border: 1px solid var(--border); border-left: 4px solid var(--series-2); border-radius: var(--radius); padding: 8px 10px; margin-bottom: 10px; background: var(--surface-2); display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  h3 { margin: 0; font-size: 14px; text-transform: none; letter-spacing: 0; color: var(--text); }
  .tag { font-weight: 700; } .band { font-size: 11px; font-weight: 600; border: 1px solid var(--border); border-radius: 8px; padding: 0 6px; color: var(--text-dim); }
  h4 { margin: 6px 0 1px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  p { margin: 0; font-size: 12px; } .top { font-size: 12.5px; font-weight: 600; } .nw b { font-family: var(--font-mono); } .fl { color: var(--text); }
  .warn { color: var(--warn); font-weight: 600; } .dim { color: var(--text-dim); }
  dl { display: grid; grid-template-columns: 1fr auto; gap: 1px 10px; margin: 0; font-size: 12px; } dt { color: var(--text-dim); } dd { margin: 0; text-align: right; font-family: var(--font-mono); }
  .bars { list-style: none; margin: 0; padding: 0; font-size: 12px; display: grid; gap: 2px; }
  .bars li { display: grid; grid-template-columns: 110px 1fr 64px; gap: 6px; align-items: center; } .bars li.i { grid-template-columns: 1fr auto; }
  .track { height: 8px; background: var(--surface); border-radius: 2px; overflow: hidden; display: block; } .bar { display: block; height: 100%; background: var(--series-2); } .bar.neg { background: var(--bad); }
  .v { text-align: right; font-family: var(--font-mono); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } thead th { font-size: 10px; text-transform: uppercase; color: var(--text-dim); } th[scope='row'] { text-align: left; font-weight: 400; } .n { text-align: right; font-family: var(--font-mono); } tr { border-top: 1px dotted var(--border); }
  .lnk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; font-size: 11px; margin-left: 6px; }
</style>
