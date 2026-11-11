<script lang="ts">
  import type { Game } from '../store';
  import { Modal } from '../ui-kit';
  import { breakdownOf, PRESSURE_NOTE } from './explain';
  import BreakdownTable from './BreakdownTable.svelte';
  import CapitalView from './CapitalView.svelte';
  import EmissionsPanel from './EmissionsPanel.svelte';
  import GiniView from './GiniView.svelte';
  import HowLink from './HowLink.svelte';
  import LegitimacyView from './LegitimacyView.svelte';
  import TaxesTable from './TaxesTable.svelte';
  import { ANCHOR_OF, TITLE_OF } from './breakdownMeta';
  import { setUi, ui } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const key = $derived($ui.breakdown);
  const snap = $derived($gs.snapshot);
  const plain = $derived(key && snap ? breakdownOf(snap, key) : null);
  const close = () => setUi({ breakdown: null });
</script>

{#if key && snap}
  <Modal title={TITLE_OF[key]} size="md" onclose={close}>
    {#if !snap.breakdowns}
      <p>The simulation did not send breakdowns (a package older than contract v1.6). Open the explorer for the general rule.</p>
    {:else if key === 'gini'}<GiniView {snap} />
    {:else if key === 'legitimacy'}<LegitimacyView {snap} />
    {:else if key === 'emissions'}<EmissionsPanel {snap} />
    {:else if key === 'taxes'}<TaxesTable {snap} />
    {:else if key === 'capitalRatio'}<CapitalView {snap} />
    {:else if plain}<BreakdownTable b={plain} />
    {:else}<p>No breakdown for this figure.</p>{/if}
    {#snippet footer()}
      <span class="note" data-testid="pressure-note">{$gs.mode === 'pressure' ? PRESSURE_NOTE : ''}</span>
      <HowLink anchor={ANCHOR_OF[key]} text="How is this calculated? (model explorer)" />
    {/snippet}
  </Modal>
{/if}
