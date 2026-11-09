<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import EndTurn from './EndTurn.svelte';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const snap = $derived($gs.snapshot);
</script>

<header>
  {#if snap}
    <div class="item"><strong>{quarterLabel(snap.turn, snap.game)}</strong> <span class="dim">Turn {Math.min(snap.turn, snap.game.lastTurn)}/{snap.game.lastTurn}</span></div>
    <div class="item" title="Political capital: spent to change policy, regenerates with legitimacy">
      <label for="pc">PC</label>
      <meter id="pc" min="0" max={snap.pc.cap} value={snap.pc.current}></meter>
      <span class="num">{Math.round(snap.pc.current)}/{snap.pc.cap}</span>
      <span class="dim">(+{snap.pc.regenNextTurn.toFixed(1)}/turn)</span>
    </div>
    <div class="item" title="Share of the carbon budget still unspent">
      <label for="cb">Carbon budget</label>
      <meter id="cb" min="0" max="1" low="0.25" high="0.5" optimum="1" value={snap.indicators.budgetShare}></meter>
      <span class="num">{Math.round(snap.indicators.budgetShare * 100)}% left</span>
    </div>
    <div class="item">System: <strong>{snap.system.current}</strong>
      {#if snap.system.transition}<span class="dim">(switching to {snap.system.transition.to}, {snap.system.transition.monthsLeft} months left)</span>{/if}
    </div>
    <div class="spacer"></div>
    <EndTurn {game} />
  {/if}
</header>

<style>
  header { display: flex; align-items: center; gap: 20px; padding: 8px 14px; background: var(--surface); border-bottom: 1px solid var(--border); }
  .item { display: flex; align-items: center; gap: 6px; white-space: nowrap; }
  .spacer { flex: 1; }
  .dim { color: var(--text-dim); }
  .num { font-family: var(--font-mono); }
  meter { width: 110px; height: 14px; }
</style>
