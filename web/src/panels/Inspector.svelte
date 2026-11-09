<script lang="ts">
  import { occupant, type Game, type Scenery } from '../store';
  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const sel = $derived($gs.selection);
  const what = $derived(sel ? occupant($gs.layout, sel) : null);
  const label = $derived(what ? ('label' in what ? (what as Scenery).label : what.kind.replaceAll('_', ' ')) : 'Empty tile');
  const hoverErr = $derived($gs.buildKind && $gs.hover ? game.tileError($gs.hover) : null);
</script>

{#if hoverErr}
  <div class="tip" role="status">Cannot build at ({$gs.hover?.x},{$gs.hover?.y}): {hoverErr}</div>
{/if}
{#if sel}
  <aside aria-label="Inspector">
    <h3>Tile ({sel.x},{sel.y})</h3>
    <div><b>{label}</b></div>
    <p class="dim">Read-only. Position is cosmetic: it has no effect on the economy.</p>
    <button onclick={() => game.select(null)}>Close</button>
  </aside>
{/if}

<style>
  aside { position: absolute; right: 12px; bottom: 56px; width: 220px; z-index: 10; background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius); box-shadow: var(--shadow); padding: 8px 10px; }
  .tip { position: absolute; left: 50%; top: 10px; transform: translateX(-50%); z-index: 12; background: var(--text); color: var(--bg); padding: 4px 10px; border-radius: var(--radius); }
  .dim { color: var(--text-dim); font-size: 12px; }
  button { font: inherit; }
</style>
