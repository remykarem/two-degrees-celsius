<script lang="ts">
  import type { HistoryPoint } from '../contract';
  import type { Game } from '../store';
  import { money, pct } from './format';
  import Sparkline from './Sparkline.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  type Tab = 'Economy' | 'Ecology' | 'Society';
  let tab = $state<Tab>('Economy');
  const tabs: Tab[] = ['Economy', 'Ecology', 'Society'];

  interface Row { label: string; series: ((h: HistoryPoint) => number) | null; now: string }
  const rows = $derived.by((): Row[] => {
    const s = $gs.snapshot;
    if (!s) return [];
    const i = s.indicators;
    const all: Record<Tab, Row[]> = {
      Economy: [
        { label: 'GDP (real, yearly)', series: (h) => h.gdpRealAnnual, now: money(i.gdpRealAnnual) },
        { label: 'GPI (yearly)', series: (h) => h.gpiAnnual, now: money(i.gpiAnnual) },
        { label: 'Unemployment', series: (h) => h.unemployment, now: pct(i.unemployment, 1) },
        { label: 'Public debt / GDP', series: (h) => h.debtToGdp, now: pct(s.government.debtToGdp, 1) },
      ],
      Ecology: [
        { label: 'Emissions (per quarter, t)', series: (h) => h.emissions, now: s.nature.grossEmissionsMonthly.toFixed(0) + '/mo' },
        { label: 'Carbon budget left', series: (h) => h.budgetShare, now: pct(i.budgetShare) },
        { label: 'Temperature (C above base)', series: null, now: i.temperature.toFixed(2) },
        { label: 'Forest', series: null, now: pct(s.nature.forest) },
      ],
      Society: [
        { label: 'Inequality (Gini)', series: (h) => h.gini, now: i.gini.toFixed(2) },
        { label: 'Wellbeing', series: (h) => h.avgWellbeing, now: i.avgWellbeing.toFixed(2) },
        { label: 'Legitimacy', series: (h) => h.legitimacy, now: pct(s.government.legitimacy) },
      ],
    };
    return all[tab];
  });
</script>

<aside aria-label="Metrics">
  <div role="tablist" aria-label="Metric groups">
    {#each tabs as t (t)}
      <button role="tab" aria-selected={tab === t} class:on={tab === t} onclick={() => (tab = t)}>{t}</button>
    {/each}
  </div>
  <ul>
    {#each rows as r (r.label)}
      <li>
        <span class="label">{r.label}</span>
        <Sparkline label={r.label} values={r.series ? ($gs.snapshot?.history ?? []).map(r.series) : []} />
        <span class="num">{r.now}</span>
      </li>
    {/each}
  </ul>
  {#if !($gs.snapshot?.history.length)}<p class="dim">Trend lines appear after the first turn.</p>{/if}
</aside>

<style>
  aside { padding: 10px; background: var(--surface); border-left: 1px solid var(--border); overflow: auto; }
  [role='tablist'] { display: flex; gap: 4px; margin-bottom: 8px; }
  button { font: inherit; padding: 4px 10px; border: 1px solid var(--border); background: var(--surface); color: var(--text); border-radius: var(--radius); cursor: pointer; }
  button.on { background: var(--accent); color: var(--accent-text); border-color: var(--accent); font-weight: 600; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  li { display: grid; grid-template-columns: 1fr auto auto; gap: 8px; align-items: center; }
  .label { font-size: 13px; }
  .num { font-family: var(--font-mono); min-width: 64px; text-align: right; }
  .dim { color: var(--text-dim); font-size: 12px; }
</style>
