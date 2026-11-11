<script lang="ts">
  import type { SystemKey } from '../contract';
  import type { Game } from '../store';
  import { totalPc } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { catalog } from '../store/catalog';
  import { leadText, pct } from './format';
  import { paramText, switchOptions } from './systemSwitch';
  import { setUi, ui } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let confirming = $state<SystemKey | null>(null);
  const snap = $derived($gs.snapshot);
  const pcFree = $derived((snap?.pc.current ?? 0) - totalPc($gs.draft.filter((d) => d.command.type !== 'switch_system')));
  const opts = $derived(snap ? switchOptions(snap, $gs.actions.find((a) => a.key === 'switch_system'), pcFree) : []);
  const close = () => { confirming = null; setUi({ systemModal: false }); };
  const conf = $derived(opts.find((o) => o.to === confirming));
  const cards = $derived(catalog().systems);
  const confCard = $derived(cards.find((c) => c.key === confirming));

  function confirm() {
    if (!confirming) return;
    game.draftCommand({ type: 'switch_system', to: confirming });
    close();
  }
</script>

{#if $ui.systemModal && snap}
  <Modal title={confirming ? 'Confirm the system switch' : 'Compare economic systems'} size="lg" onclose={() => (confirming ? (confirming = null) : close())}>
    {#if confirming && conf && confCard}
      <div class="confirm" role="alert">
        <h3>Switching to {confCard.name} is expensive</h3>
        <p>{conf.summary}.</p>
        <p>{confCard.resistance}{#if confCard.resistancePlaceholder} <span class="ph">placeholder</span>{/if}</p>
        <p>It takes effect {leadText(conf.leadTime)} after you end the turn, and you cannot start another switch while it runs. You will be asked once more when you end the turn.</p>
      </div>
    {:else}
      <p class="lead">The system sets who gets the profits, how strong lobbying is and which levers exist. Switching is slow and costs political capital. Costs and numbers below come from the simulation; the descriptions are authored in content/ui.json.</p>
      <div class="cols">
        {#each cards as c (c.key)}
          {@const k = c.key}
          {@const o = opts.find((x) => x.to === k)!}
          <section class:cur={o.current} aria-label={c.name}>
            <h3>{c.name}{#if o.current}<span class="badge">Current</span>{/if}</h3>
            <p class="tag">{c.tagline}</p>
            <dl>
              {#each c.facts as f (f.label)}<dt>{f.label}</dt><dd>{f.value}</dd>{/each}
              {#each c.params as p (p.path)}<dt>{p.label}</dt><dd>{paramText(p)}</dd>{/each}
            </dl>
            <h4>Transition</h4>
            {#if o.current}<p class="dim">You are here.</p>{:else}
              <ul>
                <li>Cost: about <b>{o.pcCost} PC</b>{#if o.moneyCost > 0}&nbsp;and ${Math.round(o.moneyCost).toLocaleString('en-US')}{/if}</li>
                <li>Starts {leadText(o.leadTime)}</li>
                <li>Takes {c.transitionMonths} months, {pct(c.outputPenalty)} less output during it</li>
              </ul>
            {/if}
            <h4>Who resists</h4>
            <p>{c.resistance}{#if c.resistancePlaceholder} <span class="ph">placeholder</span>{/if}</p>
            <h4>Trade-offs</h4>
            <ul>{#each c.tradeoffs as t (t.label)}<li><b>{t.label}:</b> {t.value}</li>{/each}</ul>
            {#if !o.current}
              <div class="act">
                <Button variant="primary" disabled={!!o.disabled} onclick={() => (confirming = k)}>Switch to {c.name}…</Button>
                {#if o.disabled}<p class="off">{o.disabled}</p>{/if}
              </div>
            {/if}
          </section>
        {/each}
      </div>
    {/if}
    {#snippet footer()}
      {#if confirming}
        <Button onclick={() => (confirming = null)}>Back</Button>
        <Button variant="primary" onclick={confirm}>Queue the switch</Button>
      {:else}<Button onclick={close}>Close</Button>{/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .lead { margin: 0 0 10px; color: var(--text-dim); }
  .cols { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
  section { border: 1px solid var(--border); border-radius: var(--radius); padding: 10px; display: flex; flex-direction: column; font-size: 13px; }
  section.cur { border: 2px solid var(--accent); background: var(--info-bg); }
  h3 { font-size: 14px; text-transform: none; letter-spacing: 0; color: var(--text); display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  h4 { margin: 8px 0 2px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-dim); }
  .tag { margin: 2px 0 6px; font-style: italic; }
  dl { margin: 0; display: grid; grid-template-columns: auto 1fr; gap: 2px 8px; }
  dt { color: var(--text-dim); } dd { margin: 0; }
  ul { margin: 0; padding-left: 16px; } p { margin: 2px 0; }
  .badge { font-size: 11px; background: var(--accent); color: var(--accent-text); border-radius: 8px; padding: 0 8px; }
  .ph { font-size: 10px; border: 1px dashed var(--warn); color: var(--warn); border-radius: 6px; padding: 0 4px; }
  .act { margin-top: auto; padding-top: 10px; }
  .off, .dim { color: var(--text-dim); font-size: 12px; }
  .confirm { background: var(--warn-bg); padding: 10px 14px; border-radius: var(--radius); }
  @media (max-width: 900px) { .cols { grid-template-columns: 1fr; } }
</style>
