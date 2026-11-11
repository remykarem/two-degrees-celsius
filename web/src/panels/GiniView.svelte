<script lang="ts">
  import type { Snapshot } from '../contract';
  import { money } from './format';
  import { lorenzPath } from './lorenz';
  import BreakdownTable from './BreakdownTable.svelte';

  let { snap }: { snap: Snapshot } = $props();
  const g = $derived(snap.breakdowns?.gini ?? null);
  const S = 200;
  const paths = $derived(g ? lorenzPath(g.lorenz, S, S, 8) : null);
</script>

{#if g && paths}
  <div class="gv" data-testid="gini-view">
    <BreakdownTable b={g} />
    <figure>
      <svg viewBox="0 0 {S} {S}" width="220" height="220" role="img" aria-label="Lorenz curve: the share of wealth held by the poorest share of people, against the line of perfect equality">
        <rect x="8" y="8" width={S - 16} height={S - 16} fill="var(--surface-2)" stroke="var(--border)" />
        <path d={paths.area} fill="var(--series-2)" opacity="0.25" />
        <path d={paths.equality} stroke="var(--text-dim)" stroke-dasharray="4 3" fill="none" />
        <path d={paths.curve} stroke="var(--series-1)" stroke-width="2" fill="none" data-testid="lorenz-curve" />
        <text x={S / 2} y={S - 0} text-anchor="middle" font-size="9" fill="var(--text-dim)">share of people, poorest to richest</text>
      </svg>
      <figcaption>Lorenz curve ({g.lorenz.length - 1} steps). The shaded gap to the dashed line of equality is what the Gini measures: a bigger gap means more inequality.</figcaption>
    </figure>
    <table>
      <caption>Wealth by household (net worth = savings + property - debt)</caption>
      <thead><tr><th scope="col">Household</th><th scope="col" class="n">People</th><th scope="col" class="n">Net worth</th><th scope="col" class="n">Per person</th></tr></thead>
      <tbody>{#each g.byHousehold as h (h.household)}<tr><th scope="row">Household {h.household}</th><td class="n">{h.people}</td><td class="n">{money(h.netWorth)}</td><td class="n">{money(h.people ? h.netWorth / h.people : 0)}</td></tr>{/each}</tbody>
    </table>
  </div>
{/if}

<style>
  .gv { display: flex; flex-direction: column; gap: 10px; }
  figure { margin: 0; display: flex; gap: 10px; align-items: center; } figcaption { font-size: 12px; color: var(--text-dim); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } caption { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-dim); }
  th[scope='row'] { text-align: left; font-weight: 400; } thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; }
  .n { text-align: right; font-family: var(--font-mono); } tr { border-top: 1px dotted var(--border); }
</style>
