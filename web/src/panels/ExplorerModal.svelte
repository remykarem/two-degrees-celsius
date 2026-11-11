<script lang="ts">
  import type { AssumptionTopic } from '../contract';
  import type { Game } from '../store';
  import { Modal } from '../ui-kit';
  import { fmtUnit } from './explain';
  import { groupTopics, relatedOf, searchTopics, STATUS_WORD, topicFor } from './explorer';
  import { PRESSURE_NOTE } from './explain';
  import { setUi, ui } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const open = $derived($ui.explorer !== null);
  const res = $derived(open ? game.assumptions() : null);
  const topics = $derived<AssumptionTopic[]>(res && res.ok ? res.value : []);
  let query = $state('');
  let status = $state<AssumptionTopic['status'] | 'all'>('all');
  let chosen = $state<string | null>(null);
  // A link names an anchor or a topic id: resolve it when the explorer opens.
  $effect(() => {
    const want = $ui.explorer?.topic;
    if (want) { const t = topics.find((x) => x.id === want) ?? topicFor(topics, want); if (t) chosen = t.id; }
  });
  const shown = $derived(searchTopics(topics, query, status));
  const groups = $derived(groupTopics(shown));
  const topic = $derived(topics.find((t) => t.id === chosen) ?? shown[0] ?? null);
  const close = () => setUi({ explorer: null });
  const val = (v: number | string | boolean, unit: string) => (typeof v === 'number' ? fmtUnit(v, unit) : String(v));
</script>

{#if open}
  <Modal title="How it works" size="lg" onclose={close}>
    {#if res && !res.ok}<p role="alert">The simulation has no explorer data: {res.error.message}</p>
    {:else}
      <div class="ex" data-testid="explorer">
        <nav aria-label="Topics">
          <input type="search" placeholder="Search topics, formulas, parameters" bind:value={query} aria-label="Search the explorer" data-autofocus />
          <label class="st">Status <select bind:value={status} aria-label="Filter by status"><option value="all">All</option>{#each Object.entries(STATUS_WORD) as [k, w] (k)}<option value={k}>{w}</option>{/each}</select></label>
          {#each groups as g (g.group)}
            <h3>{g.label}</h3>
            <ul>{#each g.topics as t (t.id)}<li><button type="button" class:on={topic?.id === t.id} aria-current={topic?.id === t.id ? 'true' : undefined} onclick={() => (chosen = t.id)}>{t.title}</button></li>{/each}</ul>
          {:else}<p class="dim">No topic matches.</p>{/each}
        </nav>
        <article aria-live="polite">
          {#if topic}
            <h3 class="t">{topic.title} <span class="badge {topic.status}" data-testid="status-badge" title={topic.statusNote}>{STATUS_WORD[topic.status]}</span></h3>
            <p>{topic.summary}</p>
            <p class="formula" data-testid="topic-formula">{topic.formula}</p>
            <p class="dim">{topic.statusNote}</p>
            {#if topic.params.length}
              <h4>Live parameters</h4>
              <table><tbody>{#each topic.params as p (p.path)}<tr><th scope="row">{p.label}<span class="path">{p.path}</span></th><td class="n">{val(p.value, p.unit)}</td></tr>{/each}</tbody></table>
            {/if}
            {#if topic.examples.length}
              <h4>Worked examples</h4>
              {#each topic.examples as ex (ex.label)}
                <div class="exm"><b>{ex.label}</b>: {Object.entries(ex.inputs).map(([k, v]) => `${k} = ${v}`).join(', ')} gives <b>{typeof ex.result.value === 'number' ? fmtUnit(ex.result.value, ex.result.unit) : ex.result.value}</b>{#if ex.note}<span class="dim"> ({ex.note})</span>{/if}</div>
              {/each}
            {/if}
            {#if topic.leavesOut.length}<h4>What it leaves out</h4><ul class="lo">{#each topic.leavesOut as l (l)}<li>{l}</li>{/each}</ul>{/if}
            {#if relatedOf(topics, topic).length}<h4>Related</h4><p>{#each relatedOf(topics, topic) as r (r.id)}<button type="button" class="rel" onclick={() => (chosen = r.id)}>{r.title}</button>{/each}</p>{/if}
            <details><summary>For developers: code paths</summary><ul>{#each topic.code as c (c)}<li><code>{c}</code></li>{/each}</ul></details>
          {/if}
        </article>
      </div>
    {/if}
    {#snippet footer()}<span class="dim">{$gs.mode === 'pressure' ? PRESSURE_NOTE : ''}</span>{/snippet}
  </Modal>
{/if}

<style>
  .ex { display: grid; grid-template-columns: 240px minmax(0, 1fr); gap: 14px; min-height: 50vh; }
  nav { display: flex; flex-direction: column; gap: 4px; overflow: auto; max-height: 68vh; } nav input { font: inherit; padding: 4px 6px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); color: var(--text); }
  nav h3 { font-size: 10px; margin: 8px 0 0; color: var(--text-dim); text-transform: uppercase; } nav ul { list-style: none; margin: 0; padding: 0; }
  nav li button { width: 100%; text-align: left; font: inherit; font-size: 12.5px; border: 0; background: transparent; color: var(--text); padding: 3px 6px; border-radius: var(--radius); cursor: pointer; }
  nav li button.on { background: var(--accent); color: var(--accent-text); } .st { font-size: 11px; color: var(--text-dim); }
  article { overflow: auto; max-height: 68vh; font-size: 13px; } article h3.t { margin: 0 0 6px; font-size: 16px; text-transform: none; letter-spacing: 0; color: var(--text); } article h4 { margin: 12px 0 3px; font-size: 11px; text-transform: uppercase; color: var(--text-dim); }
  .formula { font-family: var(--font-mono); font-size: 12px; background: var(--surface-2); padding: 6px 8px; border-radius: var(--radius); } .dim { color: var(--text-dim); font-size: 12px; }
  .badge { font-size: 10px; font-weight: 700; text-transform: uppercase; border: 1px solid var(--border); border-radius: 8px; padding: 1px 6px; vertical-align: middle; } .badge.placeholder { color: var(--warn); border-color: var(--warn); } .badge.tuned { color: var(--good); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } th[scope='row'] { text-align: left; font-weight: 400; } .path { display: block; font-family: var(--font-mono); font-size: 10px; color: var(--text-dim); } .n { text-align: right; font-family: var(--font-mono); } tr { border-top: 1px dotted var(--border); }
  .exm { margin: 3px 0; } .lo { margin: 0; padding-left: 18px; } .rel { border: 1px solid var(--border); background: var(--surface); color: var(--accent); border-radius: 10px; padding: 1px 8px; margin-right: 4px; cursor: pointer; font: inherit; font-size: 12px; }
  @media (max-width: 900px) { .ex { grid-template-columns: 1fr; } nav { max-height: 24vh; } }
</style>
