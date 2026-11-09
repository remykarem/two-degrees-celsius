import type { Command, InfraKind, Result, TurnResult } from '../contract';
import type { SimPort } from '../sim-port/port';
import { withCommand, withoutItem, withPreviews } from './draft';
import { describeTileReason, inBounds, layoutOf, pendingAt, sameTile, selectionAt, tileAt, tileKey, type Selection, type Tile } from './map';
import { nextPhase, type PhaseEvent } from './phase';
import { writable, type Readable } from './readable';
import { parseSave, SAVE_FORMAT, SAVE_VERSION, type SaveFile } from './save';
import type { CameraAngle, GameState, Lens, UiError } from './types';

export interface Game {
  state: Readable<GameState>;
  newGame(seed: number): void;
  enterPlan(): void;
  cancelPlan(): void;
  draftCommand(c: Command): void;
  removeDraft(id: number): void;
  clearDraft(): void;
  cancelPending(id: number): void;
  /** Plays the turn. Returns null (and sets `state.error`) when the sim refuses; never throws. */
  endTurn(): TurnResult | null;
  setPlaybackMonth(m: -1 | 0 | 1 | 2): void;
  finishPlayback(): void;
  continueFromResults(): void;
  rotate(dir: 1 | -1): void;
  setLens(l: Lens): void;
  select(sel: Selection | null): void;
  hover(t: Tile | null): void;
  setBuildKind(k: InfraKind | null): void;
  /** Tile click handler: places a draft when in build mode, otherwise selects. Returns an error text if invalid. */
  clickTile(t: Tile): string | null;
  /** Why the tile cannot take the current build kind (from the sim's validTiles plus this plan), or null. Null when not building. */
  tileError(t: Tile): string | null;
  /** Drafts removing what stands on the tile (public housing, plant or road). Returns an error text when the tile has nothing removable. */
  removeAt(t: Tile): string | null;
  dismissError(): void;
  dismissCrisis(id: string): void;
  /** The save file as text, or null (with `state.error` set) when the sim cannot export. */
  exportSave(): string | null;
  /** Loads a save. Returns a friendly error message, or null on success. */
  importSave(text: string): string | null;
}

/** Runs a port call, turning a thrown exception (a bug in a port) into an error result so the UI never crashes. */
function safe<T>(f: () => Result<T>): Result<T> {
  try { return f(); } catch (e) { return { ok: false, error: { code: 'internal', message: e instanceof Error ? e.message : String(e) } }; }
}

export function createGame(port: SimPort): Game {
  const store = writable<GameState>({
    phase: 'review', snapshot: null, levers: [], actions: [], draft: [], lastResult: null, previous: null,
    playback: { month: -1, skipped: false }, selection: null, hover: null,
    buildKind: null, validTiles: null, lens: 'none', camera: 0, error: null, dismissedCrises: {},
  });
  const get = store.get;
  const patch = (p: Partial<GameState>) => store.set({ ...get(), ...p });
  let nextDraftId = 1;

  const fail = (where: UiError['where'], e: { code: string; message: string }) => patch({ error: { ...e, where } });
  const fire = (e: PhaseEvent) => patch({ phase: nextPhase(get().phase, e) });

  /** Fresh previews for the current draft, as a partial state (so callers can merge it into one patch). */
  function previewsFor(draft: GameState['draft']): Partial<GameState> {
    if (!draft.length) return { draft };
    const res = safe(() => port.preview(draft.map((d) => d.command)));
    return res.ok ? { draft: withPreviews(draft, res.value) } : { draft, error: { ...res.error, where: 'preview' } };
  }

  const draftedTiles = () => get().draft.flatMap((d) => (d.command.type === 'place' ? [d.command.tile] : []));

  /** The simulation's list of tiles that can take `kind` now (a partial state, so callers can merge it into one patch). */
  function validFor(kind: GameState['buildKind']): Pick<GameState, 'validTiles'> {
    if (!kind) return { validTiles: null };
    try { return { validTiles: { kind, keys: new Set(port.validTiles(kind).map(tileKey)) } }; } catch { return { validTiles: null }; }
  }

  return {
    state: { get, subscribe: store.subscribe },

    newGame(seed) {
      const res = safe(() => port.init(seed));
      if (!res.ok) return fail('new_game', res.error);
      const snapshot = res.value;
      store.set({
        ...get(), phase: snapshot.gameOver ? 'ended' : 'review', snapshot, levers: port.levers(), actions: port.actions(), draft: [], lastResult: null, previous: null,
        playback: { month: -1, skipped: false }, selection: null, hover: null,
        buildKind: null, validTiles: null, error: null, dismissedCrises: {},
      });
    },

    enterPlan: () => fire('startPlan'),
    cancelPlan() {
      patch({ draft: [], buildKind: null });
      fire('cancelPlan');
    },

    draftCommand(c) {
      if (get().phase === 'review') fire('startPlan');
      if (get().phase !== 'plan') return;
      patch(previewsFor(withCommand(get().draft, c, nextDraftId++)));
    },
    removeDraft(id) {
      patch(previewsFor(withoutItem(get().draft, id)));
    },
    clearDraft: () => patch({ draft: [] }),

    cancelPending(id) {
      const { phase } = get();
      if (phase !== 'review' && phase !== 'plan') return;
      const res = safe(() => port.cancelPending(id));
      if (!res.ok) return fail('cancel', res.error);
      patch({ snapshot: res.value.snapshot, ...previewsFor(get().draft), ...validFor(get().buildKind) });
    },

    endTurn() {
      const { phase, draft, snapshot: before } = get();
      if (phase !== 'review' && phase !== 'plan') return null;
      const res = safe(() => port.submitTurn(draft.map((d) => d.command)));
      if (!res.ok) { fail('end_turn', res.error); return null; }
      const result = res.value;
      // ONE store update for the whole turn (snapshot, result, draft and phase together).
      store.set({
        ...get(), lastResult: result, previous: before, snapshot: result.snapshot, draft: [], buildKind: null, validTiles: null,
        playback: { month: -1, skipped: false }, phase: nextPhase(phase, 'endTurn'), error: null,
      });
      return result;
    },
    setPlaybackMonth: (m) => patch({ playback: { ...get().playback, month: m } }),
    finishPlayback() {
      if (get().phase !== 'playback') return;
      patch({ playback: { month: 2, skipped: true }, phase: nextPhase('playback', 'playbackDone') });
    },
    continueFromResults() {
      const s = get();
      if (s.phase !== 'results') return;
      fire(s.snapshot?.gameOver ? 'gameOver' : 'continue');
    },

    rotate: (dir) => patch({ camera: ((get().camera + dir + 4) % 4) as CameraAngle }),
    setLens: (lens) => patch({ lens }),
    select: (selection) => patch({ selection }),
    hover: (t) => {
      const h = get().hover;
      if (h === t || (h && t && sameTile(h, t))) return;
      patch({ hover: t });
    },
    setBuildKind(k) {
      if (k && get().phase === 'review') fire('startPlan');
      if (k && get().phase !== 'plan') return;
      patch({ buildKind: k, ...validFor(k) });
    },

    tileError(t) {
      const { buildKind, validTiles, snapshot } = get();
      if (!buildKind) return null;
      if (!inBounds(t)) return 'Outside the map';
      if (draftedTiles().some((d) => sameTile(d, t))) return 'Already in this turn\'s plan';
      if (validTiles && validTiles.kind === buildKind && validTiles.keys.has(tileKey(t))) return null;
      const kind = tileAt(snapshot, t)?.kind;
      if (kind === 'forest' || kind === 'commons') return describeTileReason('tile_invalid');
      if (kind && kind !== 'empty') return describeTileReason('tile_occupied');
      if (snapshot && pendingAt(layoutOf(snapshot), t) !== null) return describeTileReason('tile_occupied');
      return describeTileReason(undefined);
    },
    clickTile(t) {
      const { buildKind, snapshot } = get();
      if (!buildKind) {
        patch({ selection: selectionAt(snapshot, t) });
        return null;
      }
      const err = this.tileError(t);
      if (err) return err;
      this.draftCommand({ type: 'place', kind: buildKind, tile: { x: t.x, y: t.y } });
      return null;
    },
    removeAt(t) {
      const kind = tileAt(get().snapshot, t)?.kind;
      if (kind !== 'public_housing' && kind !== 'fossil_plant' && kind !== 'renewable_plant' && kind !== 'road') return describeTileReason('nothing_to_remove');
      this.draftCommand({ type: 'remove', tile: { x: t.x, y: t.y } });
      return null;
    },

    dismissError: () => patch({ error: null }),
    dismissCrisis(id) {
      const turn = get().snapshot?.turn ?? 0;
      patch({ dismissedCrises: { ...get().dismissedCrises, [id]: turn } });
    },

    exportSave() {
      const s = get();
      if (!s.snapshot) return null;
      const res = safe(() => port.exportState());
      if (!res.ok) { fail('save', res.error); return null; }
      const file: SaveFile = {
        format: SAVE_FORMAT, saveVersion: SAVE_VERSION, schemaVersion: s.snapshot.schemaVersion, simState: res.value,
        lastResult: s.lastResult, previous: s.previous, uiPrefs: { lens: s.lens, camera: s.camera }, savedAt: new Date().toISOString(),
      };
      return JSON.stringify(file);
    },
    importSave(text) {
      const parsed = parseSave(text);
      if (!parsed.ok) return parsed.message;
      const f = parsed.value;
      const res = safe(() => port.importState(f.simState));
      if (!res.ok) return res.error.message;
      const snapshot = res.value;
      store.set({
        ...get(), phase: snapshot.gameOver ? 'ended' : 'review', snapshot, levers: port.levers(), actions: port.actions(), draft: [],
        lastResult: f.lastResult, previous: f.previous, playback: { month: -1, skipped: false },
        selection: null, hover: null, buildKind: null, validTiles: null, lens: f.uiPrefs.lens, camera: f.uiPrefs.camera, error: null, dismissedCrises: {},
      });
      return null;
    },
  };
}
