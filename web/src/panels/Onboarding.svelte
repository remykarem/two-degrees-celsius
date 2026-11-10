<script lang="ts">
  import { onMount } from 'svelte';
  import type { Game } from '../store';
  import { Button } from '../ui-kit';
  import { COACH_STEPS } from './coach';
  import { markCoachDone, setUi, ui } from './ui';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const active = $derived($gs.mode === 'relaxed' && !$ui.startOpen && !$ui.coachDone && $gs.snapshot?.turn === $gs.snapshot?.game.firstTurn && ($gs.phase === 'review' || $gs.phase === 'plan') && !$ui.systemModal);
  const step = $derived(COACH_STEPS[$ui.coachStep] ?? COACH_STEPS[0]!);
  const last = $derived($ui.coachStep >= COACH_STEPS.length - 1);

  function highlight(target: string | null) {
    document.querySelectorAll('.coach-hl').forEach((el) => el.classList.remove('coach-hl'));
    if (target) document.querySelector(`[data-coach="${target}"]`)?.classList.add('coach-hl');
  }
  $effect(() => { highlight(active ? step.target : null); if (active && step.target === 'policies' && step.openDrawer) setUi({ drawerOpen: true }); });
  onMount(() => () => highlight(null));

  function finish() { markCoachDone(); setUi({ coachDone: true, coachStep: 0 }); highlight(null); }
  const go = (d: number) => setUi({ coachStep: Math.max(0, Math.min(COACH_STEPS.length - 1, $ui.coachStep + d)) });
</script>

{#if active}
  <section class="coach" aria-label="First-turn guide">
    <div class="meta">Step {$ui.coachStep + 1} of {COACH_STEPS.length}</div>
    <h3>{step.title}</h3>
    <p>{step.body}</p>
    <div class="row">
      <Button variant="ghost" onclick={finish}>Skip guide</Button>
      <span class="spacer"></span>
      {#if $ui.coachStep > 0}<Button onclick={() => go(-1)}>Back</Button>{/if}
      {#if last}<Button variant="primary" onclick={finish}>Got it</Button>{:else}<Button variant="primary" onclick={() => go(1)}>Next</Button>{/if}
    </div>
  </section>
{/if}

<style>
  .coach { position: absolute; left: 12px; bottom: 64px; width: min(340px, calc(100% - 24px)); z-index: 25; background: var(--surface);
    border: 2px solid var(--warn); border-radius: var(--radius); box-shadow: var(--shadow); padding: 10px 14px; }
  .meta { font-size: 11px; color: var(--text-dim); }
  h3 { text-transform: none; letter-spacing: 0; font-size: 14px; color: var(--text); margin: 2px 0; }
  p { margin: 4px 0 8px; }
  .row { display: flex; gap: 6px; align-items: center; } .spacer { flex: 1; }
</style>
