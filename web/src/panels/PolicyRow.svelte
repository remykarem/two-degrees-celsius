<script lang="ts">
  import type { CommandPreview, LeverInfo } from '../contract';
  import { Segmented, Slider } from '../ui-kit';
  import { formatLever, leadText, METRIC_LABEL } from './format';
  import { arrowGlyph, blockText, lobbyNote, toNum } from './policy';

  let { lever, nominal, effective, disabled, draftValue, preview, onset, onreset }: {
    lever: LeverInfo; nominal: number | boolean | string; effective: number | null; disabled: string | null;
    draftValue: number | undefined; preview: CommandPreview | undefined; onset: (v: number) => void; onreset: () => void;
  } = $props();

  const current = $derived(toNum(lever, nominal));
  const shown = $derived(draftValue ?? current);
  const id = $derived(`lever-${lever.key}`);
  const note = $derived(lobbyNote(lever, current, shown, preview));
  const options = $derived(lever.unit === 'bool' ? [{ value: '0', label: 'Off' }, { value: '1', label: 'On' }] : (lever.options ?? []).map((o, i) => ({ value: String(i), label: o[0]!.toUpperCase() + o.slice(1) })));
</script>

{#snippet resetBtn()}
  {#if draftValue !== undefined}<button class="reset" onclick={onreset} aria-label="Reset {lever.label} to {formatLever(lever, nominal)}">Reset</button>{/if}
{/snippet}

<li class:drafted={draftValue !== undefined} class:off={!!disabled}>
  {#if lever.unit === 'bool' || lever.unit === 'enum'}
    <div class="top"><span class="name">{lever.label}</span>{@render resetBtn()}</div>
    <Segmented label={lever.label} {options} value={String(shown)} disabled={!!disabled} onchange={(v) => onset(Number(v))} />
  {:else}
    <Slider {id} label={lever.label} value={shown} min={lever.min} max={lever.max} step={lever.step} disabled={!!disabled}
      format={(v) => formatLever(lever, v)} onchange={onset} />
  {/if}

  <div class="meta">
    {#if lever.unit !== 'bool' && lever.unit !== 'enum'}{@render resetBtn()}{/if}
    <span>Now <b>{formatLever(lever, nominal)}</b>{#if effective !== null && lever.lobbied} &nbsp;(about {formatLever(lever, effective)} in force){/if}</span>
    <span>Base {lever.baseCostPc} PC, {leadText(lever.leadTime)}</span>
  </div>
  {#if lever.arrows.length}
    <div class="arrows" aria-label="Expected effects">
      <span class="dim">Expected:</span>
      {#each lever.arrows as a (a.metric)}
        <span class="chip"><span aria-hidden="true">{arrowGlyph(a.direction)}</span> {METRIC_LABEL[a.metric]}<span class="sr-only"> {a.direction === 'up' ? 'rises' : 'falls'}</span></span>
      {/each}
    </div>
  {/if}
  {#if disabled}<div class="why-off" role="note">{disabled}</div>{/if}
  {#if preview}
    <div class="draft" class:bad={preview.blocked}>
      {#if preview.blocked}Blocked: {blockText(preview.blocked)}{:else}Draft: costs {preview.pc} PC, in force {leadText(preview.leadTime)}{/if}
    </div>
  {/if}
  {#if note}<div class="lobby">{note}</div>{/if}
</li>

<style>
  li { padding: 8px 6px; border-bottom: 1px solid var(--border); list-style: none; }
  li.drafted { background: var(--info-bg); border-left: 3px solid var(--accent); border-radius: var(--radius); }
  li.off .name { color: var(--text-dim); }
  .top { display: flex; justify-content: space-between; align-items: center; min-height: 20px; }
  .name { font-weight: 600; }
  .reset { border: 1px solid var(--border); background: var(--surface); color: var(--text); border-radius: var(--radius); font-size: 11px; padding: 0 8px; cursor: pointer; }
  .meta { align-items: center; display: flex; flex-wrap: wrap; gap: 0 12px; font-size: 12px; color: var(--text-dim); margin-top: 3px; }
  .arrows { display: flex; flex-wrap: wrap; gap: 2px 6px; font-size: 12px; margin-top: 2px; align-items: center; }
  .chip { border: 1px solid var(--border); border-radius: 8px; padding: 0 6px; background: var(--surface); }
  .dim { color: var(--text-dim); }
  .draft { font-size: 12px; font-weight: 600; margin-top: 3px; }
  .draft.bad { color: var(--bad); }
  .lobby { font-size: 12px; margin-top: 2px; padding: 2px 6px; background: var(--warn-bg); border-radius: var(--radius); }
  .why-off { font-size: 12px; margin-top: 3px; color: var(--warn); }
</style>
