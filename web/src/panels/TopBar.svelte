<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import EndTurn from './EndTurn.svelte';
  import { SYSTEM_NAME } from './format';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const snap = $derived($gs.snapshot);
</script>

<header>
  {#if snap}
    <div class="item"><strong>{quarterLabel(snap.turn, snap.game)}</strong> <span class="dim">Turn {Math.min(snap.turn, snap.game.lastTurn)}/{snap.game.lastTurn}</span></div>
    <div class="item" data-coach="pc" title="Political capital: spent to change policy, regenerates with legitimacy">
      <label for="pc"><span class="long">Political capital</span><span class="short">PC</span></label>
      <meter id="pc" min="0" max={snap.pc.cap} value={snap.pc.current}></meter>
      <span class="num">{Math.round(snap.pc.current)}/{snap.pc.cap}</span>
      <span class="dim">(+{snap.pc.regenNextTurn.toFixed(1)}/turn)</span>
    </div>
    <div class="item" data-coach="budget" title="Share of the carbon budget still unspent. Hitting zero ends the game.">
      <label for="cb"><span class="long">Carbon budget</span><span class="short">Carbon</span></label>
      <meter id="cb" min="0" max="1" low="0.25" high="0.5" optimum="1" value={snap.indicators.budgetShare}></meter>
      <span class="num">{Math.round(snap.indicators.budgetShare * 100)}% left</span>
    </div>
    <div class="item">System: <strong>{SYSTEM_NAME[snap.system.current]}</strong>
      {#if snap.system.transition}<span class="dim">(switching to {SYSTEM_NAME[snap.system.transition.to]}, {snap.system.transition.monthsLeft} months left)</span>{/if}
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
  meter { width: 90px; height: 14px; }
  header { flex-wrap: wrap; gap: 6px 16px; }
  .num { font-weight: 600; }
  .short { display: none; }
  @media (max-width: 1200px) { .long { display: none; } .short { display: inline; } meter { width: 70px; } header { gap: 4px 12px; padding: 6px 10px; } .dim { display: none; } }
</style>
