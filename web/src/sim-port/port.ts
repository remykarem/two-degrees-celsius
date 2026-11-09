import type { ActionInfo, Command, CommandPreview, LeverInfo, Result, Snapshot, TurnResult } from '../contract';

/**
 * The UI's only window onto the simulation (contract v0.3, section 10.7). Every fallible call returns a Result:
 * a Rust Violation becomes { ok: false }, never a thrown exception.
 */
export interface SimPort {
  init(seed: number): Result<Snapshot>;
  levers(): LeverInfo[];
  actions(): ActionInfo[];
  preview(commands: Command[]): Result<CommandPreview[]>;
  submitTurn(commands: Command[]): Result<TurnResult>;
  cancelPending(id: number): Result<{ refundPc: number; snapshot: Snapshot }>;
  exportState(): Result<string>;
  importState(blob: string): Result<Snapshot>;
}
