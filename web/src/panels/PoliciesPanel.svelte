<script lang="ts">
  import type { Game } from '../store';
  import { byGroup, draftLines, EVENT_WORD, pendingLines, policyLines, timelineLines } from './policies';
  import HowButton from './HowButton.svelte';
  import { setUi } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const snap = $derived($gs.snapshot);
  const board = $derived(snap?.policies ?? null);
  const drafts = $derived(Object.fromEntries($gs.draft.flatMap((d) => (d.command.type === 'set_policy' ? [[d.command.lever, typeof d.command.value === 'object' ? d.command.value.mode : d.command.value]] : []))));
  const lines = $derived(board ? policyLines(board, $gs.levers, drafts) : []);
  const drafted = $derived(draftLines(lines));
  const groups = $derived(byGroup(lines));
  const pending = $derived(pendingLines(lines));
  const timeline = $derived(board ? timelineLines(board, $gs.levers) : []);
  const canEdit = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !snap?.gameOver);
  const GROUP_LABEL: Record<string, string> = { taxes: 'Taxes', money: 'Money and credit', labour: 'Labour and welfare', nature: 'Nature and carbon', spending: 'Government spending', other: 'Other' };
  let showAll = $state(false);
  const initialRows = $derived(lines.filter((l) => l.changedFromInitial || showAll));
  function lever(key: string) { game.select(null); setUi({ drawerOpen: canEdit, focusLever: canEdit ? (key as never) : null }); }
</script>

{#if board}
  <div class="pol" data-testid="policies-panel">
    <p class="intro">What is in force now, what you started with at turn 0, and what is on its way. Open a policy in the drawer to change it.</p>

    <section aria-labelledby="pol-pend"><h3 id="pol-pend">Pending changes ({pending.length})</h3>
      {#if pending.length}
        <ul class="pend">{#each pending as p (p.lever)}<li><span><b>{p.label}</b>: {p.active} to {p.pending!.text}, takes effect turn {p.pending!.turn}</span>
          <button type="button" onclick={() => game.cancelPending(p.pending!.id)} aria-label="Cancel the pending change to {p.label}">Cancel</button></li>{/each}</ul>
      {:else}<p class="dim">Nothing queued.</p>{/if}
    </section>

    <section aria-labelledby="pol-draft"><h3 id="pol-draft">Drafts in your plan ({drafted.length})</h3>
      {#if drafted.length}<ul class="pend">{#each drafted as d (d.lever)}<li><span><b>{d.label}</b>: {d.active} to {d.draft} (not submitted yet)</span></li>{/each}</ul>
      {:else}<p class="dim">No policy change in the plan tray.</p>{/if}
    </section>

    <section aria-labelledby="pol-now"><h3 id="pol-now">Policies <label class="chk"><input type="checkbox" bind:checked={showAll} /> show unchanged too</label></h3>
      {#if !initialRows.length}<p class="dim">You have not changed anything from the turn 0 settings.</p>{/if}
      {#each groups as g (g.group)}
        {@const rows = g.lines.filter((l) => l.changedFromInitial || l.pending || l.draft || l.weakened || showAll)}
        {#if rows.length}
          <h4>{GROUP_LABEL[g.group] ?? g.group}</h4>
          <table>
            <thead><tr><th scope="col">Policy</th><th scope="col" class="n">Turn 0</th><th scope="col" class="n">Set</th><th scope="col" class="n">In force</th></tr></thead>
            <tbody>
              {#each rows as l (l.lever)}
                <tr data-testid="policy-{l.lever}">
                  <th scope="row"><button type="button" class="lnk" onclick={() => lever(l.lever)} title="Open this policy in the drawer">{l.label}</button> <HowButton anchor={l.lever} label={l.label} />
                    <span class="sub">last change: {l.lastChange}{#if l.pending}; pending {l.pending.text} from turn {l.pending.turn}{/if}</span>
                    {#if l.draft}<span class="sub">draft in your plan: {l.draft}</span>{/if}
                    {#if l.weakened}<span class="sub warn">{l.weakened}</span>{/if}</th>
                  <td class="n">{l.initial}</td><td class="n">{l.active}</td><td class="n">{l.effective ?? l.active}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {/each}
    </section>

    <section aria-labelledby="pol-time"><h3 id="pol-time">Timeline of changes</h3>
      {#if timeline.length}
        <ol class="tl">{#each timeline as e, i (i)}<li class={e.kind}><span class="t">turn {e.turn}</span> <span class="k">{EVENT_WORD[e.kind]}</span> {e.text}</li>{/each}</ol>
      {:else}<p class="dim">No policy has been changed yet.</p>{/if}
    </section>
  </div>
{:else}<p class="dim">The simulation did not send a policy board (a package older than contract v1.6).</p>{/if}

<style>
  .pol { display: flex; flex-direction: column; gap: 10px; min-width: 0; } .intro, .dim { font-size: 12px; color: var(--text-dim); margin: 0; }
  h4 { margin: 6px 0 2px; font-size: 11px; text-transform: uppercase; color: var(--text-dim); } .chk { font-size: 11px; font-weight: 400; text-transform: none; margin-left: 6px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; table-layout: fixed; } thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; } th.n { text-align: right; }
  thead th:first-child { width: 54%; } th[scope='row'] { text-align: left; font-weight: 400; } .n { text-align: right; font-family: var(--font-mono); overflow-wrap: anywhere; vertical-align: top; } tr { border-top: 1px dotted var(--border); }
  .sub { display: block; font-size: 10.5px; color: var(--text-dim); } .sub.warn { color: var(--warn); }
  .lnk { white-space: nowrap; border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; text-align: left; }
  .pend { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; font-size: 12px; } .pend li { display: flex; justify-content: space-between; gap: 6px; align-items: center; }
  .pend button { font: inherit; font-size: 11px; border: 1px solid var(--border); background: var(--surface); color: var(--text); border-radius: var(--radius); cursor: pointer; padding: 1px 6px; }
  .tl { margin: 0; padding: 0; list-style: none; font-size: 12px; display: flex; flex-direction: column; gap: 3px; } .tl li { border-left: 3px solid var(--border); padding-left: 6px; }
  .tl .activated { border-color: var(--good); } .tl .rejected, .tl .cancelled { border-color: var(--bad); } .tl .weakened { border-color: var(--warn); } .tl .queued { border-color: var(--accent); }
  .t { font-family: var(--font-mono); color: var(--text-dim); } .k { font-weight: 600; font-size: 11px; text-transform: uppercase; }
</style>
