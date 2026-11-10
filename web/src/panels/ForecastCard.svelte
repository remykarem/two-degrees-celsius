<script lang="ts">
  import type { RiskLevel } from '../contract';
  import { forecastView, type Game } from '../store';
  import { adapt } from './weatherActions';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const f = $derived(forecastView($gs.snapshot));
  const pips = (r: RiskLevel) => (r === 'high' ? 3 : r === 'medium' ? 2 : 1);
  const canAdapt = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !$gs.snapshot?.gameOver);
</script>

{#if f}
  <section class="card {f.level}" class:calm={f.calm} aria-label="Weather forecast" data-testid="forecast-card">
    <div class="head"><h3>Forecast</h3><span class="when">next quarter</span></div>
    <p class="headline">{f.headline}</p>
    <dl>
      <dt>Fire risk</dt>
      <dd><span class="pips" aria-hidden="true">{#each [1, 2, 3] as i (i)}<i class:on={i <= pips(f.fireRisk)}></i>{/each}</span><b>{f.fireWord}</b></dd>
      <dt>Drought risk</dt>
      <dd><span class="pips" aria-hidden="true">{#each [1, 2, 3] as i (i)}<i class:on={i <= pips(f.droughtRisk)}></i>{/each}</span><b>{f.droughtWord}</b></dd>
    </dl>
    <p class="detail">{f.detail}</p>
    {#if f.adapt}
      <button class="adapt" disabled={!canAdapt} title={canAdapt ? 'Open the resilience lever and pick the fire station to build' : 'Available during Review and Plan'} onclick={() => adapt(game)}>Adapt…</button>
    {/if}
  </section>
{/if}

<style>
  .card { border: 1px solid var(--border); border-left: 5px solid var(--muted-bar); border-radius: var(--radius); background: var(--surface-2); padding: 8px 10px; margin-bottom: 10px; }
  .card.medium { border-left-color: var(--warn); background: var(--warn-bg); }
  .card.high { border-left-color: var(--bad); background: var(--bad-bg); }
  .card.calm { border-left-color: var(--good); background: var(--surface-2); }
  .head { display: flex; justify-content: space-between; align-items: baseline; }
  .when { font-size: 11px; color: var(--text-dim); }
  .headline { margin: 2px 0 4px; font-weight: 700; font-size: 14px; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 2px 10px; margin: 0; font-size: 12px; }
  dt { color: var(--text-dim); }
  dd { margin: 0; display: flex; align-items: center; gap: 6px; }
  .pips { display: inline-flex; gap: 2px; }
  .pips i { width: 12px; height: 8px; border: 1px solid var(--text-dim); border-radius: 2px; display: inline-block; }
  .pips i.on { background: var(--text); }
  .detail { margin: 4px 0 0; font-size: 12px; color: var(--text-dim); }
  .adapt { margin-top: 6px; padding: 3px 12px; border: 1px solid var(--accent); background: var(--surface); color: var(--accent); border-radius: var(--radius); cursor: pointer; font-weight: 600; }
  .adapt:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
