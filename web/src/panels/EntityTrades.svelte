<script lang="ts">
  import type { MarketEntity } from '../contract';
  import type { Game } from '../store';
  import { fill, text } from '../store/catalog';
  import { rowSummary } from './trades';

  /** What one household, firm or person took part in this month, across every market the wire lists. Cheap: one call per market. */
  let { game, entity }: { game: Game; entity: MarketEntity } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const SHOWN = 4;
  const lists = $derived.by(() => {
    const snap = $gs.snapshot;
    return (snap?.markets ?? []).flatMap((mk) => {
      const r = game.marketTrades(mk.id, { entity });
      return r.ok && r.value.rows.length > 0 ? [{ id: mk.id, label: mk.label, m: r.value }] : [];
    });
  });
</script>

{#if lists.length > 0}
  <section class="et" aria-label={text('markets', 'trades_inspector')} data-testid="entity-trades">
    <h4>{text('markets', 'trades_inspector')}</h4>
    {#each lists as l (l.id)}
      <div class="mk" data-testid="entity-trades-{l.id}">
        <b>{l.label}</b>
        <ul>
          {#each l.m.rows.slice(0, SHOWN) as r, i (i)}<li class:dim={!r.counted}>{rowSummary(l.m, r)}</li>{/each}
        </ul>
        <button type="button" class="open" onclick={() => game.openTrades(l.id, entity)}>{fill(text('markets', 'trades_see_all'), { market: l.label })} ({fill(text('markets', 'trades_rows'), { n: l.m.rows.length })})</button>
      </div>
    {/each}
  </section>
{/if}

<style>
  h4 { margin: 8px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  .mk { font-size: 12px; margin: 4px 0; }
  ul { list-style: none; margin: 2px 0; padding: 0; } li { border-top: 1px dotted var(--border); padding: 1px 0; } li.dim { color: var(--text-dim); font-style: italic; }
  .open { font: inherit; font-size: 12px; border: 1px solid var(--accent); background: var(--surface); color: var(--accent); border-radius: var(--radius); padding: 2px 8px; cursor: pointer; width: 100%; text-align: left; }
  .open:hover { background: var(--surface-2); }
</style>
