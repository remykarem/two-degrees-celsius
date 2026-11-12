<script lang="ts">
  import type { Game } from '../store';
  import type { MarketCardModel } from './markets';
  import HowButton from './HowButton.svelte';
  import Sparkline from './Sparkline.svelte';
  import { text } from '../store/catalog';
  import { openAnchor } from './ui';

  /** One market, always the same anatomy: price, two bars on one scale (wanted, available), the gap in words, who is short. Everything comes from the wire. */
  let { card, game, noSeries }: { card: MarketCardModel; game: Game; noSeries: string } = $props();
</script>

<article class="card" data-testid="market-{card.id}" data-gap={card.gapKind} data-reconciles={card.reconciles}>
  <header>
    <h4>{card.label}</h4>
    {#each card.mechanisms as m (m.id)}<span class="tag" title={m.blurb} data-testid="mech-{m.id}">{m.label}</span>{/each}
    <HowButton anchor={card.how} label={card.label} />
  </header>
  <p class="blurb">{card.blurb}</p>

  <div class="bars" role="group" aria-label="{card.label}: wanted against available">
    {#each card.bars as b (b.kind)}
      <div class="row">
        <span class="lab">{b.label}</span>
        <span class="track" aria-hidden="true"><span class="fill {b.kind}" style="width: {(b.width * 100).toFixed(1)}%"></span></span>
        <span class="val">{b.value} <span class="u">{b.unit}</span></span>
      </div>
    {/each}
    <div class="traded">{card.traded}</div>
  </div>

  <ul class="gap {card.gapKind}" aria-label="The gap" data-testid="gap-{card.id}">
    {#each card.gap as g (g)}<li>{g}</li>{/each}
  </ul>

  {#if card.price}
    <div class="price" data-testid="price-{card.id}">
      <span class="pl">{card.price.label}</span>
      <b class="pv">{card.price.text}</b> <span class="pu">{card.price.unit}</span>
    </div>
    <div class="trend">
      {#if card.price.direction !== 'none'}
        <span class="dir {card.price.direction}"><span aria-hidden="true">{card.price.arrow}</span> {card.price.word}</span>
        <span class="cause">({card.price.cause})</span>
      {/if}
      {#if card.series.length > 1}<Sparkline label="{card.price.label}, {card.label}" values={card.series} w={90} h={22} />{:else}<span class="ns">{noSeries}</span>{/if}
    </div>
  {/if}
  <p class="note" data-testid="note-{card.id}">{card.note}</p>

  <p class="verdict">{card.verdict}</p>
  <button type="button" class="all" onclick={() => game.openTrades(card.id)} data-testid="trades-open-{card.id}">{text('markets', 'trades_button')}</button>

  {#if card.drivers.length > 0}
    <dl class="drivers">{#each card.drivers as d (d.id)}<dt>{d.label}</dt><dd>{d.value}</dd>{/each}</dl>
  {/if}

  {#if card.chips.length > 0}
    <div class="chips" aria-label="Who is short">
      {#each card.chips as c, i (c.id + i)}
        {#if c.selection}
          {@const sel = c.selection}
          <button type="button" class="chip lnk" onclick={() => game.select(sel)}>{c.label}</button>
        {:else}<span class="chip">{c.label}</span>{/if}
      {/each}
    </div>
  {/if}
  {#if card.link}
    {@const link = card.link}
    <button type="button" class="seealso" data-testid="market-link-{card.id}" onclick={() => openAnchor(link.target)}>{link.label}</button>
  {/if}
</article>

<style>
  .seealso { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 4px 0 0; font: inherit; font-size: 12px; }
  .card { border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 10px; margin: 8px 0; background: var(--surface); }
  .card[data-reconciles='false'] { border-color: var(--bad); }
  header { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  h4 { margin: 0; font-size: 14px; text-transform: none; letter-spacing: 0; color: var(--text); }
  .tag { font-size: 10.5px; border: 1px solid var(--border); border-radius: 8px; padding: 0 6px; color: var(--text-dim); background: var(--surface-2); }
  .blurb { margin: 3px 0 6px; font-size: 12px; color: var(--text-dim); }
  .bars { display: grid; gap: 3px; }
  .row { display: grid; grid-template-columns: 112px 1fr auto; gap: 6px; align-items: center; font-size: 12px; }
  .lab { color: var(--text-dim); font-size: 11px; line-height: 1.2; }
  .track { display: block; height: 10px; background: var(--surface-2); border-radius: 5px; overflow: hidden; }
  .fill { display: block; height: 100%; border-radius: 5px; min-width: 1px; }
  .fill.wanted { background: var(--series-2); } .fill.available { background: var(--series-1); }
  .val { font-family: var(--font-mono); font-size: 12px; text-align: right; white-space: nowrap; } .u { color: var(--text-dim); font-family: var(--font); font-size: 11px; }
  .traded { font-size: 11px; color: var(--text-dim); text-align: right; }
  .gap { list-style: none; margin: 6px 0 2px; padding: 0; display: flex; flex-wrap: wrap; gap: 4px 8px; font-size: 12px; font-weight: 600; }
  .gap li { padding: 1px 8px; border-radius: 10px; background: var(--surface-2); }
  .gap.short li { background: var(--bad-bg); color: var(--bad); } .gap.glut li { background: var(--warn-bg); color: var(--warn); } .gap.both li:first-child { background: var(--bad-bg); color: var(--bad); } .gap.both li:last-child { background: var(--warn-bg); color: var(--warn); }
  .gap.none li { background: var(--good-bg); color: var(--good); }
  .price { display: flex; align-items: center; gap: 4px 8px; flex-wrap: wrap; font-size: 12px; margin-top: 6px; }
  .pl { color: var(--text-dim); } .pv { font-family: var(--font-mono); font-size: 14px; } .pu { color: var(--text-dim); }
  .trend { display: flex; align-items: center; gap: 4px 8px; flex-wrap: wrap; font-size: 12px; }
  .dir { font-weight: 600; } .dir.rising { color: var(--bad); } .dir.falling { color: var(--good); } .cause { color: var(--text-dim); } .ns { color: var(--text-dim); font-size: 11px; }
  .note { margin: 3px 0 0; font-size: 11.5px; color: var(--text-dim); font-style: italic; } .note:empty { display: none; }
  .all { margin-top: 6px; font: inherit; font-size: 12px; border: 1px solid var(--accent); background: var(--surface); color: var(--accent); border-radius: var(--radius); padding: 3px 10px; cursor: pointer; } .all:hover { background: var(--surface-2); }
  .verdict { margin: 6px 0 0; font-size: 12.5px; }
  .drivers { margin: 6px 0 0; display: grid; grid-template-columns: 1fr; gap: 0; font-size: 11.5px; } dt { color: var(--text-dim); margin-top: 3px; } dd { margin: 0; font-weight: 600; }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
  .chip { font: inherit; font-size: 11.5px; border: 1px solid var(--border); border-radius: 10px; padding: 1px 8px; background: var(--surface); color: var(--text); }
  button.chip { cursor: pointer; color: var(--accent); } button.chip:hover { background: var(--surface-2); }
</style>
