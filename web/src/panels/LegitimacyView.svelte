<script lang="ts">
  import type { Snapshot } from '../contract';
  import BreakdownTable from './BreakdownTable.svelte';
  let { snap }: { snap: Snapshot } = $props();
  const l = $derived(snap.breakdowns?.legitimacy ?? null);
  const sg = (v: number) => `${v >= 0 ? '+' : ''}${v.toFixed(4)}`;
</script>

{#if l}
  <div class="lv" data-testid="legitimacy-view">
    <BreakdownTable b={l} />
    <table>
      <caption>Terms: contribution = weight x (input - threshold)</caption>
      <thead><tr><th scope="col">Term</th><th scope="col" class="n">Input</th><th scope="col" class="n">Threshold</th><th scope="col" class="n">Weight</th><th scope="col" class="n">Contribution</th></tr></thead>
      <tbody>
        {#each l.terms as t (t.id)}<tr><th scope="row">{t.label}</th><td class="n">{t.input.toFixed(3)}</td><td class="n">{t.threshold === undefined ? 'none' : t.threshold.toFixed(3)}</td><td class="n">{t.weight}</td><td class="n" class:good={t.contribution > 0} class:bad={t.contribution < 0}>{sg(t.contribution)}</td></tr>{/each}
      </tbody>
      <tfoot><tr><th scope="row" colspan="4">Trend per month (sum of the terms)</th><td class="n">{sg(l.trend)}</td></tr></tfoot>
    </table>
  </div>
{/if}

<style>
  .lv { display: flex; flex-direction: column; gap: 10px; } table { width: 100%; border-collapse: collapse; font-size: 12px; }
  caption { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-dim); } thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; }
  th[scope='row'] { text-align: left; font-weight: 400; } .n { text-align: right; font-family: var(--font-mono); } tr { border-top: 1px dotted var(--border); }
  .good { color: var(--good); } .bad { color: var(--bad); } tfoot th, tfoot td { font-weight: 700; }
</style>
