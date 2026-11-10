import type { Game } from '../store';
import { setUi } from './ui';

/**
 * The forecast card's "Adapt" shortcut: opens the policy drawer on the resilience lever and selects the fire-station build.
 * Either part is skipped quietly when it cannot apply (the plan is locked, the game is over, the sim has no fire station).
 */
export function adapt(game: Game): void {
  const s = game.state.get();
  setUi({ drawerOpen: true, focusLever: 'resilience_spending' });
  if (s.actions.some((a) => a.key === 'place:fire_station')) game.setBuildKind('fire_station');
}
