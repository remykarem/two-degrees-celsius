<script lang="ts">
  import { onMount } from 'svelte';
  import { createScene } from '../scene';
  import { sceneBus, type Game } from '../store';
  import { Button } from '../ui-kit';

  let { game }: { game: Game } = $props();
  let el: HTMLDivElement;
  let failed = $state<string | null>(null);
  let gl = $state<'ok' | 'lost' | 'unavailable'>('ok');
  let said = $state('');
  let handle: { dispose(): void } | null = null;

  function start() {
    handle?.dispose();
    handle = null;
    failed = null;
    try {
      handle = createScene(el, game);
    } catch (e) {
      failed = e instanceof Error ? e.message : String(e);
    }
  }
  onMount(() => {
    start();
    const offs = [sceneBus.gl.subscribe((v) => (gl = v)), sceneBus.announce.subscribe((v) => (said = v))];
    return () => { offs.forEach((f) => f()); handle?.dispose(); };
  });
</script>

<div class="host" bind:this={el}></div>
<div class="sr-only" role="status" aria-live="polite" aria-atomic="true">{said}</div>
{#if failed}
  <div class="msg" role="alert">
    <b>The 3D map could not start.</b>
    <p>Your browser may not support WebGL, or graphics acceleration is off. The panels still work, but the map is not shown.</p>
    <p class="dim">{failed}</p>
    <Button onclick={start}>Try again</Button>
  </div>
{:else if gl === 'lost'}
  <div class="msg" role="alert">
    <b>The map lost its graphics connection.</b>
    <p>This can happen when the computer is busy or the tab was in the background. It usually comes back by itself in a moment.</p>
    <Button onclick={start}>Restart the map</Button>
  </div>
{/if}

<style>
  .host { position: absolute; inset: 0; }
  .host :global(canvas:focus-visible) { outline: 3px solid var(--accent); outline-offset: -3px; }
  .msg { position: absolute; left: 50%; top: 40%; transform: translate(-50%, -50%); z-index: 30; max-width: 420px; padding: 14px 18px;
    background: var(--surface); border: 2px solid var(--warn); border-radius: var(--radius); box-shadow: var(--shadow); }
  .msg p { margin: 6px 0; } .dim { color: var(--text-dim); font-size: 12px; }
</style>
