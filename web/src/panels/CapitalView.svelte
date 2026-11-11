<script lang="ts">
  import type { Snapshot } from '../contract';
  import { money } from './format';
  import { capitalBacking } from './backing';
  import BreakdownTable from './BreakdownTable.svelte';
  let { snap }: { snap: Snapshot } = $props();
  const b = $derived(snap.finance.bank);
  const bd = $derived(snap.breakdowns?.capitalRatio ?? null);
</script>

<div class="cv" data-testid="capital-view">
  {#if bd}<BreakdownTable b={bd} />{/if}
  <ul>
    <li>Equity {money(b.equity)}, loans {money(b.loans)}, deposits {money(b.deposits)}.</li>
    <li>Limit {(b.capitalRatioLimit * 100).toFixed(1)}%: equity must stay above that share of loans. Headroom: the bank can lend {money(b.headroom)} more before it hits the limit.</li>
    <li>{capitalBacking(snap)}.</li>
  </ul>
</div>

<style>
  .cv { display: flex; flex-direction: column; gap: 8px; } ul { margin: 0; padding-left: 18px; font-size: 12px; }
</style>
