<script lang="ts">
  import { LENSES, sceneBus, type Game, type Lens } from '../store';
  import { Button } from '../ui-kit';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
</script>

<div class="bar" role="toolbar" aria-label="Map view">
  <Button title="Rotate left (Q)" onclick={() => game.rotate(-1)}>⟲ Q</Button>
  <Button title="Rotate right (E)" onclick={() => game.rotate(1)}>E ⟳</Button>
  <span class="angle">View {$gs.camera + 1}/4</span>
  <span class="zoom" role="group" aria-label="Zoom">
    <Button title="Zoom out (minus key on the map)" onclick={() => sceneBus.requestZoom(0.8)}><span aria-hidden="true">−</span><span class="sr-only">Zoom out</span></Button>
    <Button title="Zoom in (plus key on the map)" onclick={() => sceneBus.requestZoom(1.25)}><span aria-hidden="true">+</span><span class="sr-only">Zoom in</span></Button>
    <Button title="Reset zoom, pan and view (0 on the map)" onclick={() => sceneBus.requestReset()}>Reset view</Button>
  </span>
  <label>Lens
    <select value={$gs.lens} onchange={(e) => game.setLens((e.currentTarget as HTMLSelectElement).value as Lens)}>
      {#each LENSES as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
    </select>
  </label>
</div>

<style>
  .bar { position: absolute; left: 50%; bottom: 10px; transform: translateX(-50%); display: flex; gap: 8px; align-items: center; z-index: 10;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 4px 8px; box-shadow: var(--shadow); }
  .zoom { display: inline-flex; gap: 4px; }
  .angle { color: var(--text-dim); font-size: 12px; }
</style>
