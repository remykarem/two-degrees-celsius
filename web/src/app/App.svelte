<script lang="ts">
  import { onMount } from 'svelte';
  import { BriefingCard, BuildPalette, CrisisBanner, EndScreen, Inspector, MetricsPanel, Onboarding, PendingList, PlanTray, PlaybackTicker, PolicyDrawer, ResultsScreen, SystemModal, TopBar, ViewBar, WhyModal } from '../panels';
  import type { Game } from '../store';
  import { attachKeys } from './keys';
  import { attachPlayback } from './playback';
  import SceneHost from './SceneHost.svelte';

  let { game }: { game: Game } = $props();
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  onMount(() => {
    const offs = [attachPlayback(game), attachKeys(game)];
    return () => offs.forEach((f) => f());
  });
  const showTray = $derived($gs.phase === 'review' || $gs.phase === 'plan');
</script>

<div class="app" data-phase={$gs.phase}>
  <TopBar {game} />
  <CrisisBanner {game} />
  <BuildPalette {game} />
  <main>
    <SceneHost {game} />
    <ViewBar {game} />
    <Inspector {game} />
    <PolicyDrawer {game} />
    <BriefingCard {game} />
    <PlaybackTicker {game} />
    <Onboarding {game} />
  </main>
  <ResultsScreen {game} />
  <EndScreen {game} />
  <SystemModal {game} />
  <WhyModal {game} />
  <MetricsPanel {game} />
  <div class="bottom">
    {#if showTray}<PlanTray {game} />{/if}
    <PendingList {game} />
  </div>
</div>

<style>
  .app { height: 100vh; display: grid; grid-template-rows: auto auto minmax(0, 1fr) auto; grid-template-columns: 180px minmax(0, 1fr) 310px; }
  @media (max-width: 1200px) { .app { grid-template-columns: 150px minmax(0, 1fr) 280px; } }
  :global(header) { grid-column: 1 / -1; }
  main { position: relative; overflow: hidden; }
  .bottom { grid-column: 1 / -1; }
</style>
