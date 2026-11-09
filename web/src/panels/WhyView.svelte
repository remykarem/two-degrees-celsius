<script lang="ts">
  import type { LeverInfo, Snapshot, TraceRecord } from '../contract';
  import BarChart from './BarChart.svelte';
  import type { WhyKind } from './metrics';
  import { whyData } from './why';

  let { kind, trace, snap, levers, compact = false }: { kind: WhyKind; trace: TraceRecord[]; snap: Snapshot | null; levers: LeverInfo[]; compact?: boolean } = $props();
  const data = $derived(whyData(kind, trace, snap, levers));
</script>

<div class="why" class:compact>
  <h3>{data.title}</h3>
  <p class="blurb">{data.blurb}</p>
  {#if data.empty}
    <p class="empty">{data.empty}</p>
  {:else}
    {#each data.groups.filter((g) => g.bars.length) as g (g.heading)}
      <section><h4>{g.heading}</h4><BarChart bars={g.bars} pairLabel={g.pairLabel} /></section>
    {/each}
  {/if}
</div>

<style>
  .blurb { color: var(--text-dim); font-size: 13px; margin: 4px 0 8px; }
  .empty { padding: 8px; background: var(--surface-2); border-radius: var(--radius); }
  section { margin-top: 10px; }
  h4 { margin: 0 0 4px; font-size: 12px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.04em; }
  .compact h3 { font-size: 12px; }
</style>
