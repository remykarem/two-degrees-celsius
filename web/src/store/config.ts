/** UI-side constants derived from the contract text (turn 161 = game over, 1 turn = 1 quarter). */
export const UI_CONFIG = { startYear: 2031, lastTurn: 160 } as const;

export function quarterLabel(turn: number): string {
  const q = ((turn - 1) % 4) + 1;
  return `Q${q} ${UI_CONFIG.startYear + Math.floor((turn - 1) / 4)}`;
}
