import type { AccountRef, AgentsCatalog, AssumptionTopic, Command, LedgerKind, LedgerPage, LedgerQuery, MarketEntity, MarketTrades, TradesQuery, PersonLedger, InfraKind, MonthResult, Result, TurnResult, TurnStart } from '../contract';
import type { SimPort } from '../sim-port/port';
import { setCatalog } from './catalog';
import { relabelLevers } from './labels';
import { withCommand, withoutItem, withPreviews } from './draft';
import { describeTileReason, inBounds, layoutOf, pendingAt, sameTile, selectionAt, tileAt, tileKey, type Selection, type Tile } from './map';
import type { GameMode, TimeoutPolicy } from './clock';
import { nextPhase, type PhaseEvent } from './phase';
import { writable, type Readable } from './readable';
import { parseSave, SAVE_FORMAT, SAVE_VERSION, type SaveFile } from './save';
import type { CameraAngle, GameState, Lens, LiveSlice, UiError } from './types';

/** One preview() answer for a single place command, for display before anything is drafted. */
export interface BuildEstimate { pc: number; money: number; leadTime: number; tile: Tile; blocked: string | null }

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
  /** Contract v1.4: an account's transaction ledger, or an error result (never throws). */
  ledger(account: AccountRef, query?: LedgerQuery): Result<LedgerPage>;
  personLedger(person: number): Result<PersonLedger>;
  /** Contract v2.4: the list of trades behind one market, or an error result (never throws). */
  marketTrades(market: string, query?: TradesQuery): Result<MarketTrades>;
  /** Open the trade list of a market (null closes it); `entity` limits it to one party's trades. */
  openTrades(market: string | null, entity?: MarketEntity): void;
  /** Contract v1.5: the model explorer's topics (asked once and cached; an error result when the package has none). */
  assumptions(): Result<AssumptionTopic[]>;
  agents(): Result<AgentsCatalog>;
  /** Selects the first built tile of a kind (the Industry tab's infrastructure rows). Returns false when there is none. */
  selectFirstOf(kind: string): boolean;
  /** Opens the full transaction viewer for an account (optionally with a kind filter), or closes it with null. */
  openLedger(account: AccountRef | null, kind?: LedgerKind): void;
  hover(t: Tile | null): void;
  setBuildKind(k: InfraKind | null): void;
  /** Tile click handler: places a draft when in build mode, otherwise selects. Returns an error text if invalid. */
  clickTile(t: Tile): string | null;
  /** Why the tile cannot take the current build kind (from the sim's validTiles plus this plan), or null. Null when not building. */
  tileError(t: Tile): string | null;
  /** Drafts removing what stands on the tile (public housing, plant or road). Returns an error text when the tile has nothing removable. */
  removeAt(t: Tile): string | null;
  /**
   * The simulation's own price for building `kind` on `tile` (default: the first tile that can take it), asked through preview() without drafting.
   * This, not ActionInfo.moneyCost, is the authority; null when there is no valid tile or the call fails.
   */
  estimateBuild(kind: InfraKind, tile?: Tile): BuildEstimate | null;
  dismissError(): void;
  /** Reports a failure (used by the live loop). */
  setError(where: UiError['where'], e: { code: string; message: string }): void;

  // ---- Pressure mode (docs/town-v3.md): the quarter plays live, the player plans the next one. All return Results and never throw. ----
  /** Choose the mode for the next newGame/load. Pressure is refused (stays Relaxed) when the sim port has no live-turn calls. Returns the mode now set. */
  setMode(mode: GameMode): GameMode;
  /** beginTurn with the draft ('submit'), with nothing and the draft discarded ('discard'), or with nothing and the draft kept ('skip'). */
  liveBegin(action: TimeoutPolicy): Result<TurnStart>;
  liveMonth(): Result<MonthResult>;
  liveFinish(): Result<TurnResult>;
  /** After a turn_state error: runs advanceMonth/finishTurn until no turn is open. True when the sim is at a boundary. */
  liveRecover(): boolean;
  setLocked(locked: boolean): void;
  dismissSummary(): void;
  dismissReport(): void;
  setLiveNotice(n: string | null): void;
  /** The live loop registers how to read the clock prefs written into a save. */
  setSaveExtras(f: (() => { speed: number; slow: boolean }) | null): void;
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

/** A wasm trap (the module stays dead) comes back as this code; the UI must offer a reload, not "try again". */
export const isFatalError = (e: { code: string }) => e.code === 'wasm_panic';

export function createGame(port: SimPort): Game {
  // The UI catalog (bands, labels, explanatory text) comes from the simulation; without it there is no text to show.
  const cat = safe(() => port.catalog());
  if (cat.ok) setCatalog(cat.value);
  let assumptionsCache: Result<AssumptionTopic[]> | null = null;
  let agentsCache: Result<AgentsCatalog> | null = null;
  const liveAvailable = port.liveTurn !== false;
  const freshLive = (): LiveSlice => ({ open: false, monthsDone: 0, frame: null, frameSeq: 0, locked: false, summary: null, report: null, notice: null });
  const store = writable<GameState>({
    demo: port.demo ?? null, mode: 'relaxed', liveAvailable, live: null, savedClock: null,
    phase: 'review', snapshot: null, levers: [], actions: [], draft: [], lastResult: null, previous: null,
    playback: { month: -1, skipped: false }, selection: null, ledgerView: null, tradesView: null, hover: null,
    buildKind: null, validTiles: null, lens: 'none', camera: 0, error: cat.ok ? null : { ...cat.error, where: 'new_game' }, dismissedCrises: {},
  });
  const get = store.get;
  const patch = (p: Partial<GameState>) => store.set({ ...get(), ...p });
  let nextDraftId = 1;
  /** Pressure mode: the last turn boundary (a save is only legal there), refreshed at every boundary. */
  let boundary: { simState: string; snapshot: GameState['snapshot']; lastResult: TurnResult | null; previous: GameState['previous'] } | null = null;
  function captureBoundary() {
    const s = get();
    if (s.mode !== 'pressure' || !s.snapshot) { boundary = null; return; }
    const res = safe(() => port.exportState());
    boundary = res.ok ? { simState: res.value, snapshot: s.snapshot, lastResult: s.lastResult, previous: s.previous } : null;
  }
  let saveExtras: (() => { speed: number; slow: boolean }) | null = null;
  const locked = () => !!get().live?.locked;
  const patchLive = (p: Partial<LiveSlice>) => { const l = get().live; if (l) patch({ live: { ...l, ...p } }); };

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
        ...get(), phase: snapshot.gameOver ? 'ended' : 'review', snapshot, levers: relabelLevers(port.levers()), actions: port.actions(), draft: [], lastResult: null, previous: null,
        playback: { month: -1, skipped: false }, selection: null, ledgerView: null, tradesView: null, hover: null,
        buildKind: null, validTiles: null, error: null, dismissedCrises: {}, savedClock: null, live: get().mode === 'pressure' ? freshLive() : null,
      });
      captureBoundary();
    },

    enterPlan: () => fire('startPlan'),
    cancelPlan() {
      patch({ draft: [], buildKind: null });
      fire('cancelPlan');
    },

    draftCommand(c) {
      if (locked() || get().demo) return;
      if (get().phase === 'review') fire('startPlan');
      if (get().phase !== 'plan') return;
      patch(previewsFor(withCommand(get().draft, c, nextDraftId++)));
    },
    removeDraft(id) {
      if (locked() || get().demo) return;
      patch(previewsFor(withoutItem(get().draft, id)));
    },
    clearDraft() { if (!locked()) patch({ draft: [] }); },

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
    ledger: (account, query) => safe(() => port.ledger(account, query)),
    personLedger: (person) => safe(() => port.personLedger(person)),
    marketTrades: (market, query) => safe(() => port.marketTrades(market, query)),
    openTrades: (market, entity) => patch({ tradesView: market ? { market, ...(entity ? { entity } : {}) } : null }),
    agents() {
      if (!agentsCache) { const r = safe(() => port.agents()); if (!r.ok) return r; agentsCache = r; }
      return agentsCache;
    },
    assumptions() {
      if (!assumptionsCache) { const r = safe(() => port.assumptions()); if (!r.ok) return r; assumptionsCache = r; }
      return assumptionsCache;
    },
    selectFirstOf(kind) {
      const t = get().snapshot?.map.tiles.find((x) => x.kind === kind);
      if (!t) return false;
      patch({ selection: { kind: 'tile', x: t.x, y: t.y } });
      return true;
    },
    openLedger: (account, kind) => patch({ ledgerView: account ? { account, ...(kind ? { kind } : {}) } : null }),
    hover: (t) => {
      const h = get().hover;
      if (h === t || (h && t && sameTile(h, t))) return;
      patch({ hover: t });
    },
    setBuildKind(k) {
      if (k && (locked() || get().demo)) return;
      if (k && get().phase === 'review') fire('startPlan');
      if (k && get().phase !== 'plan') return;
      patch({ buildKind: k, ...validFor(k) });
    },

    tileError(t) {
      const { buildKind, validTiles, snapshot } = get();
      if (!buildKind) return null;
      if (get().demo) return get().demo!.note;
      if (locked()) return 'The plan is locked: unlock it to make changes';
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
      if (get().demo) return get().demo!.note;
      const kind = tileAt(get().snapshot, t)?.kind;
      if (kind !== 'public_housing' && kind !== 'fossil_plant' && kind !== 'renewable_plant' && kind !== 'road' && kind !== 'fire_station') return describeTileReason('nothing_to_remove');
      this.draftCommand({ type: 'remove', tile: { x: t.x, y: t.y } });
      return null;
    },

    estimateBuild(kind, tile) {
      let t = tile;
      if (!t) {
        try { t = port.validTiles(kind)[0]; } catch { t = undefined; }
      }
      if (!t) return null;
      const res = safe(() => port.preview([{ type: 'place', kind, tile: { x: t!.x, y: t!.y } }]));
      const p = res.ok ? res.value[0] : undefined;
      return p ? { pc: p.pc, money: p.money, leadTime: p.leadTime, tile: { x: t.x, y: t.y }, blocked: p.blocked } : null;
    },

    dismissError: () => patch({ error: null }),
    setError: (where, e) => fail(where, e),

    setMode(mode) {
      const m: GameMode = mode === 'pressure' && !liveAvailable ? 'relaxed' : mode;
      patch({ mode: m, live: m === 'pressure' ? freshLive() : null });
      return m;
    },
    liveBegin(action) {
      const s = get();
      if (s.mode !== 'pressure' || !s.live) return { ok: false, error: { code: 'turn_state', message: 'Not in Pressure mode.' } };
      if (s.phase === 'ended' || s.snapshot?.gameOver) return { ok: false, error: { code: 'game_over', message: 'The game is over.' } };
      const res = safe(() => port.beginTurn(action === 'submit' ? s.draft.map((d) => d.command) : []));
      if (!res.ok) return res;
      const start = res.value;
      const keep = action === 'skip' ? s.draft : [];
      patch({
        snapshot: start.snapshot, draft: keep, error: null, phase: keep.length ? 'plan' : s.phase === 'plan' && s.buildKind ? 'plan' : 'review',
        live: { ...s.live, open: true, monthsDone: 0, frame: null, locked: false, notice: null, report: start.rejected.length ? { turn: start.turn, rejected: start.rejected } : null },
      });
      patch({ ...previewsFor(get().draft), ...validFor(get().buildKind) });
      return res;
    },
    liveMonth() {
      const res = safe(() => port.advanceMonth());
      if (!res.ok) return res;
      const l = get().live;
      if (l) patch({ snapshot: res.value.snapshot, live: { ...l, open: true, monthsDone: (res.value.snapshot.turnPhase?.monthsDone ?? res.value.month + 1), frame: res.value.frame, frameSeq: l.frameSeq + 1 } });
      return res;
    },
    liveFinish() {
      const res = safe(() => port.finishTurn());
      if (!res.ok) return res;
      const result = res.value;
      const s = get();
      store.set({
        ...s, lastResult: result, previous: boundary?.snapshot ?? s.previous, snapshot: result.snapshot,
        phase: result.snapshot.gameOver ? 'ended' : s.phase, error: null, buildKind: result.snapshot.gameOver ? null : s.buildKind,
        live: s.live ? { ...s.live, open: false, monthsDone: 0, summary: result } : null,
      });
      captureBoundary();
      patch({ ...previewsFor(get().draft), ...validFor(get().buildKind) });
      return res;
    },
    liveRecover() {
      for (let i = 0; i < 5; i++) {
        if (this.liveFinish().ok) return true;
        this.liveMonth();
      }
      return false;
    },
    setLocked(on) {
      const l = get().live;
      if (!l || l.locked === on) return;
      patch(on ? { live: { ...l, locked: true }, buildKind: null, validTiles: null } : { live: { ...l, locked: false } });
    },
    dismissSummary: () => patchLive({ summary: null }),
    dismissReport: () => patchLive({ report: null }),
    setLiveNotice: (notice) => patchLive({ notice }),
    setSaveExtras(f) { saveExtras = f; },
    dismissCrisis(id) {
      const turn = get().snapshot?.turn ?? 0;
      patch({ dismissedCrises: { ...get().dismissedCrises, [id]: turn } });
    },

    exportSave() {
      const s = get();
      if (!s.snapshot) return null;
      // Pressure mode: a turn is open while the quarter plays, and the sim only exports at a boundary. Save the last boundary (and the drafts).
      const atBoundary = s.mode !== 'pressure' || !s.live?.open;
      const res = atBoundary ? safe(() => port.exportState()) : boundary ? ({ ok: true, value: boundary.simState } as const) : null;
      if (!res) { fail('save', { code: 'turn_state', message: 'No turn boundary has been captured yet, so there is nothing safe to save. Try again in a moment.' }); return null; }
      if (!res.ok) { fail('save', res.error); return null; }
      const b = atBoundary ? null : boundary;
      const file: SaveFile = {
        format: SAVE_FORMAT, saveVersion: SAVE_VERSION, schemaVersion: (b?.snapshot ?? s.snapshot).schemaVersion, simState: res.value,
        lastResult: b ? b.lastResult : s.lastResult, previous: b ? b.previous : s.previous, uiPrefs: { lens: s.lens, camera: s.camera }, savedAt: new Date().toISOString(),
        mode: s.mode, drafts: s.draft.map((d) => d.command), ...(saveExtras ? { clock: saveExtras() } : {}),
      };
      return JSON.stringify(file);
    },
    importSave(text) {
      const parsed = parseSave(text);
      if (!parsed.ok) return parsed.message;
      const f = parsed.value;
      const res = safe(() => port.importState(f.simState));
      if (!res.ok) return /schema|version/i.test(`${res.error.code} ${res.error.message}`) && !/weather/i.test(res.error.message) ? `${res.error.message} (Saves from an older version of the game cannot be loaded.)` : res.error.message;
      const snapshot = res.value;
      const mode: GameMode = f.mode === 'pressure' && liveAvailable ? 'pressure' : 'relaxed';
      store.set({
        ...get(), mode, live: mode === 'pressure' ? freshLive() : null, savedClock: f.clock ?? null,
        phase: snapshot.gameOver ? 'ended' : 'review', snapshot, levers: relabelLevers(port.levers()), actions: port.actions(), draft: [],
        lastResult: f.lastResult, previous: f.previous, playback: { month: -1, skipped: false },
        selection: null, ledgerView: null, tradesView: null, hover: null, buildKind: null, validTiles: null, lens: f.uiPrefs.lens, camera: f.uiPrefs.camera, error: null, dismissedCrises: {},
      });
      if (f.drafts?.length && !snapshot.gameOver) {
        let draft: GameState['draft'] = [];
        for (const c of f.drafts) draft = withCommand(draft, c, nextDraftId++);
        patch({ phase: 'plan', ...previewsFor(draft) });
      }
      captureBoundary();
      return null;
    },
  };
}
