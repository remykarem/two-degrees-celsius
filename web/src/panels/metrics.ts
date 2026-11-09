import type { HistoryPoint, MetricKey, Snapshot } from '../contract';
import { money, pct, tonnes, trendOf, type Trend } from './format';
import { unemploymentCount } from './people';

export type MetricTab = 'Economy' | 'Ecology' | 'Society' | 'People';
export const METRIC_TABS: MetricTab[] = ['Economy', 'Ecology', 'Society', 'People'];
/** Which trace the "why?" link opens. */
export type WhyKind = 'emissions' | 'income' | 'lobbying';

export interface MetricDef {
  id: string;
  metric: MetricKey;
  tab: MetricTab;
  label: string;
  /** Plain-language one-liner shown as a tooltip. */
  hint: string;
  value(s: Snapshot): number;
  fmt(v: number): string;
  /** Series for the sparkline from snapshot.history, or null when history does not carry it. */
  series: ((h: HistoryPoint) => number) | null;
  /** true: higher is better; false: lower is better; null: no judgement (neutral colour). */
  goodUp: boolean | null;
  why: WhyKind | null;
  /** Extra line under the value (for example "2 of 14 in the labour force"). */
  detail?(s: Snapshot): string;
}

export const METRICS: MetricDef[] = [
  { id: 'gdp', metric: 'gdp', tab: 'Economy', label: 'GDP (real, per year)', hint: 'Total output, adjusted for prices', value: (s) => s.indicators.gdpRealAnnual, fmt: money, series: (h) => h.gdpRealAnnual, goodUp: true, why: 'income' },
  { id: 'gpi', metric: 'gpi', tab: 'Economy', label: 'GPI (per year)', hint: 'Genuine progress: output adjusted for social and environmental costs', value: (s) => s.indicators.gpiAnnual, fmt: money, series: (h) => h.gpiAnnual, goodUp: true, why: null },
  { id: 'unemployment', metric: 'unemployment', tab: 'Economy', label: 'Unemployment', hint: 'Share of the labour force without a job', value: (s) => s.indicators.unemployment, fmt: (v) => pct(v, 1), series: (h) => h.unemployment, goodUp: false, why: null,
    detail: (s) => { const u = unemploymentCount(s); return `${u.unemployed} of ${u.labourForce} unemployed`; } },
  { id: 'avg_income', metric: 'avg_income', tab: 'Economy', label: 'Average income per person', hint: 'Total household income per person per month, children and retired people included', value: (s) => s.indicators.avgIncomePerPerson, fmt: money, series: (h) => h.avgIncomePerPerson, goodUp: true, why: 'income' },
  { id: 'median_income', metric: 'median_income', tab: 'Economy', label: 'Median income per person', hint: 'The middle person: half the town earns more, half less. Compare with the average to see skew', value: (s) => s.indicators.medianIncomePerPerson, fmt: money, series: (h) => h.medianIncomePerPerson, goodUp: true, why: 'income' },
  { id: 'debt', metric: 'debt', tab: 'Economy', label: 'Public debt / GDP', hint: 'Government debt relative to a year of output', value: (s) => s.government.debtToGdp, fmt: (v) => pct(v, 1), series: (h) => h.debtToGdp, goodUp: false, why: null },
  { id: 'wage_share', metric: 'wage_share', tab: 'Economy', label: 'Wage share', hint: 'Share of income paid as wages', value: (s) => s.indicators.wageShare, fmt: (v) => pct(v, 1), series: null, goodUp: true, why: 'income' },
  { id: 'emissions', metric: 'emissions', tab: 'Ecology', label: 'Emissions (per month)', hint: 'Gross emissions in the latest month (the history line is net monthly emissions at the end of each turn)', value: (s) => s.nature.grossEmissionsMonthly, fmt: tonnes, series: (h) => h.emissions, goodUp: false, why: 'emissions' },
  { id: 'budget', metric: 'budget', tab: 'Ecology', label: 'Carbon budget left', hint: 'Share of the carbon budget not yet used', value: (s) => s.indicators.budgetShare, fmt: (v) => pct(v, 1), series: (h) => h.budgetShare, goodUp: true, why: 'emissions' },
  { id: 'temperature', metric: 'temperature', tab: 'Ecology', label: 'Temperature rise', hint: 'Degrees C above the starting level', value: (s) => s.indicators.temperature, fmt: (v) => `${v.toFixed(2)} C`, series: null, goodUp: false, why: 'emissions' },
  { id: 'forest', metric: 'forest', tab: 'Ecology', label: 'Forest', hint: 'Forest stock, as a share of its maximum', value: (s) => s.nature.forest, fmt: (v) => pct(v, 0), series: null, goodUp: true, why: null },
  { id: 'commons', metric: 'commons', tab: 'Ecology', label: 'Commons', hint: 'Health of the shared land', value: (s) => s.nature.commons, fmt: (v) => pct(v, 0), series: null, goodUp: true, why: null },
  { id: 'gini', metric: 'gini', tab: 'Society', label: 'Inequality (Gini)', hint: '0 is perfectly equal, 1 is one person has everything', value: (s) => s.indicators.gini, fmt: (v) => v.toFixed(3), series: (h) => h.gini, goodUp: false, why: 'income' },
  { id: 'top_wealth_share', metric: 'top_wealth_share', tab: 'Society', label: 'Wealth of the richest household', hint: 'Share of all household net worth held by the single richest household', value: (s) => s.indicators.topHouseholdWealthShare, fmt: (v) => pct(v, 0), series: (h) => h.topHouseholdWealthShare, goodUp: false, why: null },
  { id: 'wellbeing', metric: 'wellbeing', tab: 'Society', label: 'Wellbeing', hint: 'Average wellbeing, 0 to 1', value: (s) => s.indicators.avgWellbeing, fmt: (v) => v.toFixed(3), series: (h) => h.avgWellbeing, goodUp: true, why: null },
  { id: 'legitimacy', metric: 'legitimacy', tab: 'Society', label: 'Legitimacy', hint: 'Public trust in government; at zero the town collapses', value: (s) => s.government.legitimacy, fmt: (v) => pct(v, 1), series: (h) => h.legitimacy, goodUp: true, why: null },
];

export const metricById = (id: string) => METRICS.find((m) => m.id === id);
export const metricsFor = (tab: MetricTab) => METRICS.filter((m) => m.tab === tab);

export type Tone = 'good' | 'bad' | 'neutral';
export function toneOf(trend: Trend, goodUp: boolean | null): Tone {
  if (trend === 'flat' || goodUp === null) return 'neutral';
  return (trend === 'up') === goodUp ? 'good' : 'bad';
}

export interface MetricChange { trend: Trend; tone: Tone; from: number; to: number }
export function changeBetween(def: MetricDef, from: Snapshot, to: Snapshot): MetricChange {
  const a = def.value(from), b = def.value(to);
  const trend = trendOf(a, b);
  return { trend, tone: toneOf(trend, def.goodUp), from: a, to: b };
}

/** Change since the previous completed turn, from history only (null with fewer than 2 points). */
export function historyChange(def: MetricDef, history: HistoryPoint[]): MetricChange | null {
  if (!def.series || history.length < 2) return null;
  const a = def.series(history[history.length - 2]!), b = def.series(history[history.length - 1]!);
  const trend = trendOf(a, b);
  return { trend, tone: toneOf(trend, def.goodUp), from: a, to: b };
}

export interface Runway {
  /** Total budget at the start (tonnes), derived from remaining / share. */
  total: number;
  remaining: number;
  /** Tonnes used per turn at the current net rate (3 months). */
  perTurn: number;
  /** Turn on which the budget runs out at the current rate, or null if it does not at this rate. */
  runOutTurn: number | null;
  lastTurn: number;
  /** Actual: remaining tonnes at the end of each completed turn. */
  actual: { turn: number; remaining: number }[];
  /** Projection from the current turn to the run-out turn (or the last turn). */
  projection: { turn: number; remaining: number }[];
  runsOutBeforeEnd: boolean;
}

export function runway(s: Snapshot): Runway | null {
  const share = s.indicators.budgetShare;
  const remaining = s.indicators.remainingBudget;
  if (share <= 0 && remaining <= 0) return null;
  const total = share > 0 ? remaining / share : 0;
  const perTurn = Math.max(0, s.nature.netEmissionsMonthly) * 3;
  const lastTurn = s.game.lastTurn;
  const now = Math.min(s.turn, lastTurn + 1);
  const runOutTurn = perTurn > 0 ? now + remaining / perTurn : null;
  const actual = s.history.map((h) => ({ turn: h.turn, remaining: h.budgetShare * total }));
  const endTurn = runOutTurn !== null ? Math.min(runOutTurn, lastTurn + 1) : lastTurn + 1;
  const projection = [{ turn: now, remaining }, { turn: endTurn, remaining: Math.max(0, remaining - perTurn * (endTurn - now)) }];
  return { total, remaining, perTurn, runOutTurn, lastTurn, actual, projection, runsOutBeforeEnd: runOutTurn !== null && runOutTurn <= lastTurn + 1 };
}

export function runwayText(r: Runway, label: (turn: number) => string): string {
  if (r.remaining <= 0) return 'The carbon budget is spent.';
  if (r.runOutTurn === null) return 'At the current rate emissions do not use up the budget.';
  const t = Math.ceil(r.runOutTurn);
  const when = label(Math.min(t, r.lastTurn));
  return r.runsOutBeforeEnd
    ? `At the current rate the budget runs out around ${when} (turn ${t}).`
    : 'At the current rate the budget lasts past the final turn.';
}
