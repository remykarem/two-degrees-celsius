<script lang="ts">
  import type { Game } from '../store';
  import { Button, Modal } from '../ui-kit';
  import { describeCommand } from './format';

  let { game, coach = false }: { game: Game; coach?: boolean } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  let confirming = $state(false);

  const recordingOver = $derived(!!$gs.demo && ($gs.snapshot?.turn ?? 0) > $gs.demo.lastTurn);
  const enabled = $derived(($gs.phase === 'review' || $gs.phase === 'plan') && !$gs.snapshot?.gameOver && !recordingOver);
  const risky = $derived($gs.draft.filter((d) =>
    !d.preview.blocked && (d.command.type === 'switch_system' || (d.command.type === 'set_policy' && d.command.lever === 'bailout' && d.command.value === true))));

  function press() {
    if (risky.length && !confirming) confirming = true;
    else {
      confirming = false;
      game.endTurn();
    }
  }
</script>

<span data-coach={coach ? 'endturn' : undefined} class="wrap"><Button variant="primary" disabled={!enabled} onclick={press} title={recordingOver ? 'The recording ends here. Build the real simulation to keep playing.' : $gs.demo ? 'Play the next recorded turn' : ''}>{$gs.demo ? 'Next recorded turn ▸' : 'End turn ▸'}</Button></span>

{#if confirming}
  <Modal title="Confirm expensive or irreversible moves" size="sm" onclose={() => (confirming = false)}>
    <ul>{#each risky as d (d.id)}<li>{describeCommand(d.command, $gs.levers, $gs.actions)} ({d.preview.pc} PC)</li>{/each}</ul>
    <p class="dim">These are hard to undo once they take effect.</p>
    {#snippet footer()}
      <Button onclick={() => (confirming = false)}>Back</Button>
      <Button variant="primary" onclick={press}>Confirm and end turn</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .wrap { display: inline-flex; }
  .dim { color: var(--text-dim); font-size: 12px; }
</style>
