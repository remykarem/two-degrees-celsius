<script lang="ts">
  import type { Game } from '../store';
  import { fill, text } from '../store/catalog';
  import { Button, Modal } from '../ui-kit';
  import TradesTable from './TradesTable.svelte';

  /** The "See all trades" window of a market (open while `tradesView` is set). The title is the wire's market label. */
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const view = $derived($gs.tradesView);
  const label = $derived(view ? ($gs.snapshot?.markets.find((x) => x.id === view.market)?.label ?? view.market) : '');
  const close = () => game.openTrades(null);
</script>

{#if view}
  <Modal title={fill(text('markets', 'trades_title'), { market: label })} size="lg" onclose={close}>
    <TradesTable {game} market={view.market} entity={view.entity} />
    {#snippet footer()}<Button variant="primary" onclick={close}>{text('markets', 'trades_close')}</Button>{/snippet}
  </Modal>
{/if}
