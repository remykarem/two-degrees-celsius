<script lang="ts">
  import type { Snapshot } from '../contract';
  import { lorenzPath } from './lorenz';
  import BreakdownTable from './BreakdownTable.svelte';
  import { giniRows, topHouseholdLine } from './estate';
  import NetWorthStatement from './NetWorthStatement.svelte';
  import WhoHoldsWhat from './WhoHoldsWhat.svelte';
  import { statementModel } from './networth';

  let { snap }: { snap: Snapshot } = $props();
  const g = $derived(snap.breakdowns?.gini ?? null);
  const S = 200;
  const paths = $derived(g ? lorenzPath(g.lorenz, S, S, 8) : null);
  const rows = $derived(giniRows(snap));
  const topLine = $derived(topHouseholdLine(snap));
  const last = $derived(g && g.lorenz.length > 1 ? g.lorenz[g.lorenz.length - 2]! : null);
</script>

{#if g && paths}
  <div class="gv" data-testid="gini-view">
    {#if topLine}<p class="top" data-testid="gini-top-line">{topLine}</p>{/if}
    <BreakdownTable b={g} />
    <figure>
      <svg viewBox="0 0 {S} {S}" width="220" height="220" role="img" aria-label="Lorenz curve: the share of wealth held by the poorest share of people, against the line of perfect equality">
        <rect x="8" y="8" width={S - 16} height={S - 16} fill="var(--surface-2)" stroke="var(--border)" />
        <path d={paths.area} fill="var(--series-2)" opacity="0.25" />
        <path d={paths.equality} stroke="var(--text-dim)" stroke-dasharray="4 3" fill="none" />
        <path d={paths.curve} stroke="var(--series-1)" stroke-width="2" fill="none" data-testid="lorenz-curve" />
        {#if last && rows[0]?.tag}<text x={S - 12} y={8 + (1 - last.wealthShare) * (S - 16) + 14} text-anchor="end" font-size="9" font-weight="700" fill="var(--text)" data-testid="lorenz-label">{rows[0].tag}</text>{/if}
        <text x={S / 2} y={S - 0} text-anchor="middle" font-size="9" fill="var(--text-dim)">share of people, poorest to richest</text>
      </svg>
      <figcaption>Lorenz curve ({g.lorenz.length - 1} steps). The shaded gap to the dashed line of equality is what the Gini measures: a bigger gap means more inequality.</figcaption>
    </figure>
    <table data-testid="gini-households">
      <caption>Wealth by household, richest first (net worth = savings + property - debt)</caption>
      <thead><tr><th scope="col">Household</th><th scope="col" class="n">People</th><th scope="col" class="n">Net worth</th><th scope="col" class="n">Per person</th><th scope="col" class="n">Share of the town's wealth</th></tr></thead>
      <tbody>{#each rows as h (h.household)}
        <tr class:estate={!!h.tag}><th scope="row">{h.label}{#if h.tag && h.tag !== h.label} <span class="chip">{h.tag}</span>{/if}{#if h.band} <span class="band">{h.band}</span>{/if}</th>
          <td class="n">{h.people}</td><td class="n">{h.netWorth}</td><td class="n">{h.perPerson}</td><td class="n">{h.shareText}</td></tr>
        {@const st = statementModel(snap, h.household)}
        {#if st}<tr class="st"><td colspan="5"><details data-testid="gini-statement-{h.household}"><summary>Net worth statement: {st.groups[0]!.rows.length} assets, {st.groups[1]!.rows.length} liabilities, {st.reconciles ? 'reconciles' : 'does not reconcile'}</summary><NetWorthStatement model={st} compact /></details></td></tr>{/if}{/each}</tbody>
    </table>
    <WhoHoldsWhat {snap} />
  </div>
{/if}

<style>
  .gv { display: flex; flex-direction: column; gap: 10px; }
  figure { margin: 0; display: flex; gap: 10px; align-items: center; } figure svg { flex: 0 0 220px; } figcaption { font-size: 12px; color: var(--text-dim); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } caption { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-dim); }
  th[scope='row'] { text-align: left; font-weight: 400; } thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; }
  .top { margin: 0; font-weight: 600; font-size: 13px; }
  .chip { font-size: 10px; font-weight: 700; background: var(--series-2); color: var(--accent-text); border-radius: 8px; padding: 0 6px; }
  .band { font-size: 10px; color: var(--text-dim); border: 1px solid var(--border); border-radius: 8px; padding: 0 5px; }
  tr.estate { background: var(--surface-2); } tr.estate th { font-weight: 700; }
  tr.st { border-top: 0; } summary { font-size: 11px; color: var(--text-dim); cursor: pointer; }
  .n { text-align: right; font-family: var(--font-mono); } tr { border-top: 1px dotted var(--border); }
</style>
