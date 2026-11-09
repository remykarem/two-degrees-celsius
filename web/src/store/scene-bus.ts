import type { InfraKind } from '../contract';
import type { SceneryKind, Tile } from './layout';
import { writable, type Readable, type Writable } from './readable';
import type { CameraAngle, Lens } from './types';

/**
 * Channel between the 3D scene and the panels. The scene writes, panels read (and send camera requests).
 * No DOM, no Three.js, no Svelte: every field is plain data. Readables follow the Svelte store contract.
 */
export type EntityKind = SceneryKind | InfraKind;
export interface EntityRef {
  kind: EntityKind;
  /** Household id for homes, firm id for firms, else null. */
  id: number | null;
  tile: Tile;
  label: string;
  /** Set for queued builds: turns until the building opens. */
  pendingTurns?: number;
}
export interface HoverInfo {
  tile: Tile;
  entity: EntityRef | null;
  /** In build mode: why the hovered tile cannot take the building (null when valid or not building). */
  invalidReason: string | null;
  buildKind: InfraKind | null;
}
export interface SelectionDetail { tile: Tile; entity: EntityRef | null; facts: { label: string; value: string }[] }
export interface LegendStop { color: string; label: string }
export interface LensLegend {
  lens: Lens;
  label: string;
  /** One or two sentences: what the colours mean. */
  meaning: string;
  scale: 'none' | 'gradient' | 'categories';
  /** Gradient: low to high. Categories: one per entry (with a shape note in the label). */
  stops: LegendStop[];
  lowLabel?: string;
  highLabel?: string;
  notes: string[];
}
export interface CameraInfo { angle: CameraAngle; zoom: number; zoomMin: number; zoomMax: number; panned: boolean; reducedMotion: boolean }
export interface PlaybackProgress { active: boolean; month: -1 | 0 | 1 | 2; monthT: number; overall: number }
export interface SceneStats { drawCalls: number; triangles: number; instances: number; entities: number }

export type SceneCommand = { type: 'zoom'; factor: number } | { type: 'reset' };

export interface SceneBus {
  hover: Writable<HoverInfo | null>;
  selection: Writable<SelectionDetail | null>;
  legend: Writable<LensLegend | null>;
  camera: Writable<CameraInfo>;
  playback: Writable<PlaybackProgress>;
  stats: Writable<SceneStats>;
  /** Panels call these (zoom buttons, reset button); the scene executes them. */
  requestZoom(factor: number): void;
  requestReset(): void;
  onCommand(fn: (c: SceneCommand) => void): () => void;
}

export function createSceneBus(): SceneBus {
  const listeners = new Set<(c: SceneCommand) => void>();
  const send = (c: SceneCommand) => listeners.forEach((f) => f(c));
  return {
    hover: writable<HoverInfo | null>(null),
    selection: writable<SelectionDetail | null>(null),
    legend: writable<LensLegend | null>(null),
    camera: writable<CameraInfo>({ angle: 0, zoom: 1, zoomMin: 0.7, zoomMax: 3.2, panned: false, reducedMotion: false }),
    playback: writable<PlaybackProgress>({ active: false, month: -1, monthT: 0, overall: 0 }),
    stats: writable<SceneStats>({ drawCalls: 0, triangles: 0, instances: 0, entities: 0 }),
    requestZoom: (factor) => send({ type: 'zoom', factor }),
    requestReset: () => send({ type: 'reset' }),
    onCommand(fn) { listeners.add(fn); return () => listeners.delete(fn); },
  };
}

/** The app-wide bus. The scene and the panels both import this one instance. */
export const sceneBus: SceneBus = createSceneBus();
export type { Readable };
