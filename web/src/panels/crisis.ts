import type { SimEvent, Snapshot, TurnResult } from '../contract';
import { budgetOf, legitimacyBacking } from './backing';
import { fireLabel, fireSelection, weatherOf } from '../store/weather';
import type { Selection } from '../store';
import { REASON_TEXT, severityWord } from './format';
import type { WhyKind } from './metrics';
import { bands, text } from '../store/catalog';

export interface Crisis {
  id: string;
  severity: 'critical' | 'warning';
  title: string;
  detail: string;
  why?: WhyKind;
  /** A map selection that shows the problem (the burning tile). */
  show?: Selection;
  /** Offer the Adapt shortcut (resilience lever and fire station). */
  adapt?: boolean;
}

const allEvents = (r: TurnResult | null): SimEvent[] => r?.months.flatMap((m) => m.events) ?? [];

/** Crises from the latest turn's events plus thresholds on the current snapshot. Critical first, then by id. */
export function deriveCrises(snap: Snapshot | null, last: TurnResult | null): Crisis[] {
  if (!snap) return [];
  const out = new Map<string, Crisis>();
  const add = (c: Crisis) => out.set(c.id, c);
  const events = allEvents(last);

  if (snap.bank.failed || events.some((e) => e.type === 'bank_failed')) {
    add({ id: 'bank_failed', severity: 'critical', title: text('crisis', 'bank_failed_title'),
      detail: text('crisis', 'bank_failed_detail') });
  }
  if (events.some((e) => e.type === 'commons_collapsed')) {
    add({ id: 'commons_collapsed', severity: 'critical', title: text('crisis', 'commons_collapsed_title'),
      detail: text('crisis', 'commons_collapsed_detail') });
  }
  for (const e of events) {
    if (e.type === 'collapse') add({ id: 'collapse', severity: 'critical', title: text('crisis', 'collapse_title'), detail: `Loss condition reached: ${e.reason.replaceAll('_', ' ')}.` });
    if (e.type === 'budget_exhausted') add({ id: 'budget_exhausted', severity: 'critical', title: text('crisis', 'budget_exhausted_title'), detail: text('crisis', 'budget_exhausted_detail'), why: 'emissions' });
    if (e.type === 'policy_blocked') {
      add({ id: `policy_blocked:${e.reason}`, severity: 'warning', title: text('crisis', 'policy_blocked_title'), detail: REASON_TEXT[e.reason] ?? e.reason });
    }
  }
  // Weather (Town v4): calm wording, never blocking. Active fires and an active drought each get one banner.
  const w = weatherOf(snap);
  if (w && w.fires.length) {
    const first = w.fires[0]!;
    const show = fireSelection(snap);
    add({ id: 'fire_active', severity: 'warning', title: fireLabel(snap, first.tile) + (w.fires.length > 1 ? ` (${w.fires.length} tiles burning)` : ''),
      detail: text('crisis', 'fire_detail'), why: 'emissions', ...(show ? { show } : {}), adapt: true });
  }
  if (w?.drought) {
    add({ id: 'drought_active', severity: 'warning', title: `A ${severityWord(w.drought.severity)} drought is under way`,
      detail: `${text('crisis', 'drought_detail')} About ${Math.ceil(w.drought.monthsLeft)} more ${w.drought.monthsLeft <= 1 ? 'month' : 'months'}.`, adapt: true });
  }
  const leg = snap.government.legitimacy;
  if (leg < bands().crisis.legitimacyBelow) {
    add({ id: 'low_legitimacy', severity: 'warning', title: text('crisis', 'low_legitimacy_title'),
      detail: `Legitimacy is ${(leg * 100).toFixed(0)}% (${legitimacyBacking(snap)}). ${text('crisis', 'low_legitimacy_detail')}` });
  }
  if (snap.indicators.budgetShare < bands().crisis.budgetBelow && snap.indicators.remainingBudget > 0) {
    add({ id: 'low_budget', severity: 'warning', title: text('crisis', 'low_budget_title'),
      detail: `Only ${(snap.indicators.budgetShare * 100).toFixed(0)}% of the carbon budget is left (${budgetOf(snap)}).`, why: 'emissions' });
  }
  return [...out.values()].sort((a, b) => (a.severity === b.severity ? a.id.localeCompare(b.id) : a.severity === 'critical' ? -1 : 1));
}
