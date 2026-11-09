<script lang="ts">
  import { onMount } from 'svelte';
  import { BriefingCard, BuildPalette, CrisisBanner, EndScreen, ErrorScreen, Inspector, MetricsPanel, Onboarding, PendingList, PlanTray, PlaybackTicker, PolicyDrawer, ResultsScreen, SystemModal, TopBar, ViewBar, WhyModal } from '../panels';
  import type { Game } from '../store';
  import { attachFocus } from './focus';
  import { attachKeys } from './keys';
  import { attachPlayback } from './playback';
  import SceneHost from './SceneHost.svelte';

  let { game, simNotice = undefined }: { game: Game; simNotice?: string } = $props();
  let noticeOpen = $state(true);
  // svelte-ignore state_referenced_locally
  const gs = game.state;
  onMount(() => {
    const offs = [attachPlayback(game), attachKeys(game), attachFocus((fn) => gs.subscribe((s) => fn(s.phase)))];
    return () => offs.forEach((f) => f());
  });
  const showTray = $derived($gs.phase === 'review' || $gs.phase === 'plan');
</script>

<div class="narrow" role="note">
  <h1>Desktop only</h1>
  <p>This game needs a window at least 720 pixels wide: the map, the policy drawer and the plan tray do not fit on a phone. Please open it on a computer or a tablet held sideways.</p>
</div>
<div class="app" data-phase={$gs.phase}>
  {#if simNotice && noticeOpen}
    <div class="simnotice" role="status"><span>{simNotice}</span> <button type="button" onclick={() => (noticeOpen = false)}>Dismiss</button></div>
  {/if}
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
  <ErrorScreen {game} />
  <MetricsPanel {game} />
  <div class="bottom">
    {#if showTray}<PlanTray {game} />{/if}
    <PendingList {game} />
  </div>
</div>

<style>
  .simnotice { display: flex; gap: 12px; align-items: center; padding: 6px 14px; background: var(--surface); border-bottom: 1px dashed var(--warn); color: var(--warn); font-size: 13px; }
  .narrow { display: none; }
  @media (max-width: 719px) {
    .narrow { display: block; position: fixed; inset: 0; z-index: 500; background: var(--surface); color: var(--text); padding: 32px 20px; }
    .app { display: none; }
  }
  .app { height: 100vh; display: grid; grid-template-rows: auto auto minmax(0, 1fr) auto; grid-template-columns: 180px minmax(0, 1fr) 310px; }
  @media (max-width: 1200px) { .app { grid-template-columns: 150px minmax(0, 1fr) 280px; } }
  :global(header) { grid-column: 1 / -1; }
  main { position: relative; overflow: hidden; }
  .bottom { grid-column: 1 / -1; }
</style>
