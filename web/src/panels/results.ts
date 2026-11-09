
import { METRICS, changeBetween, type MetricChange, type MetricDef } from './metrics';
import type { ActionInfo, LeverInfo, PendingItem, SimEvent, Snapshot, TurnResult } from '../contract';
import { describeCommand, formatEvent, formatLever, pct, REASON_TEXT } from './format';

/** Events that are noise on the results screen (covered by other sections or by the animation). */
const HIDDEN: SimEvent['type'][] = ['command_activated', 'policy_blocked', 'commuted'];

/** Folds repeated hired/fired events for the same firm into one with the summed head count. */
export function mergeEvents(events: SimEvent[]): SimEvent[] {
  const out: SimEvent[] = [];
  for (const e of events) {
    if (e.type === 'hired' || e.type === 'fired') {
      const prev = out.find((x) => x.type === e.type && (x as typeof e).firm === e.firm);
      if (prev && (prev.type === 'hired' || prev.type === 'fired')) {
        prev.people += e.people;
        prev.importance = Math.max(prev.importance, e.importance) as 1 | 2 | 3;
        continue;
      }
      out.push({ ...e });
    } else out.push(e);
  }
  return out;
}

/** Highest importance first; equal importance keeps the order the events happened in. */
export function rankEvents(events: SimEvent[], limit = Infinity): SimEvent[] {
  return mergeEvents(events.filter((e) => !HIDDEN.includes(e.type)))
    .map((e, i) => ({ e, i }))
    .sort((a, b) => b.e.importance - a.e.importance || a.i - b.i)
    .map((x) => x.e)
    .slice(0, limit);
}

export interface RankedLine { importance: 1 | 2 | 3; text: string }
/** ONE formatter: ranking and wording both come from the payload. */
export function eventLines(events: SimEvent[], snap: Snapshot | null, limit = 8): { lines: RankedLine[]; more: number } {
  const ranked = rankEvents(events);
  return { lines: ranked.slice(0, limit).map((e) => ({ importance: e.importance, text: formatEvent(e, snap) })), more: Math.max(0, ranked.length - limit) };
}

export const RESULT_METRICS = ['gdp', 'gpi', 'unemployment', 'gini', 'wellbeing', 'emissions', 'budget', 'legitimacy'];
export interface DeltaRow { def: MetricDef; change: MetricChange; from: string; to: string }
export function metricDeltas(before: Snapshot | null, after: Snapshot | null): DeltaRow[] {
  if (!before || !after) return [];
  return METRICS.filter((m) => RESULT_METRICS.includes(m.id)).map((def) => {
    const change = changeBetween(def, before, after);
    return { def, change, from: def.fmt(change.from), to: def.fmt(change.to) };
  });
}


export interface LobbyLine { kind: 'blocked' | 'weakened'; text: string }
/** Rejected commands and lobbying gaps, worded from the payload. */
export function lobbyLines(r: TurnResult, levers: LeverInfo[], actions: ActionInfo[]): LobbyLine[] {
  const out: LobbyLine[] = r.rejected.map((x) => ({ kind: 'blocked' as const, text: `${describeCommand(x.command, levers, actions)}: ${REASON_TEXT[x.reason] ?? x.reason}` }));
  for (const g of r.policyGaps) {
    const l = levers.find((x) => x.key === g.lever);
    const f = (v: number) => (l ? formatLever(l, v) : v.toFixed(2));
    const lost = g.nominal !== 0 ? Math.max(0, (g.nominal - g.effective) / g.nominal) : 0;
    out.push({ kind: 'weakened', text: `${l?.label ?? g.lever}: you set ${f(g.nominal)}, about ${f(g.effective)} in force (${pct(lost)} lost to lobbying)` });
  }
  return out;
}

export function comingNextTurn(snap: Snapshot): { next: PendingItem[]; later: number } {
  const next = snap.pending.filter((p) => p.effectiveTurn === snap.turn + 1);
  return { next, later: snap.pending.length - next.length };
}
