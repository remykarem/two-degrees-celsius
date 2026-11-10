<script lang="ts">
  import type { Game } from '../store';
  import { householdRows, peopleGroups, selectedHouseholdId, selectionAnnouncement, type HouseholdSortKey } from './people';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let sort = $state<HouseholdSortKey>('id');
  let announce = $state('');
  const snap = $derived($gs.snapshot);
  const rows = $derived(snap ? householdRows(snap, sort) : []);
  const selHh = $derived(selectedHouseholdId(snap, $gs.selection));
  const groups = $derived(snap ? peopleGroups(snap, selHh) : []);
  const selPerson = $derived($gs.selection?.kind === 'person' ? $gs.selection.id : null);

  function pickHousehold(id: number) {
    game.select({ kind: 'household', id });
    announce = selectionAnnouncement(game.state.get().snapshot, { kind: 'household', id });
  }
  function pickPerson(id: number) {
    game.select({ kind: 'person', id });
    announce = selectionAnnouncement(game.state.get().snapshot, { kind: 'person', id });
  }
  /** Arrow keys move between rows of a list; Enter and Space are the buttons' own. */
  function onListKey(e: KeyboardEvent) {
    const d = e.key === 'ArrowDown' ? 1 : e.key === 'ArrowUp' ? -1 : 0;
    if (!d) return;
    const list = e.currentTarget as HTMLElement;
    const btns = [...list.querySelectorAll<HTMLButtonElement>('button.row')];
    const i = btns.indexOf(document.activeElement as HTMLButtonElement);
    if (i < 0) return;
    e.preventDefault();
    btns[Math.max(0, Math.min(btns.length - 1, i + d))]?.focus();
  }
</script>

<div class="people">
  <div class="sort">
    <label for="hh-sort">Households, sorted by</label>
    <select id="hh-sort" bind:value={sort}>
      <option value="id">order</option><option value="netWorth">net worth</option><option value="income">income</option>
    </select>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <ul class="list" aria-label="Households" onkeydown={onListKey}>
    {#each rows as r (r.id)}
      <li>
        <button class="row" class:on={selHh === r.id && !selPerson} class:unhoused={!r.housed} aria-pressed={selHh === r.id && !selPerson} onclick={() => pickHousehold(r.id)}>
          <span class="name">{r.label}</span><span class="cls">{r.class}</span>
          <span class="meta">{r.people} {r.people === 1 ? 'person' : 'people'} ·
            {#if r.housed}{r.homeText}{:else}<b class="nohome">no home</b>{/if}</span>
          <span class="nums"><span title="Net worth">{r.netWorthText}</span> <span class="dim" title="Income per month">{r.incomeText}/mo</span></span>
        </button>
      </li>
    {/each}
  </ul>

  {#if selHh !== null}
    <p class="dim">Savings, debt and net worth: <button class="lnk" onclick={() => game.openLedger({ kind: 'household', id: selHh })}>see this household's transactions</button></p>
  {/if}
  <h3>{selHh !== null ? 'Members' : 'People'}</h3>
  {#if selHh !== null}<p class="dim"><button class="lnk" onclick={() => game.select(null)}>Show everyone</button></p>{/if}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="plist" role="group" aria-label="People" onkeydown={onListKey}>
    {#each groups as g (g.household)}
      <h4>{g.label}</h4>
      <ul>
        {#each g.people as p (p.id)}
          <li><button class="row small" class:on={selPerson === p.id} aria-pressed={selPerson === p.id} onclick={() => pickPerson(p.id)}><span class="name">{p.label}</span><span class="dim">{p.detail}</span></button></li>
        {/each}
      </ul>
    {/each}
  </div>
  <p class="sr-only" role="status" aria-live="polite">{announce}</p>
</div>

<style>
  .people { display: flex; flex-direction: column; gap: 4px; }
  .sort { display: flex; gap: 6px; align-items: center; font-size: 12px; color: var(--text-dim); }
  select { font: inherit; font-size: 12px; background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 3px; }
  .row { display: grid; grid-template-columns: 1fr auto; width: 100%; text-align: left; gap: 0 8px; padding: 4px 8px; background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: var(--radius); cursor: pointer; font: inherit; font-size: 12px; }
  .row:hover { background: var(--surface-2); }
  .row.on { border-color: var(--accent); box-shadow: inset 3px 0 0 var(--accent); background: var(--surface-2); }
  .row:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .row.unhoused { border-style: dashed; border-color: var(--warn); }
  .name { font-weight: 600; font-size: 13px; }
  .cls { text-align: right; color: var(--text-dim); text-transform: capitalize; }
  .meta { color: var(--text-dim); }
  .nohome { color: var(--warn); }
  .nums { text-align: right; font-family: var(--font-mono); }
  .dim { color: var(--text-dim); }
  .row.small { display: flex; justify-content: space-between; padding: 2px 8px; }
  .row.small .name { font-weight: 500; font-size: 12px; }
  h3 { margin: 8px 0 0; font-size: 13px; }
  h4 { margin: 6px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  p { margin: 0; font-size: 12px; }
  .lnk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; }
</style>
