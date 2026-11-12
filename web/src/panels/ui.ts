import { writable } from '../store';
import type { LeverKey, MetricKey } from '../contract';
import type { BreakdownKey, WhyKind } from './metrics';

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
  /** The breakdown popover (contract v1.6) for this key, or null. */
  breakdown: BreakdownKey | null;
  /** The "How it works" explorer: open when not null; `topic` is a UI anchor (metric key, lever key, panel id) or a topic id to show first. */
  explorer: { topic: string | null } | null;
  /** Another panel asked for the Markets view (a sub-view of the Economy tab); the metrics panel consumes it. */
  marketsRequested: boolean;
  /** Another panel asked for a metrics tab by its wire anchor (`panel:ecology`); the metrics panel consumes it. */
  tabRequested: string | null;
}

const KEY = 'panels.coach.done';
const read = (): boolean => { try { return localStorage.getItem(KEY) === '1'; } catch { return false; } };
export const markCoachDone = () => { try { localStorage.setItem(KEY, '1'); } catch { /* storage may be unavailable */ } };

export const ui = writable<PanelUi>({ drawerOpen: false, focusLever: null, systemModal: false, why: null, coachStep: 0, coachDone: read(), startOpen: false, nextSeed: null, breakdown: null, explorer: null, marketsRequested: false, tabRequested: null });
export const setUi = (p: Partial<PanelUi>) => ui.set({ ...ui.get(), ...p });
export const openBreakdown = (key: BreakdownKey) => setUi({ breakdown: key });
export const openExplorer = (topic: string | null = null) => setUi({ explorer: { topic }, breakdown: null });
/** Show the Markets view (Economy tab, Markets segment). */
export const openMarkets = () => setUi({ marketsRequested: true });
/** Follow a link's wire target (`panel:ecology`) to the metrics tab it names. */
export const openAnchor = (target: string) => setUi({ tabRequested: target });
