<script lang="ts">
  import type { Game } from '../store';
  import { text } from '../store/catalog';
  import HowLink from './HowLink.svelte';
  import MarketStrip from './MarketStrip.svelte';
  import { marketsModel } from './markets';

  /** The Markets view (a sub-view of Economy): an intro on coordination, how prices are set here, then one strip per market in the wire's list. */
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const m = $derived(marketsModel($gs.snapshot));
</script>

<div class="mk" data-testid="markets-panel">
  <section class="intro" aria-labelledby="mk-intro">
    <h3 id="mk-intro">{m.intro.title}</h3>
    <p>{m.intro.body}</p>
    <ul class="mechs" aria-label={m.mechanisms.title}>
      {#each m.mechanisms.items as x (x.id)}<li data-testid="mechanism-{x.id}"><b>{x.label}.</b> {x.blurb} <span class="ex">{m.mechanisms.examplePrefix} {x.example}</span></li>{/each}
    </ul>
    <p class="dim">{m.mechanisms.foot} {m.intro.notCapitalism} <HowLink anchor={m.intro.linkAnchor} text={m.intro.link} /></p>
  </section>

  <section class="cp" aria-labelledby="mk-cp">
    <h3 id="mk-cp">{m.costPlus.title}</h3>
    <p>{m.costPlus.body} <HowLink anchor={m.costPlus.anchor} text={text('markets', 'how_label')} /></p>
    <p class="dim">{m.smallTown}</p>
  </section>

  {#if m.ready}
    {#each m.groups as g (g.id)}
      <section class="grp" aria-labelledby="mk-g-{g.id}" data-testid="markets-group-{g.id}">
        <h3 id="mk-g-{g.id}">{g.label}</h3>
        {#if g.blurb}<p class="dim">{g.blurb}</p>{/if}
        {#each g.cards as c (c.id)}<MarketStrip card={c} {game} noSeries={text('markets', 'no_series')} />{/each}
      </section>
    {/each}
  {:else}<p class="dim" data-testid="markets-empty">{m.empty}</p>{/if}
</div>

<style>
  .mk { display: block; }
  h3 { font-size: 12px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); margin: 10px 0 4px; }
  p { margin: 3px 0; font-size: 12.5px; } .dim { color: var(--text-dim); font-size: 12px; }
  .intro, .cp { border: 1px solid var(--border); border-radius: var(--radius); background: var(--info-bg); padding: 4px 10px 8px; margin: 8px 0; }
  .mechs { margin: 4px 0; padding-left: 16px; font-size: 12px; } .mechs li { margin: 2px 0; } .ex { color: var(--text-dim); }
</style>
