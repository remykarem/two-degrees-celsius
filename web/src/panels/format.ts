import type { Command, LeverInfo, MetricKey } from '../contract';

export const METRIC_LABEL: Record<MetricKey, string> = {
  gdp: 'GDP', unemployment: 'Unemployment', gini: 'Inequality', emissions: 'Emissions', budget: 'Carbon budget',
  legitimacy: 'Legitimacy', wellbeing: 'Wellbeing', gpi: 'GPI', debt: 'Public debt', prices: 'Prices',
  housing: 'Housing', forest: 'Forest',
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
    case 'enum': return v ? 'community' : 'open';
  }
}

export function describeCommand(c: Command, levers: LeverInfo[], infraLabel: (k: string) => string): string {
  switch (c.type) {
    case 'set_policy': {
      const l = levers.find((x) => x.key === c.lever);
      const v = typeof c.value === 'object' ? c.value.mode : c.value;
      return `${l?.label ?? c.lever} -> ${l && typeof v !== 'string' ? formatLever(l, v) : String(v)}`;
    }
    case 'switch_system': return `Switch system to ${c.to}`;
    case 'place': return `Build ${infraLabel(c.kind)}${c.tile ? ` @ (${c.tile.x},${c.tile.y})` : ''}`;
    case 'remove': return `Remove ${infraLabel(c.kind)}`;
  }
}

export const leadText = (n: number) => (n <= 0 ? 'immediately' : n === 1 ? 'next turn' : `in ${n} turns`);
