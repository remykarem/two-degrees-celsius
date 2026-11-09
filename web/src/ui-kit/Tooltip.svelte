<script lang="ts">
  import type { Snippet } from 'svelte';
  let { text, children }: { text: string; children: Snippet } = $props();
</script>

<span class="tip">{@render children()}<span class="bubble" role="tooltip">{text}</span></span>

<style>
  .tip { position: relative; display: inline-flex; }
  .bubble { position: absolute; left: 0; top: 100%; z-index: 50; margin-top: 4px; padding: 4px 8px; width: max-content; max-width: 260px;
    background: var(--text); color: var(--bg); border-radius: var(--radius); font-size: 12px; display: none; pointer-events: none; }
  .tip:hover .bubble, .tip:focus-within .bubble { display: block; }
</style>
