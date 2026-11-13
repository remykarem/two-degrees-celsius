<script lang="ts">
  import { hasCatalog, fill, text } from '../store/catalog';
  import { catalog } from '../store/catalog';
  import { entryQuantity, inGraph } from './causes';
  import { openCauses } from './ui';

  /** The small "why?" affordance beside a figure: opens the Why? view of the quantity the wire names for `anchor` (a metric, a lever or a market id), or of `quantity` directly. Absent when the quantity is not in the catalogue. */
  let { anchor = undefined, quantity = undefined, label }: { anchor?: string | undefined; quantity?: string | null | undefined; label: string } = $props();
  const q = $derived(hasCatalog() ? (quantity ?? (anchor ? entryQuantity(catalog().causal, anchor) : null)) : null);
  const shown = $derived(q && inGraph(catalog().causal, q) ? q : null);
</script>

{#if shown}
  <button type="button" class="why" data-testid="why-{anchor ?? shown}" aria-haspopup="dialog" aria-label={fill(text('causes', 'button_aria'), { quantity: label })} title={fill(text('causes', 'button_aria'), { quantity: label })}
    onclick={() => openCauses(shown)}>{text('causes', 'button')}</button>
{/if}

<style>
  .why { border: 1px solid var(--border); background: var(--surface); color: var(--accent); border-radius: 9px; padding: 0 6px; font: inherit; font-size: 10.5px; cursor: pointer; line-height: 15px; white-space: nowrap; }
  .why:hover { background: var(--surface-2); }
</style>
