<script lang="ts">
  import type { Snapshot } from '../contract';
  import HowButton from './HowButton.svelte';
  import { townModel } from './networth';

  let { snap }: { snap: Snapshot } = $props();
  const m = $derived(townModel(snap));
</script>

{#if m}
  <section class="wh" aria-labelledby="wh-h" data-testid="who-holds-what">
    <h3 id="wh-h">{m.title}{#if m.holders[0]?.how} <HowButton anchor={m.holders[0].how} label={m.title} />{/if}</h3>
    <p class="dim">{m.intro}</p>
    <table>
      <thead><tr><th scope="col">Holder</th><th scope="col" class="n">Assets</th><th scope="col" class="n">Liabilities</th><th scope="col" class="n">Net worth</th></tr></thead>
      <tbody>{#each m.holders as h (h.id)}
        <tr class:memo={h.memo} data-testid="holder-{h.id}">
          <th scope="row">{h.label}{#if h.counts}<span class="sub">{h.counts}</span>{/if}</th>
          <td class="n">{h.assets}</td><td class="n">{h.liabilities}</td><td class="n">{h.netWorth}</td></tr>{/each}</tbody>
      <tfoot><tr><th scope="row" colspan="3">Total of the households' groups</th><td class="n" data-testid="holders-total">{m.total}</td></tr></tfoot>
    </table>
    <p class="rc" class:bad={!m.reconciles} role="status" data-testid="holders-reconciles" data-reconciles={m.reconciles}>{m.reconciles ? 'Reconciles. ' : 'Does not reconcile. '}{m.reconcileText}</p>
  </section>
{/if}

<style>
  .wh { margin: 8px 0; } .dim { color: var(--text-dim); font-size: 12px; margin: 2px 0 6px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; }
  th, td { padding: 2px 4px; } thead th { white-space: nowrap; }
  th[scope='row'] { text-align: left; font-weight: 400; } tr { border-top: 1px dotted var(--border); } .n { text-align: right; font-family: var(--font-mono); }
  .sub { display: block; font-size: 10.5px; color: var(--text-dim); } tr.memo { color: var(--text-dim); font-style: italic; } tfoot th { font-weight: 700; } tfoot td { font-weight: 700; }
  .rc { margin: 4px 0 0; font-size: 12px; color: var(--text-dim); } .rc.bad { color: var(--bad); font-weight: 700; }
</style>
