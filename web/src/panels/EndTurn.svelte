<script lang="ts">
  import type { Game } from '../store';
  import { Button } from '../ui-kit';
  import { PALETTE } from '../store';
  import { describeCommand } from './format';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let confirming = $state(false);

  const enabled = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !$gs.snapshot?.gameOver);
  const risky = $derived($gs.draft.filter((d) =>
    !d.preview.blocked && (d.command.type === 'switch_system' || (d.command.type === 'set_policy' && d.command.lever === 'bailout' && Number(d.command.value) > 0))));
  const infraLabel = (k: string) => PALETTE.find((p) => p.kind === k)?.label ?? k;

  function press() {
    if (risky.length && !confirming) confirming = true;
    else {
      confirming = false;
      game.endTurn();
    }
  }
</script>

<Button variant="primary" disabled={!enabled} onclick={press}>End turn ▸</Button>

{#if confirming}
  <div class="backdrop" role="presentation">
    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="ct">
      <h3 id="ct">Confirm expensive or irreversible moves</h3>
      <ul>{#each risky as d (d.id)}<li>{describeCommand(d.command, $gs.levers, infraLabel)} ({d.preview.pc} PC)</li>{/each}</ul>
      <div class="row"><Button onclick={() => (confirming = false)}>Back</Button><Button variant="primary" onclick={press}>Confirm and end turn</Button></div>
    </div>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.4); display: grid; place-items: center; z-index: 100; }
  .dialog { background: var(--surface); padding: 16px; border-radius: var(--radius); box-shadow: var(--shadow); min-width: 320px; }
  .row { display: flex; gap: 8px; justify-content: flex-end; margin-top: 12px; }
</style>
