import type { ActionInfo, Command, CommandPreview, LeverInfo, Snapshot, TurnResult } from '../contract';

export interface SimPort {
  init(seed: number): Snapshot;
  levers(): LeverInfo[];
  actions(): ActionInfo[];
  preview(commands: Command[]): CommandPreview[];
  submitTurn(commands: Command[]): TurnResult;
  cancelPending(id: number): { refundPc: number; snapshot: Snapshot };
  exportState(): string;
  importState(blob: string): Snapshot;
}
