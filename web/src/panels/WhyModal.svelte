<script lang="ts">
  import type { Game } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { setUi, ui } from './ui';
  import { text } from '../store/catalog';
  import WhyView from './WhyView.svelte';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const close = () => setUi({ why: null });
</script>

{#if $ui.why}
  <Modal title={text('misc', 'what_changed_title')} size="md" onclose={close}>
    <WhyView kind={$ui.why.kind} trace={$gs.lastResult?.trace ?? []} snap={$gs.snapshot} levers={$gs.levers} />
    {#if !$gs.lastResult}<p class="dim">{text('misc', 'what_changed_empty')}</p>{/if}
    {#snippet footer()}<Button variant="primary" onclick={close}>Close</Button>{/snippet}
  </Modal>
{/if}

<style>.dim { color: var(--text-dim); font-size: 12px; }</style>
