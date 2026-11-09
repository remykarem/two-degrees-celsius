<script lang="ts">
  import { paletteActions, totalPc, type Game } from '../store';
  import { Button } from '../ui-kit';
  import { costLine } from './costs';
  import { leadText, money } from './format';
  import { builtCount } from './inspect';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const locked = $derived($gs.phase === 'playback' || $gs.phase === 'results' || $gs.phase === 'ended');
  const pcFree = $derived(($gs.snapshot?.pc.current ?? 0) - totalPc($gs.draft));
  const items = $derived(paletteActions($gs.actions));
  // Price from the simulation's preview() (a default valid tile, or the hovered tile while building); the action table is only a labelled fallback.
  const hoverOk = $derived($gs.hover && $gs.buildKind && !game.tileError($gs.hover) ? $gs.hover : null);
  const estimates = $derived.by(() => {
    void $gs.snapshot?.turn; void $gs.draft;
    return Object.fromEntries(items.map((i) => [i.kind, game.estimateBuild(i.kind, $gs.buildKind === i.kind ? hoverOk ?? undefined : undefined)]));
  });
  // Building costs have no lobbying opposition: the cost model is exact for one building (preview() still has the last word).
  const reasonFor = (cost: number) => (locked ? null : cost > pcFree + 1e-9 ? `Needs about ${Math.round(cost * 10) / 10} PC; ${Math.floor(pcFree)} free` : null);
  const planned = (kind: string) => $gs.draft.filter((d) => d.command.type === 'place' && d.command.kind === kind).length;
</script>

<aside aria-label="Build palette" data-coach="build">
  <h2>Build</h2>
  {#each items as item (item.kind)}
    {@const line = costLine(item.action, estimates[item.kind] ?? null)}
    {@const why = reasonFor(line.pc)}
    <div class="item">
      <Button pressed={$gs.buildKind === item.kind} disabled={locked || !!why} title={why ?? ''}
        onclick={() => game.setBuildKind($gs.buildKind === item.kind ? null : item.kind)}>{item.action.label}</Button>
      <div class="cost" title={line.exact ? 'Price from the simulation (preview), for a free tile' : 'Estimate from the action table; the simulation confirms the price once a tile is chosen'}>
        {line.exact ? '' : 'Est. '}{Math.round(line.pc * 10) / 10} PC · {line.money > 0 ? `${money(line.money)} · ` : ''}ready {leadText(line.leadTime)}{line.exact ? '' : ' (estimate)'}
      </div>
      <div class="cost">Built {builtCount($gs.snapshot, item.kind)}{#if planned(item.kind)}&nbsp;· planned {planned(item.kind)}{/if}</div>
      {#if why}<div class="off">{why}</div>{/if}
    </div>
  {/each}
  <p class="hint">
    {#if $gs.buildKind === 'road'}Click free ground to lay a road tile. Commutes that follow roads are faster, so connect homes to workplaces. Esc to stop.{:else if $gs.buildKind}Free tiles are shaded green; click one to place it. Prices are the simulation's own. Esc to stop.{:else}Pick a building, then click the map.{/if}
  </p>

  <p class="hint">To remove a road, plant or public housing, click it on the map and press Remove in the panel. Firms, private homes and forest cannot be removed.</p>
</aside>

<style>
  aside { display: flex; flex-direction: column; gap: 6px; padding: 10px; background: var(--surface); border-right: 1px solid var(--border); overflow: auto; min-width: 0; }
  .item { display: flex; flex-direction: column; gap: 1px; margin-bottom: 4px; }
  .cost { font-size: 11px; color: var(--text-dim); }
  .off { font-size: 11px; color: var(--warn); }
  .hint { color: var(--text-dim); font-size: 12px; margin: 4px 0; }
</style>
