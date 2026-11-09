<script lang="ts">
  import type { Game } from '../store';
  import { Button, Modal } from '../ui-kit';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const err = $derived($gs.error);
  const WHERE: Record<string, string> = {
    new_game: 'starting the game', preview: 'working out what your plan costs', end_turn: 'playing the turn', cancel: 'cancelling that item', save: 'saving', load: 'loading',
  };
</script>

{#if err}
  <Modal title="Something went wrong" size="sm" onclose={() => game.dismissError()}>
    <p role="alert">The simulation reported a problem while {WHERE[err.where] ?? 'working'}.</p>
    <p class="msg">{err.message} <span class="code">({err.code})</span></p>
    <p class="dim">Your game is unchanged. Close this and try again, or start a new game.</p>
    {#snippet footer()}
      <Button onclick={() => game.newGame(Date.now() % 100000)}>New game</Button>
      <Button variant="primary" onclick={() => game.dismissError()}>Close and keep playing</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .msg { font-weight: 600; } .code { font-family: var(--font-mono); font-weight: 400; color: var(--text-dim); }
  .dim { color: var(--text-dim); }
</style>
