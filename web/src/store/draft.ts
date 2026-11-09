import type { Command, CommandPreview } from '../contract';
import type { DraftItem } from './types';

/** Key that identifies "the same slot" in a plan: one draft per lever, one per tile, one per system switch. */
export function slotKey(c: Command): string {
  switch (c.type) {
    case 'set_policy': return `policy:${c.lever}`;
    case 'switch_system': return 'system';
    case 'place': return `place:${c.tile?.x},${c.tile?.y}`;
    case 'remove': return `remove:${c.kind}`;
  }
}

/** Replaces any draft in the same slot, then appends. Pure. */
export function withCommand(draft: DraftItem[], c: Command, nextId: number): DraftItem[] {
  const k = slotKey(c);
  const kept = draft.filter((d) => slotKey(d.command) !== k);
  return [...kept, { id: nextId, command: c, preview: placeholderPreview(c) }];
}

export const withoutItem = (draft: DraftItem[], id: number) => draft.filter((d) => d.id !== id);

/** Re-attaches fresh previews (same order as the draft). */
export function withPreviews(draft: DraftItem[], previews: CommandPreview[]): DraftItem[] {
  return draft.map((d, i) => ({ ...d, preview: previews[i] ?? d.preview }));
}

function placeholderPreview(command: Command): CommandPreview {
  return { command, pc: 0, money: 0, leadTime: 0, effectiveTurn: 0, opposition: 0, blocked: null };
}

export const totalPc = (draft: DraftItem[]) => draft.filter((d) => !d.preview.blocked).reduce((a, d) => a + d.preview.pc, 0);
export const totalMoney = (draft: DraftItem[]) => draft.filter((d) => !d.preview.blocked).reduce((a, d) => a + d.preview.money, 0);
