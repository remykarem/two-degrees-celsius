<script lang="ts">
  import { compact } from './format';
  import type { Runway } from './metrics';
  let { runway, label, width = 280, height = 120 }: { runway: Runway; label: (turn: number) => string; width?: number; height?: number } = $props();
  const pad = { l: 34, r: 8, t: 8, b: 20 };
  const x0 = $derived(runway.actual[0]?.turn ?? runway.projection[0]!.turn);
  const x1 = $derived(runway.lastTurn + 1);
  const sx = $derived((t: number) => pad.l + ((t - x0) / Math.max(1, x1 - x0)) * (width - pad.l - pad.r));
  const sy = $derived((v: number) => pad.t + (1 - v / Math.max(1, runway.total)) * (height - pad.t - pad.b));
  const path = (pts: { turn: number; remaining: number }[]) => pts.map((p) => `${sx(p.turn).toFixed(1)},${sy(p.remaining).toFixed(1)}`).join(' ');
  const runOut = $derived(runway.runOutTurn !== null && runway.runOutTurn <= x1 ? runway.runOutTurn : null);
  const summary = $derived(runway.runOutTurn === null ? 'budget not used up at the current rate' : `budget runs out around turn ${Math.ceil(runway.runOutTurn)}`);
</script>

<svg {width} {height} viewBox="0 0 {width} {height}" role="img" aria-label="Carbon budget runway: {compact(runway.remaining)} tonnes left, {summary}">
  <line x1={pad.l} x2={width - pad.r} y1={sy(0)} y2={sy(0)} stroke="var(--border)" />
  <line x1={pad.l} x2={pad.l} y1={pad.t} y2={sy(0)} stroke="var(--border)" />
  <text x={pad.l - 4} y={pad.t + 4} text-anchor="end">{compact(runway.total)}</text>
  <text x={pad.l - 4} y={sy(0) + 3} text-anchor="end">0</text>
  <text x={pad.l} y={height - 5}>{label(x0)}</text>
  <text x={width - pad.r} y={height - 5} text-anchor="end">{label(runway.lastTurn)}</text>
  {#if runway.actual.length > 1}<polyline points={path(runway.actual)} fill="none" stroke="var(--series-1)" stroke-width="2" />{/if}
  <polyline points={path(runway.projection)} fill="none" stroke="var(--series-2)" stroke-width="2" stroke-dasharray="5 3" />
  {#if runOut !== null}
    <line x1={sx(runOut)} x2={sx(runOut)} y1={pad.t} y2={sy(0)} stroke="var(--bad)" stroke-dasharray="2 2" />
    <circle cx={sx(runOut)} cy={sy(0)} r="3.5" fill="var(--bad)" />
  {/if}
</svg>
<p class="legend"><span class="k a"></span>Budget left <span class="k b"></span>At current rate{#if runOut !== null} <span class="k c"></span>Runs out{/if}</p>

<style>
  text { font-size: 10px; fill: var(--text-dim); }
  .legend { font-size: 11px; color: var(--text-dim); margin: 2px 0 0; }
  .k { display: inline-block; width: 14px; height: 0; border-top: 2px solid; margin: 0 4px 3px 8px; }
  .k:first-child { margin-left: 0; }
  .a { border-color: var(--series-1); } .b { border-color: var(--series-2); border-top-style: dashed; } .c { border-color: var(--bad); border-top-style: dotted; }
</style>
