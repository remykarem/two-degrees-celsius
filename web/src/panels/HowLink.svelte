<script lang="ts">
  import { openExplorer } from './ui';
  /** "How is this calculated?": opens the explorer on the topic that lists `anchor` in its usedBy. */
  let { anchor, text = 'How is this calculated?' }: { anchor: string; text?: string } = $props();
</script>

<button type="button" class="hl" data-testid="howlink-{anchor}" onclick={() => openExplorer(anchor)}>{text}</button>

<style>
  .hl { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; font-size: 11px; }
</style>
