<script lang="ts">
  import type { CommandPreview, LeverInfo, SystemKey } from '../contract';
  import { Slider } from '../ui-kit';
  import { formatLever, leadText, METRIC_LABEL, REASON_TEXT } from './format';

  let { lever, nominal, effective, system, draftValue, preview, onset }: {
    lever: LeverInfo; nominal: number | boolean | string; effective: number | null; system: SystemKey;
    draftValue: number | undefined; preview: CommandPreview | undefined; onset: (v: number) => void;
  } = $props();

  const nominalNum = $derived(typeof nominal === 'boolean' ? (nominal ? 1 : 0) : typeof nominal === 'string' ? Math.max(0, lever.options?.indexOf(nominal) ?? 0) : nominal);
  const shown = $derived(draftValue ?? nominalNum);
  const available = $derived(lever.availableIn.includes(system));
  const id = $derived(`lever-${lever.key}`);
  const lobbyNote = $derived(lever.lobbied && preview && !preview.blocked && preview.opposition > 0
    ? `You set ${formatLever(lever, shown)}; expect about ${formatLever(lever, nominalNum + (shown - nominalNum) * (1 - preview.opposition))} in force (lobbying)` : '');
</script>

<li class:drafted={draftValue !== undefined}>
  {#if !available}
    <div class="name">{lever.label}</div>
    <div class="note">Not available in the {system} system</div>
  {:else if lever.unit === 'bool'}
    <label class="check"><input type="checkbox" checked={shown === 1} onchange={(e) => onset((e.currentTarget as HTMLInputElement).checked ? 1 : 0)} /> {lever.label}</label>
  {:else if lever.unit === 'enum'}
    <label class="check" for={id}>{lever.label}</label>
    <select {id} value={String(shown)} onchange={(e) => onset(Number((e.currentTarget as HTMLSelectElement).value))}>
      {#each lever.options ?? [] as o, i (o)}<option value={String(i)}>{o}</option>{/each}
    </select>
  {:else}
    <Slider {id} label={lever.label} value={shown} min={lever.min} max={lever.max} step={lever.step}
      format={(v) => formatLever(lever, v)} onchange={onset} />
  {/if}
  {#if available}
    <div class="meta">
      <span>Now {formatLever(lever, nominal)}{#if effective !== null}&nbsp;(in force ~{formatLever(lever, effective)}){/if}</span>
      <span>Base {lever.baseCostPc} PC · {leadText(lever.leadTime)}{lever.lobbied ? ' · lobbied' : ''}</span>
      <span class="arrows">{#each lever.arrows as a (a.metric)}<span title="{METRIC_LABEL[a.metric]} {a.direction === 'up' ? 'rises' : 'falls'}">{a.direction === 'up' ? '▲' : '▼'} {METRIC_LABEL[a.metric]}</span>{/each}</span>
    </div>
    {#if preview}
      <div class="draft" class:bad={preview.blocked}>
        {#if preview.blocked}Blocked: {REASON_TEXT[preview.blocked]}{:else}Draft: {preview.pc} PC, takes effect {leadText(preview.leadTime)}{/if}
      </div>
    {/if}
    {#if lobbyNote}<div class="note">{lobbyNote}</div>{/if}
  {/if}
</li>

<style>
  li { padding: 8px 0; border-bottom: 1px solid var(--border); list-style: none; }
  li.drafted { background: var(--surface-2); margin: 0 -8px; padding: 8px; border-radius: var(--radius); }
  .meta { display: flex; flex-wrap: wrap; gap: 2px 12px; font-size: 12px; color: var(--text-dim); margin-top: 2px; }
  .arrows span { margin-right: 8px; }
  .draft { font-size: 12px; font-weight: 600; margin-top: 2px; }
  .draft.bad { color: var(--bad); }
  .note { font-size: 12px; color: var(--warn); }
  .name { font-weight: 600; }
  .check { font-weight: 600; }
</style>
