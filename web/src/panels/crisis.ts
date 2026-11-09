import type { SimEvent, Snapshot, TurnResult } from '../contract';
import { REASON_TEXT } from './format';
import type { WhyKind } from './metrics';

export const LEGITIMACY_WARNING = 0.2;
export const BUDGET_WARNING = 0.15;

export interface Crisis {
  id: string;
  severity: 'critical' | 'warning';
  title: string;
  detail: string;
  why?: WhyKind;
}

const allEvents = (r: TurnResult | null): SimEvent[] => r?.months.flatMap((m) => m.events) ?? [];

/** Crises from the latest turn's events plus thresholds on the current snapshot. Critical first, then by id. */
export function deriveCrises(snap: Snapshot | null, last: TurnResult | null): Crisis[] {
  if (!snap) return [];
  const out = new Map<string, Crisis>();
  const add = (c: Crisis) => out.set(c.id, c);
  const events = allEvents(last);

  if (snap.bank.failed || events.some((e) => e.type === 'bank_failed')) {
    add({ id: 'bank_failed', severity: 'critical', title: 'The bank has failed',
      detail: 'Its equity ran out. Savers may lose part of their deposits unless the government bails it out (a bailout also costs legitimacy).' });
  }
  if (events.some((e) => e.type === 'commons_collapsed')) {
    add({ id: 'commons_collapsed', severity: 'critical', title: 'The commons collapsed',
      detail: 'The shared land is exhausted. Farm output is likely to suffer.' });
  }
  for (const e of events) {
    if (e.type === 'collapse') add({ id: 'collapse', severity: 'critical', title: 'The town has collapsed', detail: `Loss condition reached: ${e.reason.replaceAll('_', ' ')}.` });
    if (e.type === 'budget_exhausted') add({ id: 'budget_exhausted', severity: 'critical', title: 'The carbon budget is exhausted', detail: 'Further emissions are overshoot.', why: 'emissions' });
    if (e.type === 'policy_blocked') {
      add({ id: `policy_blocked:${e.reason}`, severity: 'warning', title: 'A policy was blocked', detail: REASON_TEXT[e.reason] ?? e.reason });
    }
  }
  const leg = snap.government.legitimacy;
  if (leg < LEGITIMACY_WARNING) {
    add({ id: 'low_legitimacy', severity: 'warning', title: 'Collapse warning: legitimacy is very low',
      detail: `Legitimacy is ${(leg * 100).toFixed(0)}%. At zero the town collapses.` });
  }
  if (snap.indicators.budgetShare < BUDGET_WARNING && snap.indicators.remainingBudget > 0) {
    add({ id: 'low_budget', severity: 'warning', title: 'Collapse warning: carbon budget nearly spent',
      detail: `Only ${(snap.indicators.budgetShare * 100).toFixed(0)}% of the carbon budget is left.`, why: 'emissions' });
  }
  return [...out.values()].sort((a, b) => (a.severity === b.severity ? a.id.localeCompare(b.id) : a.severity === 'critical' ? -1 : 1));
}
