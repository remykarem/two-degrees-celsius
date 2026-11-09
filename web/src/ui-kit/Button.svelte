<script lang="ts">
  import type { Snippet } from 'svelte';
  let { variant = 'normal', disabled = false, title = '', pressed, onclick, children }: {
    variant?: 'normal' | 'primary' | 'ghost'; disabled?: boolean; title?: string; pressed?: boolean;
    onclick?: (e: MouseEvent) => void; children: Snippet;
  } = $props();
</script>

<button type="button" class={variant} {disabled} {title} aria-pressed={pressed} {onclick}>{@render children()}</button>

<style>
  button { font: inherit; padding: 6px 12px; border-radius: var(--radius); border: 1px solid var(--border);
    background: var(--surface); color: var(--text); cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-2); }
  button.primary { background: var(--accent); color: var(--accent-text); border-color: var(--accent); font-weight: 600; }
  button.primary:hover:not(:disabled) { filter: brightness(1.1); background: var(--accent); }
  button.ghost { border-color: transparent; background: transparent; }
  button[aria-pressed='true'] { outline: 2px solid var(--accent); }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
