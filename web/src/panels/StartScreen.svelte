<script lang="ts">
  import { fill, secondsPerQuarter, text, type GameMode, type Game, type Live } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { setUi, ui } from './ui';

  let { game, live, seed }: { game: Game; live: Live; seed: number } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let mode = $state<GameMode>('relaxed'); // Relaxed is always preselected
  // svelte-ignore state_referenced_locally
  const spq = Math.round(secondsPerQuarter(live.config));
  const ok = $derived($gs.liveAvailable);
  const pitch = $derived.by(() => {
    const g = $gs.snapshot?.game;
    return g ? fill(text('rules', 'start_pitch'), { years: g.lastTurn / g.turnsPerYear, limit: g.temperatureLimit }) : '';
  });
  function start() {
    live.newGame($ui.nextSeed ?? seed, mode);
    setUi({ startOpen: false, nextSeed: null });
  }
</script>

{#if $ui.startOpen}
  <Modal title="New game" size="md" dismissible={false}>
    <p>{pitch}</p>
    <div class="choices" role="radiogroup" aria-label="Game mode">
      <label class="opt" class:on={mode === 'relaxed'}>
        <input type="radio" name="mode" value="relaxed" checked={mode === 'relaxed'} onchange={() => (mode = 'relaxed')} data-autofocus />
        <span><strong>Relaxed</strong> <em>(recommended)</em><br />The game waits for you. Plan as long as you like, then press End turn. Nothing moves until you say so.</span>
      </label>
      <label class="opt" class:on={mode === 'pressure'} class:off={!ok}>
        <input type="radio" name="mode" value="pressure" checked={mode === 'pressure'} disabled={!ok} onchange={() => (mode = 'pressure')} />
        <span><strong>Pressure</strong><br />Time keeps moving. Each quarter plays live (about {spq} seconds) while you plan the next one. Lock your plan when ready; if the clock runs out, your drafts are submitted. You can pause, slow down or fast-forward.
          {#if !ok}<br /><b class="bad">Not available here: this simulation build has no live-turn support. Rebuild it with npm run build:wasm.</b>{/if}</span>
      </label>
    </div>
    {#snippet footer()}<Button variant="primary" onclick={start}>Start game</Button>{/snippet}
  </Modal>
{/if}

<style>
  .choices { display: flex; flex-direction: column; gap: 8px; margin: 10px 0; }
  .opt { display: flex; gap: 10px; align-items: flex-start; padding: 10px 12px; border: 1px solid var(--border); border-radius: var(--radius); cursor: pointer; }
  .opt.on { border-color: var(--accent); outline: 2px solid var(--accent); }
  .opt.off { opacity: 0.65; cursor: not-allowed; }
  em { font-style: normal; color: var(--text-dim); font-size: 12px; }
  .bad { color: var(--bad); }
</style>
