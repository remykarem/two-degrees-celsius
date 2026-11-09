<script lang="ts">
  import { onMount } from 'svelte';
  import { createScene } from '../scene';
  import type { Game } from '../store';

  let { game }: { game: Game } = $props();
  let el: HTMLDivElement;
  onMount(() => {
    const handle = createScene(el, game);
    return () => handle.dispose();
  });
</script>

<div class="host" bind:this={el}></div>

<style>
  .host { position: absolute; inset: 0; }
</style>
