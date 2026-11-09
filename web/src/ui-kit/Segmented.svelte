<script lang="ts">
  let { label, options, value, disabled = false, onchange }: {
    label: string; options: { value: string; label: string }[]; value: string; disabled?: boolean; onchange: (v: string) => void;
  } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as o (o.value)}
    <button type="button" role="radio" aria-checked={value === o.value} class:on={value === o.value} {disabled} onclick={() => onchange(o.value)}>
      {#if value === o.value}<span aria-hidden="true">✓ </span>{/if}{o.label}
    </button>
  {/each}
</div>

<style>
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  button { border: 0; border-right: 1px solid var(--border); background: var(--surface); color: var(--text); padding: 4px 12px; cursor: pointer; }
  button:last-child { border-right: 0; }
  button.on { background: var(--accent); color: var(--accent-text); font-weight: 600; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
