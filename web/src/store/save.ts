import type { Snapshot, TurnResult } from '../contract';
import type { MapLayout } from './layout';
import type { CameraAngle, Lens } from './types';

export const SAVE_FORMAT = 'ecotown-save';
export const SAVE_VERSION = 1;

/** docs/snapshot-contract.md section 6, plus the last result and the previous snapshot so the briefing and results still work after a load. */
export interface SaveFile {
  format: typeof SAVE_FORMAT;
  saveVersion: number;
  /** Snapshot.schemaVersion at save time. */
  schemaVersion: number;
  simState: string;
  layout: MapLayout;
  lastResult: TurnResult | null;
  previous: Snapshot | null;
  uiPrefs: { lens: Lens; camera: CameraAngle };
  savedAt: string;
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const LENS = ['none', 'emissions', 'class', 'wealth', 'employment', 'flows'];

export type SaveParse = { ok: true; value: SaveFile } | { ok: false; message: string };

/** Schema check with friendly messages. Does not look inside the sim blob: the port does that. */
export function parseSave(text: string): SaveParse {
  let raw: unknown;
  try { raw = JSON.parse(text); } catch { return { ok: false, message: 'That file is not a saved game (it is not valid JSON).' }; }
  if (!isObj(raw) || raw.format !== SAVE_FORMAT) return { ok: false, message: 'That file is not a saved game from this app.' };
  if (raw.saveVersion !== SAVE_VERSION) return { ok: false, message: `That save uses file version ${String(raw.saveVersion)}; this app reads version ${SAVE_VERSION}.` };
  if (typeof raw.schemaVersion !== 'number') return { ok: false, message: 'The save has no schema version, so it cannot be checked.' };
  if (typeof raw.simState !== 'string' || !raw.simState) return { ok: false, message: 'The save has no simulation state.' };
  const l = raw.layout;
  if (!isObj(l) || typeof l.gridSize !== 'number' || !Array.isArray(l.scenery) || !Array.isArray(l.placed) || !isObj(l.pendingTiles)) {
    return { ok: false, message: 'The save has a damaged map layout.' };
  }
  if (raw.lastResult !== null && !(isObj(raw.lastResult) && Array.isArray(raw.lastResult.months))) return { ok: false, message: 'The save has a damaged last result.' };
  if (raw.previous !== null && !(isObj(raw.previous) && typeof raw.previous.turn === 'number')) return { ok: false, message: 'The save has a damaged previous snapshot.' };
  const ui = raw.uiPrefs;
  if (!isObj(ui) || !LENS.includes(String(ui.lens)) || ![0, 1, 2, 3].includes(Number(ui.camera))) return { ok: false, message: 'The save has damaged view settings.' };
  return { ok: true, value: raw as unknown as SaveFile };
}
