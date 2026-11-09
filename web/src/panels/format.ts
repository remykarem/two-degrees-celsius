import type { ActionInfo, Command, LeverInfo, MetricKey, SimEvent, Snapshot, TurnResult } from '../contract';

export const METRIC_LABEL: Record<MetricKey, string> = {
  gdp: 'GDP', gpi: 'GPI', unemployment: 'Unemployment', gini: 'Inequality', wage_share: 'Wage share', profit_share: 'Profit share',
  wellbeing: 'Wellbeing', emissions: 'Emissions', budget: 'Carbon budget', temperature: 'Temperature', legitimacy: 'Legitimacy',
  debt: 'Public debt', money: 'Money supply', prices: 'Prices', housing: 'Housing', forest: 'Forest', commons: 'Commons',
  avg_income: 'Average income', median_income: 'Median income', top_wealth_share: 'Top household wealth',
};

export const REASON_TEXT: Record<string, string> = {
  transition_in_progress: 'A system transition is already in progress',
  nothing_to_remove: 'Nothing of that kind to remove',
  same_system: 'Already in that system',
  lever_unavailable: 'Not available in this system',
  insufficient_political_capital: 'Not enough political capital',
  insufficient_funds: 'Not enough money',
  invalid_command: 'Invalid value',
  tile_occupied: 'Something is already built (or queued) on that tile',
  tile_invalid: 'Nothing can be built on that tile',
};

export const SYSTEM_NAME = { market: 'Market capitalism', regulated: 'Regulated capitalism', coop: 'Worker cooperatives' } as const;
export const pct = (v: number, d = 0) => `${(v * 100).toFixed(d)}%`;
export const money = (v: number) => `${v < 0 ? '-' : ''}$${compact(Math.abs(v))}`;
/** 1234 -> 1.2k, 56000 -> 56k, 2.5e6 -> 2.5M, 12.5 -> 12.5, 0 -> 0. */
export function compact(v: number): string {
  if (v >= 1e6) return `${trim((v / 1e6).toFixed(2))}M`;
  if (v >= 1e4) return `${(v / 1e3).toFixed(0)}k`;
  if (v >= 1e3) return `${trim((v / 1e3).toFixed(1))}k`;
  if (v >= 100) return v.toFixed(0);
  return trim(v.toFixed(v >= 10 ? 1 : 2));
}
const trim = (s: string) => (s.includes('.') ? s.replace(/0+$/, '').replace(/\.$/, '') : s);
/** Tonnes with a unit: 1200 -> "1.2k t". */
export const tonnes = (v: number) => `${v < 0 ? '-' : ''}${compact(Math.abs(v))} t`;
export const signed = (v: number, f: (x: number) => string) => (v > 0 ? `+${f(v)}` : v < 0 ? `-${f(-v)}` : f(0));
export type Trend = 'up' | 'down' | 'flat';
export const ARROW: Record<Trend, string> = { up: '▲', down: '▼', flat: '■' };
export const TREND_WORD: Record<Trend, string> = { up: 'up', down: 'down', flat: 'unchanged' };
/** Direction of a change, flat when the move is below 0.1% of the larger magnitude. */
export function trendOf(from: number, to: number): Trend {
  const d = to - from;
  const scale = Math.max(Math.abs(from), Math.abs(to), 1e-9);
  return Math.abs(d) / scale < 1e-3 ? 'flat' : d > 0 ? 'up' : 'down';
}

export function formatLever(l: LeverInfo, v: number | boolean | string): string {
  if (typeof v === 'boolean') return v ? 'on' : 'off';
  if (typeof v === 'string') return v;
  if (l.zeroMeansNone && v === 0) return 'no cap';
  switch (l.unit) {
    case 'fraction': return pct(v, l.step < 0.01 ? 2 : 0);
    case 'usd': return money(v);
    case 'usd_per_month': return `${money(v)}/month`;
    case 'usd_per_tonne': return `$${compact(v)}/t`;
    case 'tonnes_per_month': return l.zeroMeansNone && v === 0 ? 'no cap' : `${compact(v)} t/month`;
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
      return `${l?.label ?? c.lever} → ${l && typeof v !== 'string' ? formatLever(l, v) : String(v)}`;
    }
    case 'switch_system': return `Switch system to ${SYSTEM_NAME[c.to]}`;
    case 'place': return `${actionLabel(actions, `place:${c.kind}`)} @ (${c.tile.x},${c.tile.y})`;
    case 'remove': return `Remove what stands at (${c.tile.x},${c.tile.y})`;
  }
}

/** People are floats in the real sim (2.0000000000000057); show at most one decimal. */
const n = (v: number) => String(Math.round(v * 10) / 10);

/** All event wording lives here: the contract carries payloads only. */
export function formatEvent(e: SimEvent, snap: Snapshot | null): string {
  const hh = (id: number) => snap?.households.find((h) => h.id === id)?.label ?? `household ${id}`;
  const person = (id: number) => { const p = snap?.people.find((x) => x.id === id); return p ? `${hh(p.household)} ${p.label}` : `person ${id}`; };
  const firm = (id: number) => snap?.firms.find((f) => f.id === id)?.name ?? `firm ${id}`;
  switch (e.type) {
    case 'commuted': return `${person(e.person)} commutes to ${firm(e.firm)}`;
    case 'person_hired': return `${firm(e.firm)} hired ${person(e.person)}`;
    case 'person_fired': return `${firm(e.firm)} laid off ${person(e.person)}`;
    case 'household_moved': { const t = snap?.homes.find((x) => x.id === e.to)?.tile; return `${hh(e.household)} moved into home ${e.to}${t ? ` at (${t.x},${t.y})` : ''}`; }
    case 'household_evicted': return `${hh(e.household)} was evicted and has no home`;
    case 'built': return `A ${e.kind.replaceAll('_', ' ')} was built at (${e.tile.x},${e.tile.y})`;
    case 'removed': return `A building or road was removed at (${e.tile.x},${e.tile.y})`;
    case 'electricity_rationed': return `Electricity was rationed (${pct(e.served)} of demand served)`;
    case 'purchase': return `${hh(e.household)} spent ${money(e.amount)} at ${firm(e.firm)}`;
    case 'hired': return `${firm(e.firm)} hired ${n(e.people)}`;
    case 'fired': return `${firm(e.firm)} laid off ${n(e.people)}`;
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
    case 'system_switch_started': return `The switch to ${SYSTEM_NAME[e.to]} began`;
    case 'system_switch_completed': return `The town is now ${SYSTEM_NAME[e.to]}`;
    case 'budget_exhausted': return 'The carbon budget is exhausted';
    case 'collapse': return `The town collapsed: ${e.reason.replaceAll('_', ' ')}`;
  }
}

/** Lobbying only counts as a weakening when the value in force really differs from the one set (beyond rounding noise). */
export const lobbyingDiffers = (nominal: number, effective: number) => Math.abs(nominal - effective) > Math.max(1e-6, Math.abs(nominal) * 1e-4);

/** 3 to 5 lines for the review-phase briefing card. */
export function briefingLines(r: TurnResult | null, levers: LeverInfo[], actions: ActionInfo[]): string[] {
  if (!r) return ['Welcome. Review the town, then plan your first quarter.'];
  const lines: string[] = [];
  r.activated.forEach((a) => lines.push(`Took effect: ${describeCommand(a.command, levers, actions)}`));
  r.policyGaps.forEach((g) => {
    const l = levers.find((x) => x.key === g.lever);
    if (!lobbyingDiffers(g.nominal, g.effective)) return;
    lines.push(`Lobbying weakened ${l?.label ?? g.lever}: set ${l ? formatLever(l, g.nominal) : g.nominal}, about ${l ? formatLever(l, g.effective) : g.effective.toFixed(2)} in force`);
  });
  r.rejected.forEach((x) => lines.push(`Blocked: ${describeCommand(x.command, levers, actions)} (${REASON_TEXT[x.reason] ?? x.reason})`));
  r.months.flatMap((m) => m.events).filter((e) => e.importance >= 2 && e.type !== 'command_activated' && e.type !== 'policy_blocked')
    .slice(0, 3).forEach((e) => lines.push(formatEvent(e, r.snapshot)));
  return lines.length ? lines.slice(0, 5) : ['A quiet quarter.'];
}

/** Turns until a queued item takes effect. It fires at the start of the submit of turn `effectiveTurn`, so effectiveTurn == snapshot.turn is "next turn". */
export const turnsUntil = (effectiveTurn: number, snapTurn: number) => Math.max(1, effectiveTurn - snapTurn + 1);
/** "next turn", "in 3 turns", or a range like "in 2 to 3 turns". */
export function leadRangeText(leads: number[]): string {
  if (!leads.length) return '';
  const lo = Math.min(...leads), hi = Math.max(...leads);
  return lo === hi ? leadText(lo) : lo <= 0 || lo === 1 ? `${leadText(lo)} to ${hi} turns` : `in ${lo} to ${hi} turns`;
}
export const leadText = (n: number) => (n <= 0 ? 'immediately' : n === 1 ? 'next turn' : `in ${n} turns`);
