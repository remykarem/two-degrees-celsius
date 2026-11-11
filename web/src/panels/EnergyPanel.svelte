<script lang="ts">
  import type { Game } from '../store';
  import { energyBalance, energyText, intensityText, plantRows, unitHelp, unitLabel } from './energy';
  import { pct, tonnes } from './format';
  import { pctWith } from './backing';
  import HowButton from './HowButton.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const e = $derived($gs.snapshot?.energy ?? null);
  const plants = $derived(e ? plantRows(e) : []);
  const bal = $derived(e ? energyBalance(e) : null);
  const focus = (t: { x: number; y: number }) => game.select({ kind: 'tile', x: t.x, y: t.y });
</script>

{#if e && bal}
  <section class="energy" aria-labelledby="energy-h" data-testid="energy-panel">
    <h3 id="energy-h">Power grid <HowButton anchor="energy" label="the power grid" /> <button type="button" class="unit" title={unitHelp(e)} aria-label="Unit: {unitLabel(e)}. {unitHelp(e)}">unit: {unitLabel(e)} (?)</button></h3>
    {#if bal.warning}<p class="warn" role="alert">{bal.warning}</p>{/if}
    <dl class="kv">
      <dt>Capacity</dt><dd>{energyText(bal.capacity, e)}</dd>
      <dt>Demand</dt><dd>{energyText(bal.demand, e)}</dd>
      <dt>Served share</dt><dd class:bad={bal.rationed}>{bal.servedText}</dd>
      <dt>Renewable share</dt><dd>{pctWith(e.renewableShare, `${plants.filter((p) => p.kind === 'renewable').length} of ${plants.length} plants`)}</dd>
      <dt>Grid intensity</dt><dd>{intensityText(e)}</dd>
    </dl>
    <div class="split" role="img" aria-label="Demand split: firms {energyText(e.demand.firms, e)}, households {energyText(e.demand.households, e)}">
      <span class="f" style:width="{bal.firmsShare * 100}%"></span><span class="h" style:width="{bal.householdsShare * 100}%"></span>
    </div>
    <div class="splitl"><span><i class="f"></i>Firms {energyText(e.demand.firms, e)}</span><span><i class="h"></i>Households {energyText(e.demand.households, e)}</span></div>
    <table>
      <thead><tr><th scope="col">Plant</th><th scope="col">Kind</th><th scope="col" class="n">Capacity</th><th scope="col" class="n">Share</th><th scope="col" class="n">Emits</th></tr></thead>
      <tbody>
        {#each plants as p (`${p.tile.x},${p.tile.y}`)}
          <tr><th scope="row"><button type="button" class="lnk" onclick={() => focus(p.tile)}>({p.tile.x}, {p.tile.y})</button></th><td>{p.kindLabel}</td><td class="n">{p.capacityText}</td><td class="n">{pct(p.outputShare)}</td><td class="n">{p.emissions > 0 ? `${tonnes(p.emissions)}/mo` : 'none'}</td></tr>
        {:else}<tr><td colspan="5" class="dim">No power plants.</td></tr>{/each}
      </tbody>
    </table>
  </section>
{/if}

<style>
  .energy { margin: 0 0 12px; }
  .unit { background: transparent; border-top: 0; border-left: 0; border-right: 0; font: inherit; font-size: 11px; font-weight: 400; text-transform: none; letter-spacing: 0; font-size: 11px; color: var(--accent); cursor: help; margin-left: 4px; border-bottom: 1px dotted var(--accent); }
  .warn { margin: 4px 0; padding: 4px 6px; background: var(--bad-bg); color: var(--bad); border: 1px solid var(--bad); border-radius: var(--radius); font-size: 12px; }
  .kv { display: grid; grid-template-columns: 1fr auto; gap: 1px 8px; font-size: 12px; margin: 4px 0; }
  .kv dt { color: var(--text-dim); } .kv dd { margin: 0; text-align: right; font-family: var(--font-mono); } .kv dd.bad { color: var(--bad); font-weight: 700; }
  .split { display: flex; height: 8px; border-radius: 4px; overflow: hidden; background: var(--surface-2); }
  .f { background: var(--series-1); } .h { background: var(--series-2); }
  .splitl { display: flex; justify-content: space-between; gap: 6px; font-size: 11px; margin: 2px 0 6px; flex-wrap: wrap; }
  .splitl i { display: inline-block; width: 8px; height: 8px; margin-right: 3px; }
  .splitl i.f { background: var(--series-1); } .splitl i.h { background: var(--series-2); }
  table { width: 100%; border-collapse: collapse; font-size: 11.5px; }
  th, td { padding: 2px 3px; text-align: left; border-top: 1px dotted var(--border); } thead th { font-size: 10px; text-transform: uppercase; color: var(--text-dim); border-top: 0; }
  th[scope='row'] { font-weight: 400; }
  .n { text-align: right; font-family: var(--font-mono); white-space: nowrap; } .dim { color: var(--text-dim); }
  .lnk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; }
</style>
