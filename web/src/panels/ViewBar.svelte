<script lang="ts">
  import { LENSES, type Game, type Lens } from '../store';
  import { Button } from '../ui-kit';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
</script>

<div class="bar" role="toolbar" aria-label="Map view">
  <Button title="Rotate left (Q)" onclick={() => game.rotate(-1)}>⟲ Q</Button>
  <Button title="Rotate right (E)" onclick={() => game.rotate(1)}>E ⟳</Button>
  <span class="angle">View {$gs.camera + 1}/4</span>
  <label>Lens
    <select value={$gs.lens} onchange={(e) => game.setLens((e.currentTarget as HTMLSelectElement).value as Lens)}>
      {#each LENSES as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
    </select>
  </label>
</div>

<style>
  .bar { position: absolute; left: 50%; bottom: 10px; transform: translateX(-50%); display: flex; gap: 8px; align-items: center; z-index: 10;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 4px 8px; box-shadow: var(--shadow); }
  .angle { color: var(--text-dim); font-size: 12px; }
</style>
