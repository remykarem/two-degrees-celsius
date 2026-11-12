import { mount } from 'svelte';
import '../ui-kit/tokens.css';
import { selectSimPort } from '../sim-port';
import { configFromSearch, createGame, createLive } from '../store';
import { setUi } from '../panels/ui';
import App from './App.svelte';
import { boot } from './boot';

await boot(document.getElementById('app')!, async () => {
  const search = location.search;
  const { port, kind, notice } = await selectSimPort(search);
  const game = createGame(port);
  const params = new URLSearchParams(search);
  const seed = Number(params.get('seed') ?? 42);
  const config = configFromSearch(search);
  const live = createLive(game, config);
  // Development links (?seed= or ?mode=) start straight away; a plain visit shows the New game screen with Relaxed preselected.
  const direct = params.has('seed') || params.has('mode');
  if (direct) live.newGame(seed, config.mode);
  else { game.newGame(seed); setUi({ startOpen: true }); }
  const started = game.state.get();
  if (!started.snapshot) throw new Error(started.error?.message ?? 'the simulation returned no game');
  const pressureWanted = (direct && config.mode === 'pressure');
  const unavailable = pressureWanted && game.state.get().mode !== 'pressure'
    ? 'Pressure mode needs the live-turn simulation calls, which this simulation build does not have, so the game runs in Relaxed mode. Rebuild the wasm package (npm run build:wasm) to enable it.' : undefined;
  if (import.meta.env.DEV) Object.assign(window, { __game: game, __live: live });
  document.documentElement.dataset.sim = kind;

  const target = document.getElementById('app')!;
  target.textContent = '';
  mount(App, { target, props: { game, live, seed, simNotice: [notice, unavailable].filter(Boolean).join(' ') || undefined } });
});
