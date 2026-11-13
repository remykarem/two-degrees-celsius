<script lang="ts">
  import { openBreakdown, openExplorer } from './ui';
  import type { BreakdownKey } from './metrics';
  import CausesButton from './CausesButton.svelte';

  /** The small "how?" affordance beside a metric: opens the wire's breakdown when the metric has one, else the explorer topic for the anchor. */
  let { breakdown = undefined, anchor, label }: { breakdown?: BreakdownKey | undefined; anchor: string; label: string } = $props();
</script>

<button type="button" class="how" data-testid="how-{anchor}" aria-haspopup="dialog" aria-label="How is {label} calculated?" title="How is {label} calculated?"
  onclick={() => (breakdown ? openBreakdown(breakdown) : openExplorer(anchor))}>how?</button><CausesButton {anchor} {label} />

<style>
  .how { border: 1px solid var(--border); background: var(--surface); color: var(--accent); border-radius: 9px; padding: 0 6px; font: inherit; font-size: 10.5px; cursor: pointer; line-height: 15px; white-space: nowrap; }
  .how:hover { background: var(--surface-2); }
</style>