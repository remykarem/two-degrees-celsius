import { writable } from '../store';
import type { LeverKey, MetricKey } from '../contract';
import type { WhyKind } from './metrics';

/** Panel-only UI state shared between components. Nothing here is game state. */
export interface PanelUi {
  drawerOpen: boolean;
  /** A lever the drawer should scroll to and highlight (the Adapt shortcut). */
  focusLever: LeverKey | null;
  systemModal: boolean;
  why: { kind: WhyKind; metric?: MetricKey } | null;
  coachStep: number;
  coachDone: boolean;
  /** The New game screen (mode choice) is open. */
  startOpen: boolean;
  nextSeed: number | null;
}

const KEY = 'panels.coach.done';
const read = (): boolean => { try { return localStorage.getItem(KEY) === '1'; } catch { return false; } };
export const markCoachDone = () => { try { localStorage.setItem(KEY, '1'); } catch { /* storage may be unavailable */ } };

export const ui = writable<PanelUi>({ drawerOpen: false, focusLever: null, systemModal: false, why: null, coachStep: 0, coachDone: read(), startOpen: false, nextSeed: null });
export const setUi = (p: Partial<PanelUi>) => ui.set({ ...ui.get(), ...p });
