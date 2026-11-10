<script lang="ts">
  import { stressGauge, type Game } from '../store';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const g = $derived(stressGauge($gs.snapshot));
  // The axis is "carbon budget left": 100% on the left, 0% on the right, so the gauge fills as the budget is used. Stress starts at `onset` and is full at `full`.
  const x = (share: number) => 4 + (1 - share) * 262;
</script>

{#if g}
  <section class="gauge" aria-label="Climate stress" data-testid="stress-gauge">
    <div class="head">
      <h3>Climate stress</h3>
      <span class="val"><b>{g.percent}%</b> {g.word}</span>
    </div>
    <svg viewBox="0 0 270 44" width="100%" height="44" role="meter" aria-label="Climate stress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={g.percent} aria-valuetext="{g.percent}%, {g.word}">
      <defs>
        <linearGradient id="stress-ramp" x1="0" x2="1"><stop offset="0" stop-color="var(--warn-bg)" /><stop offset="1" stop-color="var(--bad)" /></linearGradient>
      </defs>
      <rect x="4" y="10" width="262" height="12" rx="6" fill="var(--surface-2)" stroke="var(--border)" />
      <rect x={x(g.onset)} y="10" width={x(g.full) - x(g.onset)} height="12" fill="url(#stress-ramp)" opacity="0.55" />
      <rect x="4" y="10" width={Math.max(0, x(g.remaining) - 4)} height="12" rx="6" fill="var(--text-dim)" opacity="0.18" />
      <line x1={x(g.onset)} x2={x(g.onset)} y1="6" y2="26" stroke="var(--text)" stroke-width="1.5" />
      <line x1={x(g.full)} x2={x(g.full)} y1="6" y2="26" stroke="var(--text)" stroke-width="1.5" />
      <text x={x(g.onset)} y="39" text-anchor="middle" font-size="10" fill="var(--text-dim)">onset {Math.round(g.onset * 100)}% left</text>
      <text x={x(g.full)} y="39" text-anchor="middle" font-size="10" fill="var(--text-dim)">full {Math.round(g.full * 100)}% left</text>
      <path d="M {x(g.remaining)} 7 l -5 -6 h 10 z" fill="var(--text)" />
      <line x1={x(g.remaining)} x2={x(g.remaining)} y1="7" y2="22" stroke="var(--text)" stroke-width="2" />
    </svg>
    <p class="text">{g.text} The marker shows the carbon budget left now ({Math.round(g.remaining * 100)}%).</p>
  </section>
{/if}

<style>
  .gauge { margin-bottom: 10px; }
  .head { display: flex; justify-content: space-between; align-items: baseline; }
  .val { font-size: 12px; font-family: var(--font-mono); }
  .text { margin: 2px 0 0; font-size: 12px; color: var(--text-dim); }
</style>
