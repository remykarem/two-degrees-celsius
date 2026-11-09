<script lang="ts">
  let { values, label, w = 96, h = 24 }: { values: number[]; label: string; w?: number; h?: number } = $props();
  const points = $derived.by(() => {
    if (values.length < 2) return '';
    const lo = Math.min(...values), hi = Math.max(...values), span = hi - lo || 1;
    return values.map((v, i) => `${((i / (values.length - 1)) * (w - 2) + 1).toFixed(1)},${(h - 2 - ((v - lo) / span) * (h - 4)).toFixed(1)}`).join(' ');
  });
</script>

<svg width={w} height={h} viewBox="0 0 {w} {h}" role="img" aria-label="{label} trend, {values.length} points">
  {#if points}<polyline {points} fill="none" stroke="var(--accent)" stroke-width="1.5" />{:else}<line x1="1" x2={w - 1} y1={h / 2} y2={h / 2} stroke="var(--border)" stroke-dasharray="2 2" />{/if}
</svg>
