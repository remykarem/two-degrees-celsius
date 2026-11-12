<script lang="ts">
  import type { MarketEntity, MarketTrades } from '../contract';
  import type { Game } from '../store';
  import { text } from '../store/catalog';
  import { entitySelection, nextSort, periodFromValue, periodValue, reconcileLine, sortRows, type SortState } from './trades';

  /** The individual trades behind one market. Everything shown (columns, help, cells, totals, periods) is the wire's. */
  let { game, market, entity = undefined }: { game: Game; market: string; entity?: MarketEntity } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let chosen = $state<string>('');
  let sort = $state<SortState | null>(null);
  let appliedMarket = '';
  $effect(() => { if (market !== appliedMarket) { appliedMarket = market; chosen = ''; sort = null; } });

  const res = $derived.by((): ReturnType<Game['marketTrades']> => {
    void $gs.snapshot;
    return game.marketTrades(market, { ...(chosen ? periodFromValue(chosen) : {}), ...(entity ? { entity } : {}) });
  });
  const m = $derived<MarketTrades | null>(res.ok ? res.value : null);
  const rows = $derived(m ? sortRows(m.rows, sort) : []);
  const rec = $derived(m ? reconcileLine(m) : null);
  const ariaSort = (i: number) => (sort?.column === i ? (sort.dir === 1 ? 'ascending' : 'descending') : 'none');
</script>

{#if !res.ok}
  <p class="err" role="alert" data-testid="trades-error">{res.error.code === 'unsupported' ? text('markets', 'trades_unavailable') : res.error.message}</p>
{:else if m}
  <p class="intro" data-testid="trades-intro">{m.intro}</p>
  {#if m.periods.length > 0}
    <label class="per">{text('markets', 'trades_period')}
      <select value={chosen || periodValue(m.period)} onchange={(e) => (chosen = (e.currentTarget as HTMLSelectElement).value)} data-testid="trades-period">
        {#each m.periods as p (periodValue(p))}<option value={periodValue(p)}>{p.label}</option>{/each}
      </select></label>
  {/if}
  {#if m.columns.length === 0}
    <p class="dim" data-testid="trades-empty">{m.empty}</p>
  {:else}
    <p class="dim">{text('markets', 'trades_how_to_sort')}</p>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="scroll" role="region" aria-label={m.label} tabindex="0">
      <table data-testid="trades-table">
        <thead>
          <tr>
            {#each m.columns as c, i (c.id)}
              <th scope="col" class:n={c.numeric} aria-sort={ariaSort(i)} title={c.help}>
                <button type="button" onclick={() => (sort = nextSort(sort, i))} data-testid="sort-{c.id}">{c.label}<span class="ar" aria-hidden="true">{sort?.column === i ? (sort.dir === 1 ? ' ▲' : ' ▼') : ''}</span></button>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each rows as r, ri (ri)}
            <tr class:uncounted={!r.counted} data-testid="trade-row">
              {#each r.cells as c, i (i)}
                <td class:n={m.columns[i]?.numeric || (c.sort !== undefined && !c.entity && m.columns[i]?.id !== 'month')}>
                  {#if c.entity}{@const e = c.entity}<button type="button" class="lnk" onclick={() => game.select(entitySelection(e))}>{c.text}</button>{:else}{c.text}{/if}
                </td>
              {/each}
            </tr>
          {:else}
            <tr><td colspan={m.columns.length} class="empty" data-testid="trades-empty">{m.empty}</td></tr>
          {/each}
        </tbody>
        {#if rows.length}
          <tfoot>
            <tr data-testid="trades-totals">
              {#each m.totals.cells as c, i (i)}<td class:n={m.columns[i]?.numeric || c.sort !== undefined}>{i === 0 ? `${text('markets', 'trades_totals')}: ${c.text}` : c.text}</td>{/each}
            </tr>
          </tfoot>
        {/if}
      </table>
    </div>
    {#if m.totals.text}<p class="tot" data-testid="trades-total-text">{m.totals.text}</p>{/if}
    {#if m.uncounted}<p class="dim">{m.uncounted}</p>{/if}
    {#if rec}<p class="rec" class:good={rec.ok} class:bad={!rec.ok} role="status" data-testid="trades-reconcile"><span aria-hidden="true">{rec.ok ? '✓' : '✗'}</span> {rec.text}</p>{/if}
    {#if m.period.rolled}<p class="dim">{text('markets', 'trades_older')}</p>{/if}
    <details class="help">
      <summary>{m.label}</summary>
      <dl data-testid="trades-help">{#each m.columns as c (c.id)}<dt>{c.label}</dt><dd>{c.help}</dd>{/each}</dl>
    </details>
  {/if}
{/if}

<style>
  .intro { margin: 0 0 6px; font-size: 13px; }
  .dim { color: var(--text-dim); font-size: 12px; margin: 4px 0; }
  .per { display: inline-flex; gap: 6px; align-items: center; font-size: 12px; color: var(--text-dim); margin-bottom: 6px; }
  select { font: inherit; font-size: 12px; background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); padding: 2px 4px; }
  .scroll { overflow: auto; max-height: 360px; border: 1px solid var(--border); border-radius: var(--radius); }
  table { border-collapse: collapse; width: 100%; font-size: 12px; }
  th, td { text-align: left; padding: 3px 8px; border-bottom: 1px solid var(--border); white-space: nowrap; }
  th { position: sticky; top: 0; background: var(--surface-2); z-index: 1; padding: 0; }
  th button { font: inherit; font-weight: 600; width: 100%; text-align: inherit; background: transparent; border: 0; color: var(--text); padding: 3px 8px; cursor: pointer; }
  th button:hover { background: var(--surface); }
  .n { text-align: right; font-family: var(--font-mono); } th.n button { text-align: right; }
  tr.uncounted td { color: var(--text-dim); font-style: italic; }
  tfoot td { font-weight: 600; background: var(--surface-2); position: sticky; bottom: 0; }
  .lnk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; }
  .empty { text-align: center; color: var(--text-dim); padding: 14px; }
  .tot { font-size: 12.5px; margin: 6px 0 2px; }
  .rec { font-size: 12px; font-weight: 600; margin: 2px 0; } .rec.good { color: var(--good); } .rec.bad { color: var(--bad); }
  .help { margin-top: 6px; font-size: 12px; } summary { cursor: pointer; color: var(--accent); }
  .help dl { display: grid; grid-template-columns: max-content 1fr; gap: 2px 10px; margin: 4px 0; } .help dt { font-weight: 600; } .help dd { margin: 0; color: var(--text-dim); }
  .err { color: var(--bad); }
</style>
