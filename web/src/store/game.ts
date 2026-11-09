import type { Command, InfraKind, Result, TurnResult } from '../contract';
import type { SimPort } from '../sim-port/port';
import { withCommand, withoutItem, withPreviews } from './draft';
import { emptyLayout, reconcileLayout, sameTile, tileBlock, type BuildHint, type Tile } from './layout';
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
  select(t: Tile | null): void;
  hover(t: Tile | null): void;
  setBuildKind(k: InfraKind | null): void;
  /** Tile click handler: places a draft when in build mode, otherwise selects. Returns an error text if invalid. */
  clickTile(t: Tile): string | null;
  tileError(t: Tile): string | null;
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
    playback: { month: -1, skipped: false }, layout: emptyLayout(), selection: null, hover: null,
    buildKind: null, lens: 'none', camera: 0, error: null, dismissedCrises: {},
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

  const draftedTiles = () => get().draft.flatMap((d) => (d.command.type === 'place' && d.command.tile ? [d.command.tile] : []));

  return {
    state: { get, subscribe: store.subscribe },

    newGame(seed) {
      const res = safe(() => port.init(seed));
      if (!res.ok) return fail('new_game', res.error);
      const snapshot = res.value;
      store.set({
        ...get(), phase: snapshot.gameOver ? 'ended' : 'review', snapshot, levers: port.levers(), actions: port.actions(), draft: [], lastResult: null, previous: null,
        playback: { month: -1, skipped: false }, layout: reconcileLayout(null, snapshot), selection: null, hover: null,
        buildKind: null, error: null, dismissedCrises: {},
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
      const { phase, layout } = get();
      if (phase !== 'review' && phase !== 'plan') return;
      const res = safe(() => port.cancelPending(id));
      if (!res.ok) return fail('cancel', res.error);
      const next = reconcileLayout(layout, res.value.snapshot);
      patch({ snapshot: res.value.snapshot, layout: next, ...previewsFor(get().draft) });
    },

    endTurn() {
      const { phase, draft, layout, snapshot: before } = get();
      if (phase !== 'review' && phase !== 'plan') return null;
      const res = safe(() => port.submitTurn(draft.map((d) => d.command)));
      if (!res.ok) { fail('end_turn', res.error); return null; }
      const result = res.value;
      // Tiles that queued builds were reserved on become their finished tiles.
      const hints: BuildHint[] = result.activated.flatMap((a) => {
        if (a.command.type !== 'place') return [];
        const tile = layout.pendingTiles[a.id] ?? a.command.tile;
        return tile ? [{ kind: a.command.kind, tile }] : [];
      });
      // The real sim has no tiles and drops `tile` from accepted commands: match accepted items to the drafts that produced them (same order,
      // skipping rejected indexes) to keep the tile the player clicked.
      const rejectedAt = new Set(result.rejected.map((x) => x.index));
      const sent = draft.filter((_, i) => !rejectedAt.has(i));
      const claims: Record<number, Tile> = {};
      result.accepted.forEach((a, i) => {
        const c = sent[i]?.command;
        if (a.command.type === 'place' && c?.type === 'place' && c.tile) claims[a.id] = c.tile;
      });
      // ONE store update for the whole turn (snapshot, result, layout, draft and phase together).
      store.set({
        ...get(), lastResult: result, previous: before, snapshot: result.snapshot, draft: [], buildKind: null,
        playback: { month: -1, skipped: false }, layout: reconcileLayout(layout, result.snapshot, hints, claims),
        phase: nextPhase(phase, 'endTurn'), error: null,
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
      patch({ buildKind: k });
    },

    tileError: (t) => tileBlock(get().layout, t, draftedTiles()),
    clickTile(t) {
      const { buildKind } = get();
      if (!buildKind) {
        patch({ selection: t });
        return null;
      }
      const err = tileBlock(get().layout, t, draftedTiles());
      if (err) return err;
      this.draftCommand({ type: 'place', kind: buildKind, count: 1, tile: t });
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
        format: SAVE_FORMAT, saveVersion: SAVE_VERSION, schemaVersion: s.snapshot.schemaVersion, simState: res.value, layout: s.layout,
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
        lastResult: f.lastResult, previous: f.previous, playback: { month: -1, skipped: false }, layout: reconcileLayout(f.layout, snapshot),
        selection: null, hover: null, buildKind: null, lens: f.uiPrefs.lens, camera: f.uiPrefs.camera, error: null, dismissedCrises: {},
      });
      return null;
    },
  };
}
