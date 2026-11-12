<script lang="ts">
  import type { Snapshot } from '../contract';
  import { text } from '../store/catalog';
  import { yieldIntro, yieldModels } from './yields';
  import HowLink from './HowLink.svelte';
  import Sparkline from './Sparkline.svelte';

  /** The sustainable-yield view (Ecology): per stock, what regrows against what was taken, the over-the-line flag and the months left. Wire words only. */
  let { snap }: { snap: Snapshot } = $props();
  const W = 270, H = 70;
  const models = $derived(yieldModels(snap, W, H));
</script>

{#if models.length > 0}
  <section class="yv" aria-labelledby="yv-h" data-testid="yield-panel">
    <h3 id="yv-h">{text('ecology', 'yield_title')}</h3>
    <p class="intro">{yieldIntro(snap)}</p>
    {#each models as m (m.id)}
      <article class="stock" class:over={m.over} data-testid="yield-{m.id}" data-over={m.over} data-reconciles={m.reconciles}>
        <header>
          <h4>{m.title}</h4>
          {#if m.tag}<span class="tag" title={m.tag.mechanism} data-testid="yield-tag">{m.tag.label}</span>{/if}
        </header>
        <p class="blurb">{m.blurb}</p>
        <p class="verdict" class:bad={m.over} data-testid="yield-verdict-{m.id}" role={m.over ? 'status' : undefined}>
          <b>{m.headline}.</b> {m.verdict}
        </p>
        {#if m.chart}
          <svg viewBox="0 0 {W} {H}" width="100%" role="img" aria-label="{text('ecology', 'yield_chart_label')}: {m.title}">
            <rect x="0" y="0" width={W} height={H} fill="var(--surface-2)" rx="4" />
            {#if m.chart.msy !== null}
              <line x1="2" x2={W - 2} y1={m.chart.msy} y2={m.chart.msy} stroke="var(--text-dim)" stroke-dasharray="4 3" stroke-width="1" />
              <text x={W - 4} y={Number(m.chart.msy) + 10} font-size="9" text-anchor="end" fill="var(--text-dim)">{text('ecology', 'yield_best_line')}</text>
            {/if}
            <polyline fill="none" stroke="var(--series-1)" stroke-width="2" points={m.chart.regrowth} />
            <polyline fill="none" stroke="var(--bad)" stroke-width="2" stroke-dasharray="5 2" points={m.chart.taken} />
          </svg>
          <div class="legend"><span class="s1"></span>{m.rows.find((r) => r.key === 'regrowth')?.label} <span class="s2"></span>{m.rows.find((r) => r.key === 'taken')?.label}</div>
        {/if}
        {#if m.stock.length > 1}
          <div class="spark"><span class="k">{text('ecology', 'yield_stock_label')}</span><Sparkline label={m.title} values={m.stock} tone={m.over ? 'bad' : 'neutral'} w={150} /></div>
        {/if}
        <table>
          <tbody>
            {#each m.rows as r (r.key)}
              <tr class:flag={r.flag} data-row={r.key}><th scope="row">{r.label}</th><td>{r.text}</td></tr>
            {/each}
          </tbody>
        </table>
        <HowLink anchor={m.how} />
      </article>
    {/each}
  </section>
{/if}

<style>
  .yv { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
  h3, h4 { margin: 0; } h4 { font-size: 13px; }
  .intro, .blurb { font-size: 12px; color: var(--text-dim); margin: 0; }
  .stock { border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 10px; display: flex; flex-direction: column; gap: 6px; background: var(--surface); }
  .stock.over { border-color: var(--bad); }
  header { display: flex; gap: 6px; align-items: baseline; flex-wrap: wrap; }
  .tag { font-size: 10.5px; border: 1px solid var(--border); border-radius: 9px; padding: 0 6px; color: var(--text-dim); }
  .verdict { margin: 0; font-size: 12.5px; } .verdict.bad { color: var(--bad); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  tr { border-top: 1px dotted var(--border); }
  th[scope='row'] { text-align: left; font-weight: 400; padding: 2px 4px 2px 0; }
  td { text-align: right; font-family: var(--font-mono); padding: 2px 0; }
  tr.flag th, tr.flag td { color: var(--bad); font-weight: 700; }
  .legend { font-size: 11px; color: var(--text-dim); }
  .legend span { display: inline-block; width: 14px; border-top: 2px solid var(--series-1); margin: 0 4px 3px 0; }
  .legend .s2 { border-color: var(--bad); border-top-style: dashed; margin-left: 8px; }
  .spark { display: flex; align-items: center; gap: 6px; } .k { font-size: 11px; color: var(--text-dim); text-transform: uppercase; }
</style>
