<script lang="ts">
  import { quarterLabel, type Game } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { pct } from './format';
  import Sparkline from './Sparkline.svelte';
  import { generateStory } from './story';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const snap = $derived($gs.snapshot);
  const over = $derived(snap?.gameOver);
  const hist = $derived(snap?.history ?? []);
  const story = $derived(snap && over ? generateStory(snap, $gs.lastResult) : null);
  const series = $derived([
    { label: 'Emissions per turn', v: hist.map((h) => h.emissions), fmt: (x: number) => `${Math.round(x)} t` },
    { label: 'Carbon budget left', v: hist.map((h) => h.budgetShare), fmt: (x: number) => pct(x) },
    { label: 'Inequality (Gini)', v: hist.map((h) => h.gini), fmt: (x: number) => x.toFixed(2) },
    { label: 'Unemployment', v: hist.map((h) => h.unemployment), fmt: (x: number) => pct(x, 1) },
    { label: 'Legitimacy', v: hist.map((h) => h.legitimacy), fmt: (x: number) => pct(x) },
  ]);
</script>

{#if $gs.phase === 'ended' && over && snap && story}
  <Modal title="The end · {quarterLabel(Math.min(over.turn, snap.game.lastTurn), snap.game)} (turn {Math.min(over.turn, snap.game.lastTurn)})" size="lg" dismissible={false}>
    <div class="banner {story.outcome}" role="status">
      <span class="icon" aria-hidden="true">{story.outcome === 'win' ? '✓' : '✕'}</span>
      <div><div class="verdict">{story.outcome === 'win' ? 'You won' : 'You lost'}</div><h3>{story.headline}</h3><p>{story.summary}</p></div>
    </div>
    <div class="cols">
      <div>
        <h3>What happened, and why</h3>
        <ul class="story">{#each story.paragraphs as p, i (i)}<li>{p}</li>{/each}</ul>
        {#if !story.paragraphs.length}<p class="dim">Not enough history to tell a story.</p>{/if}
      </div>
      <div>
        <h3>The run in five lines</h3>
        {#each series as s (s.label)}
          <div class="spark">
            <span>{s.label}</span>
            <Sparkline label={s.label} values={s.v} w={120} h={28} />
            <span class="num">{s.v.length ? `${s.fmt(s.v[0]!)} → ${s.fmt(s.v[s.v.length - 1]!)}` : ''}</span>
          </div>
        {/each}
      </div>
    </div>
    {#snippet footer()}<Button variant="primary" onclick={() => game.newGame(Date.now() % 100000)}>New game</Button>{/snippet}
  </Modal>
{/if}

<style>
  .banner { display: flex; gap: 14px; align-items: center; padding: 12px 16px; border-radius: var(--radius); margin-bottom: 12px; border: 2px solid; }
  .banner.win { background: var(--good-bg); border-color: var(--good); } .banner.lose { background: var(--bad-bg); border-color: var(--bad); }
  .icon { font-size: 32px; font-weight: 700; } .win .icon { color: var(--good); } .lose .icon { color: var(--bad); }
  .verdict { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; }
  .banner h3 { font-size: 18px; text-transform: none; letter-spacing: 0; color: var(--text); margin: 0; }
  .banner p { margin: 2px 0 0; }
  .cols { display: grid; grid-template-columns: 1.2fr 1fr; gap: 20px; }
  @media (max-width: 860px) { .cols { grid-template-columns: 1fr; } }
  .story { margin: 6px 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; }
  .spark { display: grid; grid-template-columns: 1fr 120px auto; align-items: center; gap: 8px; font-size: 13px; padding: 2px 0; }
  .num { font-family: var(--font-mono); font-size: 12px; }
  .dim { color: var(--text-dim); }
</style>
