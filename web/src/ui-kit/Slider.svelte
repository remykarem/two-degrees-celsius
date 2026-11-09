<script lang="ts">
  let { id, label, value, min, max, step, disabled = false, format = (v: number) => String(v), onchange }: {
    id: string; label: string; value: number; min: number; max: number; step: number; disabled?: boolean;
    format?: (v: number) => string; onchange: (v: number) => void;
  } = $props();
</script>

<div class="slider">
  <label for={id}>{label}</label>
  <input {id} type="range" {min} {max} {step} {value} {disabled}
    aria-valuetext={format(value)}
    oninput={(e) => onchange(Number((e.currentTarget as HTMLInputElement).value))} />
  <output for={id}>{format(value)}</output>
</div>

<style>
  .slider { display: grid; grid-template-columns: 1fr auto; gap: 2px 8px; align-items: center; }
  label { grid-column: 1; font-weight: 600; }
  output { grid-column: 2; grid-row: 1; font-family: var(--font-mono); }
  input { grid-column: 1 / span 2; width: 100%; }
</style>
