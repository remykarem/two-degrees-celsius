<script lang="ts">
  import type { Game } from '../store';
  import { earningsRows, PEOPLE_NOTE } from './ledger';
  import { formatEvent } from './format';

  let { game, person }: { game: Game; person: number } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const res = $derived.by((): ReturnType<Game['personLedger']> => { void $gs.snapshot; return game.personLedger(person); });
  const rows = $derived(res.ok ? earningsRows(res.value.earnings, $gs.snapshot, res.value.household).slice(0, 8) : []);
  const total = $derived(res.ok ? res.value.earnings.reduce((a, e) => a + e.amount, 0) : 0);
  const events = $derived(res.ok ? res.value.events.slice(-4).reverse() : []);
</script>

<section aria-label="Earnings and events">
  <h4>Earnings and job events</h4>
  <p class="note">{PEOPLE_NOTE}</p>
  {#if !res.ok}
    <p class="note">{res.error.code === 'unsupported' ? 'This simulation package has no ledger yet.' : res.error.message}</p>
  {:else}
    {#if rows.length}
      <ul class="last" aria-label="Wages earned">
        {#each rows as r (r.key)}<li><span class="m" title={r.memo}>{r.date}, {r.counterpartyLabel}</span><span class="a">{r.amountText}</span></li>{/each}
      </ul>
      <p class="note">Wages in the recent window: ${Math.round(total).toLocaleString('en-US')}. Each was paid to their household.</p>
      <button type="button" class="open" onclick={() => game.openLedger({ kind: 'household', id: res.value.household }, 'wage')}>See the household's wage transactions</button>
    {:else}<p class="note">No wages earned in the recent window.</p>{/if}
    {#if events.length}
      <ul class="last" aria-label="Job events">{#each events as ev, i (i)}<li><span class="m">{formatEvent(ev, $gs.snapshot)}</span></li>{/each}</ul>
    {/if}
  {/if}
</section>

<style>
  h4 { margin: 8px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  .note { font-size: 12px; color: var(--text-dim); margin: 2px 0; }
  .last { list-style: none; margin: 4px 0; padding: 0; font-size: 12px; }
  .last li { display: flex; justify-content: space-between; gap: 6px; border-top: 1px dotted var(--border); padding: 1px 0; }
  .m { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a { font-family: var(--font-mono); color: var(--good); white-space: nowrap; }
  .open { margin-top: 4px; font: inherit; font-size: 12px; border: 1px solid var(--accent); background: var(--surface); color: var(--accent); border-radius: var(--radius); padding: 3px 8px; cursor: pointer; width: 100%; }
</style>
