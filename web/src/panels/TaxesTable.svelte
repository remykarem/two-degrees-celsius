<script lang="ts">
  import type { Snapshot } from '../contract';
  import { taxLines, GST_REASON } from './taxes';
  import HowLink from './HowLink.svelte';
  let { snap }: { snap: Snapshot } = $props();
  const rows = $derived(taxLines(snap));
</script>

{#if rows.length}
  <section class="tx" data-testid="taxes-table" aria-label="Taxes">
    <table>
      <thead><tr><th scope="col">Tax</th><th scope="col" class="n">Rate</th><th scope="col" class="n">Month</th><th scope="col" class="n">Quarter</th></tr></thead>
      <tbody>
        {#each rows as r (r.id)}
          <tr class="top" class:off={!r.modelled}><th scope="row">{r.label}</th>
            {#if r.modelled}<td class="n">{r.nominal}{#if r.effective !== 'same as set' && r.effective !== r.nominal}<span class="ap">in force {r.effective}</span>{/if}</td><td class="n">{r.month}</td><td class="n">{r.quarter}</td>
            {:else}<td class="n" colspan="3">not modelled: no revenue</td>{/if}</tr>
          <tr class="sub" class:off={!r.modelled}><td colspan="4" class="ap">{r.appliesTo}</td></tr>
        {/each}
      </tbody>
    </table>
    {#if rows.some((r) => !r.modelled)}<p class="dim" data-testid="gst-note">GST/VAT: not modelled. {GST_REASON}</p>{/if}
    <HowLink anchor="taxes" />
  </section>
{/if}

<style>
  .tx { display: flex; flex-direction: column; gap: 4px; } table { width: 100%; border-collapse: collapse; font-size: 12px; } thead th:first-child { width: 30%; }
  thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; } th.n { text-align: right; }
  th[scope='row'] { text-align: left; font-weight: 600; vertical-align: top; } .ap { display: block; font-weight: 400; font-size: 10.5px; color: var(--text-dim); font-family: inherit; white-space: normal; text-align: left; } td.n .ap { text-align: right; }
  .n { text-align: right; font-family: var(--font-mono); white-space: nowrap; vertical-align: top; } tr.top { border-top: 1px dotted var(--border); } tr.sub td { padding: 0 0 3px; }
  tr.off { color: var(--text-dim); font-style: italic; } .dim { color: var(--text-dim); font-size: 11px; margin: 0; }
</style>
