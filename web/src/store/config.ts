import type { GameCalendar } from '../contract';

/** "Q2 2031" from the snapshot calendar (nothing hard-coded). */
export function quarterLabel(turn: number, g: GameCalendar): string {
  const i = turn - g.firstTurn;
  const q = (i % g.turnsPerYear) + 1;
  return `Q${q} ${g.startYear + Math.floor(i / g.turnsPerYear)}`;
}

/**
 * The turn to show: the turn being planned, or, once the game is over, the turn that ended it. The snapshot's own `turn` has already
 * moved on by then, which put the top bar one quarter ahead of the end screen.
 */
export function shownTurn(snap: { turn: number; game: GameCalendar; gameOver: null | { turn: number } }): number {
  return snap.gameOver ? Math.min(snap.gameOver.turn, snap.game.lastTurn) : snap.turn;
}
