<script lang="ts">
  let { values, label, w = 96, h = 24, tone = 'neutral' }: { values: number[]; label: string; w?: number; h?: number; tone?: 'good' | 'bad' | 'neutral' } = $props();
  const geo = $derived.by(() => {
    if (values.length < 2) return null;
    const lo = Math.min(...values), hi = Math.max(...values), span = hi - lo || 1;
    const pts = values.map((v, i) => [(i / (values.length - 1)) * (w - 4) + 2, h - 3 - ((v - lo) / span) * (h - 6)] as const);
    return { line: pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' '), last: pts[pts.length - 1]! };
  });
</script>

<svg width={w} height={h} viewBox="0 0 {w} {h}" role="img" aria-label="{label} trend, {values.length} points">
  {#if geo}
    <polyline points={geo.line} fill="none" stroke="var(--series-1)" stroke-width="1.5" stroke-linejoin="round" />
    <circle cx={geo.last[0]} cy={geo.last[1]} r="2.5" class={tone} />
  {:else}<line x1="2" x2={w - 2} y1={h / 2} y2={h / 2} stroke="var(--border)" stroke-dasharray="2 2" />{/if}
</svg>

<style>
  circle { fill: var(--text-dim); }
  circle.good { fill: var(--good); } circle.bad { fill: var(--bad); }
</style>
