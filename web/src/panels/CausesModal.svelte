<script lang="ts">
  import { tick } from 'svelte';
  import type { Game } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { catalog, fill, text } from '../store/catalog';
  import { PRESSURE_NOTE } from './explain';
  import { back, causeModel, layoutCauses, LAYOUT, NARROW_PX, placeChips, recentre, route, startDepth, type CauseEdgeModel, type CauseNodeModel } from './causes';
  import { setUi, ui } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const t = (k: string, vars: Record<string, string | number> = {}): string => fill(text('causes', k), vars);

  const cs = $derived($ui.causes);
  const g = $derived(catalog().causal);
  const narrow = typeof window !== 'undefined' && typeof window.matchMedia === 'function' && window.matchMedia(`(max-width: ${NARROW_PX}px)`).matches;
  // The player's choices. Depth starts where the wire's range allows; narrow screens open on the list.
  let depth = $state<number | null>(null);
  let listPick = $state<boolean | null>(null);
  let showNone = $state(false);
  let active = $state<string | null>(null);
  let announce = $state('');
  const d = $derived(depth ?? cs?.depth ?? startDepth(g.upstreamDepth, narrow));
  const list = $derived(listPick ?? (cs?.view ? cs.view === 'list' : narrow));
  const model = $derived(cs ? causeModel(g, catalog(), $gs.snapshot, $gs.levers, cs.quantity, d, showNone) : null);
  const layout = $derived(model ? layoutCauses(model) : null);
  const chips = $derived(model && layout ? placeChips(layout, model.edges.filter((e) => !e.selfLoop)) : new Map<string, { x: number; y: number }>());
  const activeEdge = $derived(model?.edges.find((e) => e.key === active) ?? model?.undrawn.find((e) => e.key === active) ?? null);
  const label = (id: string): string => g.nodes.find((n) => n.id === id)?.label ?? id;
  const close = () => { setUi({ causes: null }); depth = null; active = null; listPick = null; showNone = false; };

  const centreOn = async (n: CauseNodeModel) => {
    if (!cs || n.focus || !n.hasCauses) return;
    setUi({ causes: recentre(cs, n.id) });
    active = null;
    await tick();
    announce = t('announce', { quantity: n.label, n: model?.nodes.length ? model.nodes.length - 1 : 0, depth: d });
    root?.querySelector<HTMLElement>('[data-focus="true"]')?.focus();
  };
  const goBack = async () => {
    if (!cs) return;
    setUi({ causes: back(cs) });
    active = null;
    await tick();
    root?.querySelector<HTMLElement>('[data-focus="true"]')?.focus();
  };
  let root = $state<HTMLElement | null>(null);
  let wrap = $state<HTMLElement | null>(null);
  // Keep the centre in view: a deep graph is taller than the box, so scroll to the focus node (and to the right edge) whenever the picture changes.
  $effect(() => {
    const p = model && layout ? layout.byId.get(model.focus.id) : null;
    if (!wrap || !p) return;
    wrap.scrollTop = Math.max(0, p.y + p.h / 2 - wrap.clientHeight / 2);
    wrap.scrollLeft = Math.max(0, p.x + p.w - wrap.clientWidth + LAYOUT.pad);
  });

  /** Arrow keys follow the edges: Left to a cause that feeds this node, Right to the node it feeds, Up and Down to the neighbour in the same step. */
  function arrows(e: KeyboardEvent, n: CauseNodeModel) {
    if (!model || !layout || !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return;
    let target: string | undefined;
    if (e.key === 'ArrowLeft') target = model.edges.find((x) => x.to === n.id && !x.selfLoop)?.from;
    else if (e.key === 'ArrowRight') target = model.edges.find((x) => x.from === n.id && !x.selfLoop)?.to;
    else {
      const col = layout.nodes.filter((p) => p.depth === n.depth).sort((a, b) => a.y - b.y);
      const i = col.findIndex((p) => p.id === n.id);
      target = col[i + (e.key === 'ArrowDown' ? 1 : -1)]?.id;
    }
    if (!target) return;
    e.preventDefault();
    root?.querySelector<HTMLElement>(`[data-testid="cause-node"][data-id="${CSS.escape(target)}"]`)?.focus();
  }

  const lagChip = (e: CauseEdgeModel): string => (e.lagMonths > 0 ? ` ${t('lag_short', { n: e.lagMonths })}` : '');
  const nodeAria = (n: CauseNodeModel): string =>
    [n.label, n.value ? `${n.value}${n.trendWord ? `, ${n.trendWord}` : ''}` : t('not_published', { reason: n.unpublished ?? '' }), n.netLabel ? t('net_on', { quantity: model?.focus.label ?? '', sign: n.netLabel }) : t('focus'), ...n.loops.map((l) => `${l.label} ${l.count}`), n.startLabel ?? ''].filter(Boolean).join('. ');
  const canEdit = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !$gs.snapshot?.gameOver && !$gs.demo);
  const openLever = (key: string) => { if (canEdit) setUi({ causes: null, drawerOpen: true, focusLever: key as never }); };
</script>

{#if cs && model && layout}
  <Modal title={t('title', { quantity: model.focus.label })} size="lg" onclose={close}>
    <div class="cm" bind:this={root} data-testid="causes-modal" data-focus-id={model.focus.id}>
      <p class="intro">{t('intro', { quantity: model.focus.label })}</p>

      <div class="bar" role="toolbar" aria-label={t('depth')}>
        <button type="button" class="btn" onclick={goBack} disabled={cs.trail.length === 0} data-testid="causes-back">{t('back')}{cs.trail.length ? ` (${label(cs.trail[cs.trail.length - 1]!)})` : ''}</button>
        <span class="lbl" id="depth-l">{t('depth')}</span>
        <span class="seg" role="group" aria-labelledby="depth-l">
          {#each Array.from({ length: g.upstreamDepth }, (_, i) => i + 1) as n (n)}
            <button type="button" class:on={d === n} aria-pressed={d === n} aria-label={t('depth_aria', { n })} data-testid="depth-{n}" onclick={() => (depth = n)}>{n}</button>
          {/each}
        </span>
        <button type="button" class="btn" onclick={() => (listPick = !list)} data-testid="causes-toggle-list">{list ? t('show_graph') : t('show_list')}</button>
        {#if model.noneHidden > 0 || showNone}
          <label class="chk"><input type="checkbox" bind:checked={showNone} data-testid="causes-show-none" /> {t('show_none', { n: model.noneHidden })}</label>
        {/if}
      </div>
      <p class="dim small">{t('depth_note')}</p>

      <ul class="legend" aria-label={t('legend')} data-testid="causes-legend">
        {#each ['up', 'down', 'varies', 'none'] as s (s)}
          <li><svg width="30" height="10" aria-hidden="true"><line x1="1" y1="5" x2="29" y2="5" class="ln {s}" /></svg><span class="glyph {s}" aria-hidden="true">{s === 'up' ? '+' : s === 'down' ? '-' : s === 'varies' ? '+/-' : '0'}</span> {t(`legend_${s}`)}</li>
        {/each}
        {#each g.loopKinds as k (k.id)}<li title={k.text}><span class="lpb {k.id}" aria-hidden="true">{k.badge}</span> {k.label}</li>{/each}
      </ul>

      <div class="live" aria-live="polite" class:sr-only={true} data-testid="causes-announce">{announce}</div>

      {#if model.nodes.length === 1}
        <p class="empty" data-testid="causes-empty">{t('no_causes', { quantity: model.focus.label })}</p>
      {:else if !list}
        <div class="wrap" bind:this={wrap} role="group" aria-label={t('graph_aria', { quantity: model.focus.label })} data-testid="causes-graph">
          <div class="canvas" style="width:{layout.width}px;height:{layout.height}px">
            <svg width={layout.width} height={layout.height} aria-hidden="true">
              <defs>
                {#each ['up', 'down', 'varies', 'none'] as s (s)}<marker id="ah-{s}" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0 L8 4 L0 8 z" class="ah {s}" /></marker>{/each}
              </defs>
              {#each model.edges.filter((e) => !e.selfLoop) as e (e.key)}
                {@const r = route(layout, e)}
                {#if r}<path d={r.d} class="ln {e.sign}" class:act={active === e.key} marker-end="url(#ah-{e.sign})" data-testid="cause-line" data-key={e.key} />{/if}
              {/each}
            </svg>
            {#each model.nodes as n (n.id)}
              {@const p = layout.byId.get(n.id)!}
              <button type="button" class="node {n.netSign ?? 'focus'}" class:foc={n.focus} class:leaf={!n.hasCauses && !n.focus} style="left:{p.x}px;top:{p.y}px;width:{p.w}px;height:{p.h}px"
                data-testid="cause-node" data-id={n.id} data-depth={n.depth} data-focus={n.focus ? 'true' : undefined} data-autofocus={n.focus ? '' : undefined}
                aria-label={nodeAria(n)} aria-current={n.focus ? 'true' : undefined} aria-disabled={n.focus || !n.hasCauses ? 'true' : undefined}
                title={n.netLabel ? `${n.label}: ${t('net_on', { quantity: model.focus.label, sign: n.netLabel })}` : n.label}
                onclick={() => centreOn(n)} onkeydown={(e) => arrows(e, n)}>
                <span class="nl">{n.label}</span>
                <span class="nv">{#if n.value}{n.value}{#if n.trendArrow} <span class="ta" aria-hidden="true">{n.trendArrow}</span><span class="sr-only"> {n.trendWord}</span>{/if}{:else}<span class="dim">{t('not_published', { reason: n.unpublished ?? '' })}</span>{/if}</span>
                <span class="nb">
                  {#if n.netLabel}<span class="net {n.netSign}" title={t('net_on', { quantity: model.focus.label, sign: n.netLabel })}>{n.netLabel}</span>{/if}
                  {#each n.loops as l (l.kind)}<span class="lpb {l.kind}" title={t('loop_badge_in', { kind: l.label.toLowerCase() })}>{l.badge}</span>{/each}
                  {#if n.selfLoop}<span class="self" title={t('feeds_itself')} aria-hidden="true">&#8635;</span>{/if}
                  {#if n.lever}<span class="lev" title={t('lever_lead', { when: n.lever.leadText })}>{n.lever.label}</span>{:else if n.startLabel && !n.focus}<span class="st" title={t('start_point')}>{n.startLabel}</span>{/if}
                </span>
              </button>
              {#each model.edges.filter((e) => e.from === n.id && !e.selfLoop) as e (e.key)}
                {@const r = chips.get(e.key)}
                {#if r}
                  <button type="button" class="chip {e.sign}" style="left:{r.x}px;top:{r.y}px" data-testid="cause-edge" data-key={e.key} data-sign={e.sign}
                    aria-label={`${e.sentence} ${e.lagLabel}`} title={e.sentence}
                    onmouseenter={() => (active = e.key)} onmouseleave={() => (active = null)} onfocus={() => (active = e.key)} onblur={() => (active = null)}>
                    <span class="gl">{e.glyph}</span>{#if e.lagMonths > 0}<span class="lg">{lagChip(e)}</span>{/if}
                  </button>
                {/if}
              {/each}
            {/each}
          </div>
        </div>
        <div class="detail" data-testid="causes-detail" aria-live="polite">
          {#if activeEdge}
            <b class="glyph {activeEdge.sign}">{activeEdge.glyph}</b> {activeEdge.sentence} <span class="dim">({activeEdge.lagLabel}; {t('via', { via: activeEdge.viaLabel })}). {activeEdge.checked}</span>
          {:else}<span class="dim">{t('how_to')}</span>{/if}
        </div>
      {:else}
        <ol class="items" aria-label={t('list_aria', { quantity: model.focus.label })} data-testid="causes-list">
          {#each model.nodes as n (n.id)}
            <li data-testid="cause-item" data-id={n.id} data-depth={n.depth}>
              {#if n.hasCauses && !n.focus}<button type="button" class="lk" onclick={() => centreOn(n)}>{n.label}</button>{:else}<b>{n.label}</b>{/if}
              <span class="dim">{n.value ?? t('not_published', { reason: n.unpublished ?? '' })}{#if n.trendWord}, {n.trendWord}{/if}.</span>
              {#if n.netLabel}<span class="net {n.netSign}">{t('net_on', { quantity: model.focus.label, sign: n.netLabel })}.</span>{/if}
              {#each n.loops as l (l.kind)}<span class="lpb {l.kind}" aria-hidden="true">{l.badge}</span><span class="sr-only">{t('loop_badge_in', { kind: l.label.toLowerCase() })}</span>{/each}
              {#if n.startLabel && !n.focus}<span class="dim">{n.startLabel}.</span>{/if}
              {#if n.lever}<span class="lev">{t('lever_lead', { when: n.lever.leadText })}</span>{/if}
              {#if n.selfLoop}<span class="dim">{t('feeds_itself')}.</span>{/if}
              {#if model.edges.some((e) => e.from === n.id && !e.selfLoop)}
                <ul>{#each model.edges.filter((e) => e.from === n.id && !e.selfLoop) as e (e.key)}
                  <li data-testid="cause-line-item" data-key={e.key}><span class="glyph {e.sign}">{e.glyph}</span> <b>{label(e.from)}</b> &rarr; <b>{label(e.to)}</b>: {e.sentence} <span class="dim">({e.lagLabel}; {t('via', { via: e.viaLabel })})</span></li>
                {/each}</ul>
              {/if}
            </li>
          {/each}
        </ol>
      {/if}

      {#if model.undrawn.length || model.edges.some((e) => e.selfLoop)}
        <details class="others" open={list}>
          <summary>{t('other_links')}</summary>
          {#if model.undrawn.length}<p class="dim small">{t('hidden_links', { n: model.undrawn.length })}</p>{/if}
          <ul>
            {#each [...model.undrawn, ...model.edges.filter((e) => e.selfLoop)] as e (e.key)}
              <li data-testid="cause-other" data-key={e.key}><span class="glyph {e.sign}">{e.glyph}</span> <b>{label(e.from)}</b> &rarr; <b>{label(e.to)}</b>: {e.sentence} <span class="dim">({e.lagLabel}; {t('via', { via: e.viaLabel })})</span></li>
            {/each}
          </ul>
        </details>
      {/if}

      {#if model.nodes.some((n) => n.netSign === 'mixed')}<p class="dim small" data-testid="causes-mixed-note">{g.netSigns.find((s) => s.id === 'mixed')?.text}</p>{/if}

      <section aria-labelledby="lv-h" data-testid="causes-levers">
        <h3 id="lv-h">{t('levers_title')}</h3>
        {#if model.levers.length === 0}<p class="dim">{t('levers_none')}</p>
        {:else}
          <p class="dim small">{t('levers_intro')}</p>
          <ul class="levs">
            {#each model.levers as l (l.node.id)}
              <li><button type="button" class="lk" data-testid="open-lever" data-lever={l.lever.key} disabled={!canEdit} title={canEdit ? undefined : t('lever_locked')} onclick={() => openLever(l.lever.key)}>{t('open_lever', { lever: l.lever.label })}</button>
                <span class="dim block">{l.node.netLabel ? `${t('net_on', { quantity: model.focus.label, sign: l.node.netLabel })}. ` : ''}{t('lever_lead', { when: l.lever.leadText })}</span></li>
            {/each}
          </ul>
        {/if}
      </section>

      <section aria-labelledby="lp-h" data-testid="causes-loops">
        <h3 id="lp-h">{t('loops_title', { quantity: model.focus.label })}</h3>
        {#if model.loops.length === 0}<p class="dim">{t('loops_none', { quantity: model.focus.label, max: g.loopMaxLen })}</p>
        {:else}
          <p class="dim small">{t('loops_note', { n: Math.min(5, model.loops.length), total: model.loops.length, max: g.loopMaxLen })}</p>
          <details class="kinds"><summary class="dim small">{t('kinds_what')}</summary><ul>{#each g.loopKinds as k (k.id)}<li class="small"><span class="lpb {k.id}">{k.badge}</span> <b>{k.label}</b>: {k.text}</li>{/each}</ul></details>
          <ul class="loops">
            {#each model.loops.slice(0, 5) as l (l.id)}
              <li data-testid="cause-loop" data-kind={l.kind}><span class="lpb {l.kind}" title={l.kindLabel}>{l.badge}</span> <span class="small">{l.kindLabel}</span> <b>{l.title}</b>
                <p class="small">{l.sentence} <span class="dim">{l.delay}</span></p></li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
    {#snippet footer()}<span class="dim small" data-testid="pressure-note">{$gs.mode === 'pressure' ? PRESSURE_NOTE : ''}</span><Button variant="primary" onclick={close}>{t('close_button')}</Button>{/snippet}
  </Modal>
{/if}

<style>
  .cm { display: flex; flex-direction: column; gap: 8px; font-size: 13px; min-width: 0; }
  .intro { margin: 0; color: var(--text-dim); }
  .dim { color: var(--text-dim); } .small { font-size: 12px; margin: 0; }
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .btn { font: inherit; padding: 3px 10px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--surface); color: var(--text); cursor: pointer; } .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .lbl { font-size: 12px; color: var(--text-dim); }
  .seg { display: inline-flex; } .seg button { font: inherit; border: 1px solid var(--border); background: var(--surface); color: var(--text); padding: 3px 10px; cursor: pointer; min-width: 32px; }
  .seg button:first-child { border-radius: var(--radius) 0 0 var(--radius); } .seg button:last-child { border-radius: 0 var(--radius) var(--radius) 0; } .seg button.on { background: var(--accent); color: var(--accent-text); }
  .chk { font-size: 12px; display: inline-flex; gap: 4px; align-items: center; }
  .legend { list-style: none; margin: 0; padding: 5px 8px; display: flex; flex-wrap: wrap; gap: 3px 16px; font-size: 11.5px; background: var(--surface-2); border-radius: var(--radius); }
  .legend li { display: flex; align-items: center; gap: 6px; }
  .glyph { font-family: var(--font-mono); font-weight: 700; min-width: 1.6em; text-align: center; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .empty { padding: 10px; background: var(--surface-2); border-radius: var(--radius); margin: 0; }

  .wrap { overflow: auto; max-height: 52vh; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); }
  .canvas { position: relative; }
  svg { position: absolute; left: 0; top: 0; overflow: visible; }
  .ln { fill: none; stroke-width: 1.8; stroke: var(--series-1); }
  .ln.down { stroke: var(--series-2); stroke-dasharray: 7 4; }
  .ln.varies { stroke: var(--warn); stroke-dasharray: 2 4; stroke-linecap: round; }
  .ln.none { stroke: var(--muted-bar); stroke-width: 1; stroke-dasharray: 1 5; stroke-linecap: round; opacity: 0.8; }
  .ln.act { stroke-width: 3.4; }
  .ah { fill: var(--series-1); } .ah.down { fill: var(--series-2); } .ah.varies { fill: var(--warn); } .ah.none { fill: var(--muted-bar); }
  .legend .ln { stroke-width: 2; }

  .node { position: absolute; box-sizing: border-box; text-align: left; font: inherit; font-size: 12px; background: var(--surface); color: var(--text); border: 1.5px solid var(--border); border-radius: var(--radius);
    padding: 4px 6px; display: flex; flex-direction: column; gap: 1px; overflow: hidden; cursor: pointer; z-index: 1; }
  .node:hover { background: var(--surface-2); }
  .node.leaf { cursor: default; }
  .node.foc { border-color: var(--accent); border-width: 2.5px; background: var(--info-bg); cursor: default; }
  .node.mixed { border-style: double; border-width: 4px; } .node.varies { border-style: dotted; } .node.none { border-style: dashed; opacity: 0.85; }
  .nl { font-weight: 600; line-height: 1.15; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .nv { font-family: var(--font-mono); font-size: 11px; line-height: 1.2; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .nv .dim { font-family: var(--font); font-size: 10.5px; }
  .ta { font-size: 9px; }
  .nb { display: flex; flex-wrap: wrap; gap: 3px; align-items: center; margin-top: auto; font-size: 10px; }
  .net { border: 1px solid var(--border); border-radius: 8px; padding: 0 5px; font-size: 10px; white-space: nowrap; }
  .net.up { border-style: solid; } .net.down { border-style: dashed; } .net.varies, .net.mixed { border-style: dotted; }
  .st, .lev { border: 1px solid var(--border); border-radius: 3px; padding: 0 4px; color: var(--text-dim); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lev { color: var(--accent); border-color: var(--accent); }
  .self { font-size: 12px; }
  .lpb { display: inline-block; min-width: 1.5em; text-align: center; font-weight: 700; font-size: 11px; padding: 0 3px; border: 1.5px solid currentColor; line-height: 1.3; }
  .lpb.R { border-radius: 50%; } .lpb.B { border-radius: 2px; } .lpb.depends { border-radius: 2px; transform: rotate(45deg) scale(0.75); }

  .chip { position: absolute; transform: translate(-50%, -50%); z-index: 2; font: inherit; font-size: 11px; line-height: 1; padding: 2px 5px; background: var(--surface); color: var(--text); border: 1.5px solid var(--series-1); border-radius: 9px; cursor: help; white-space: nowrap; }
  .chip.down { border-color: var(--series-2); border-style: dashed; } .chip.varies { border-color: var(--warn); border-style: dotted; } .chip.none { border-color: var(--muted-bar); border-style: dotted; color: var(--text-dim); }
  .chip .gl { font-family: var(--font-mono); font-weight: 700; } .chip .lg { color: var(--text-dim); font-size: 10px; }
  .chip:hover, .chip:focus-visible { background: var(--info-bg); z-index: 5; }
  .detail { min-height: 2.6em; padding: 6px 8px; background: var(--surface-2); border-radius: var(--radius); font-size: 12.5px; }

  .items, .items ul, .others ul, .levs, .loops { margin: 0; padding-left: 18px; } .items > li { margin: 6px 0; } .items ul { margin: 2px 0; font-size: 12px; } li { margin: 2px 0; }
  .block { display: block; }
  .lk:disabled { color: var(--text-dim); cursor: not-allowed; }
  .lk { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; }
  .others summary { cursor: pointer; font-size: 12px; color: var(--text-dim); }
  section h3 { margin: 6px 0 2px; } .loops { list-style: none; padding: 0; } .loops p { margin: 2px 0 0 30px; }
  @media (prefers-reduced-motion: no-preference) { .node, .chip { transition: background 0.12s; } .ln { transition: stroke-width 0.12s; } }
  @media (max-width: 900px) { .wrap { max-height: 46vh; } }
</style>
