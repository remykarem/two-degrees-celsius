import type { ActionInfo, Command, CommandPreview, InfraKind, LeverInfo, Result, Snapshot, TileRef, TurnResult } from '../contract';

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
  cancelPending(id: number): Result<{ refundPc: number; snapshot: Snapshot }>;
  exportState(): Result<string>;
  importState(blob: string): Result<Snapshot>;
}
