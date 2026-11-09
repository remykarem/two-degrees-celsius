<script lang="ts">
  import type { Game } from '../store';
  import { deriveCrises } from './crisis';
  import { setUi } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let dismissed = $state<Record<string, number>>({});
  const turn = $derived($gs.snapshot?.turn ?? 0);
  const crises = $derived(deriveCrises($gs.snapshot, $gs.lastResult));
  const visible = $derived($gs.phase === 'ended' ? [] : crises.filter((c) => !(c.severity === 'warning' && dismissed[c.id] === turn)));
  const hasCritical = $derived(visible.some((c) => c.severity === 'critical'));
</script>

<div class="banners" aria-live={hasCritical ? 'assertive' : 'polite'} role={hasCritical ? 'alert' : 'status'}>
  {#each visible as c (c.id)}
    <div class="crisis {c.severity}">
      <span class="icon" aria-hidden="true">{c.severity === 'critical' ? '⛔' : '⚠'}</span>
      <span class="sev">{c.severity === 'critical' ? 'Crisis' : 'Warning'}</span>
      <strong>{c.title}.</strong>
      <span class="detail">{c.detail}</span>
      {#if c.why}<button class="link" onclick={() => setUi({ why: { kind: c.why! } })}>why?</button>{/if}
      {#if c.severity === 'warning'}<button class="x" aria-label="Dismiss warning for this turn" onclick={() => (dismissed[c.id] = turn)}>✕</button>{/if}
    </div>
  {/each}
</div>

<style>
  .banners { grid-column: 1 / -1; }
  .crisis { display: flex; gap: 8px; align-items: baseline; padding: 6px 14px; border-bottom: 1px solid var(--border); font-size: 13px; }
  .critical { background: var(--bad-bg); border-left: 6px solid var(--bad); }
  .warning { background: var(--warn-bg); border-left: 6px solid var(--warn); }
  .sev { text-transform: uppercase; font-size: 11px; font-weight: 700; letter-spacing: 0.05em; }
  .detail { flex: 1; }
  .link { border: 0; background: transparent; color: var(--accent); text-decoration: underline; cursor: pointer; }
  .x { border: 0; background: transparent; color: var(--text); cursor: pointer; }
</style>
