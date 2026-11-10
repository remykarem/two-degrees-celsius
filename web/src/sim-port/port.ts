import type { ActionInfo, Command, CommandPreview, InfraKind, LeverInfo, Result, Snapshot, MonthResult, TileRef, TurnResult, TurnStart } from '../contract';

/**
 * The UI's only window onto the simulation (contract v1.0, docs/snapshot-contract.md sections 10.7 and 11). Every fallible call returns a Result:
 * a Rust Violation becomes { ok: false }, never a thrown exception.
 */
export interface SimPort {
  init(seed: number): Result<Snapshot>;
  levers(): LeverInfo[];
  actions(): ActionInfo[];
  preview(commands: Command[]): Result<CommandPreview[]>;
  /** Tiles that can take a new `kind` right now (empty and buildable), to shade the grid. preview() stays the authority per command. */
  validTiles(kind: InfraKind): TileRef[];
  submitTurn(commands: Command[]): Result<TurnResult>;
  /**
   * Contract v1.1 live turn (section 12), used by Pressure mode. In order, beginTurn + 3 x advanceMonth + finishTurn equal submitTurn.
   * Out-of-order calls return { ok: false, error: { code: 'turn_state' } } and change nothing.
   */
  beginTurn(commands: Command[]): Result<TurnStart>;
  advanceMonth(): Result<MonthResult>;
  finishTurn(): Result<TurnResult>;
  /** False when the implementation lacks the v1.1 calls (an older wasm package): Pressure mode is then unavailable. Absent means true. */
  readonly liveTurn?: boolean;
  cancelPending(id: number): Result<{ refundPc: number; snapshot: Snapshot }>;
  exportState(): Result<string>;
  importState(blob: string): Result<Snapshot>;
}
