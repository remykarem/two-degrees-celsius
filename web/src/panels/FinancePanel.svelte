<script lang="ts">
  import type { AccountRef } from '../contract';
  import type { Game } from '../store';
  import { capitalGauge, financeEmpty, financeSections, fmtFin } from './finance';
  import { accountLabel } from './ledger';
  import CircularFlow from './CircularFlow.svelte';
  import Sparkline from './Sparkline.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const snap = $derived($gs.snapshot);
  const sections = $derived(snap ? financeSections(snap) : []);
  const empty = $derived(snap ? financeEmpty(snap.finance) : true);
  const gauge = $derived(snap ? capitalGauge(snap.finance.bank) : null);
  const open = (a: AccountRef, kind?: Parameters<Game['openLedger']>[1]) => { const sel = a.kind === 'government' ? { kind: 'government' as const } : a.kind === 'bank' ? { kind: 'bank' as const } : a.kind === 'household' ? { kind: 'household' as const, id: a.id } : a.kind === 'firm' ? { kind: 'firm' as const, id: a.id } : null; if (sel) game.select(sel); game.openLedger(a, kind); };
</script>

{#if snap}
  <div class="fin" data-testid="finance-panel">
    <p class="intro">Dollars. <b>Month</b> is the latest month; <b>Quarter</b> is the last three months added up. Click "ledger ›" (see transactions) to trace a figure to the ledger it comes from.{#if snap.mock} <span class="mockn">Mock sim: invented numbers, consistent ledger.</span>{/if}</p>
    <div class="entry" role="group" aria-label="Open a ledger">
      <button type="button" onclick={() => open({ kind: 'government' })}>Government ledger</button>
      <button type="button" onclick={() => open({ kind: 'bank' })}>Bank ledger</button>
    </div>
    {#if empty}<p class="dim">No month has been played yet, so monthly flows are zero. Play a turn to fill this in.</p>{/if}

    {#each sections as sec (sec.id)}
      <section aria-labelledby="fin-{sec.id}">
        <h3 id="fin-{sec.id}">{sec.title}</h3>
        {#if sec.id === 'bank' && gauge}
          <div class="gauge {gauge.status}" role="meter" aria-label="Capital ratio against its limit" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(gauge.at * 100)} aria-valuetext="Capital ratio {fmtFin(snap.finance.bank.capitalRatio, 'pct')}, limit {fmtFin(snap.finance.bank.capitalRatioLimit, 'pct')}. {gauge.text}" data-testid="capital-gauge">
            <div class="track"><div class="fill" style:width="{gauge.at * 100}%"></div><div class="lim" style:left="{gauge.limitAt * 100}%" title="Limit"></div></div>
            <div class="gl"><span>Capital ratio {fmtFin(snap.finance.bank.capitalRatio, 'pct')}</span><span>limit {fmtFin(snap.finance.bank.capitalRatioLimit, 'pct')}</span></div>
            <div class="gt">{gauge.text}</div>
          </div>
        {/if}
        <table>
          <thead><tr><th scope="col"><span class="sr-only">Figure</span></th><th scope="col" class="n">Month</th><th scope="col" class="n">Quarter</th><th scope="col"><span class="sr-only">Trend and ledger</span></th></tr></thead>
          <tbody>
            {#each sec.rows as r (r.id)}
              <tr class:tot={r.id.endsWith('-in') || r.id.endsWith('-out')}>
                <th scope="row" title={r.hint}>{r.label}</th>
                <td class="n">{fmtFin(r.month, r.fmt)}</td>
                <td class="n q">{r.stock ? '' : r.quarter === null ? 'n/a' : fmtFin(r.quarter, r.fmt)}</td>
                <td class="x">
                  {#if r.spark && r.spark.length > 1}<Sparkline label={r.label} values={r.spark} w={34} h={16} />{/if}
                  {#if r.ledger}<button type="button" class="see" aria-label="See transactions for {r.label}" onclick={() => open(r.ledger!.account, r.ledger!.kind)} title="See transactions">ledger ›</button>{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if sec.ledgers}
          <div class="entry small" role="group" aria-label="Ledgers of each {sec.id === 'households' ? 'household' : 'firm'}">
            {#each sec.ledgers as l (l.label)}<button type="button" onclick={() => open(l.account)} title="See transactions: {accountLabel(snap, l.account)}">{l.label}</button>{/each}
          </div>
        {/if}
      </section>
    {/each}

    <section aria-labelledby="fin-flow"><h3 id="fin-flow">Circular flow, last quarter</h3><CircularFlow finance={snap.finance} /></section>
  </div>
{/if}

<style>
  .fin { display: flex; flex-direction: column; gap: 10px; }
  .fin { min-width: 0; }
  .intro, .dim { font-size: 12px; color: var(--text-dim); margin: 0; }
  .mockn { color: var(--warn); }
  .entry { display: flex; flex-wrap: wrap; gap: 4px; }
  .entry button { font: inherit; font-size: 12px; background: var(--surface); color: var(--accent); border: 1px solid var(--accent); border-radius: var(--radius); padding: 2px 8px; cursor: pointer; }
  .entry.small { margin-top: 4px; } .entry.small button { font-size: 11px; padding: 1px 6px; border-color: var(--border); }
  .entry button:hover { background: var(--surface-2); }
  section h3 { margin-bottom: 3px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; table-layout: fixed; }
  th:first-child { width: 33%; } th.n { width: 19%; } thead th:last-child { width: 29%; }
  th[scope='row'] { overflow-wrap: anywhere; }
  th[scope='row'] { text-align: left; font-weight: 400; padding: 2px 4px 2px 0; }
  thead th { font-size: 10px; text-transform: uppercase; color: var(--text-dim); font-weight: 600; }
  td, thead th { padding: 2px 3px; }
  .n { text-align: right; font-family: var(--font-mono); white-space: nowrap; }
  .q { color: var(--text-dim); }
  tr { border-top: 1px dotted var(--border); }
  tr.tot th, tr.tot td { font-weight: 700; }
  .x { text-align: right; white-space: nowrap; }
  .x :global(svg) { vertical-align: middle; margin-right: 3px; }
  .see { border: 0; background: transparent; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0; font: inherit; font-size: 10.5px; }
  .gauge { margin: 2px 0 6px; }
  .track { position: relative; height: 12px; background: var(--surface-2); border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
  .fill { height: 100%; background: var(--good); }
  .tight .fill { background: var(--warn); } .breach .fill { background: var(--bad); }
  .lim { position: absolute; top: -2px; bottom: -2px; width: 2px; background: var(--text); }
  .gl { display: flex; justify-content: space-between; font-size: 11px; color: var(--text-dim); }
  .gt { font-size: 11px; }
  .breach .gt { color: var(--bad); font-weight: 700; } .tight .gt { color: var(--warn); }
</style>
