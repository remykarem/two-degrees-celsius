<script lang="ts">
  import type { Bar } from './why';
  let { bars, pairLabel }: { bars: Bar[]; pairLabel?: [string, string] } = $props();
  const max = $derived(Math.max(1e-9, ...bars.map((b) => Math.max(Math.abs(b.value), Math.abs(b.pair ?? 0)))));
  const w = (v: number) => `${Math.max(1, (Math.abs(v) / max) * 100)}%`;
</script>

{#if pairLabel}<p class="legend"><span class="sw a"></span>{pairLabel[0]} <span class="sw b"></span>{pairLabel[1]}</p>{/if}
<ul class="bars">
  {#each bars as b (b.label)}
    <li>
      <div class="top"><span class="name">{b.label}</span><span class="val">{b.text}</span></div>
      <div class="track" role="img" aria-label="{b.label}: {b.text}{b.sub ? `, ${b.sub}` : ''}">
        <div class="bar" class:sink={b.tone === 'sink'} style:width={w(b.value)}></div>
      </div>
      {#if b.pair !== undefined}<div class="track"><div class="bar pair" style:width={w(b.pair)}></div></div>{/if}
      {#if b.sub}<div class="sub">{b.sub}</div>{/if}
    </li>
  {/each}
</ul>

<style>
  .bars { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .top { display: flex; justify-content: space-between; gap: 8px; font-size: 13px; }
  .val { font-family: var(--font-mono); }
  .track { height: 10px; background: var(--surface-2); border-radius: 3px; margin-top: 2px; overflow: hidden; }
  .bar { height: 100%; background: var(--series-1); border-radius: 3px; }
  .bar.sink { background: var(--series-3); background-image: repeating-linear-gradient(45deg, transparent 0 4px, rgba(255, 255, 255, 0.4) 4px 6px); }
  .bar.pair { background: var(--series-2); }
  .sub { font-size: 12px; color: var(--text-dim); }
  .legend { font-size: 12px; margin: 0 0 6px; color: var(--text-dim); }
  .sw { display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin: 0 4px 0 8px; vertical-align: -1px; }
  .sw:first-child { margin-left: 0; }
  .sw.a { background: var(--series-1); } .sw.b { background: var(--series-2); }
</style>
