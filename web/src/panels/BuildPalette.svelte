<script lang="ts">
  import { paletteActions, totalPc, type Game } from '../store';
  import { Button } from '../ui-kit';
  import { actionModel, estimatePc } from './costs';
  import { leadText, money } from './format';
  import { builtCount, INFRA_LABEL } from './inspect';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const locked = $derived($gs.phase === 'playback' || $gs.phase === 'results' || $gs.phase === 'ended');
  const pcFree = $derived(($gs.snapshot?.pc.current ?? 0) - totalPc($gs.draft));
  const items = $derived(paletteActions($gs.actions));
  // Building costs have no lobbying opposition: the cost model is exact for one building (preview() still has the last word).
  const reasonFor = (cost: number) => (locked ? null : cost > pcFree + 1e-9 ? `Needs about ${Math.round(cost * 10) / 10} PC; ${Math.floor(pcFree)} free` : null);
  const planned = (kind: string) => $gs.draft.filter((d) => d.command.type === 'place' && d.command.kind === kind).length;
  const removing = (kind: string) => $gs.draft.some((d) => d.command.type === 'remove' && d.command.kind === kind);
  const removeCost = (kind: string) => $gs.actions.find((a) => a.key === `remove:${kind}`);

  function toggleRemove(kind: (typeof items)[number]['kind']) {
    const d = $gs.draft.find((x) => x.command.type === 'remove' && x.command.kind === kind);
    if (d) game.removeDraft(d.id);
    else game.draftCommand({ type: 'remove', kind, count: 1 });
  }
</script>

<aside aria-label="Build palette" data-coach="build">
  <h2>Build</h2>
  {#each items as item (item.kind)}
    {@const why = reasonFor(estimatePc(actionModel(item.action), 1))}
    <div class="item">
      <Button pressed={$gs.buildKind === item.kind} disabled={locked || !!why} title={why ?? ''}
        onclick={() => game.setBuildKind($gs.buildKind === item.kind ? null : item.kind)}>{item.action.label}</Button>
      <div class="cost">About {Math.round(estimatePc(actionModel(item.action), 1) * 10) / 10} PC · {item.action.moneyCost > 0 ? `about ${money(item.action.moneyCost)} · ` : ''}ready {leadText(item.action.leadTime)}</div>
      <div class="cost">Built {builtCount($gs.snapshot, item.kind)}{#if planned(item.kind)}&nbsp;· planned {planned(item.kind)}{/if}</div>
      {#if why}<div class="off">{why}</div>{/if}
    </div>
  {/each}
  <p class="hint">
    {#if $gs.buildKind}Click a free tile on the map to place it. Esc to stop.{:else}Pick a building, then click the map.{/if}
  </p>

  <h3>Remove</h3>
  {#each items as item (item.kind)}
    {@const n = builtCount($gs.snapshot, item.kind)}
    {#if n > 0 || removing(item.kind)}
      <div class="rm">
        <span>{INFRA_LABEL[item.kind]} ×{n}</span>
        <Button disabled={locked} pressed={removing(item.kind)} title={removeCost(item.kind) ? `About ${removeCost(item.kind)!.baseCostPc} PC` : ''}
          onclick={() => toggleRemove(item.kind)}>{removing(item.kind) ? 'Undo' : 'Remove one'}</Button>
      </div>
    {/if}
  {/each}
  {#if $gs.snapshot && items.every((i) => builtCount($gs.snapshot, i.kind) < 1)}<p class="hint">Nothing built yet, so there is nothing to remove.</p>{/if}
  <p class="hint">You can also select a building on the map to remove it. Firms, homes and forest are fixed. Tile position is cosmetic: it does not change the economy.</p>
</aside>

<style>
  aside { display: flex; flex-direction: column; gap: 6px; padding: 10px; background: var(--surface); border-right: 1px solid var(--border); overflow: auto; min-width: 0; }
  .item { display: flex; flex-direction: column; gap: 1px; margin-bottom: 4px; }
  .cost { font-size: 11px; color: var(--text-dim); }
  .off { font-size: 11px; color: var(--warn); }
  .rm { display: flex; flex-direction: column; gap: 2px; font-size: 12px; margin-bottom: 4px; }
  .hint { color: var(--text-dim); font-size: 12px; margin: 4px 0; }
  h3 { margin-top: 8px; }
</style>
