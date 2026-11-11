<script lang="ts">
  import { healthMeters, type Game } from '../store';
  import { pct } from './format';
  import { forestTiles, stockOf } from './backing';
  import HowButton from './HowButton.svelte';

  let { game, compact = false }: { game: Game; compact?: boolean } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const meters = $derived(healthMeters($gs.snapshot));
  // Stock out of its maximum (real wire) or the forest tiles standing: the numbers behind each percent.
  function backing(id: string): string {
    const s = $gs.snapshot;
    if (!s) return '';
    if (id === 'forest') return stockOf(s.nature.forestStock, s.nature.forest) ?? forestTiles(s);
    if (id === 'commons') return stockOf(s.nature.commonsStock, s.nature.commons) ?? 'stock n/a';
    return `forest ${stockOf(s.nature.forestStock, s.nature.forest) ?? '?'}, commons ${stockOf(s.nature.commonsStock, s.nature.commons) ?? '?'}${s.weather?.drought ? `, drought ${s.weather.drought.monthsLeft} months left` : ''}`;
  }
  const ANCHOR = { forest: 'forest_health', commons: 'commons_health', farm: 'farm_health' } as const;
</script>

<!-- Colour is never the only cue: every meter shows its number and a word, and the bar has a different fill pattern per level. -->
{#if meters.length}
  <ul class="meters" class:compact aria-label="Forest, commons and farm health" data-testid="health-meters">
    {#each meters as m (m.id)}
      <li title="{m.hint} Backing: {backing(m.id)}">
        <span class="name">{m.label}</span>
        <span class="track" role="meter" aria-label="{m.label} health" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(m.value * 100)} aria-valuetext="{pct(m.value)}, {m.word}">
          <span class="fill {m.level}" style="width: {Math.round(m.value * 100)}%"></span>
        </span>
        <span class="num">{pct(m.value)}</span>
        <span class="word {m.level}" class:hide={compact && m.level === 'healthy'}>{m.word}</span>
        {#if !compact}<span class="bk">{backing(m.id)} <HowButton anchor={ANCHOR[m.id]} label="{m.label} health" /></span>{/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .meters { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  li { display: grid; grid-template-columns: 62px 1fr 38px 62px; align-items: center; gap: 6px; font-size: 12px; }
  .name { color: var(--text-dim); }
  .track { height: 10px; border: 1px solid var(--border); border-radius: 5px; background: var(--surface-2); overflow: hidden; display: block; }
  .fill { display: block; height: 100%; }
  .fill.healthy { background: var(--good); }
  .fill.strained { background: var(--warn); }
  .fill.poor { background: repeating-linear-gradient(45deg, var(--bad) 0 4px, var(--warn) 4px 8px); }
  .fill.failing { background: repeating-linear-gradient(45deg, var(--bad) 0 3px, var(--surface) 3px 6px); }
  .num { font-family: var(--font-mono); font-weight: 600; text-align: right; }
  .word { font-weight: 600; font-size: 11px; }
  .word.healthy { color: var(--good); } .word.strained { color: var(--warn); } .word.poor, .word.failing { color: var(--bad); }
  .hide { visibility: hidden; }
  .bk { grid-column: 1 / -1; font-size: 10.5px; color: var(--text-dim); margin: -3px 0 2px; }
  .compact .hide { display: none; }
  /* Compact: a single row for the top bar. */
  .compact { flex-direction: row; gap: 10px; }
  .compact li { grid-template-columns: auto 34px auto auto; gap: 4px; }
  .compact .track { width: 34px; height: 8px; }
</style>
