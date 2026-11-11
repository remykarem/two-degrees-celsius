import type { AccountRef, ActionInfo, LedgerKind, Command, CommandPreview, InfraKind, LeverInfo, MonthFrame, RejectedCommand, Snapshot, TurnResult } from '../contract';
import type { GameMode } from './clock';
import type { Selection, Tile } from './map';
import type { Phase } from './phase';

export type Lens = 'none' | 'emissions' | 'class' | 'wealth' | 'employment' | 'flows' | 'commute' | 'fire';
export const LENSES: { id: Lens; label: string }[] = [
  { id: 'none', label: 'None' }, { id: 'emissions', label: 'Emissions' }, { id: 'class', label: 'Class' },
  { id: 'wealth', label: 'Wealth' }, { id: 'employment', label: 'Employment' }, { id: 'flows', label: 'Money flows' }, { id: 'commute', label: 'Commute' },
  { id: 'fire', label: 'Fire and drought risk' },
];

export type CameraAngle = 0 | 1 | 2 | 3;

export interface DraftItem { id: number; command: Command; preview: CommandPreview }

export interface Playback { month: -1 | 0 | 1 | 2; skipped: boolean }

/** A recoverable failure from the sim port. The screen offers retry (dismiss), load and new game. */
export interface UiError { code: string; message: string; where: 'new_game' | 'preview' | 'end_turn' | 'cancel' | 'save' | 'load' | 'live' }

/** Pressure mode only (null in Relaxed): the live quarter, as the game store sees it. The clock itself lives in store/clock.ts. */
export interface LiveSlice {
  /** A turn is open in the sim (beginTurn done, finishTurn not yet). */
  open: boolean;
  monthsDone: number;
  /** The month frame that landed last, and a counter so the scene notices a new one. */
  frame: MonthFrame | null;
  frameSeq: number;
  /** The plan for the next quarter is locked (edits are refused until unlocked). */
  locked: boolean;
  /** The finished quarter, shown as a non-blocking card. */
  summary: TurnResult | null;
  /** Commands that beginTurn refused, shown in a non-blocking card. */
  report: { turn: number; rejected: RejectedCommand[] } | null;
  /** A short plain-text message about the live loop (resync, an error). */
  notice: string | null;
}

export interface GameState {
  mode: GameMode;
  /** False when the sim port lacks the v1.1 live-turn calls: Pressure mode is then unavailable. */
  liveAvailable: boolean;
  live: LiveSlice | null;
  /** Clock prefs found in the last loaded save (speed, slow pace). */
  savedClock: { speed: number; slow: boolean } | null;
  phase: Phase;
  snapshot: Snapshot | null;
  levers: LeverInfo[];
  actions: ActionInfo[];
  draft: DraftItem[];
  lastResult: TurnResult | null;
  /** Snapshot from before the last turn, for result deltas. */
  previous: Snapshot | null;
  playback: Playback;
  selection: Selection | null;
  /** Contract v1.4: the full transaction viewer, open for one account (null = closed). `kind` pre-sets its kind filter. */
  ledgerView: { account: AccountRef; kind?: LedgerKind } | null;
  hover: Tile | null;
  buildKind: InfraKind | null;
  /** Tiles that can take `buildKind` right now, from the simulation's validTiles() (null when not building). Keys are "x,y". */
  validTiles: { kind: InfraKind; keys: Set<string> } | null;
  lens: Lens;
  camera: CameraAngle;
  error: UiError | null;
  /** Warnings the player dismissed, by crisis id -> turn. Reset by New game and Load. */
  dismissedCrises: Record<string, number>;
}
