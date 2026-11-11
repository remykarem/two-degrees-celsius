<script lang="ts">
  import type { Game } from '../store';
  import { firmLines, infraLines, noStateOwnership } from './industry';
  import { money } from './format';
  import HowButton from './HowButton.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const ind = $derived($gs.snapshot?.industry ?? null);
  const firms = $derived(ind ? firmLines(ind, $gs.snapshot?.households ?? []) : []);
  const infra = $derived(ind ? infraLines(ind) : []);
  const selected = $derived($gs.selection);
</script>

{#if ind}
  <div class="ind" data-testid="industry-panel">
    <p class="note" data-testid="industry-note">{ind.note}{#if noStateOwnership(ind)}<b class="zs">The state share is 0 in every firm.</b>{/if}</p>

    <section aria-labelledby="ind-firms"><h3 id="ind-firms">Private firms (goods and services) <HowButton anchor="industry" label="private firms" /></h3>
      {#each firms as f (f.firm)}
        <article class:sel={selected?.kind === 'firm' && selected.id === f.firm}>
          <h4><button type="button" class="lnk" onclick={() => game.select({ kind: 'firm', id: f.firm })} aria-label="Select {f.name} on the map">{f.name}</button> <span class="sec">{f.sector}</span></h4>
          <dl>{#each f.cells as c (c.label)}<div class="pr"><dt>{c.label}</dt><dd>{c.text}</dd></div>{/each}</dl>
          <p class="own"><b>Ownership:</b> {f.ownership}. <span class:zero={true}>{f.state}.</span></p>
        </article>
      {:else}<p class="dim">No active firms.</p>{/each}
    </section>

    <section aria-labelledby="ind-infra"><h3 id="ind-infra">Public infrastructure</h3>
      <table>
        <thead><tr><th scope="col">Kind</th><th scope="col" class="n">Count</th><th scope="col" class="n">Build cost</th><th scope="col" class="n">Upkeep</th></tr></thead>
        <tbody>
          {#each infra as i (i.kind)}<tr><th scope="row"><button type="button" class="lnk" disabled={i.count === 0} onclick={() => game.selectFirstOf(i.kind === 'public_housing' ? 'public_housing' : i.kind)} aria-label="Select the first {i.label} on the map">{i.label}</button></th>
            <td class="n">{i.count}</td><td class="n">{i.cost}</td><td class="n">{i.upkeep}<span class="sub">{i.upkeepTotal} in all</span></td></tr>{/each}
        </tbody>
      </table>
    </section>

    <section aria-labelledby="ind-pub"><h3 id="ind-pub">Public goods and services</h3>
      <ul>{#each ind.publicGoods as g (g.id)}<li><span><b>{g.label}</b>: {money(g.spendingMonthly)}/month</span><span class="sub">{g.buys}</span></li>{/each}</ul>
    </section>
  </div>
{:else}<p class="dim">The simulation did not send an industry view (a package older than contract v1.6).</p>{/if}

<style>
  .ind { display: flex; flex-direction: column; gap: 10px; min-width: 0; } .note, .dim { font-size: 12px; color: var(--text-dim); margin: 0; }
  article { border: 1px solid var(--border); border-radius: var(--radius); padding: 6px 8px; margin-bottom: 6px; } article.sel { border-color: var(--accent); }
  h4 { margin: 0 0 3px; font-size: 13px; text-transform: none; } .sec { font-size: 11px; color: var(--text-dim); font-weight: 400; }
  dl { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1px 12px; margin: 0; font-size: 12px; } .pr { display: flex; justify-content: space-between; gap: 6px; min-width: 0; } dt { color: var(--text-dim); } dd { margin: 0; font-family: var(--font-mono); text-align: right; }
  .zs { margin-left: 4px; }
  .own { margin: 4px 0 0; font-size: 11.5px; } .zero { color: var(--text-dim); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } thead th { font-size: 10px; text-align: left; color: var(--text-dim); text-transform: uppercase; } th.n { text-align: right; }
  th[scope='row'] { text-align: left; font-weight: 400; } .n { text-align: right; font-family: var(--font-mono); vertical-align: top; } tr { border-top: 1px dotted var(--border); } .sub { display: block; font-size: 10.5px; color: var(--text-dim); }
  ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 5px; font-size: 12px; } li { display: flex; flex-direction: column; }
  .lnk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; } .lnk:disabled { color: var(--text-dim); text-decoration: none; cursor: default; }
</style>
