<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { ARROW, money, pct, TREND_WORD } from './format';
  import { historyChange, METRIC_BREAKDOWN, METRIC_LEDGER, METRIC_TABS, metricById, metricsFor, runway, runwayText, seriesOf, type MetricTab } from './metrics';
  import { forecastView } from '../store';
  import ClimateGauge from './ClimateGauge.svelte';
  import EnergyPanel from './EnergyPanel.svelte';
  import FinancePanel from './FinancePanel.svelte';
  import ForecastCard from './ForecastCard.svelte';
  import HealthMeters from './HealthMeters.svelte';
  import PeoplePanel from './PeoplePanel.svelte';
  import RunwayChart from './RunwayChart.svelte';
  import Sparkline from './Sparkline.svelte';
  import { setUi } from './ui';
  import EmissionsPanel from './EmissionsPanel.svelte';
  import HowButton from './HowButton.svelte';
  import IndustryPanel from './IndustryPanel.svelte';
  import PoliciesPanel from './PoliciesPanel.svelte';
  import TaxesTable from './TaxesTable.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let tab = $state<MetricTab>('Economy');

  const snap = $derived($gs.snapshot);
  const history = $derived(snap?.history ?? []);
  // Forest and commons stocks are the same numbers as the health bars above (Ecology tab), so they are not listed twice.
  const defs = $derived(metricsFor(tab).filter((m) => !(tab === 'Economy' && (m.id === 'gdp' || m.id === 'gpi')) && !(tab === 'Ecology' && (m.id === 'forest' || m.id === 'commons'))));
  const rw = $derived(snap ? runway(snap) : null);
  const turnLabel = (t: number) => (snap ? quarterLabel(t, snap.game) : String(t));
  const fc = $derived(forecastView(snap));
  const gdp = $derived(metricById('gdp')!);
  const gpi = $derived(metricById('gpi')!);

  function onTabKey(e: KeyboardEvent) {
    const from = ((e.target as HTMLElement).id || '').replace('tab-', '') as MetricTab;
    const i = METRIC_TABS.includes(from) ? METRIC_TABS.indexOf(from) : METRIC_TABS.indexOf(tab);
    const n = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : e.key === 'Home' ? 0 : e.key === 'End' ? METRIC_TABS.length - 1 : -1;
    if (n < 0) return;
    e.preventDefault();
    tab = METRIC_TABS[(n + METRIC_TABS.length) % METRIC_TABS.length]!;
    queueMicrotask(() => document.getElementById(`tab-${tab}`)?.focus());
  }
</script>

<aside aria-label="Metrics">
  <div role="tablist" aria-label="Metric groups" tabindex="-1" onkeydown={onTabKey}>
    {#each METRIC_TABS as t (t)}
      <button role="tab" id="tab-{t}" aria-selected={tab === t} aria-controls="metrics-body" tabindex={tab === t ? 0 : -1} class:on={tab === t} onclick={() => (tab = t)}>{t}</button>
    {/each}
  </div>

  {#if snap}
    {#if fc && !fc.calm && tab !== 'Ecology'}<ForecastCard {game} />{/if}
    <div id="metrics-body" role="tabpanel" aria-labelledby="tab-{tab}">
      {#if tab === 'Economy'}
        <section class="pair" aria-label="GDP beside GPI">
          <div><div class="k">GDP <HowButton breakdown="gdp" anchor="gdp" label="GDP" /></div><div class="big">{money(snap.indicators.gdpRealAnnual)}</div></div>
          <div><div class="k">GPI <HowButton breakdown="gpi" anchor="gpi" label="GPI" /></div><div class="big">{money(snap.indicators.gpiAnnual)}</div></div>
          <div class="ratio">GPI is {money(snap.indicators.gpiAnnual)} of the {money(snap.indicators.gdpRealAnnual)} GDP ({pct(snap.indicators.gdpRealAnnual > 0 ? snap.indicators.gpiAnnual / snap.indicators.gdpRealAnnual : 0)}).
            GPI is output adjusted for social and environmental costs.</div>
          {#if history.length > 1}
            <svg viewBox="0 0 270 44" width="100%" height="44" role="img" aria-label="GDP and GPI trend on one scale">
              {#each [gdp, gpi] as d, k (d.id)}
                {@const vals = history.map((h) => d.series!(h))}
                {@const lo = Math.min(...history.map((h) => Math.min(h.gdpRealAnnual, h.gpiAnnual)))}
                {@const hi = Math.max(...history.map((h) => Math.max(h.gdpRealAnnual, h.gpiAnnual)))}
                <polyline fill="none" stroke={k === 0 ? 'var(--series-1)' : 'var(--series-2)'} stroke-width="1.6" stroke-dasharray={k === 0 ? undefined : '4 2'}
                  points={vals.map((v, i) => `${(2 + (i / (vals.length - 1)) * 266).toFixed(1)},${(42 - ((v - lo) / (hi - lo || 1)) * 40).toFixed(1)}`).join(' ')} />
              {/each}
            </svg>
            <div class="legend"><span class="s1"></span>GDP <span class="s2"></span>GPI (dashed)</div>
          {/if}
          <button class="why" onclick={() => setUi({ why: { kind: 'income', metric: 'gdp' } })}>Why is this changing?</button>
        </section>
      {/if}

      {#if tab === 'Ecology'}
        <ForecastCard {game} />
        <ClimateGauge {game} />
        <section class="health" aria-label="Health of the land"><h3>Health of the land</h3><HealthMeters {game} /></section>
        <EnergyPanel {game} />
        <EmissionsPanel {snap} />
      {/if}

      {#if tab === 'Finance'}
        <FinancePanel {game} />
        <TaxesTable {snap} />
      {/if}
      {#if tab === 'Policies'}<PoliciesPanel {game} />{/if}
      {#if tab === 'Industry'}<IndustryPanel {game} />{/if}

      {#if tab === 'People'}
        <PeoplePanel {game} />
      {/if}
      <ul>
        {#each defs as m (m.id)}
          {@const ch = historyChange(m, history)}
          <li>
            <div class="head">
              <span class="label" title={m.hint}>{m.label}</span>
              {#if METRIC_LEDGER[m.id]}<button class="why" onclick={() => { const l = METRIC_LEDGER[m.id]!; game.select({ kind: 'government' }); game.openLedger(l.account, l.kind); }}>see transactions</button>{/if}
              <HowButton breakdown={METRIC_BREAKDOWN[m.id]} anchor={m.metric} label={m.label} />
              {#if m.why}<button class="why" aria-label="Why is {m.label} changing?" onclick={() => setUi({ why: { kind: m.why!, metric: m.metric } })}>why?</button>{/if}
            </div>
            <div class="line">
              <span class="num">{m.fmt(m.value(snap))}</span>
              {#if ch}<span class="chg {ch.tone}" title="Change since last turn: {TREND_WORD[ch.trend]}"><span aria-hidden="true">{ARROW[ch.trend]}</span><span class="sr-only">{TREND_WORD[ch.trend]} since last turn</span></span>{:else}<span class="chg"></span>{/if}
              <Sparkline label={m.label} values={seriesOf(m, history)} tone={ch?.tone ?? 'neutral'} w={110} />
            </div>
            {#if m.detail}<div class="detail">{m.detail(snap)}</div>{/if}
          </li>
        {/each}
      </ul>

      {#if tab === 'Ecology'}
        <section class="runway" aria-label="Carbon budget runway">
          <h3>Carbon budget runway</h3>
          {#if rw}
            <RunwayChart runway={rw} label={turnLabel} width={270} />
            <p class="rtext">{runwayText(rw, turnLabel)}</p>
          {:else}<p class="rtext">The carbon budget is spent.</p>{/if}
        </section>
      {/if}
      {#if !history.length}<p class="dim">Trend lines appear after the first turn.</p>{/if}
    </div>
  {/if}
</aside>

<style>
  aside { padding: 10px; background: var(--surface); border-left: 1px solid var(--border); overflow: auto; min-width: 0; }
  [role='tablist'] { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 10px; }
  [role='tab'] { flex: 1 0 22%; padding: 5px 1px; font-size: 12px; border: 1px solid var(--border); background: var(--surface); color: var(--text); border-radius: var(--radius); cursor: pointer; }
  [role='tab'].on { background: var(--accent); color: var(--accent-text); border-color: var(--accent); font-weight: 600; }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 2px 8px; padding: 8px; background: var(--surface-2); border-radius: var(--radius); margin-bottom: 10px; }
  .k { font-size: 11px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.04em; }
  .big { font-size: 20px; font-weight: 700; font-family: var(--font-mono); }
  .ratio { grid-column: 1 / -1; font-size: 12px; color: var(--text-dim); }
  .pair svg { grid-column: 1 / -1; }
  .legend { grid-column: 1 / -1; font-size: 11px; color: var(--text-dim); }
  .legend span { display: inline-block; width: 14px; border-top: 2px solid var(--series-1); margin: 0 4px 3px 0; }
  .legend .s2 { border-color: var(--series-2); border-top-style: dashed; margin-left: 8px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 10px; }
  .head { display: flex; justify-content: space-between; align-items: baseline; gap: 4px; flex-wrap: wrap; }
  .label { font-size: 13px; }
  .line { display: grid; grid-template-columns: 78px 18px 1fr; align-items: center; gap: 4px; }
  .num { font-family: var(--font-mono); font-size: 15px; font-weight: 600; }
  .detail { font-size: 12px; color: var(--text-dim); }
  .chg { text-align: center; font-size: 12px; color: var(--text-dim); }
  .chg.good { color: var(--good); } .chg.bad { color: var(--bad); }
  .why { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0 2px; font-size: 12px; }
  .pair .why { grid-column: 1 / -1; justify-self: start; }
  .health { margin-bottom: 12px; }
  .health h3 { margin-bottom: 4px; }
  .runway { margin-top: 12px; }
  .rtext { font-size: 12px; margin: 4px 0 0; }
  .dim { color: var(--text-dim); font-size: 12px; }
</style>
