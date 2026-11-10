<script lang="ts">
  import type { FinanceView } from '../contract';
  import { circularFlow } from './finance';
  import { money } from './format';

  let { finance }: { finance: FinanceView } = $props();
  const flow = $derived(circularFlow(finance));
  // Four nodes on a diamond. Edges between the same pair are drawn as two offset arrows (one each way).
  const NODE = { households: { x: 135, y: 22 }, firms: { x: 135, y: 150 }, government: { x: 38, y: 86 }, bank: { x: 236, y: 86 } } as const;
  const NAME = { households: 'Households', firms: 'Firms', government: 'Government', bank: 'Bank' } as const;
  const max = $derived(Math.max(1, ...flow.edges.map((e) => e.amount)));
  function geo(e: (typeof flow.edges)[number]) {
    const a = NODE[e.from], b = NODE[e.to];
    const dx = b.x - a.x, dy = b.y - a.y, len = Math.hypot(dx, dy) || 1;
    const nx = -dy / len, ny = dx / len, off = 5; // offset to the right of travel so the reverse arrow sits beside it
    const pad = 30;
    const x1 = a.x + (dx / len) * pad + nx * off, y1 = a.y + (dy / len) * pad + ny * off, x2 = b.x - (dx / len) * pad + nx * off, y2 = b.y - (dy / len) * pad + ny * off;
    return { x1, y1, x2, y2, mx: (x1 + x2) / 2 + nx * 7, my: (y1 + y2) / 2 + ny * 7 };
  }
</script>

<figure aria-label="Circular flow of money over the last quarter">
  <svg viewBox="0 0 270 176" width="100%" role="img" aria-label="Money flows between households, firms, government and bank in the last quarter">
    <defs><marker id="arr" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" markerUnits="userSpaceOnUse" orient="auto-start-reverse"><path d="M0,0 L8,4 L0,8 z" fill="var(--text-dim)" /></marker></defs>
    {#each flow.edges as e (e.from + e.to + e.label)}
      {@const g = geo(e)}
      <line x1={g.x1} y1={g.y1} x2={g.x2} y2={g.y2} stroke="var(--series-1)" stroke-opacity="0.75" stroke-width={0.8 + (e.amount / max) * 4.2} marker-end="url(#arr)"><title>{NAME[e.from]} to {NAME[e.to]}: {e.label}, {money(e.amount)}</title></line>
    {/each}
    {#each Object.entries(NODE) as [k, p] (k)}
      <rect x={p.x - 36} y={p.y - 11} width="72" height="22" rx="5" fill="var(--surface-2)" stroke="var(--border)" />
      <text x={p.x} y={p.y + 4} text-anchor="middle" font-size="10.5" fill="var(--text)" font-weight="600">{NAME[k as keyof typeof NAME]}</text>
    {/each}
  </svg>
  <figcaption>
    <ul class="legend">
      {#each flow.edges as e (e.from + e.to + e.label)}<li><span>{NAME[e.from]} to {NAME[e.to]}: {e.label}</span><b>{money(e.amount)}</b></li>{/each}
    </ul>
    <p class="dim">Dollars over the last quarter; arrow width follows the amount.{flow.approx ? ' (Three times the latest month: the quarter total is not on the wire.)' : ''}</p>
  </figcaption>
</figure>

<style>
  figure { margin: 0; }
  .legend { list-style: none; margin: 4px 0; padding: 0; font-size: 11px; display: grid; gap: 1px; }
  .legend li { display: flex; justify-content: space-between; gap: 6px; }
  .legend b { font-family: var(--font-mono); font-weight: 600; }
  .dim { margin: 2px 0 0; font-size: 11px; color: var(--text-dim); }
</style>
