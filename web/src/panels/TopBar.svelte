<script lang="ts">
  import { quarterLabel, shownTurn, weatherOf, type Game, type Live } from '../store';
  import SaveLoad from './SaveLoad.svelte';
  import ClockStrip from './ClockStrip.svelte';
  import HealthMeters from './HealthMeters.svelte';
  import EndTurn from './EndTurn.svelte';
  import { SYSTEM_NAME } from './format';
  let { game, live = undefined }: { game: Game; live?: Live } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const snap = $derived($gs.snapshot);
</script>

<header>
  {#if snap}
    <div class="item"><strong>{quarterLabel(shownTurn(snap), snap.game)}</strong> <span class="dim">Turn {shownTurn(snap)}/{snap.game.lastTurn}</span>{#if snap.mock}<span class="mock" title="The numbers on screen come from a web-side mock, not the real simulation. Rules and costs are copied from the real content files, but GDP, unemployment, wage share, profit share, money and the other economy figures are invented toy dynamics.">Mock data</span>{:else}<span class="real" title="The numbers on screen come from the real Rust simulation, running in your browser as WebAssembly.">Real simulation</span>{/if}</div>
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
    {#if weatherOf(snap)}
      <div class="item cluster" data-coach="health" title="Forest, commons and farm health, and climate stress (it rises as the carbon budget runs down)">
        <HealthMeters {game} compact />
        <span class="stress" aria-label="Climate stress {Math.round((weatherOf(snap)?.stress ?? 0) * 100)} percent">Stress <b class="num">{Math.round((weatherOf(snap)?.stress ?? 0) * 100)}%</b></span>
      </div>
    {/if}
    <div class="item"><span class="long">System:</span> <strong>{SYSTEM_NAME[snap.system.current]}</strong>
      {#if snap.system.transition}<span class="dim">(switching to {SYSTEM_NAME[snap.system.transition.to]}, {snap.system.transition.monthsLeft} months left)</span>{/if}
    </div>
    <div class="spacer"></div>
    <SaveLoad {game} />
    {#if $gs.mode === 'pressure' && live}<ClockStrip {game} {live} />{:else}<EndTurn {game} />{/if}
  {/if}
</header>

<style>
  header { display: flex; align-items: center; gap: 20px; padding: 8px 14px; background: var(--surface); border-bottom: 1px solid var(--border); }
  .item { display: flex; align-items: center; gap: 6px; white-space: nowrap; }
  .spacer { flex: 1; }
  .mock { margin-left: 8px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; padding: 1px 6px; border: 1px dashed var(--warn); color: var(--warn); border-radius: 8px; cursor: help; }
  .real { margin-left: 8px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; padding: 1px 6px; border: 1px solid var(--border); color: var(--text-dim); border-radius: 8px; cursor: help; }
  .dim { color: var(--text-dim); }
  .num { font-family: var(--font-mono); }
  meter { width: 90px; height: 14px; }
  header { flex-wrap: wrap; gap: 6px 16px; }
  .num { font-weight: 600; }
  .short { display: none; }
  .cluster { gap: 10px; font-size: 12px; }
  .stress { color: var(--text-dim); }
  @media (max-width: 1200px) { header { flex-wrap: nowrap; } .item { gap: 4px; } .long { display: none; } .short { display: inline; } meter { width: 70px; } header { gap: 4px 12px; padding: 6px 10px; } .dim { display: none; } }
</style>
