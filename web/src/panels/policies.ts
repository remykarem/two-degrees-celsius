import type { LeverInfo, PolicyBoard, PolicyEvent, PolicyRow, PolicyValue } from '../contract';
import { formatLever } from './format';

/** Pure derivations for the Policies tab (contract v1.6). Values come from the wire; this only formats and orders. */
const fmt = (l: LeverInfo | undefined, v: PolicyValue | unknown): string => {
  if (v === null || v === undefined) return 'none';
  if (typeof v === 'object') return JSON.stringify(v);
  return l ? formatLever(l, v as number | boolean | string) : String(v);
};

export interface PolicyLine {
  lever: PolicyRow['lever']; label: string; group: string; initial: string; active: string; effective: string | null; changedFromInitial: boolean;
  pending: { id: number; text: string; turn: number } | null; /** A change in the plan tray that has not been submitted yet. */ draft: string | null; lastChange: string; weakened: string | null;
}
export interface PolicyEventLine { turn: number; lever: string; label: string; kind: PolicyEvent['kind']; text: string }

export const EVENT_WORD: Record<PolicyEvent['kind'], string> = { queued: 'queued', activated: 'took effect', cancelled: 'cancelled', rejected: 'rejected', weakened: 'weakened by lobbying' };

export function policyLines(board: PolicyBoard, levers: LeverInfo[], drafts: Record<string, PolicyValue> = {}): PolicyLine[] {
  const info = new Map(levers.map((l) => [l.key, l]));
  return board.rows.map((r) => {
    const l = info.get(r.lever);
    return {
      lever: r.lever, label: l?.label ?? r.label, group: r.group, initial: fmt(l, r.initial), active: fmt(l, r.active),
      effective: r.effective === null ? null : fmt(l, r.effective), changedFromInitial: String(r.initial) !== String(r.active),
      pending: r.pending ? { id: r.pending.id, text: fmt(l, r.pending.value), turn: r.pending.effectiveTurn } : null,
      draft: r.lever in drafts ? fmt(l, drafts[r.lever]) : null,
      lastChange: r.lastChangeTurn === null ? 'never changed' : `turn ${r.lastChangeTurn}`,
      weakened: r.weakenedBy !== null && r.weakenedBy > 0 ? `lobbying weakens this: ${(r.weakenedBy * 100).toFixed(0)}% of its effect is lost (set ${fmt(l, r.active)}, in force ${r.effective === null ? '?' : fmt(l, r.effective)})` : null,
    };
  });
}

/** Newest first; within a turn the wire's order is kept. */
export function timelineLines(board: PolicyBoard, levers: LeverInfo[]): PolicyEventLine[] {
  const info = new Map(levers.map((l) => [l.key, l]));
  return board.timeline.map((e, i) => ({ e, i })).sort((a, b) => b.e.turn - a.e.turn || b.i - a.i).map(({ e }) => {
    const l = info.get(e.lever);
    const label = l?.label ?? e.lever;
    return { turn: e.turn, lever: e.lever, label, kind: e.kind, text: `${label} ${EVENT_WORD[e.kind]}: ${fmt(l, e.from)} to ${fmt(l, e.to)}${e.reason ? ` (${e.reason})` : ''}` };
  });
}

export const pendingLines = (lines: PolicyLine[]) => lines.filter((l) => l.pending !== null);
export const draftLines = (lines: PolicyLine[]) => lines.filter((l) => l.draft !== null);
/** Group lines by `group` in first-seen order. */
export function byGroup(lines: PolicyLine[]): { group: string; lines: PolicyLine[] }[] {
  const out: { group: string; lines: PolicyLine[] }[] = [];
  for (const l of lines) { const g = out.find((x) => x.group === l.group); if (g) g.lines.push(l); else out.push({ group: l.group, lines: [l] }); }
  return out;
}
