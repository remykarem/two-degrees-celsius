<script lang="ts">
  import type { Game } from '../store';
  import { Button } from '../ui-kit';
  import { INFRA_LABEL, resolveInspect } from './inspect';
  import { REASON_TEXT } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const pinned = $derived($gs.selection);
  const hovering = $derived(!pinned && !$gs.buildKind ? $gs.hover : null);
  const tile = $derived(pinned ?? hovering);
  const model = $derived($gs.snapshot && tile ? resolveInspect($gs.snapshot, $gs.layout, tile, $gs.draft) : null);
  const hoverErr = $derived($gs.buildKind && $gs.hover ? game.tileError($gs.hover) : null);
  const canEdit = $derived($gs.phase === 'review' || $gs.phase === 'plan');
  const maxIncome = $derived(Math.max(1, ...(model?.income ?? []).map((p) => p.value)));

  function queueRemoval() {
    if (model?.removable) game.draftCommand({ type: 'remove', kind: model.removable.kind, count: 1 });
  }
  function undoRemoval() {
    const d = $gs.draft.find((x) => x.command.type === 'remove' && model?.removable && x.command.kind === model.removable.kind);
    if (d) game.removeDraft(d.id);
  }
</script>

{#if hoverErr}
  <div class="tip" role="status">Cannot build at ({$gs.hover?.x},{$gs.hover?.y}): {hoverErr}</div>
{/if}
{#if model}
  <aside aria-label="Inspector" class:hover={!pinned}>
    <header>
      <h3>{model.title}</h3>
      {#if pinned}<button class="close" aria-label="Close inspector" onclick={() => game.select(null)}>✕</button>{:else}<span class="chip">hover</span>{/if}
    </header>
    <p class="sub">{model.subtitle}</p>
    <p class="ro">Read-only. Click a tile to keep this open.</p>

    {#each model.sections as sec (sec.heading)}
      <section>
        <h4>{sec.heading}</h4>
        <dl>{#each sec.rows as r (r.label)}<dt>{r.label}</dt><dd>{r.value}</dd>{/each}</dl>
      </section>
    {/each}

    {#if model.income}
      <section>
        <h4>Income (per month)</h4>
        <ul class="inc">
          {#each model.income as p (p.label)}
            <li><span>{p.label}</span><span class="track" aria-hidden="true"><span class="bar" style:width="{(p.value / maxIncome) * 100}%"></span></span><span class="v">{p.text}</span></li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if model.removable}
      <section class="rm">
        {#if model.removable.already}
          <p>Removal of one {INFRA_LABEL[model.removable.kind].toLowerCase()} is in your plan.</p>
          <Button onclick={undoRemoval} disabled={!canEdit}>Undo removal</Button>
        {:else}
          <Button onclick={queueRemoval} disabled={!canEdit || !!model.removable.reason}
            title={model.removable.reason ?? 'Queue removal of one of this kind'}>Remove {INFRA_LABEL[model.removable.kind].toLowerCase()}…</Button>
          {#if !canEdit}<p class="hint">Removal can be queued in Review or Plan.</p>{:else if model.removable.reason}<p class="hint">{REASON_TEXT.nothing_to_remove}</p>{/if}
        {/if}
      </section>
    {/if}
    {#if model.note}<p class="note">{model.note}</p>{/if}
  </aside>
{/if}

<style>
  aside { position: absolute; left: 12px; bottom: 56px; width: 264px; max-height: calc(100% - 70px); overflow: auto; z-index: 12; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow); padding: 8px 12px; }
  aside.hover { opacity: 0.96; pointer-events: none; }
  header { display: flex; justify-content: space-between; align-items: center; }
  h3 { font-size: 14px; text-transform: none; letter-spacing: 0; color: var(--text); }
  h4 { margin: 8px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  .close { border: 0; background: transparent; color: var(--text); cursor: pointer; font-size: 14px; }
  .chip { font-size: 11px; color: var(--text-dim); border: 1px solid var(--border); border-radius: 8px; padding: 0 6px; }
  .sub, .ro, .note, .hint { font-size: 12px; color: var(--text-dim); margin: 2px 0; }
  .ro { font-style: italic; }
  dl { display: grid; grid-template-columns: 1fr auto; gap: 1px 10px; margin: 0; font-size: 13px; }
  dt { color: var(--text-dim); } dd { margin: 0; text-align: right; font-family: var(--font-mono); }
  .inc { list-style: none; margin: 0; padding: 0; font-size: 12px; display: grid; gap: 2px; }
  .inc li { display: grid; grid-template-columns: 64px 1fr 56px; gap: 6px; align-items: center; }
  .track { height: 8px; background: var(--surface-2); border-radius: 2px; overflow: hidden; display: block; }
  .bar { display: block; height: 100%; background: var(--series-1); }
  .v { text-align: right; font-family: var(--font-mono); }
  .rm { margin-top: 8px; }
  .tip { position: absolute; left: 50%; top: 10px; transform: translateX(-50%); z-index: 12; background: var(--text); color: var(--bg); padding: 4px 10px; border-radius: var(--radius); }
</style>
