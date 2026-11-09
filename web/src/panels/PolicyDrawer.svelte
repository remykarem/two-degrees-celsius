<script lang="ts">
  import type { LeverInfo, SystemKey } from '../contract';
  import type { Game } from '../store';
  import { Button } from '../ui-kit';
  import PolicyRow from './PolicyRow.svelte';
  import { leadText, REASON_TEXT } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let open = $state(false);
  const systems: SystemKey[] = ['market', 'regulated', 'coop'];
  const canEdit = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !$gs.snapshot?.gameOver);

  $effect(() => { if (!canEdit) open = false; });

  function draftFor(key: string) {
    return $gs.draft.find((d) => d.command.type === 'set_policy' && d.command.lever === key);
  }
  function setLever(l: LeverInfo, v: number) {
    const snap = $gs.snapshot;
    const cur = snap?.policy[l.key].nominal;
    const curNum = typeof cur === 'boolean' ? (cur ? 1 : 0) : typeof cur === 'string' ? (cur === 'open' ? 0 : 1) : (cur ?? 0);
    const existing = draftFor(l.key);
    if (v === curNum) {
      if (existing) game.removeDraft(existing.id);
      return;
    }
    const value = l.unit === 'bool' ? v === 1 : l.unit === 'enum' ? (v === 1 ? { mode: 'community' as const, cap: 0.5 } : { mode: 'open' as const }) : v;
    game.draftCommand({ type: 'set_policy', lever: l.key, value });
  }
  function draftNumber(key: string): number | undefined {
    const d = draftFor(key);
    if (!d || d.command.type !== 'set_policy') return undefined;
    const v = d.command.value;
    return typeof v === 'boolean' ? (v ? 1 : 0) : typeof v === 'object' ? (v.mode === 'open' ? 0 : 1) : v;
  }
  const systemDraft = $derived($gs.draft.find((d) => d.command.type === 'switch_system'));
</script>

<div class="wrap">
  <Button pressed={open} onclick={() => (open = !open)}>Policies {open ? '▸' : '◂'}</Button>
  {#if open}
    <section class="drawer" aria-label="Policy drawer">
      {#if !canEdit}<p class="note">Policies can be changed during Review and Plan.</p>{/if}
      <fieldset disabled={!canEdit}>
        <legend>Economic system</legend>
        <div class="systems">
          {#each systems as k (k)}
            <Button pressed={($gs.snapshot?.system.current === k && !systemDraft) || (systemDraft?.command.type === 'switch_system' && systemDraft.command.to === k)}
              onclick={() => (k === $gs.snapshot?.system.current ? systemDraft && game.removeDraft(systemDraft.id) : game.draftCommand({ type: 'switch_system', to: k }))}>{k}</Button>
          {/each}
        </div>
        {#if systemDraft}
          <div class="draft" class:bad={systemDraft.preview.blocked}>
            {#if systemDraft.preview.blocked}Blocked: {REASON_TEXT[systemDraft.preview.blocked]}{:else}Switch costs {systemDraft.preview.pc} PC, starts {leadText(systemDraft.preview.leadTime)}; needs confirmation. Transition takes months and some groups will resist.{/if}
          </div>
        {/if}
        <ul>
          {#each $gs.levers as l (l.key)}
            {@const p = $gs.snapshot?.policy[l.key]}
            {#if p}
              <PolicyRow lever={l} nominal={p.nominal} effective={p.effective} system={$gs.snapshot!.system.current}
                draftValue={draftNumber(l.key)} preview={draftFor(l.key)?.preview} onset={(v) => setLever(l, v)} />
            {/if}
          {/each}
        </ul>
      </fieldset>
    </section>
  {/if}
</div>

<style>
  .wrap { position: absolute; top: 8px; right: 8px; z-index: 20; display: flex; flex-direction: column; align-items: flex-end; gap: 6px; max-height: calc(100% - 16px); }
  .drawer { width: 380px; overflow: auto; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); box-shadow: var(--shadow); padding: 10px; }
  fieldset { border: 0; padding: 0; margin: 0; min-width: 0; }
  legend { font-weight: 700; padding: 0; margin-bottom: 4px; }
  .systems { display: flex; gap: 6px; }
  ul { margin: 6px 0 0; padding: 0; }
  .draft { font-size: 12px; font-weight: 600; margin-top: 4px; }
  .draft.bad { color: var(--bad); }
  .note { color: var(--text-dim); font-size: 12px; margin: 0 0 6px; }
</style>
