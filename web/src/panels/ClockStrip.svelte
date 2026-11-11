<script lang="ts">
  import type { Game, Live } from '../store';
  import { Segmented } from '../ui-kit';

  let { game, live }: { game: Game; live: Live } = $props();
  // svelte-ignore state_referenced_locally
  const v = live.view;
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const c = $derived($v.clock);
  const months = $derived($gs.snapshot?.game.ticksPerTurn ?? 0); // months (monthly ticks) in a quarter, from the wire
  const R = 15;
  const CIRC = 2 * Math.PI * R;
  const secs = $derived(Number.isFinite(c.secondsLeft) ? Math.max(0, Math.ceil(c.secondsLeft)) : null);
  const near = $derived($v.active && c.phase !== 'paused' && c.secondsLeft <= 3);
  const why = $derived(c.pausedBy.includes('modal') ? 'a dialog is open' : c.pausedBy.includes('hidden') ? 'this tab is hidden' : 'paused by you');
  const manual = $derived(c.pausedBy.includes('manual'));
  const announce = $derived($v.active ? `Month ${c.month + 1} of ${months} of ${$v.playing ?? 'the quarter'}.` : '');
  const budget = $derived(c.pauseBudgetLeft === null ? null : Math.ceil(c.pauseBudgetLeft));
  const tick = (f: number) => ({ x1: 18 + (R - 3) * Math.sin(2 * Math.PI * f), y1: 18 - (R - 3) * Math.cos(2 * Math.PI * f), x2: 18 + (R + 3) * Math.sin(2 * Math.PI * f), y2: 18 - (R + 3) * Math.cos(2 * Math.PI * f) });
</script>

{#if $gs.mode === 'pressure'}
  <div class="strip" class:near class:paused={c.phase === 'paused'} aria-label="Game clock" role="group">
    <div class="date" title="In-game date and hour. The quarter you are playing is shown on the right.">
      <span class="num">{$v.date?.label ?? '--'}</span>
    </div>
    <svg class="ring" width="36" height="36" viewBox="0 0 36 36" role="img"
      aria-label={secs === null ? 'Clock paused' : `${Math.round(c.frac * 100)} percent of the quarter has passed; about ${secs} seconds to the next quarter`}>
      <circle cx="18" cy="18" r={R} fill="none" stroke="var(--border)" stroke-width="4" />
      <circle class="arc" cx="18" cy="18" r={R} fill="none" stroke="var(--accent)" stroke-width="4" stroke-linecap="butt" transform="rotate(-90 18 18)"
        stroke-dasharray="{CIRC * c.frac} {CIRC}" />
      {#each $v.markers as m (m)}{@const t = tick(m)}<line {...t} stroke="var(--text)" stroke-width="2" />{/each}
      <line {...tick(0)} stroke="var(--text)" stroke-width="2" />
    </svg>
    <div class="q">
      <strong>{$v.playing ? `Playing ${$v.playing}` : 'Clock stopped'}</strong>
      <span class="dim">month {Math.min(months, c.month + 1)} of {months}{#if secs !== null}, next quarter in {secs} s{/if}</span>
    </div>
    <div class="q planning">
      <strong>Planning {$v.planning ?? ''}</strong>
      <span class="dim">{$gs.live?.locked ? 'plan locked' : 'your plan applies when it begins'}</span>
    </div>
    {#if near}<span class="cue" role="status">Quarter ends in {secs} s{$gs.live?.locked ? '' : ': your plan will be submitted'}</span>{/if}
    <span class="spacer"></span>
    {#if c.phase === 'paused'}<span class="why" role="status">Paused: {why}</span>{/if}
    <button type="button" class="pause" aria-pressed={manual} disabled={!live.config.allowPause && !manual} onclick={() => live.togglePause()}
      title={live.config.allowPause ? 'Pause or resume the clock (P)' : 'Pausing is turned off in this game'}>{manual ? '▶ Resume' : '⏸ Pause'}</button>
    {#if budget !== null}<span class="dim" title="Pause time left">{budget} s pause left</span>{/if}
    <Segmented label="Clock speed" value={String(c.speed)} options={live.config.speeds.map((n) => ({ value: String(n), label: `${n}x` }))} onchange={(x) => live.setSpeed(Number(x))} />
    <label class="slow" title="Run the clock at {live.config.slowPace}x speed, for more thinking time"><input type="checkbox" checked={c.slow} onchange={() => live.toggleSlow()} /> Slow pace</label>
    <span class="sr-only" role="status" aria-live="polite">{announce}</span>
  </div>
{/if}

<style>
  .strip { flex-basis: 100%; display: flex; align-items: center; gap: 6px 16px; flex-wrap: wrap; padding: 4px 0 2px; border-top: 1px solid var(--border); }
  .date { min-width: 170px; font-size: 15px; }
  .num { font-family: var(--font-mono); font-weight: 600; }
  .ring { flex: none; }
  .arc { transition: none; }
  .q { display: flex; flex-direction: column; line-height: 1.2; font-size: 13px; }
  .dim { color: var(--text-dim); font-size: 12px; }
  .spacer { flex: 1; }
  .why { color: var(--warn); font-weight: 600; font-size: 13px; }
  .cue { color: var(--bad); font-weight: 700; font-size: 13px; animation: pulse 0.9s ease-in-out infinite; }
  .near .ring { filter: drop-shadow(0 0 3px var(--bad)); }
  @keyframes pulse { 50% { opacity: 0.45; } }
  .pause { font: inherit; padding: 4px 10px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface); color: var(--text); cursor: pointer; }
  .pause[aria-pressed='true'] { outline: 2px solid var(--accent); }
  .pause:disabled { opacity: 0.5; cursor: not-allowed; }
  .slow { display: inline-flex; gap: 4px; align-items: center; cursor: pointer; font-size: 13px; }
  .paused .date { opacity: 0.7; }
  @media (prefers-reduced-motion: reduce) { .cue { animation: none; } .arc { transition: none; } }
  @media (max-width: 1200px) { .planning .dim { display: none; } .strip { gap: 4px 10px; } .date { min-width: 140px; font-size: 13px; } }
</style>
