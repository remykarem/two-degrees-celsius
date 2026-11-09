<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { Button } from '../ui-kit';
  import Sparkline from './Sparkline.svelte';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const over = $derived($gs.snapshot?.gameOver);
  const hist = $derived($gs.snapshot?.history ?? []);
  const story = $derived.by(() => {
    const o = over?.outcome;
    if (o === 'budget_exhausted') return 'The carbon budget ran out. Emissions kept the town on a path the planet could not absorb.';
    if (o === 'survived') return 'The town reached the end of the horizon within its carbon budget.';
    if (o && typeof o === 'object') return `The town collapsed: ${o.collapse.replaceAll('_', ' ')}.`;
    return '';
  });
</script>

{#if $gs.phase === 'ended' && over}
  <div class="backdrop">
    <div class="panel" role="dialog" aria-modal="true" aria-labelledby="et">
      <h2 id="et">The end · {quarterLabel(over.turn)} (turn {over.turn})</h2>
      <p>{story}</p>
      <p class="dim">Causal story: placeholder. The real simulation will list the main causes from the trace.</p>
      <div class="spark"><span>Emissions</span><Sparkline label="Emissions" values={hist.map((h) => h.emissions)} w={160} /></div>
      <div class="spark"><span>Legitimacy</span><Sparkline label="Legitimacy" values={hist.map((h) => h.legitimacy)} w={160} /></div>
      <div class="spark"><span>Unemployment</span><Sparkline label="Unemployment" values={hist.map((h) => h.unemployment)} w={160} /></div>
      <div class="row"><Button variant="primary" onclick={() => game.newGame(Date.now() % 100000)}>New game</Button></div>
    </div>
  </div>
{/if}

<style>
  .backdrop { position: absolute; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 40; }
  .panel { background: var(--surface); border-radius: var(--radius); padding: 16px 20px; width: min(520px, 92%); box-shadow: var(--shadow); }
  .spark { display: flex; justify-content: space-between; align-items: center; }
  .dim { color: var(--text-dim); font-size: 12px; }
  .row { display: flex; justify-content: flex-end; margin-top: 12px; }
</style>
