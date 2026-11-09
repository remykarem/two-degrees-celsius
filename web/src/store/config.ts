import type { GameCalendar } from '../contract';

/** "Q2 2031" from the snapshot calendar (nothing hard-coded). */
export function quarterLabel(turn: number, g: GameCalendar): string {
  const i = turn - g.firstTurn;
  const q = (i % g.turnsPerYear) + 1;
  return `Q${q} ${g.startYear + Math.floor(i / g.turnsPerYear)}`;
}
