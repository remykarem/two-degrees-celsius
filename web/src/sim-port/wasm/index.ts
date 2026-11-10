import type { ActionInfo, Command, CommandPreview, InfraKind, LeverInfo, Result, Snapshot, MonthResult, TileRef, TurnResult, TurnStart } from '../../contract';
import type { SimPort } from '../port';

/**
 * The real Rust simulation, through the wasm package built by `npm run build:wasm` (crates/sim-wasm).
 * Only this folder knows the package exists. The wasm side takes and returns JSON strings; every reply is already a contract `Result`.
 */

/** The part of the generated module this wrapper uses (the package's own .d.ts is not needed, so typecheck works without a build). */
export interface WasmSimInstance {
  init(seed: number, paramsJson?: string | null): string;
  levers(): string;
  actions(): string;
  snapshot(): string;
  /** Contract v1.0 helper. May be missing in a package built before it landed: the wrapper then derives the answer from the snapshot. */
  valid_tiles?(kind: string): string;
  preview(commandsJson: string): string;
  submit_turn(commandsJson: string): string;
  /** Contract v1.1 live-turn exports. Missing in a package built before they landed: Pressure mode is then unavailable. */
  begin_turn?(commandsJson: string): string;
  advance_month?(): string;
  finish_turn?(): string;
  cancel_pending(id: number): string;
  export_state(): string;
  import_state(blob: string): string;
}
export interface WasmModuleLike {
  WasmSim: new () => WasmSimInstance;
  install_panic_hook(): void;
  last_panic(): string;
}

const fail = (code: string, message: string) => ({ ok: false as const, error: { code, message } });

/** Wraps an initialised module (the caller has run the generated `init`/`initSync`). */
export function createWasmPort(mod: WasmModuleLike): SimPort {
  mod.install_panic_hook();
  const sim = new mod.WasmSim();
  /** Calls into wasm. A panic (the module traps and stays dead) comes back as an error result, never an exception. */
  const call = <T>(f: () => string): Result<T> => {
    try {
      return JSON.parse(f()) as Result<T>;
    } catch (e) {
      let detail = '';
      try { detail = mod.last_panic(); } catch { /* the module is gone */ }
      return fail('wasm_panic', `The simulation stopped unexpectedly. ${detail || String(e)}`);
    }
  };
  const value = <T>(f: () => string, what: string): T => {
    const r = call<T>(f);
    if (!r.ok) throw new Error(`${what}: ${r.error.code}: ${r.error.message}`);
    return r.value;
  };
  return {
    init: (seed) => call<Snapshot>(() => sim.init(seed, null)),
    levers: () => value<LeverInfo[]>(() => sim.levers(), 'levers'),
    actions: () => value<ActionInfo[]>(() => sim.actions(), 'actions'),
    preview: (commands: Command[]) => call<CommandPreview[]>(() => sim.preview(JSON.stringify(commands))),
    validTiles(kind: InfraKind): TileRef[] {
      if (sim.valid_tiles) {
        const r = call<TileRef[]>(() => sim.valid_tiles!(kind));
        return r.ok ? r.value : [];
      }
      const r = call<Snapshot>(() => sim.snapshot());
      return r.ok ? r.value.map.tiles.filter((t) => t.kind === 'empty' && t.buildable).map(({ x, y }) => ({ x, y })) : [];
    },
    submitTurn: (commands: Command[]) => call<TurnResult>(() => sim.submit_turn(JSON.stringify(commands))),
    liveTurn: typeof sim.begin_turn === 'function' && typeof sim.advance_month === 'function' && typeof sim.finish_turn === 'function',
    beginTurn: (commands: Command[]) => sim.begin_turn
      ? call<TurnStart>(() => sim.begin_turn!(JSON.stringify(commands))) : fail('unsupported', 'This simulation package has no live-turn calls (contract v1.1). Rebuild it with npm run build:wasm.'),
    advanceMonth: () => sim.advance_month
      ? call<MonthResult>(() => sim.advance_month!()) : fail('unsupported', 'This simulation package has no live-turn calls (contract v1.1).'),
    finishTurn: () => sim.finish_turn
      ? call<TurnResult>(() => sim.finish_turn!()) : fail('unsupported', 'This simulation package has no live-turn calls (contract v1.1).'),
    cancelPending: (id) => call(() => sim.cancel_pending(id)),
    exportState: () => call<string>(() => sim.export_state()),
    importState: (blob) => call<Snapshot>(() => sim.import_state(blob)),
  };
}

// import.meta.glob matches nothing (instead of failing the build) when the package has not been built.
const modules = import.meta.glob<WasmModuleLike & { default: (arg?: unknown) => Promise<unknown> }>('./pkg/sim_wasm.js');
const urls = import.meta.glob<string>('./pkg/sim_wasm_bg.wasm', { query: '?url', import: 'default' });

export function wasmPackagePresent(): boolean {
  return Object.keys(modules).length > 0;
}

/** Loads and initialises the generated package in a browser. Rejects if it is absent or fails to load. */
export async function loadWasmPort(): Promise<SimPort> {
  const load = modules['./pkg/sim_wasm.js'];
  const wasmUrl = urls['./pkg/sim_wasm_bg.wasm'];
  if (!load || !wasmUrl) throw new Error('the wasm package is not built (run npm run build:wasm in web/)');
  const mod = await load();
  await mod.default({ module_or_path: await wasmUrl() });
  const port = createWasmPort(mod);
  // A package built before contract v1.0 (schemaVersion 1) must not drive this UI: refuse, and the caller falls back to the mock.
  const probe = port.init(1);
  if (!probe.ok || probe.value.schemaVersion !== 2 || !probe.value.map) {
    throw new Error('the wasm package speaks an older contract (schemaVersion is not 2); rebuild it with npm run build:wasm');
  }
  return port;
}
