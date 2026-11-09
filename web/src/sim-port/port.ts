import type { Command, CommandPreview, LeverInfo, Snapshot, TurnResult } from '../contract';

export interface SimPort {
  init(seed: number): Snapshot;
  levers(): LeverInfo[];
  preview(commands: Command[]): CommandPreview[];
  submitTurn(commands: Command[]): TurnResult;
  cancelPending(id: number): { refundPc: number; snapshot: Snapshot };
  exportState(): string;
  importState(blob: string): Snapshot;
}
