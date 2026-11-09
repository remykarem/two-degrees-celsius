import { mount } from 'svelte';
import '../ui-kit/tokens.css';
import { selectSimPort } from '../sim-port';
import { createGame } from '../store';
import App from './App.svelte';

const search = location.search;
const { port, kind, notice } = await selectSimPort(search);
const game = createGame(port);
game.newGame(Number(new URLSearchParams(search).get('seed') ?? 42));
if (import.meta.env.DEV) (window as unknown as { __game: typeof game; __sim: string }).__game = game;
document.documentElement.dataset.sim = kind;

mount(App, { target: document.getElementById('app')!, props: { game, simNotice: notice } });
