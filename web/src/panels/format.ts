import type { ActionInfo, Command, LeverInfo, MetricKey, SimEvent, Snapshot, TurnResult } from '../contract';

export const METRIC_LABEL: Record<MetricKey, string> = {
  gdp: 'GDP', gpi: 'GPI', unemployment: 'Unemployment', gini: 'Inequality', wage_share: 'Wage share', profit_share: 'Profit share',
  wellbeing: 'Wellbeing', emissions: 'Emissions', budget: 'Carbon budget', temperature: 'Temperature', legitimacy: 'Legitimacy',
  debt: 'Public debt', money: 'Money supply', prices: 'Prices', housing: 'Housing', forest: 'Forest', commons: 'Commons',
};

export const REASON_TEXT: Record<string, string> = {
  transition_in_progress: 'A system transition is already in progress',
  nothing_to_remove: 'Nothing of that kind to remove',
  same_system: 'Already in that system',
  lever_unavailable: 'Not available in this system',
  insufficient_political_capital: 'Not enough political capital',
  insufficient_funds: 'Not enough money',
  invalid_command: 'Invalid value',
};

export const pct = (v: number, d = 0) => `${(v * 100).toFixed(d)}%`;
export const money = (v: number) => `${v < 0 ? '-' : ''}$${compact(Math.abs(v))}`;
export function compact(v: number): string {
  if (v >= 1e6) return `${(v / 1e6).toFixed(1)}M`;
  if (v >= 1e3) return `${(v / 1e3).toFixed(v >= 1e4 ? 0 : 1)}k`;
  return v.toFixed(0);
}

export function formatLever(l: LeverInfo, v: number | boolean | string): string {
  if (typeof v === 'boolean') return v ? 'on' : 'off';
  if (typeof v === 'string') return v;
  switch (l.unit) {
    case 'fraction': return pct(v, l.step < 0.01 ? 2 : 0);
    case 'usd': return money(v);
    case 'usd_per_tonne': return `$${v}/t`;
    case 'tonnes': return v === 0 ? 'none' : `${v} t`;
    case 'bool': return v ? 'on' : 'off';
    case 'enum': return l.options?.[v] ?? String(v);
  }
}

const actionLabel = (actions: ActionInfo[], key: string) => actions.find((a) => a.key === key)?.label ?? key;

export function describeCommand(c: Command, levers: LeverInfo[], actions: ActionInfo[]): string {
  switch (c.type) {
    case 'set_policy': {
      const l = levers.find((x) => x.key === c.lever);
      const v = typeof c.value === 'object' ? c.value.mode : c.value;
      return `${l?.label ?? c.lever} -> ${l && typeof v !== 'string' ? formatLever(l, v) : String(v)}`;
    }
    case 'switch_system': return `Switch system to ${c.to}`;
    case 'place': return `Build ${actionLabel(actions, `place:${c.kind}`)}${c.tile ? ` @ (${c.tile.x},${c.tile.y})` : ''}`;
    case 'remove': return actionLabel(actions, `remove:${c.kind}`);
  }
}

/** All event wording lives here: the contract carries payloads only. */
export function formatEvent(e: SimEvent, snap: Snapshot | null): string {
  const hh = (id: number) => snap?.households.find((h) => h.id === id)?.label ?? `household ${id}`;
  const firm = (id: number) => snap?.firms.find((f) => f.id === id)?.name ?? `firm ${id}`;
  switch (e.type) {
    case 'commuted': return `${e.people} ${hh(e.household)} commute to ${firm(e.firm)}`;
    case 'hired': return `${firm(e.firm)} hired ${e.people}`;
    case 'fired': return `${firm(e.firm)} laid off ${e.people}`;
    case 'loan_defaulted': return `${firm(e.firm)} defaulted on a loan`;
    case 'firm_founded': return `${firm(e.firm)} opened`;
    case 'firm_closed': return `${firm(e.firm)} closed`;
    case 'forest_shrank': return `The forest shrank (stock ${pct(e.stock)})`;
    case 'commons_collapsed': return 'The commons collapsed';
    case 'commons_enclosed': return `The commons were enclosed by ${hh(e.owner)}`;
    case 'policy_blocked': return `A policy was blocked: ${REASON_TEXT[e.reason] ?? e.reason}`;
    case 'command_activated': return `Pending item #${e.id} took effect`;
    case 'bank_failed': return 'The bank failed';
    case 'rent_shortfall': return `${hh(e.household)} fell ${money(e.amount)} short on rent`;
    case 'spending_cut': return `Spending cut by ${money(e.shortfall)}`;
    case 'bank_bailout': return `Bank bailed out for ${money(e.amount)}`;
    case 'class_changed': return `${hh(e.household)} moved to the ${e.to} class`;
    case 'system_switch_started': return `Switch to the ${e.to} system began`;
    case 'system_switch_completed': return `Now a ${e.to} system`;
    case 'budget_exhausted': return 'The carbon budget is exhausted';
    case 'collapse': return `The town collapsed: ${e.reason.replaceAll('_', ' ')}`;
  }
}

/** 3 to 5 lines for the review-phase briefing card. */
export function briefingLines(r: TurnResult | null, levers: LeverInfo[], actions: ActionInfo[]): string[] {
  if (!r) return ['Welcome. Review the town, then plan your first quarter.'];
  const lines: string[] = [];
  r.activated.forEach((a) => lines.push(`Took effect: ${describeCommand(a.command, levers, actions)}`));
  r.policyGaps.forEach((g) => {
    const l = levers.find((x) => x.key === g.lever);
    lines.push(`Lobbying weakened ${l?.label ?? g.lever}: set ${l ? formatLever(l, g.nominal) : g.nominal}, about ${l ? formatLever(l, g.effective) : g.effective.toFixed(2)} in force`);
  });
  r.rejected.forEach((x) => lines.push(`Blocked: ${describeCommand(x.command, levers, actions)} (${REASON_TEXT[x.reason] ?? x.reason})`));
  r.months.flatMap((m) => m.events).filter((e) => e.importance >= 2 && e.type !== 'command_activated' && e.type !== 'policy_blocked')
    .slice(0, 3).forEach((e) => lines.push(formatEvent(e, r.snapshot)));
  return lines.length ? lines.slice(0, 5) : ['A quiet quarter.'];
}

export const leadText = (n: number) => (n <= 0 ? 'immediately' : n === 1 ? 'next turn' : `in ${n} turns`);
