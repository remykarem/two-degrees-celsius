import type { Command, CommandPreview, InfraKind, LeverInfo, Snapshot, TurnResult } from '../contract';
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

export interface GameState {
  phase: Phase;
  snapshot: Snapshot | null;
  levers: LeverInfo[];
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
  /** Message shown in the briefing card on Review. */
  briefing: string[];
}
