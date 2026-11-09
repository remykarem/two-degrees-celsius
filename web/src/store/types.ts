import type { ActionInfo, Command, CommandPreview, InfraKind, LeverInfo, Snapshot, TurnResult } from '../contract';
import type { MapLayout, Tile } from './layout';
import type { Phase } from './phase';

export type Lens = 'none' | 'emissions' | 'class' | 'wealth' | 'employment' | 'flows';
export const LENSES: { id: Lens; label: string }[] = [
  { id: 'none', label: 'None' }, { id: 'emissions', label: 'Emissions' }, { id: 'class', label: 'Class' },
  { id: 'wealth', label: 'Wealth' }, { id: 'employment', label: 'Employment' }, { id: 'flows', label: 'Money flows' },
];

export type CameraAngle = 0 | 1 | 2 | 3;

export interface DraftItem { id: number; command: Command; preview: CommandPreview }

export interface Playback { month: -1 | 0 | 1 | 2; skipped: boolean }

/** A recoverable failure from the sim port. The screen offers retry (dismiss), load and new game. */
export interface UiError { code: string; message: string; where: 'new_game' | 'preview' | 'end_turn' | 'cancel' | 'save' | 'load' }

export interface GameState {
  phase: Phase;
  snapshot: Snapshot | null;
  levers: LeverInfo[];
  actions: ActionInfo[];
  draft: DraftItem[];
  lastResult: TurnResult | null;
  /** Snapshot from before the last turn, for result deltas. */
  previous: Snapshot | null;
  playback: Playback;
  layout: MapLayout;
  selection: Tile | null;
  hover: Tile | null;
  buildKind: InfraKind | null;
  lens: Lens;
  camera: CameraAngle;
  error: UiError | null;
  /** Warnings the player dismissed, by crisis id -> turn. Reset by New game and Load. */
  dismissedCrises: Record<string, number>;
}
