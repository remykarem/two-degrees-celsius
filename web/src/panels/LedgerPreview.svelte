<script lang="ts">
  import type { AccountRef } from '../contract';
  import type { Game } from '../store';
  import { exact, ledgerRows, NO_FILTER, firmRevenueNote, positionNames, reconcile } from './ledger';

  let { game, account }: { game: Game; account: AccountRef } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const res = $derived.by((): ReturnType<Game['ledger']> => { void $gs.snapshot; return game.ledger(account); });
  const page = $derived(res.ok ? res.value : null);
  const rec = $derived(page ? reconcile(page) : null);
  const names = $derived(positionNames(account));
  const rows = $derived(page ? ledgerRows(page, $gs.snapshot, NO_FILTER).slice(0, 5) : []);
</script>

<section class="tx" aria-label="Transactions">
  <h4>Transactions</h4>
  {#if !res.ok}
    <p class="dim">{res.error.code === 'unsupported' ? 'This simulation package has no ledger yet.' : res.error.message}</p>
  {:else if page && rec}
    <p class="bal"><span class="badge" class:good={rec.ok} class:bad={!rec.ok} data-testid="reconciliation-badge"><span aria-hidden="true">{rec.ok ? '✓' : '✗'}</span> {rec.ok ? 'Reconciled' : 'Does not reconcile'}</span></p>
    <dl><dt>{names.balance}, opening (turn 0)</dt><dd>{exact(rec.opening)}</dd><dt>{names.balance}, now</dt><dd>{exact(rec.closing)}</dd>{#if rec.openingDebt > 0 || rec.closingDebt > 0}<dt>{names.debt}, opening</dt><dd>{exact(rec.openingDebt)}</dd><dt>{names.debt}, now</dt><dd>{exact(rec.closingDebt)}</dd>{/if}</dl>
    {#if account.kind === 'firm'}<p class="dim" data-testid="firm-revenue-note">{firmRevenueNote()}</p>{/if}
    {#if rows.length}
      <ul class="last" aria-label="Latest transactions">
        {#each rows as r (r.key)}
          <li><span class="m" title={r.memo}>{r.memo}</span><span class="a" class:pos={r.amount >= 0} class:neg={r.amount < 0}>{r.amountText}</span></li>
        {/each}
      </ul>
    {:else}<p class="dim">No transactions yet.</p>{/if}
    <button type="button" class="open" onclick={() => game.openLedger(account)}>Open full ledger ({page.entries.length} recent entries{page.summary.length ? ', older periods rolled up' : ''})</button>
  {/if}
</section>

<style>
  h4 { margin: 8px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  .dim { font-size: 12px; color: var(--text-dim); margin: 2px 0; }
  .bal { margin: 2px 0; }
  .badge { display: inline-block; padding: 0 8px; border-radius: 999px; font-size: 11px; font-weight: 700; border: 1px solid; }
  .badge.good { color: var(--good); background: var(--good-bg); border-color: var(--good); }
  .badge.bad { color: var(--bad); background: var(--bad-bg); border-color: var(--bad); }
  dl { display: grid; grid-template-columns: 1fr auto; gap: 1px 10px; margin: 0; font-size: 12px; }
  dt { color: var(--text-dim); } dd { margin: 0; text-align: right; font-family: var(--font-mono); }
  .last { list-style: none; margin: 4px 0; padding: 0; font-size: 12px; }
  .last li { display: flex; justify-content: space-between; gap: 6px; border-top: 1px dotted var(--border); padding: 1px 0; }
  .m { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a { font-family: var(--font-mono); white-space: nowrap; }
  .pos { color: var(--good); } .neg { color: var(--bad); }
  .open { margin-top: 4px; font: inherit; font-size: 12px; border: 1px solid var(--accent); background: var(--surface); color: var(--accent); border-radius: var(--radius); padding: 3px 8px; cursor: pointer; width: 100%; }
  .open:hover { background: var(--surface-2); }
</style>
