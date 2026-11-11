<script lang="ts">
  import type { Breakdown } from '../contract';
  import { popoverModel } from './explain';
  let { b }: { b: Breakdown } = $props();
  const m = $derived(popoverModel(b));
</script>

<div class="bt" data-testid="breakdown-{b.id}">
  <p class="formula"><span class="k">Formula</span> {m.formula}</p>
  <table>
    <caption class="k">Inputs (live values)</caption>
    <tbody>{#each m.inputs as i (i.label)}<tr><th scope="row">{i.label}</th><td class="n">{i.text}</td></tr>{/each}</tbody>
  </table>
  <p class="result"><span class="k">Result</span> <b data-testid="breakdown-result">{m.resultText}</b></p>
  {#if m.parts.length}
    <table>
      <caption class="k">Parts that add up to the result</caption>
      <tbody>{#each m.parts as p (p.label)}<tr><th scope="row">{p.label}{#if p.note}<span class="note"> ({p.note})</span>{/if}</th><td class="n">{p.text}</td></tr>{/each}</tbody>
      {#if m.sums}<tfoot><tr class:bad={!m.sums.ok}><th scope="row">Sum of parts</th><td class="n">{m.sums.sumText}</td></tr></tfoot>{/if}
    </table>
    {#if m.sums}<p class="sums" class:bad={!m.sums.ok} data-testid="sums-check">{m.sums.ok && m.sums.wireFlag !== false ? `Parts add up to the result (${m.sums.sumText}).` : `Parts do NOT add up: ${m.sums.sumText} against ${m.sums.resultText}${m.sums.wireFlag === false ? ' (the simulation says so too)' : ''}.`}</p>{/if}
  {/if}
</div>

<style>
  .bt { display: flex; flex-direction: column; gap: 8px; font-size: 13px; }
  .k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); font-weight: 600; margin-right: 6px; }
  caption.k { text-align: left; padding-bottom: 2px; }
  .formula, .result, .sums { margin: 0; }
  .formula { font-family: var(--font-mono); font-size: 12px; background: var(--surface-2); padding: 6px 8px; border-radius: var(--radius); }
  table { width: 100%; border-collapse: collapse; }
  th[scope='row'] { text-align: left; font-weight: 400; padding: 2px 6px 2px 0; }
  td.n { text-align: right; font-family: var(--font-mono); white-space: nowrap; }
  tr { border-top: 1px dotted var(--border); }
  tfoot th, tfoot td { font-weight: 700; }
  .note { color: var(--text-dim); font-size: 11px; }
  .result b { font-family: var(--font-mono); font-size: 16px; }
  .sums { color: var(--good); font-size: 12px; } .bad, .sums.bad { color: var(--bad); font-weight: 700; }
</style>
