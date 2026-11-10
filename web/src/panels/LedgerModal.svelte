<script lang="ts">
  import type { AccountRef } from '../contract';
  import type { Game } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { accountLabel, accountSelection } from './ledger';
  import LedgerTable from './LedgerTable.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const view = $derived($gs.ledgerView);
  const close = () => game.openLedger(null);
  /** Clicking a counterparty switches the open ledger to that account and selects it in the inspector. */
  function go(a: AccountRef) {
    const sel = accountSelection(a);
    if (sel) game.select(sel);
    game.openLedger(a);
  }
</script>

{#if view}
  <Modal title="Transactions: {accountLabel($gs.snapshot, view.account)}" size="lg" onclose={close}>
    <LedgerTable {game} account={view.account} kind={view.kind} onselect={go} />
    {#snippet footer()}<Button variant="primary" onclick={close}>Close</Button>{/snippet}
  </Modal>
{/if}
