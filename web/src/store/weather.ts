import type { ForestState, RiskLevel, Snapshot, TileRef, Weather } from '../contract';
import { layoutOf, sameTile, type Selection } from './map';
import { bands, fill, text, wordOf } from './catalog';

/**
 * Weather as the panels read it (Town v4). Everything here is DERIVED from the snapshot; nothing is stored, so the mock and the real sim
 * go through the same code. A package built before contract v1.2 has no `weather`: every function then reports "no weather".
 */

export const weatherOf = (snap: Snapshot | null): Weather | null => {
  const w = (snap as { weather?: Weather } | null)?.weather;
  return w && typeof w.stress === 'number' ? w : null;
};

/** Health bands (ids and words) come from content/ui.json `bands.health`: a value at or above a band's `min` is in it. */
export type HealthLevel = 'healthy' | 'strained' | 'poor' | 'failing';
const bandOf = (v: number) => bands().health.find((b) => v >= b.min) ?? bands().health[bands().health.length - 1]!;
export const healthLevel = (v: number): HealthLevel => bandOf(v).id as HealthLevel;
export const healthWord = (v: number): string => bandOf(v).word;

export interface HealthMeter { id: 'forest' | 'commons' | 'farm'; label: string; value: number; level: HealthLevel; word: string; hint: string }
/** Forest, commons and farm health (0..1). Falls back to the stock shares for the first two when a package has no health fields. */
export function healthMeters(snap: Snapshot | null): HealthMeter[] {
  if (!snap) return [];
  const n = snap.nature as Partial<Snapshot['nature']> & { forest: number; commons: number };
  const forest = n.forestHealth ?? n.forest, commons = n.commonsHealth ?? n.commons;
  const farm = n.farmHealth ?? (forest + commons) / 2;
  const mk = (id: HealthMeter['id'], label: string, value: number, hint: string): HealthMeter => {
    const v = Math.min(1, Math.max(0, Number.isFinite(value) ? value : 0));
    const level = healthLevel(v);
    return { id, label, value: v, level, word: healthWord(v), hint };
  };
  return [
    mk('forest', 'Forest', forest, text('weather', 'health_forest')),
    mk('commons', 'Commons', commons, text('weather', 'health_commons')),
    mk('farm', 'Farm', farm, text('weather', 'health_farm')),
  ];
}

export const RISK_WORD: Record<RiskLevel, string> = { low: 'Low', medium: 'Medium', high: 'High' };
const RANK: Record<RiskLevel, number> = { low: 0, medium: 1, high: 2 };

export interface StressGauge {
  stress: number;
  percent: number;
  /** Budget share remaining, 0..1. */
  remaining: number;
  onset: number;
  full: number;
  /** Where the stress starts and ends on the "budget left" axis, 0..1 (same numbers; here so the gauge needs no constants). */
  word: string;
  text: string;
}
export function stressGauge(snap: Snapshot | null): StressGauge | null {
  const w = weatherOf(snap);
  if (!snap || !w) return null;
  const s = w.stress;
  const word = s <= 0 ? text('weather', 'stress_calm') : wordOf(bands().stressWords, s);
  const { onset, full } = w;
  const line = s <= 0
    ? fill(text('weather', 'stress_none'), { onset: `${Math.round(onset * 100)}%` })
    : fill(text('weather', 'stress_active'), { full: `${Math.round(full * 100)}%` });
  return { stress: s, percent: Math.round(s * 100), remaining: snap.indicators.budgetShare, onset, full, word, text: line };
}

export interface ForecastView {
  /** Calm one-line headline, e.g. "Dry spell likely". */
  headline: string;
  detail: string;
  fireWord: string;
  droughtWord: string;
  fireRisk: RiskLevel;
  droughtRisk: RiskLevel;
  /** Highest of the two risk levels. */
  level: RiskLevel;
  active: { fire: number; drought: boolean };
  /** True when there is nothing to warn about (low risk, nothing burning, no drought). */
  calm: boolean;
  /** Worth offering the Adapt shortcut. */
  adapt: boolean;
}
export function forecastView(snap: Snapshot | null): ForecastView | null {
  const w = weatherOf(snap);
  if (!w) return null;
  const { fireRisk, droughtRisk } = w.forecast;
  const fires = w.fires.length;
  const dry = w.drought !== null;
  const level: RiskLevel = RANK[fireRisk] >= RANK[droughtRisk] ? fireRisk : droughtRisk;
  const parts: string[] = [];
  if (fires) parts.push(`Forest fire burning on ${fires} ${fires === 1 ? 'tile' : 'tiles'}`);
  if (dry) parts.push(`Drought under way, ${w.drought!.monthsLeft} ${w.drought!.monthsLeft === 1 ? 'month' : 'months'} left`);
  let headline: string;
  if (parts.length) headline = parts.join('. ');
  else if (fireRisk === 'high') headline = text('weather', 'headline_fire_high');
  else if (droughtRisk === 'high') headline = text('weather', 'headline_drought_high');
  else if (fireRisk === 'medium') headline = text('weather', 'headline_fire_medium');
  else if (droughtRisk === 'medium') headline = text('weather', 'headline_drought_medium');
  else headline = text('weather', 'headline_calm');
  const detail = w.stress <= 0 ? text('weather', 'forecast_none')
    : level === 'low' ? text('weather', 'forecast_quiet')
    : text('weather', 'forecast_adapt');
  return {
    headline, detail, fireWord: RISK_WORD[fireRisk], droughtWord: RISK_WORD[droughtRisk], fireRisk, droughtRisk, level,
    active: { fire: fires, drought: dry }, calm: level === 'low' && !fires && !dry, adapt: RANK[level] >= 1 || fires > 0 || dry,
  };
}

export const forestStateOf = (snap: Snapshot | null, t: TileRef): ForestState | null => {
  const tile = snap?.map.tiles[t.y * snap.map.width + t.x];
  return tile && tile.kind === 'forest' ? tile.state ?? 'standing' : null;
};
/** Burning tiles with their months left. */
export const fireTiles = (snap: Snapshot | null): { tile: TileRef; monthsLeft: number }[] => weatherOf(snap)?.fires ?? [];

/** The selection that points at the first fire (for a "show me" link), or null. */
export function fireSelection(snap: Snapshot | null): Selection | null {
  const f = fireTiles(snap)[0];
  return f ? { kind: 'tile', x: f.tile.x, y: f.tile.y } : null;
}

const NEAR_MAX = 4;
/** Name of the nearest building within a few tiles (homes by household, firms by name, plants, the bank, the town hall), or null. */
export function nearestLandmark(snap: Snapshot | null, t: TileRef): string | null {
  if (!snap) return null;
  const layout = layoutOf(snap);
  let best: { d: number; label: string } | null = null;
  const consider = (p: TileRef, label: string) => {
    const d = Math.hypot(p.x - t.x, p.y - t.y);
    if (d <= NEAR_MAX && (!best || d < best.d)) best = { d, label };
  };
  for (const s of layout.scenery) if (s.kind !== 'forest' && s.kind !== 'commons') consider(s, s.label);
  for (const p of layout.placed) consider(p, p.kind.replaceAll('_', ' '));
  return (best as { label: string } | null)?.label ?? null;
}
/** "Forest fire near Chen home", or "Forest fire near (4,7)" when no building is close. */
export const fireLabel = (snap: Snapshot | null, t: TileRef): string => `Forest fire near ${nearestLandmark(snap, t) ?? `(${t.x},${t.y})`}`;

export const sameFire = (a: TileRef, b: TileRef) => sameTile(a, b);
