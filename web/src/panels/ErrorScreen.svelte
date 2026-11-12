<script lang="ts">
  import { isFatalError, type Game, type Live } from '../store';
  import { Button, Modal } from '../ui-kit';

  let { game, live }: { game: Game; live: Live } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  const err = $derived($gs.error);
  // A wasm trap leaves the module dead: nothing in this tab can continue, so the only way forward is a reload.
  const crashed = $derived(err ? isFatalError(err) : false);
  const WHERE: Record<string, string> = {
    new_game: 'starting the game', preview: 'working out what your plan costs', end_turn: 'playing the turn', live: 'running the live clock', cancel: 'cancelling that item', save: 'saving', load: 'loading',
  };
</script>

{#if err && crashed}
  <Modal title="The simulation crashed" size="sm" dismissible={false}>
    <p role="alert">The simulation stopped unexpectedly while {WHERE[err.where] ?? 'working'} and cannot continue in this tab. Reload to start again. Saves are downloaded files, so a save you made earlier can be loaded after the reload.</p>
    <p class="msg">{err.message} <span class="code">({err.code})</span></p>
    {#snippet footer()}
      <Button variant="primary" onclick={() => location.reload()}>Reload</Button>
    {/snippet}
  </Modal>
{:else if err}
  <Modal title="Something went wrong" size="sm" onclose={() => game.dismissError()}>
    <p role="alert">The simulation reported a problem while {WHERE[err.where] ?? 'working'}.</p>
    <p class="msg">{err.message} <span class="code">({err.code})</span></p>
    <p class="dim">Your game is unchanged. Close this and try again, or start a new game.</p>
    {#snippet footer()}
      <Button onclick={() => live.newGame(Date.now() % 100000, game.state.get().mode)}>New game</Button>
      <Button variant="primary" onclick={() => game.dismissError()}>Close and keep playing</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .msg { font-weight: 600; } .code { font-family: var(--font-mono); font-weight: 400; color: var(--text-dim); }
  .dim { color: var(--text-dim); }
</style>
