<script lang="ts">
  import HowButton from './HowButton.svelte';
  import { layoutOf, type Game, type Selection } from '../store';
  import { Button } from '../ui-kit';
  import { INFRA_LABEL, resolveSelection, resolveTile } from './inspect';
  import { agentsAnchorFor, decisionTag } from './explorer';
  import { openExplorer } from './ui';
  import { selectionAccount } from './ledger';
  import LedgerPreview from './LedgerPreview.svelte';
  import PersonEarnings from './PersonEarnings.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const pinned = $derived($gs.selection);
  const hovering = $derived(!pinned && !$gs.buildKind ? $gs.hover : null);
  const layout = $derived(layoutOf($gs.snapshot));
  const model = $derived(!$gs.snapshot ? null : pinned ? resolveSelection($gs.snapshot, layout, pinned, $gs.draft) : hovering ? resolveTile($gs.snapshot, layout, hovering, $gs.draft) : null);
  const hoverErr = $derived($gs.buildKind && $gs.hover ? game.tileError($gs.hover) : null);
  const canEdit = $derived($gs.phase === 'review' || $gs.phase === 'plan');
  const tag = $derived.by(() => {
    if (pinned?.kind !== 'person') return null;
    const r = game.agents();
    return decisionTag(r.ok ? r.value : null, $gs.snapshot, pinned.id);
  });
  const acct = $derived(pinned ? selectionAccount($gs.snapshot, pinned) : null);
  const maxIncome = $derived(Math.max(1, ...(model?.income ?? []).map((p) => p.value)));

  const open = (s: Selection) => game.select(s);
  function queueRemoval() {
    if (model?.removable) game.removeAt(model.removable.tile);
  }
  function undoRemoval() {
    const r = model?.removable;
    const d = $gs.draft.find((x) => r && x.command.type === 'remove' && x.command.tile.x === r.tile.x && x.command.tile.y === r.tile.y);
    if (d) game.removeDraft(d.id);
  }
</script>

{#if hoverErr}
  <div class="tip" role="status">Cannot build at ({$gs.hover?.x},{$gs.hover?.y}): {hoverErr}</div>
{/if}
{#if model}
  <aside aria-label="Inspector" class:hover={!pinned}>
    <header>
      <h3>{model.title} <HowButton anchor="inspector" label="these fields" /></h3>
      {#if pinned}<button class="close" aria-label="Close inspector" onclick={() => game.select(null)}>✕</button>{:else}<span class="chip">hover</span>{/if}
    </header>
    <p class="sub">{model.subtitle}</p>
    {#if !pinned}<p class="ro">Click a tile to keep this open.</p>{/if}

    {#each model.sections as sec (sec.heading)}
      <section>
        <h4>{sec.heading}</h4>
        <dl>{#each sec.rows as r (r.label)}<dt title={r.hint}>{r.label}</dt><dd>{r.value}</dd>{/each}</dl>
      </section>
    {/each}

    {#if model.household}
      <p class="link"><button class="lnk" onclick={() => open(model.household!.selection)}>{model.household.label} ({model.household.detail})</button></p>
    {/if}
    {#if model.members}
      <section>
        <h4>People</h4>
        <ul class="ppl">{#each model.members as m (m.label)}<li><button class="lnk" onclick={() => open(m.selection)}>{m.label}</button> <span>{m.detail}</span></li>{/each}</ul>
      </section>
    {/if}

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

    {#if acct}<LedgerPreview {game} account={acct} />{/if}
    {#if tag}<p class="tag" data-testid="decision-tag"><b>{tag.variant.label} household:</b> {tag.variant.decision} <button type="button" class="lnk" data-testid="howlink-decisions" onclick={() => openExplorer(agentsAnchorFor(tag.cls))}>How decisions are made</button></p>{/if}
    {#if pinned?.kind === 'person'}<PersonEarnings {game} person={pinned.id} />{/if}

    {#if model.removable}
      <section class="rm">
        {#if model.removable.already}
          <p>Removal of this {INFRA_LABEL(model.removable.kind).toLowerCase()} is in your plan.</p>
          <Button onclick={undoRemoval} disabled={!canEdit}>Undo removal</Button>
        {:else}
          <Button onclick={queueRemoval} disabled={!canEdit} title="Queue removal of this tile's building">Remove {INFRA_LABEL(model.removable.kind).toLowerCase()}…</Button>
          {#if !canEdit}<p class="hint">Removal can be queued in Review or Plan.</p>{/if}
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
  .tag { font-size: 12px; margin: 6px 0; }
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
  .ppl { list-style: none; margin: 0; padding: 0; font-size: 13px; }
  .ppl span { color: var(--text-dim); font-size: 12px; }
  .lnk { border: 0; background: transparent; color: var(--series-1); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; }
  .link { margin: 4px 0; font-size: 13px; }
  aside:not(.hover) { pointer-events: auto; }
  .tip { position: absolute; left: 50%; top: 10px; transform: translateX(-50%); z-index: 12; background: var(--text); color: var(--bg); padding: 4px 10px; border-radius: var(--radius); }
</style>
