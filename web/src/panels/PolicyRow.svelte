<script lang="ts">
  import type { CommandPreview, LeverInfo, SystemKey } from '../contract';
  import { Segmented, Slider } from '../ui-kit';
  import { leverCostHint } from './costs';
  import { formatLever, leadText, lobbyingDiffers, METRIC_LABEL, SYSTEM_NAME } from './format';
  import HowButton from './HowButton.svelte';
  import { arrowGlyph, arrowsShown, blockText, lobbyNote, toNum } from './policy';

  let { lever, nominal, effective, disabled, system, draftValue, preview, onset, onreset }: {
    lever: LeverInfo; nominal: number | boolean | string; effective: number | null; disabled: string | null; system: SystemKey;
    draftValue: number | undefined; preview: CommandPreview | undefined; onset: (v: number) => void; onreset: () => void;
  } = $props();

  const current = $derived(toNum(lever, nominal));
  const shown = $derived(draftValue ?? current);
  const id = $derived(`lever-${lever.key}`);
  const note = $derived(lobbyNote(lever, current, shown, preview));
  type Opt = { value: string; label: string; disabled?: boolean; title?: string };
  const options = $derived<Opt[]>(lever.unit === 'bool' ? [{ value: '0', label: 'Off' }, { value: '1', label: 'On' }]
    : (lever.options ?? []).map((o, i) => {
        const where = lever.optionAvailableIn?.[o];
        const off = !!where && !where.includes(system);
        return { value: String(i), label: o[0]!.toUpperCase() + o.slice(1), disabled: off, title: off ? `Needs ${where!.map((s) => SYSTEM_NAME[s]).join(' or ')}` : undefined };
      }));
  const shown2 = $derived(arrowsShown(lever, current, draftValue));
  const basisText = $derived(shown2.basis === 'raise' ? 'If raised:' : shown2.basis === 'raised' ? 'Your change (up):' : 'Your change (down):');
  const hiddenOptions = $derived(options.filter((o) => o.disabled));
</script>

{#snippet resetBtn()}
  {#if draftValue !== undefined}<button class="reset" onclick={onreset} aria-label="Reset {lever.label} to {formatLever(lever, nominal)}">Reset</button>{/if}
{/snippet}

<li data-lever={lever.key} class:drafted={draftValue !== undefined} class:off={!!disabled}>
  {#if lever.unit === 'bool' || lever.unit === 'enum'}
    <div class="top"><span class="name">{lever.label}</span>{@render resetBtn()}</div>
    <Segmented label={lever.label} {options} value={String(shown)} disabled={!!disabled} onchange={(v) => onset(Number(v))} />
  {:else}
    <Slider {id} label={lever.label} value={shown} min={lever.min} max={lever.max} step={lever.step} disabled={!!disabled}
      format={(v) => formatLever(lever, v)} onchange={onset} />
  {/if}

  <div class="meta">
    {#if lever.unit !== 'bool' && lever.unit !== 'enum'}{@render resetBtn()}{/if}
    <HowButton anchor={lever.key} label={lever.label} />
    <span>Now <b>{formatLever(lever, nominal)}</b>{#if effective !== null && lever.lobbied && typeof nominal === 'number' && lobbyingDiffers(nominal, effective)} &nbsp;(about {formatLever(lever, effective)} in force){/if}</span>
    <span title="An estimate. The exact price (including lobbying opposition) appears once you draft the change.">{leverCostHint(lever)}; in force {leadText(lever.leadTime)}</span>
  </div>
  {#if hiddenOptions.length}<div class="why-off" role="note">{hiddenOptions.map((o) => `${o.label}: ${o.title ?? ''}`).join('. ')}.</div>{/if}
  {#if shown2.arrows.length}
    <div class="arrows" aria-label="Expected effects">
      <span class="dim">{basisText}</span>
      {#each shown2.arrows as a (a.metric)}
        <span class="chip"><span aria-hidden="true">{arrowGlyph(a.direction)}</span> {METRIC_LABEL[a.metric]}<span class="sr-only"> {a.direction === 'up' ? 'rises' : 'falls'}</span></span>
      {/each}
    </div>
  {/if}
  {#if disabled}<div class="why-off" role="note">{disabled}</div>{/if}
  {#if preview}
    <div class="draft" class:bad={preview.blocked}>
      {#if preview.blocked}Blocked: {blockText(preview.blocked)}{:else}Draft: costs {Math.round(preview.pc * 10) / 10} PC{#if preview.opposition > 0} (includes {Math.round(preview.opposition * 100)}% lobbying opposition){/if}, in force {leadText(preview.leadTime)}{/if}
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
