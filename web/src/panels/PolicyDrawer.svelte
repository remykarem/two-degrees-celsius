<script lang="ts">
  import type { LeverInfo } from '../contract';
  import type { Game } from '../store';
  import { Button } from '../ui-kit';
  import { SYSTEM_NAME } from './format';
  import { draftNumber, groupLevers, leverStatus, toCommandValue, toNum } from './policy';
  import PolicyRow from './PolicyRow.svelte';
  import { setUi, ui } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const canEdit = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !$gs.snapshot?.gameOver);
  const planLocked = $derived(!!$gs.live?.locked);
  const snap = $derived($gs.snapshot);
  const groups = $derived(groupLevers($gs.levers));
  const systemDraft = $derived($gs.draft.find((d) => d.command.type === 'switch_system'));
  let closedGroups = $state<Record<string, boolean>>({ money: true, spending: true });

  $effect(() => { if (!canEdit && $ui.drawerOpen) setUi({ drawerOpen: false }); });

  const draftOf = (key: string) => $gs.draft.find((d) => d.command.type === 'set_policy' && d.command.lever === key);

  function setLever(l: LeverInfo, n: number) {
    const cur = snap ? toNum(l, snap.policy[l.key].nominal) : 0;
    const existing = draftOf(l.key);
    if (n === cur) { if (existing) game.removeDraft(existing.id); return; }
    game.draftCommand({ type: 'set_policy', lever: l.key, value: toCommandValue(l, n) });
  }
  function resetLever(l: LeverInfo) {
    const d = draftOf(l.key);
    if (d) game.removeDraft(d.id);
  }
  const draftCount = (keys: string[]) => keys.filter((k) => draftOf(k)).length;
  const open = $derived($ui.drawerOpen);
</script>

<div class="wrap">
  <div class="tog"><Button pressed={open} onclick={() => setUi({ drawerOpen: !open })}><span data-coach="policies">Policies {open ? '▸' : '◂'}</span></Button></div>
  {#if open && snap}
    <section class="drawer" aria-label="Policy drawer">
      {#if !canEdit}<p class="note">Policies can be changed during Review and Plan.</p>{:else if planLocked}<p class="note">The plan is locked. Unlock it (Plan tray) to change policies.</p>{/if}

      <div class="system">
        <div>
          <h3>Economic system</h3>
          <b>{SYSTEM_NAME[snap.system.current]}</b>
          {#if snap.system.transition}<span class="dim"> switching to {SYSTEM_NAME[snap.system.transition.to]}, {snap.system.transition.monthsLeft} months left</span>{/if}
          {#if systemDraft && systemDraft.command.type === 'switch_system'}
            <div class="draft" class:bad={systemDraft.preview.blocked}>Queued: switch to {SYSTEM_NAME[systemDraft.command.to]}{systemDraft.preview.blocked ? ' (blocked)' : ''}</div>
          {/if}
        </div>
        <Button disabled={!canEdit || planLocked} onclick={() => setUi({ systemModal: true })}>Compare systems…</Button>
      </div>

      {#each groups as { group, levers } (group.id)}
        <details open={!closedGroups[group.id]} ontoggle={(e) => (closedGroups[group.id] = !(e.currentTarget as HTMLDetailsElement).open)}>
          <summary><span class="gname">{group.label}</span>
            {#if draftCount(group.keys)}<span class="badge">{draftCount(group.keys)} drafted</span>{/if}</summary>
          {#if group.blurb}<p class="blurb">{group.blurb}</p>{/if}
          <ul>
            {#each levers as l (l.key)}
              {@const p = snap.policy[l.key]}
              {@const st = leverStatus(l, snap, $gs.draft)}
              <PolicyRow lever={l} system={snap.system.current} nominal={p.nominal} effective={p.effective} disabled={canEdit ? (planLocked ? 'The plan is locked' : st.disabled) : 'Locked while the turn plays out'}
                draftValue={draftNumber(l, $gs.draft)} preview={draftOf(l.key)?.preview} onset={(v) => setLever(l, v)} onreset={() => resetLever(l)} />
            {/each}
          </ul>
        </details>
      {/each}
    </section>
  {/if}
</div>

<style>
  .wrap { position: absolute; top: 8px; right: 8px; z-index: 20; display: flex; flex-direction: column; align-items: flex-end; gap: 6px; max-height: calc(100% - 16px); max-width: calc(100% - 16px); width: min(390px, calc(100% - 16px));
    /* The wrapper is only a layout box: without this it was invisibly wider than the drawer and swallowed map clicks beside it. */
    pointer-events: none; }
  .wrap > * { pointer-events: auto; }
  .drawer { width: min(390px, 100%); flex: 1 1 auto; min-height: 0; overflow: auto; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow); padding: 10px; }
  @media (max-width: 1200px) { .drawer { width: min(340px, 100%); } }
  .tog { flex: none; }
  .system { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
  details { border-bottom: 1px solid var(--border); }
  summary { cursor: pointer; padding: 8px 0; display: flex; gap: 8px; align-items: center; font-weight: 700; }
  .badge { font-size: 11px; font-weight: 600; background: var(--info-bg); border: 1px solid var(--accent); border-radius: 8px; padding: 0 6px; }
  ul { margin: 0; padding: 0; }
  .blurb { margin: 0 0 4px; font-size: 12px; color: var(--text-dim); }
  .draft { font-size: 12px; font-weight: 600; margin-top: 2px; } .draft.bad { color: var(--bad); }
  .note { color: var(--text-dim); font-size: 12px; margin: 0 0 6px; }
  .dim { color: var(--text-dim); font-size: 12px; }
</style>
