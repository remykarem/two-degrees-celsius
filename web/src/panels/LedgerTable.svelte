<script lang="ts">
  import type { AccountRef, LedgerKind, LedgerPage } from '../contract';
  import type { Game } from '../store';
  import { accountKey, accountLabel, accountSelection, counterpartiesIn, exact, KIND_LABEL, kindsIn, ledgerRows, monthLabel, NO_FILTER, paginate, reconcile, rollupGroups, rollupTotals, rowTotals, sameAccount, signedExact, turnsIn, type LedgerFilter } from './ledger';

  let { game, account, kind = undefined, onselect }: { game: Game; account: AccountRef; kind?: LedgerKind; onselect: (a: AccountRef) => void } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const SIZE = 25;
  let filter = $state<LedgerFilter>({ ...NO_FILTER });
  let pageNo = $state(0);
  let appliedKind: string | undefined;
  $effect(() => {
    const k = `${accountKey(account)}|${kind ?? ''}`;
    if (k !== appliedKind) { appliedKind = k; filter = { ...NO_FILTER, kind: kind ?? 'all' }; pageNo = 0; }
  });

  const snap = $derived($gs.snapshot);
  const res = $derived.by((): ReturnType<Game['ledger']> => { void $gs.snapshot; return game.ledger(account); });
  const page = $derived<LedgerPage | null>(res.ok ? res.value : null);
  const rec = $derived(page ? reconcile(page) : null);
  const rows = $derived(page ? ledgerRows(page, snap, filter) : []);
  const slice = $derived(paginate(rows, pageNo, SIZE));
  const totals = $derived(rowTotals(rows));
  const groups = $derived(page ? rollupGroups(page, snap, filter) : []);
  const gtot = $derived(rollupTotals(groups));
  const kinds = $derived(page ? kindsIn(page) : []);
  const cps = $derived(page ? counterpartiesIn(page) : []);
  const turns = $derived(page ? turnsIn(page) : []);
  const cpKey = $derived(filter.counterparty ? accountKey(filter.counterparty) : '');
  const label = (a: AccountRef) => accountLabel(snap, a);
  const filtered = $derived(filter.kind !== 'all' || filter.fromTurn !== null || filter.toTurn !== null || filter.counterparty !== null);

  function setCp(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    filter = { ...filter, counterparty: v ? cps.find((c) => accountKey(c) === v) ?? null : null };
    pageNo = 0;
  }
  const setTurn = (which: 'fromTurn' | 'toTurn') => (e: Event) => {
    const v = (e.currentTarget as HTMLSelectElement).value;
    filter = { ...filter, [which]: v === '' ? null : Number(v) };
    pageNo = 0;
  };
  function setKind(e: Event) { filter = { ...filter, kind: (e.currentTarget as HTMLSelectElement).value as LedgerFilter['kind'] }; pageNo = 0; }
  function onKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement).tagName === 'SELECT') return;
    if (e.key === 'PageDown') { e.preventDefault(); pageNo = Math.min(slice.pages - 1, slice.page + 1); }
    else if (e.key === 'PageUp') { e.preventDefault(); pageNo = Math.max(0, slice.page - 1); }
  }
</script>

{#if !res.ok}
  <p class="err" role="alert">{res.error.code === 'unsupported' ? 'This simulation package has no transaction ledger yet. Rebuild it (npm run build:wasm) or use the mock.' : res.error.message}</p>
{:else if page && rec}
  <div class="top">
    <div class="who"><b>{label(account)}</b><span class="dim">&nbsp;ledger. Money in is green (+), money out is red (-).</span></div>
    <div class="badge" class:good={rec.ok} class:bad={!rec.ok} role="status" data-testid="reconciliation" title="Opening balance + every entry (detail and rolled-up) = closing balance">
      <span aria-hidden="true">{rec.ok ? '✓' : '✗'}</span> {rec.ok ? 'Reconciled' : 'DOES NOT RECONCILE'}
    </div>
  </div>
  <p class="sum" data-testid="reconciliation-sum">
    Opening {exact(rec.opening)} {rec.entries >= 0 ? '+' : '-'} entries {exact(Math.abs(rec.entries))} {rec.rolled >= 0 ? '+' : '-'} older periods {exact(Math.abs(rec.rolled))} = <b>{exact(rec.opening + rec.entries + rec.rolled)}</b>; closing balance <b>{exact(rec.closing)}</b>{#if !rec.ok} (difference {exact(rec.diff)}){/if}.
  </p>

  <form class="filters" aria-label="Filter transactions" onsubmit={(e) => e.preventDefault()}>
    <label>Kind
      <select value={filter.kind} onchange={setKind}><option value="all">All kinds</option>{#each kinds as k (k)}<option value={k}>{KIND_LABEL[k]}</option>{/each}</select></label>
    <label>From turn
      <select value={filter.fromTurn ?? ''} onchange={setTurn('fromTurn')}><option value="">Start</option>{#each turns as t (t)}<option value={t}>{snap ? monthLabel(t, 0, snap.game) : t}</option>{/each}</select></label>
    <label>To turn
      <select value={filter.toTurn ?? ''} onchange={setTurn('toTurn')}><option value="">Now</option>{#each turns as t (t)}<option value={t}>{snap ? monthLabel(t, 2, snap.game) : t}</option>{/each}</select></label>
    <label>Counterparty
      <select value={cpKey} onchange={setCp}><option value="">Everyone</option>{#each cps as c (accountKey(c))}<option value={accountKey(c)}>{label(c)}</option>{/each}</select></label>
    {#if filtered}<button type="button" class="clear" onclick={() => { filter = { ...NO_FILTER }; pageNo = 0; }}>Clear filters</button>{/if}
  </form>

  <h3>Transactions, last {page.entries.length ? `turns from ${snap ? monthLabel(page.detailFromTurn, 0, snap.game) : page.detailFromTurn}` : 'few turns'} <span class="dim">(newest first; {rows.length} shown)</span></h3>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex -->
  <div class="scroll" role="region" aria-label="Transactions table" tabindex="0" onkeydown={onKey}>
    <table>
      <thead><tr><th scope="col">Date</th><th scope="col">Counterparty</th><th scope="col">Kind</th><th scope="col">Detail</th><th scope="col">Memo</th><th scope="col" class="n">Amount</th><th scope="col" class="n">Balance</th></tr></thead>
      <tbody>
        {#each slice.items as r (r.key)}
          <tr>
            <td class="date" title="Turn {r.turn}, month {r.month + 1}">{r.date}</td>
            <td>{#if r.selectable}<button type="button" class="lnk" onclick={() => onselect(r.counterparty)} aria-label="Open {r.counterpartyLabel}">{r.counterpartyLabel}</button>{:else}<span class="ext" title="An account outside the town (no ledger)">{r.counterpartyLabel}</span>{/if}</td>
            <td>{r.kindLabel}</td><td class="dim">{r.detail}</td><td class="memo">{r.memo}</td>
            <td class="n amt" class:pos={r.amount >= 0} class:neg={r.amount < 0}>{r.amountText}</td><td class="n">{r.balanceText}</td>
          </tr>
        {:else}
          <tr><td colspan="7" class="empty">{page.entries.length ? 'No transactions match these filters.' : 'No transactions yet. They appear once a month has been played.'}</td></tr>
        {/each}
      </tbody>
      {#if rows.length}
        <tfoot><tr><td colspan="5">Total of the {rows.length} shown ({totals.count} entries): in {exact(totals.inflow)}, out {exact(totals.outflow)}</td><td class="n amt" class:pos={totals.net >= 0} class:neg={totals.net < 0}>{signedExact(totals.net)}</td><td></td></tr></tfoot>
      {/if}
    </table>
  </div>
  {#if slice.pages > 1}
    <nav class="pager" aria-label="Pages">
      <button type="button" onclick={() => (pageNo = slice.page - 1)} disabled={slice.page === 0}>Previous</button>
      <span>Rows {slice.from} to {slice.to} of {slice.total} (page {slice.page + 1} of {slice.pages})</span>
      <button type="button" onclick={() => (pageNo = slice.page + 1)} disabled={slice.page >= slice.pages - 1}>Next</button>
    </nav>
  {/if}

  <h3>Older periods, rolled up <span class="dim">(before {snap ? monthLabel(page.detailFromTurn, 0, snap.game) : `turn ${page.detailFromTurn}`}: monthly totals by counterparty and kind)</span></h3>
  {#if groups.length}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="scroll short" role="region" aria-label="Rolled-up older periods" tabindex="0">
      <table data-testid="rollup">
        <thead><tr><th scope="col">Counterparty</th><th scope="col">Kind</th><th scope="col">Turns</th><th scope="col" class="n">Entries</th><th scope="col" class="n">In</th><th scope="col" class="n">Out</th><th scope="col" class="n">Net</th></tr></thead>
        <tbody>
          {#each groups as g (accountKey(g.counterparty) + g.kind)}
            <tr>
              <td>{#if accountSelection(g.counterparty) && !sameAccount(g.counterparty, account)}<button type="button" class="lnk" onclick={() => onselect(g.counterparty)}>{g.counterpartyLabel}</button>{:else}{g.counterpartyLabel}{/if}</td>
              <td>{g.kindLabel}</td><td class="dim">{g.fromTurn === g.toTurn ? g.fromTurn : `${g.fromTurn} to ${g.toTurn}`}</td><td class="n">{g.count}</td>
              <td class="n">{exact(g.inflow)}</td><td class="n">{exact(g.outflow)}</td><td class="n amt" class:pos={g.net >= 0} class:neg={g.net < 0}>{signedExact(g.net)}</td>
            </tr>
          {/each}
        </tbody>
        <tfoot><tr><td colspan="3">Total of older periods</td><td class="n">{gtot.count}</td><td class="n">{exact(gtot.inflow)}</td><td class="n">{exact(gtot.outflow)}</td><td class="n amt" class:pos={gtot.net >= 0} class:neg={gtot.net < 0}>{signedExact(gtot.net)}</td></tr></tfoot>
      </table>
    </div>
  {:else}
    <p class="dim">{page.summary.length ? 'No older periods match these filters.' : 'Nothing is rolled up yet: the game keeps full detail for the last few turns and summarises older ones.'}</p>
  {/if}
{/if}

<style>
  .top { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .dim { color: var(--text-dim); font-size: 12px; }
  .badge { padding: 2px 10px; border-radius: 999px; font-weight: 700; font-size: 12px; border: 1px solid; }
  .badge.good { color: var(--good); background: var(--good-bg); border-color: var(--good); }
  .badge.bad { color: var(--bad); background: var(--bad-bg); border-color: var(--bad); }
  .sum { margin: 6px 0; font-size: 12px; font-family: var(--font-mono); }
  .filters { display: flex; gap: 10px; flex-wrap: wrap; align-items: end; margin: 6px 0 8px; }
  label { display: flex; flex-direction: column; font-size: 11px; color: var(--text-dim); gap: 2px; }
  select { font: inherit; font-size: 12px; background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); padding: 2px 4px; max-width: 160px; }
  .clear, .pager button { font: inherit; font-size: 12px; background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); padding: 3px 8px; cursor: pointer; }
  .pager button:disabled { opacity: 0.5; cursor: default; }
  h3 { margin: 10px 0 4px; }
  .scroll { overflow: auto; max-height: 340px; border: 1px solid var(--border); border-radius: var(--radius); }
  .scroll.short { max-height: 200px; }
  table { border-collapse: collapse; width: 100%; font-size: 12px; }
  th, td { text-align: left; padding: 3px 8px; border-bottom: 1px solid var(--border); white-space: nowrap; }
  th { position: sticky; top: 0; background: var(--surface-2); font-weight: 600; z-index: 1; }
  td.memo { white-space: normal; min-width: 160px; }
  .n { text-align: right; font-family: var(--font-mono); }
  .date { color: var(--text-dim); }
  .amt.pos { color: var(--good); } .amt.neg { color: var(--bad); }
  tfoot td { font-weight: 600; background: var(--surface-2); position: sticky; bottom: 0; }
  .empty { text-align: center; color: var(--text-dim); padding: 14px; }
  .lnk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; }
  .ext { color: var(--text-dim); font-style: italic; }
  .pager { display: flex; align-items: center; gap: 10px; margin: 6px 0; font-size: 12px; }
  .err { color: var(--bad); }
</style>
