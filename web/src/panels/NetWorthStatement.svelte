<script lang="ts">
  import HowButton from './HowButton.svelte';
  import type { StatementModel } from './networth';

  let { model, compact = false }: { model: StatementModel; compact?: boolean } = $props();
  const maxV = $derived(Math.max(1, ...model.groups.flatMap((g) => g.rows.map((r) => Math.abs(r.amount)))));
</script>

<div class="nw" data-testid="networth-statement">
  {#if !compact}<h4>{model.title}</h4>{/if}
  {#each model.groups as g (g.category)}
    {#if g.rows.length}
      <div class="grp" data-testid="networth-{g.category}">
        <div class="gh"><span>{g.heading}</span><span class="n">{g.total}</span></div>
        <ul>{#each g.rows as r (r.id)}
          <li data-testid="networth-line-{r.id}">
            <span class="l">{r.label}{#if r.count}<span class="c"> &middot; {r.count}</span>{/if}{#if r.how} <HowButton anchor={r.how} label={r.label} />{/if}</span>
            <span class="track" aria-hidden="true"><span class="bar" class:liab={g.category === 'liability'} style:width="{(Math.abs(r.amount) / maxV) * 100}%"></span></span>
            <span class="n">{g.category === 'liability' ? '-' : ''}{r.value}</span>
          </li>{/each}</ul>
      </div>
    {/if}
  {/each}
  <div class="gh net"><span>{model.netLabel}</span><span class="n" data-testid="networth-net">{model.netWorth}</span></div>
  <p class="rc" class:bad={!model.reconciles} role="status" data-testid="networth-reconciles" data-reconciles={model.reconciles}>{model.reconciles ? 'Reconciles. ' : 'Does not reconcile. '}{model.reconcileText}</p>
  {#if !compact}<p class="dim">{model.note}</p><p class="dim">{model.unpaidRentNote}</p>{/if}
</div>

<style>
  .nw { font-size: 12px; } h4 { margin: 8px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  .gh { display: flex; justify-content: space-between; font-weight: 700; margin-top: 4px; border-bottom: 1px solid var(--border); } .net { border-top: 2px solid var(--border); border-bottom: 0; margin-top: 6px; }
  ul { list-style: none; margin: 0; padding: 0; } li { display: grid; grid-template-columns: minmax(0, 1fr) 40px 62px; gap: 6px; align-items: center; padding: 1px 0; }
  .l { overflow-wrap: anywhere; } .c { color: var(--text-dim); } .n { text-align: right; font-family: var(--font-mono); }
  .track { height: 6px; background: var(--surface-2); border-radius: 2px; overflow: hidden; display: block; } .bar { display: block; height: 100%; background: var(--series-1); } .bar.liab { background: var(--series-2); }
  .rc { margin: 4px 0 0; color: var(--text-dim); } .rc.bad { color: var(--bad); font-weight: 700; } .dim { margin: 2px 0; color: var(--text-dim); font-size: 11px; }
</style>
