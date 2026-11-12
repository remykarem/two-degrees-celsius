import type { AccountRef, LedgerKind, HistoryPoint, MetricKey, Snapshot } from '../contract';
import { money, pct, tonnes, trendOf, type Trend } from './format';
import { healthWord } from '../store/weather';
import { text } from '../store/catalog';
import { unemploymentCount } from './people';
import { budgetOf, debtBacking, forestTiles, legitimacyBacking, richestBacking, shareBacking, stockOf } from './backing';

export type MetricTab = 'Economy' | 'Ecology' | 'Society' | 'People' | 'Finance' | 'Policies' | 'Industry';
export const METRIC_TABS: MetricTab[] = ['Economy', 'Ecology', 'Society', 'People', 'Finance', 'Policies', 'Industry'];
export type BreakdownKey = 'gdp' | 'gpi' | 'wageShare' | 'profitShare' | 'debtToGdp' | 'gini' | 'legitimacy' | 'capitalRatio' | 'emissions' | 'taxes';
/** Which trace the "why?" link opens. */
export type WhyKind = 'emissions' | 'income' | 'lobbying';

export interface MetricDef {
  id: string;
  metric: MetricKey;
  tab: MetricTab;
  label: string;
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
  { id: 'gdp', metric: 'gdp', tab: 'Economy', label: 'GDP (real, per year)', value: (s) => s.indicators.gdpRealAnnual, fmt: money, series: (h) => h.gdpRealAnnual, goodUp: true, why: 'income' },
  { id: 'gpi', metric: 'gpi', tab: 'Economy', label: 'GPI (per year)', value: (s) => s.indicators.gpiAnnual, fmt: money, series: (h) => h.gpiAnnual, goodUp: true, why: null },
  { id: 'unemployment', metric: 'unemployment', tab: 'Economy', label: 'Unemployment', value: (s) => s.indicators.unemployment, fmt: (v) => pct(v, 1), series: (h) => h.unemployment, goodUp: false, why: null,
    detail: (s) => { const u = unemploymentCount(s); return `${u.unemployed} of ${u.labourForce} unemployed`; } },
  { id: 'avg_income', metric: 'avg_income', tab: 'Economy', label: 'Average income per person', value: (s) => s.indicators.avgIncomePerPerson, fmt: money, series: (h) => h.avgIncomePerPerson, goodUp: true, why: 'income' },
  { id: 'median_income', metric: 'median_income', tab: 'Economy', label: 'Median income per person', value: (s) => s.indicators.medianIncomePerPerson, fmt: money, series: (h) => h.medianIncomePerPerson, goodUp: true, why: 'income' },
  { id: 'debt', metric: 'debt', tab: 'Economy', label: 'Public debt / GDP', value: (s) => s.government.debtToGdp, fmt: (v) => pct(v, 1), series: (h) => h.debtToGdp, goodUp: false, why: null, detail: debtBacking },
  { id: 'wage_share', metric: 'wage_share', tab: 'Economy', label: 'Wage share', value: (s) => s.indicators.wageShare, fmt: (v) => pct(v, 1), series: null, goodUp: true, why: 'income', detail: (s) => shareBacking(s, 'wage') },
  { id: 'emissions', metric: 'emissions', tab: 'Ecology', label: 'Emissions, net (per month)', value: (s) => s.breakdowns?.emissions.month.headline ?? s.nature.netEmissionsMonthly, fmt: tonnes, series: (h) => h.emissions, goodUp: false, why: 'emissions',
    detail: (s) => { const e = s.breakdowns?.emissions; return e ? `this turn ${tonnes(e.turn.net)}; accumulated ${tonnes(e.cumulative.tonnes)} of ${tonnes(e.cumulative.budget)}` : `accumulated ${tonnes(s.nature.cumulativeTonnes)}`; } },
  { id: 'budget', metric: 'budget', tab: 'Ecology', label: 'Carbon budget left', value: (s) => s.indicators.budgetShare, fmt: (v) => pct(v, 1), series: (h) => h.budgetShare, goodUp: true, why: 'emissions', detail: budgetOf },
  { id: 'temperature', metric: 'temperature', tab: 'Ecology', label: 'Temperature rise', value: (s) => s.indicators.temperature, fmt: (v) => `${v.toFixed(2)} C`, series: null, goodUp: false, why: 'emissions' },
  { id: 'forest', metric: 'forest', tab: 'Ecology', label: 'Forest', value: (s) => s.nature.forest, fmt: (v) => pct(v, 0), series: null, goodUp: true, why: null, detail: (s) => stockOf(s.nature.forestStock, s.nature.forest) ?? forestTiles(s) },
  { id: 'commons', metric: 'commons', tab: 'Ecology', label: 'Commons', value: (s) => s.nature.commons, fmt: (v) => pct(v, 0), series: null, goodUp: true, why: null, detail: (s) => stockOf(s.nature.commonsStock, s.nature.commons) ?? 'stock not on the wire' },
  { id: 'forest_health', metric: 'forest_health', tab: 'Ecology', label: 'Forest health', value: (s) => s.nature.forestHealth ?? s.nature.forest, fmt: (v) => pct(v, 0), series: (h) => h.forestHealth, goodUp: true, why: 'emissions',
    detail: (s) => `${healthWord(s.nature.forestHealth ?? s.nature.forest)}: ${forestTiles(s)}` },
  { id: 'commons_health', metric: 'commons_health', tab: 'Ecology', label: 'Commons health', value: (s) => s.nature.commonsHealth ?? s.nature.commons, fmt: (v) => pct(v, 0), series: (h) => h.commonsHealth, goodUp: true, why: null,
    detail: (s) => `${healthWord(s.nature.commonsHealth ?? s.nature.commons)}: ${stockOf(s.nature.commonsStock, s.nature.commons) ?? 'commons stock not on the wire'}` },
  { id: 'farm_health', metric: 'farm_health', tab: 'Ecology', label: 'Farm health', value: (s) => s.nature.farmHealth ?? ((s.nature.forest + s.nature.commons) / 2), fmt: (v) => pct(v, 0), series: (h) => h.farmHealth, goodUp: true, why: null,
    detail: (s) => `${healthWord(s.nature.farmHealth ?? ((s.nature.forest + s.nature.commons) / 2))}: forest ${stockOf(s.nature.forestStock, s.nature.forest) ?? '?'}, commons ${stockOf(s.nature.commonsStock, s.nature.commons) ?? '?'}${s.weather?.drought ? `, drought ${s.weather.drought.monthsLeft} months left` : ''}` },
  { id: 'climate_stress', metric: 'climate_stress', tab: 'Ecology', label: 'Climate stress', value: (s) => s.weather?.stress ?? 0, fmt: (v) => pct(v, 0), series: (h) => h.climateStress, goodUp: false, why: 'emissions', detail: (s) => `budget left ${budgetOf(s)}` },
  { id: 'gini', metric: 'gini', tab: 'Society', label: 'Inequality (Gini)', value: (s) => s.indicators.gini, fmt: (v) => v.toFixed(3), series: (h) => h.gini, goodUp: false, why: 'income' },
  { id: 'top_wealth_share', metric: 'top_wealth_share', tab: 'Society', label: 'Wealth of the richest household', value: (s) => s.indicators.topHouseholdWealthShare, fmt: (v) => pct(v, 0), series: (h) => h.topHouseholdWealthShare, goodUp: false, why: null, detail: richestBacking },
  { id: 'wellbeing', metric: 'wellbeing', tab: 'Society', label: 'Wellbeing', value: (s) => s.indicators.avgWellbeing, fmt: (v) => v.toFixed(3), series: (h) => h.avgWellbeing, goodUp: true, why: null },
  { id: 'legitimacy', metric: 'legitimacy', tab: 'Society', label: 'Legitimacy', value: (s) => s.government.legitimacy, fmt: (v) => pct(v, 1), series: (h) => h.legitimacy, goodUp: true, why: null, detail: legitimacyBacking },
];

/** Metric id -> the wire breakdown (snapshot.breakdowns key) that explains it. Metrics without one open the explorer topic instead. */
export const METRIC_BREAKDOWN: Record<string, BreakdownKey | undefined> = {
  gdp: 'gdp', gpi: 'gpi', debt: 'debtToGdp', wage_share: 'wageShare', gini: 'gini', legitimacy: 'legitimacy', emissions: 'emissions', budget: 'emissions',
  top_wealth_share: 'gini',
};

/** The value and its backing numbers as one string (what the lint-style test checks for bare percentages). */
export const metricText = (m: MetricDef, s: Snapshot): string => (m.detail ? `${m.fmt(m.value(s))} (${m.detail(s)})` : m.fmt(m.value(s)));

/** The history values of a metric, or [] when the package's history lacks the field (a package built before contract v1.2). */
export function seriesOf(def: MetricDef, history: HistoryPoint[]): number[] {
  if (!def.series) return [];
  const v = history.map(def.series);
  return v.every((x) => typeof x === 'number' && Number.isFinite(x)) ? v : [];
}
/** The plain-language one-liner shown as a tooltip: authored in content/ui.json `text.metric_hint`. */
export const metricHint = (id: string): string => text('metric_hint', id);
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
  if (!def.series || history.length < 2 || seriesOf(def, history.slice(-2)).length < 2) return null;
  const a = def.series(history[history.length - 2]!), b = def.series(history[history.length - 1]!);
  const trend = trendOf(a, b);
  return { trend, tone: toneOf(trend, def.goodUp), from: a, to: b };
}

export interface Runway {
  /** Total budget at the start (tonnes), derived from remaining / share. */
  total: number;
  remaining: number;
  /** Tonnes used per turn at the current net rate (the sim's breakdowns.emissions.runway). */
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

/** The budget runway as the simulation computes it (`breakdowns.emissions.runway`); this only adds the chart series. Null without the breakdowns or when the budget is gone. */
export function runway(s: Snapshot): Runway | null {
  const share = s.indicators.budgetShare;
  const remaining = s.indicators.remainingBudget;
  const r = s.breakdowns?.emissions.runway;
  if (!r || (share <= 0 && remaining <= 0)) return null;
  const total = s.breakdowns?.emissions.cumulative.budget ?? 0;
  const perTurn = r.perTurn, runOutTurn = r.runOutTurn ?? null;
  const lastTurn = s.game.lastTurn;
  const now = Math.min(s.turn, lastTurn + 1);
  const actual = s.history.map((h) => ({ turn: h.turn, remaining: h.budgetShare * total }));
  const endTurn = runOutTurn !== null ? Math.min(runOutTurn, lastTurn + 1) : lastTurn + 1;
  const projection = [{ turn: now, remaining }, { turn: endTurn, remaining: Math.max(0, remaining - perTurn * (endTurn - now)) }];
  return { total, remaining, perTurn, runOutTurn, lastTurn, actual, projection, runsOutBeforeEnd: r.runsOutBeforeEnd };
}

export function runwayText(r: Runway, label: (turn: number) => string): string {
  if (r.remaining <= 0) return text('rules', 'budget_spent');
  if (r.runOutTurn === null) return 'At the current rate emissions do not use up the budget.';
  const t = Math.ceil(r.runOutTurn);
  const when = label(Math.min(t, r.lastTurn));
  return r.runsOutBeforeEnd
    ? `At the current rate the budget runs out around ${when} (turn ${t}).`
    : 'At the current rate the budget lasts past the final turn.';
}

/** Metrics that come from a ledger: the "see transactions" link beside them (contract v1.4). */
export const METRIC_LEDGER: Record<string, { account: AccountRef; kind?: LedgerKind }> = {
  debt: { account: { kind: 'government' }, kind: 'loan_new' },
};
