<script lang="ts">
  import type { Snapshot } from '../contract';
  import { cumulativeSeries, emissionsView } from './emissions';
  import { tonnes } from './format';
  import HowLink from './HowLink.svelte';

  let { snap, compact = false }: { snap: Snapshot; compact?: boolean } = $props();
  const e = $derived(snap.breakdowns?.emissions ?? null);
  const v = $derived(e ? emissionsView(e) : null);
  const series = $derived(e ? cumulativeSeries(snap.history, e.cumulative.budget) : []);
  const W = 270, H = 90;
  const maxT = $derived(e ? Math.max(e.cumulative.budget, e.cumulative.tonnes) : 1);
  const last = $derived(snap.game.lastTurn || 1);
  const x = (turn: number) => 4 + (turn / last) * (W - 8);
  const y = (t: number) => H - 6 - (t / maxT) * (H - 14);
  const pts = $derived([{ turn: snap.history[0] ? snap.history[0].turn - 1 : 0, tonnes: 0 }, ...series, ...(e ? [{ turn: snap.turn, tonnes: e.cumulative.tonnes }] : [])]);
</script>

{#if v && e}
  <section class="em" aria-labelledby="em-h" data-testid="emissions-panel">
    <h3 id="em-h">Emissions: month, turn and accumulated</h3>
    <table>
      <thead><tr><th scope="col"><span class="sr-only">Source</span></th><th scope="col" class="n">Last month</th><th scope="col" class="n">This turn (3 months)</th></tr></thead>
      <tbody>
        {#each v.rows as r (r.id)}<tr class={r.kind}><th scope="row">{r.label}</th><td class="n">{r.month}</td><td class="n">{r.turn}</td></tr>{/each}
        <tr class="cum"><th scope="row">Accumulated (whole game)</th><td class="n" colspan="2" data-testid="cumulative">{v.cumulative.used} of {v.cumulative.budget} budget, {v.cumulative.remaining} left</td></tr>
      </tbody>
    </table>
    <p class="recon" class:bad={!v.reconciliation.ok} data-testid="reconciliation" role={v.reconciliation.ok ? undefined : 'alert'}>
      <b>{v.reconciliation.ok ? 'Reconciles' : 'DOES NOT RECONCILE'}.</b> {v.reconciliation.text} {v.cumulative.shareText}.
      {#if v.reconciliation.wireFlag !== v.reconciliation.ok}<span> The simulation's own flag says {v.reconciliation.wireFlag ? 'it does' : 'it does not'}.</span>{/if}
    </p>
    {#if !compact}
      <svg viewBox="0 0 {W} {H}" width="100%" role="img" aria-label="Accumulated emissions by turn against the {tonnes(e.cumulative.budget)} budget line">
        <rect x="0" y="0" width={W} height={H} fill="var(--surface-2)" rx="4" />
        <line x1="4" x2={W - 4} y1={y(e.cumulative.budget)} y2={y(e.cumulative.budget)} stroke="var(--bad)" stroke-dasharray="5 3" stroke-width="1.5" />
        <text x="6" y={y(e.cumulative.budget) + 11} font-size="10" fill="var(--bad)">budget {tonnes(e.cumulative.budget)}</text>
        <polyline fill="none" stroke="var(--series-1)" stroke-width="2" points={pts.map((p) => `${x(p.turn).toFixed(1)},${y(p.tonnes).toFixed(1)}`).join(' ')} />
        <text x={W - 6} y={H - 4} font-size="10" text-anchor="end" fill="var(--text-dim)">accumulated {tonnes(e.cumulative.tonnes)}</text>
      </svg>
    {/if}
    {#if v.reconciliation.note}<p class="dim">{v.reconciliation.note}</p>{/if}
    <HowLink anchor="emissions" />
  </section>
{:else}
  <p class="dim">The simulation did not send an emissions breakdown (an older package); the plain monthly figure is below.</p>
{/if}

<style>
  .em { display: flex; flex-direction: column; gap: 6px; margin-bottom: 10px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  thead th { font-size: 10px; text-transform: uppercase; color: var(--text-dim); }
  th[scope='row'] { text-align: left; font-weight: 400; }
  .n { text-align: right; font-family: var(--font-mono); white-space: nowrap; }
  tr { border-top: 1px dotted var(--border); }
  tr.total th, tr.total td, tr.net th, tr.net td { font-weight: 700; }
  tr.sink td { color: var(--good); }
  tr.cum td { font-size: 11px; white-space: normal; }
  .recon { margin: 0; font-size: 12px; color: var(--good); } .recon.bad { color: var(--bad); }
  .dim { color: var(--text-dim); font-size: 11px; margin: 0; }
</style>
