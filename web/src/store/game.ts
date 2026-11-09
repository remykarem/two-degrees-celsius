import type { Command, InfraKind, TurnResult } from '../contract';
import type { SimPort } from '../sim-port/port';
import { withCommand, withoutItem, withPreviews } from './draft';
import { initialLayout, sameTile, tileBlock, type Tile } from './layout';
import { nextPhase, type PhaseEvent } from './phase';
import { writable, type Readable } from './readable';
import type { CameraAngle, GameState, Lens } from './types';

export interface Game {
  state: Readable<GameState>;
  newGame(seed: number): void;
  /** Stores */
  enterPlan(): void;
  cancelPlan(): void;
  draftCommand(c: Command): void;
  removeDraft(id: number): void;
  clearDraft(): void;
  cancelPending(id: number): void;
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
}

export function createGame(port: SimPort): Game {
  const store = writable<GameState>({
    phase: 'review', snapshot: null, levers: [], actions: [], draft: [], lastResult: null, previous: null,
    playback: { month: -1, skipped: false }, layout: initialLayout(), selection: null, hover: null,
    buildKind: null, lens: 'none', camera: 0,
  });
  const get = store.get;
  const patch = (p: Partial<GameState>) => store.set({ ...get(), ...p });
  let nextDraftId = 1;

  const fire = (e: PhaseEvent) => patch({ phase: nextPhase(get().phase, e) });

  function refreshPreviews() {
    const { draft } = get();
    patch({ draft: draft.length ? withPreviews(draft, port.preview(draft.map((d) => d.command))) : draft });
  }

  const draftedTiles = () => get().draft.flatMap((d) => (d.command.type === 'place' && d.command.tile ? [d.command.tile] : []));

  function applyResultToLayout(r: TurnResult) {
    const layout = get().layout;
    const placed = [...layout.placed, ...r.accepted.flatMap((a) =>
      a.command.type === 'place' && a.command.tile ? [{ kind: a.command.kind, ...a.command.tile, effectiveTurn: a.effectiveTurn }] : [])];
    for (const a of r.activated) {
      if (a.command.type !== 'remove') continue;
      const idx = placed.map((p) => p.kind).lastIndexOf(a.command.kind);
      if (idx >= 0) placed.splice(idx, 1);
    }
    patch({ layout: { ...layout, placed } });
  }

  return {
    state: { get, subscribe: store.subscribe },

    newGame(seed) {
      const snapshot = port.init(seed);
      store.set({
        ...get(), phase: 'review', snapshot, levers: port.levers(), actions: port.actions(), draft: [], lastResult: null, previous: null,
        playback: { month: -1, skipped: false }, layout: initialLayout(), selection: null, hover: null,
        buildKind: null,
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
      patch({ draft: withCommand(get().draft, c, nextDraftId++) });
      refreshPreviews();
    },
    removeDraft(id) {
      patch({ draft: withoutItem(get().draft, id) });
      refreshPreviews();
    },
    clearDraft: () => patch({ draft: [] }),

    cancelPending(id) {
      const { phase, layout, snapshot } = get();
      if (phase !== 'review' && phase !== 'plan') return;
      const item = snapshot?.pending.find((p) => p.id === id);
      const res = port.cancelPending(id);
      const placed = item?.command.type === 'place' && item.command.tile
        ? layout.placed.filter((p) => !(sameTile(p, item.command.type === 'place' ? item.command.tile! : p) && p.effectiveTurn === item.effectiveTurn))
        : layout.placed;
      patch({ snapshot: res.snapshot, layout: { ...layout, placed } });
      refreshPreviews();
    },

    endTurn() {
      const { phase, draft } = get();
      if (phase !== 'review' && phase !== 'plan') return null;
      const result = port.submitTurn(draft.map((d) => d.command));
      patch({ lastResult: result, previous: get().snapshot, snapshot: result.snapshot, draft: [], buildKind: null, playback: { month: -1, skipped: false } });
      applyResultToLayout(result);
      fire('endTurn');
      return result;
    },
    setPlaybackMonth: (m) => patch({ playback: { ...get().playback, month: m } }),
    finishPlayback() {
      if (get().phase !== 'playback') return;
      patch({ playback: { month: 2, skipped: true } });
      fire('playbackDone');
    },
    continueFromResults() {
      const s = get();
      if (s.phase !== 'results') return;
      if (s.snapshot?.gameOver) return fire('gameOver');
      fire('continue');
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

    tileError(t) {
      return tileBlock(get().layout, t, draftedTiles());
    },
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
  };
}
