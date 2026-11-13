<script lang="ts">
  import type { AgentsCatalog, AssumptionTopic } from '../contract';
  import type { Game } from '../store';
  import { Modal } from '../ui-kit';
  import { fmtUnit } from './explain';
  import type { LoopLine } from './explorer';
  import { ACTOR_WORD, DISTRIBUTION_WORD, causalLines, decisionLine, decisionsOfKind, groupTopics, isAgentsAnchor, kindForAnchor, liveLookup, mechanismLine, effectLines, loopLines, loopSummary, mechanismsOfKind, nodeLines, relatedOf, searchAgents, searchTopics, STATUS_WORD, topicFor } from './explorer';
  import { PRESSURE_NOTE } from './explain';
  import { openCauses, setUi, ui } from './ui';
  import { catalog, fill, hasCatalog, text } from '../store/catalog';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const open = $derived($ui.explorer !== null);
  const res = $derived(open ? game.assumptions() : null);
  const topics = $derived<AssumptionTopic[]>(res && res.ok ? res.value : []);
  let query = $state('');
  let status = $state<AssumptionTopic['status'] | 'all'>('all');
  let chosen = $state<string | null>(null);
  let section = $state<'topics' | 'agents'>('topics');
  let chosenKind = $state<string | null>(null);
  const ares = $derived(open ? game.agents() : null);
  const agents = $derived<AgentsCatalog | null>(ares && ares.ok ? ares.value : null);
  const found = $derived(agents ? searchAgents(agents, query) : { kinds: [], draws: [] });
  const kind = $derived(found.kinds.find((k) => k.id === chosenKind) ?? found.kinds[0] ?? null);
  const kindMechanisms = $derived(kind && open && hasCatalog() ? mechanismsOfKind(catalog().mechanisms, kind.id) : []);
  const kindDecisions = $derived(kind && open && hasCatalog() ? decisionsOfKind(catalog().decisions, kind.id) : []);
  const LOOP_PREVIEW = 8;
  const kindLoops = $derived(kind && open && hasCatalog() ? loopLines(catalog().causal, [...kindDecisions.map((d) => `decision:${d.id}`), ...kindMechanisms.map((m) => `mechanism:${m.id}`)]) : []);
  // A link names an anchor or a topic id: resolve it when the explorer opens.
  $effect(() => {
    const want = $ui.explorer?.topic;
    if (isAgentsAnchor(want)) {
      section = 'agents';
      const k = agents && want ? kindForAnchor(agents, want) : null;
      if (k) chosenKind = k.id;
    } else if (want) {
      section = 'topics'; const t = topics.find((x) => x.id === want) ?? topicFor(topics, want); if (t) chosen = t.id; }
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
          <div class="tabs" role="tablist" aria-label="Explorer sections">
            <button type="button" role="tab" aria-selected={section === 'topics'} class:on={section === 'topics'} data-testid="tab-topics" onclick={() => (section = 'topics')}>Topics</button>
            <button type="button" role="tab" aria-selected={section === 'agents'} class:on={section === 'agents'} data-testid="tab-agents" onclick={() => (section = 'agents')}>Agents</button>
          </div>
          <input type="search" placeholder={section === 'agents' ? 'Search agents and random draws' : 'Search topics, formulas, parameters'} bind:value={query} aria-label="Search the explorer" data-autofocus />
          {#if section === 'agents'}
            <h3>Who decides</h3>
            <ul>{#each found.kinds as k (k.id)}<li><button type="button" class:on={kind?.id === k.id} aria-current={kind?.id === k.id ? 'true' : undefined} onclick={() => (chosenKind = k.id)}>{k.title}</button></li>{:else}<li class="dim">No agent matches.</li>{/each}</ul>
            <h3>Chance</h3>
            <ul><li><button type="button" class="jump" data-testid="jump-draws" onclick={() => document.getElementById('random-draws')?.scrollIntoView({ block: 'start' })}>Where randomness enters ({found.draws.length})</button></li></ul>
          {:else}
          <label class="st">Status <select bind:value={status} aria-label="Filter by status"><option value="all">All</option>{#each Object.entries(STATUS_WORD) as [k, w] (k)}<option value={k}>{w}</option>{/each}</select></label>
          {#each groups as g (g.group)}
            <h3>{g.label}</h3>
            <ul>{#each g.topics as t (t.id)}<li><button type="button" class:on={topic?.id === t.id} aria-current={topic?.id === t.id ? 'true' : undefined} onclick={() => (chosen = t.id)}>{t.title}</button></li>{/each}</ul>
          {:else}<p class="dim">No topic matches.</p>{/each}
          {/if}
        </nav>
        {#snippet whyRow(qs: { id: string; label: string }[])}
          {#if qs.length}<p class="dim edge" data-testid="agent-why-row">{text('causes', 'explain_row')} {#each qs as q (q.id)}<button type="button" class="qwhy" data-testid="agent-why" aria-label={fill(text('causes', 'causes_of_aria'), { quantity: q.label })} onclick={() => openCauses(q.id)}>{q.label}</button>{/each}</p>{/if}
        {/snippet}
        {#snippet loopItem(l: LoopLine)}
  <li class="lp" data-testid="agent-loop" data-id={l.id} data-kind={l.kind}><span class="lpb {l.kind}" title={l.kindLabel} aria-label={l.kindLabel}>{l.badge}</span> <span class="lpk">{l.kindLabel}</span> <b>{l.title}</b>{#if l.incomplete.length} <span class="lpi" title={l.incomplete.join(' ')}>(partly modelled)</span>{/if}
    <details><summary class="dim">{l.title === l.chain ? 'What this means' : l.chain}</summary><p>{l.sentence}</p>{#if l.story}<p>{l.story}</p>{/if}<p class="dim">{l.delay}{#if l.incomplete.length} {l.incomplete.join('; ')}.{/if}</p></details></li>
{/snippet}
<article aria-live="polite">
          {#if section === 'agents'}
            {#if !agents}<p role="alert">The simulation has no agents catalogue{ares && !ares.ok ? `: ${ares.error.message}` : ''}.</p>
            {:else}
              {#if kind}
                <h3 class="t" data-testid="agent-title">{kind.title} <span class="badge {kind.actor}" title="Agents decide; the others follow fixed rules or chance">{ACTOR_WORD[kind.actor]}</span></h3>
                <p>{kind.summary}</p>
                {#if kind.decides.length}<h4>Decides</h4><ul class="lo">{#each kind.decides as d (d)}<li>{d}</li>{/each}</ul>{/if}
                <h4>Observes</h4><ul class="lo">{#each kind.observes as d (d)}<li>{d}</li>{/each}</ul>
                {#if kindDecisions.length}
                  <h4>Decisions (generated from the code)</h4>
                  <ul class="lo" data-testid="agent-decisions">
                    {#each kindDecisions as d (d.id)}
                      {@const l = decisionLine(d)}
                      {@const cl = causalLines(catalog().causal, d.id, liveLookup(catalog(), $gs.snapshot))}
                      {@const fl = effectLines(catalog().causal, `decision:${d.id}`)}
                      <li data-testid="agent-decision" data-id={d.id}><span class="dim">{l.inputs}</span> &rarr; <b>{l.title}</b> &rarr; <span class="dim">{l.outputs}</span>
                        <p class="dim edge" data-testid="agent-edges">Reads: {cl.inputs}. Writes: {cl.outputs}.</p>
                        {#if loopSummary(catalog().causal, `decision:${d.id}`)}<p class="dim edge" data-testid="agent-loop-summary">{loopSummary(catalog().causal, `decision:${d.id}`)}.</p>{/if}
                        {#if fl.length}<details data-testid="agent-effects"><summary class="dim">How each input pushes each output ({fl.length})</summary><ul class="lo">{#each fl as f (f.text)}<li class="fx {f.sign}" title={f.sentence} data-testid="agent-effect"><span class="fxl">{f.text}</span><details><summary class="dim">why</summary><p>{f.sentence}</p><p class="dim">{f.checked}</p></details></li>{/each}</ul></details>{/if}
                        {#each cl.upstream as u (u.label)}<p class="dim edge" data-testid="agent-upstream">Causes of <button type="button" class="qwhy" data-testid="agent-why" aria-label={fill(text('causes', 'causes_of_aria'), { quantity: u.label })} onclick={() => openCauses(u.id)}>{u.label}</button> (up to {catalog().causal.upstreamDepth} steps back): {u.causes}.</p>{/each}
                        {@render whyRow(cl.quantities)}
                        <details><summary class="dim">{d.rule.label}</summary><p>{d.rule.text}</p><p class="dim">Runs in {d.system}.{d.params.length ? ` Parameters: ${d.params.map((p) => p.label).join('; ')}.` : ''}</p></details></li>
                    {/each}
                  </ul>
                {/if}
                {#if kindMechanisms.length}
                  <h4>Mechanisms (plain code, not choices)</h4>
                  <ul class="lo" data-testid="agent-mechanisms">
                    {#each kindMechanisms as m (m.id)}
                      {@const l = mechanismLine(m)}
                      {@const cl = nodeLines(catalog().causal, `mechanism:${m.id}`, liveLookup(catalog(), $gs.snapshot))}
                      {@const fl = effectLines(catalog().causal, `mechanism:${m.id}`)}
                      <li data-testid="agent-mechanism" data-id={m.id}><span class="dim">{l.inputs}</span> &rarr; <b>{l.title}</b> &rarr; <span class="dim">{l.outputs}</span>
                        <p class="dim edge" data-testid="agent-mechanism-edges">Reads: {cl.inputs}. Writes: {cl.outputs}.</p>
                        {@render whyRow(cl.quantities)}
                        {#if loopSummary(catalog().causal, `mechanism:${m.id}`)}<p class="dim edge" data-testid="agent-loop-summary">{loopSummary(catalog().causal, `mechanism:${m.id}`)}.</p>{/if}
                        {#if fl.length}<details data-testid="agent-effects"><summary class="dim">How each input pushes each output ({fl.length})</summary><ul class="lo">{#each fl as f (f.text)}<li class="fx {f.sign}" title={f.sentence} data-testid="agent-effect"><span class="fxl">{f.text}</span><details><summary class="dim">why</summary><p>{f.sentence}</p><p class="dim">{f.checked}</p></details></li>{/each}</ul></details>{/if}
                        <details><summary class="dim">{m.rule.label}</summary><p>{m.explains}</p><p>{m.rule.text}</p><p class="dim">Runs in {m.system}.{m.params.length ? ` Parameters: ${m.params.map((p) => p.label).join('; ')}.` : ''}</p></details></li>
                    {/each}
                  </ul>
                {/if}
                {#if kindLoops.length}
                  <h4>Feedback loops ({kindLoops.length})</h4>
                  <p class="dim" data-testid="agent-loops-intro">Loops that pass through this kind's decisions and mechanisms: a change goes round and comes back. Shortest first.</p>
                  <ul class="lo loops" data-testid="agent-loops">
                    {#each kindLoops.slice(0, LOOP_PREVIEW) as l (l.id)}{@render loopItem(l)}{/each}
                  </ul>
                  {#if kindLoops.length > LOOP_PREVIEW}
                    <details data-testid="agent-loops-more"><summary class="dim">Show the other {kindLoops.length - LOOP_PREVIEW} loops</summary>
                      <ul class="lo loops">{#each kindLoops.slice(LOOP_PREVIEW) as l (l.id)}{@render loopItem(l)}{/each}</ul>
                    </details>
                  {/if}
                {/if}
                {#if kind.notes.length}
                  <h4>Not decided here</h4><ul class="lo" data-testid="agent-notes">{#each kind.notes as n (n)}<li>{n}</li>{/each}</ul>
                {/if}
                <h4>Rule</h4><p class="formula" style="white-space: pre-line" data-testid="agent-rule">{kind.rule}</p>
                {#if kind.variants.length}
                  <h4>Classes (worked out from what a household owns and earns, never chosen)</h4>
                  <table><tbody>{#each kind.variants as v (v.id)}<tr><th scope="row">{v.label}<span class="path">{v.rule}</span></th><td>{v.decision}</td></tr>{/each}</tbody></table>
                  <table><tbody>{#each kind.classParams as p (p.path)}<tr><th scope="row">{p.label}<span class="path">{p.path}</span></th><td class="n">{val(p.value, p.unit)}</td></tr>{/each}</tbody></table>
                {/if}
                {#if kind.params.length}
                  <h4>Live parameters</h4>
                  <table><tbody>{#each kind.params as p (p.path)}<tr><th scope="row">{p.label}<span class="path">{p.path}</span></th><td class="n">{val(p.value, p.unit)}</td></tr>{/each}</tbody></table>
                {/if}
                <details><summary>For developers: code paths</summary><ul>{#each kind.code as c (c)}<li><code>{c}</code></li>{/each}</ul></details>
              {/if}
              <h3 class="t" id="random-draws">Where randomness enters</h3>
              <p class="dim" data-testid="agent-determinism">{agents.deterministic.join(' ')}</p>
              <table class="draws" data-testid="randomness-table">
                <thead><tr><th>Event</th><th>Kind of draw</th><th>Odds and when</th></tr></thead>
                <tbody>
                  {#each found.draws as r (r.id)}
                    <tr>
                      <td><b>{r.event}</b><span class="path">{r.code}</span></td>
                      <td>{DISTRIBUTION_WORD[r.distribution]}<br /><span class="dim">{r.stream === 'weather' ? 'weather stream' : 'shared stream'}</span></td>
                      <td>{r.probability}<br /><span class="dim">{r.when}</span>
                        {#each r.params as p (p.path)}<br /><span class="dim">{p.label}: <span class="n">{val(p.value, p.unit)}</span> <span class="path inl">{p.path}</span></span>{/each}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {:else if topic}
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
  .tabs { display: flex; gap: 4px; } .tabs button { flex: 1; font: inherit; font-size: 12px; border: 1px solid var(--border); background: var(--surface); color: var(--text); border-radius: var(--radius); padding: 3px 6px; cursor: pointer; } .tabs button.on { background: var(--accent); color: var(--accent-text); }
  .jump { font-size: 12.5px; color: var(--accent); padding: 3px 6px; display: block; } .badge.non_agent { color: var(--text-dim); } .badge.agent { color: var(--good); } .draws th { text-align: left; font-size: 11px; color: var(--text-dim); } .draws td { vertical-align: top; padding: 4px 6px 4px 0; } .path.inl { display: inline; }
  .ex { display: grid; grid-template-columns: 240px minmax(0, 1fr); gap: 14px; min-height: 50vh; }
  nav { display: flex; flex-direction: column; gap: 4px; overflow: auto; max-height: 68vh; } nav input { font: inherit; padding: 4px 6px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); color: var(--text); }
  nav h3 { font-size: 10px; margin: 8px 0 0; color: var(--text-dim); text-transform: uppercase; } nav ul { list-style: none; margin: 0; padding: 0; }
  nav li button { width: 100%; text-align: left; font: inherit; font-size: 12.5px; border: 0; background: transparent; color: var(--text); padding: 3px 6px; border-radius: var(--radius); cursor: pointer; }
  nav li button.on { background: var(--accent); color: var(--accent-text); } .st { font-size: 11px; color: var(--text-dim); }
  article { overflow: auto; max-height: 68vh; font-size: 13px; } article h3.t { margin: 0 0 6px; font-size: 16px; text-transform: none; letter-spacing: 0; color: var(--text); } article h4 { margin: 12px 0 3px; font-size: 11px; text-transform: uppercase; color: var(--text-dim); }
  .formula { font-family: var(--font-mono); font-size: 12px; background: var(--surface-2); padding: 6px 8px; border-radius: var(--radius); } .dim { color: var(--text-dim); font-size: 12px; }
  .badge { font-size: 10px; font-weight: 700; text-transform: uppercase; border: 1px solid var(--border); border-radius: 8px; padding: 1px 6px; vertical-align: middle; } .badge.placeholder { color: var(--warn); border-color: var(--warn); } .badge.tuned { color: var(--good); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; } th[scope='row'] { text-align: left; font-weight: 400; } .path { display: block; font-family: var(--font-mono); font-size: 10px; color: var(--text-dim); } .n { text-align: right; font-family: var(--font-mono); } tr { border-top: 1px dotted var(--border); }
  .fx.none { opacity: 0.6; } .fx details { display: inline; } .fx details > p { margin: 2px 0; }
  .lpb { display: inline-block; min-width: 1.5em; text-align: center; font-weight: 700; font-size: 12px; padding: 0 4px; border: 1.5px solid currentColor; }
  .lpb.R { border-radius: 50%; } .lpb.B { border-radius: 2px; } .lpb.depends { border-radius: 2px; transform: rotate(45deg) scale(0.8); display: inline-block; }
  .qwhy { border: 1px solid var(--border); background: var(--surface); color: var(--accent); border-radius: 10px; padding: 0 7px; margin: 1px 3px 1px 0; cursor: pointer; font: inherit; font-size: 11.5px; }
  .lpk { font-size: 12px; } .lpi { font-size: 12px; opacity: 0.75; } .loops li { margin: 4px 0; }
  .exm { margin: 3px 0; } .lo { margin: 0; padding-left: 18px; } .edge { margin: 2px 0; } .rel { border: 1px solid var(--border); background: var(--surface); color: var(--accent); border-radius: 10px; padding: 1px 8px; margin-right: 4px; cursor: pointer; font: inherit; font-size: 12px; }
  @media (max-width: 900px) { .ex { grid-template-columns: 1fr; } nav { max-height: 24vh; } }
</style>
