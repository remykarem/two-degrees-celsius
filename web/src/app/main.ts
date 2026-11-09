import { mount } from 'svelte';
import '../ui-kit/tokens.css';
import { createSimPort } from '../sim-port';
import { createGame } from '../store';
import App from './App.svelte';

const game = createGame(createSimPort());
game.newGame(Number(new URLSearchParams(location.search).get('seed') ?? 42));
if (import.meta.env.DEV) (window as unknown as { __game: typeof game }).__game = game;

mount(App, { target: document.getElementById('app')!, props: { game } });
