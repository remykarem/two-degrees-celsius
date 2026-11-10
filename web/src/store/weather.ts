import type { ForestState, RiskLevel, Snapshot, TileRef, Weather } from '../contract';
import { layoutOf, sameTile, type Selection } from './map';

/**
 * Weather as the panels read it (Town v4). Everything here is DERIVED from the snapshot; nothing is stored, so the mock and the real sim
 * go through the same code. A package built before contract v1.2 has no `weather`: every function then reports "no weather".
 */

/**
 * Copies of numbers that live in content/params.json `weather` but are NOT on the wire (contract issue, docs/web-notes-v4.md): used only to
 * draw the stress gauge's marks, the fire-station coverage ring and the smoke haze. The real numbers decide the simulation, these only decide pixels.
 */
export const WEATHER_UI = { onset: 0.5, full: 0.1, curve: 1.5, coverageRadius: 3, smokeRadius: 2 } as const;

/** The documented curve, for the gauge's marks and tests; the snapshot's own `weather.stress` is the authority. */
export function stressAt(budgetShare: number, p: { onset: number; full: number; curve: number } = WEATHER_UI): number {
  return Math.min(1, Math.max(0, (p.onset - budgetShare) / (p.onset - p.full))) ** p.curve;
}

export const weatherOf = (snap: Snapshot | null): Weather | null => {
  const w = (snap as { weather?: Weather } | null)?.weather;
  return w && typeof w.stress === 'number' ? w : null;
};

export type HealthLevel = 'healthy' | 'strained' | 'poor' | 'failing';
export const healthLevel = (v: number): HealthLevel => (v >= 0.6 ? 'healthy' : v >= 0.35 ? 'strained' : v >= 0.15 ? 'poor' : 'failing');
export const HEALTH_WORD: Record<HealthLevel, string> = { healthy: 'Healthy', strained: 'Strained', poor: 'Poor', failing: 'Failing' };

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
    return { id, label, value: v, level, word: HEALTH_WORD[level], hint };
  };
  return [
    mk('forest', 'Forest', forest, 'Share of the forest still standing. Fires burn it and the carbon sink shrinks with it.'),
    mk('commons', 'Commons', commons, 'Health of the shared land. Droughts and over-harvesting wear it down.'),
    mk('farm', 'Farm', farm, 'How well the farm can produce: it follows forest and commons health, and falls in a drought.'),
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
  const word = s <= 0 ? 'Calm' : s < 0.25 ? 'Rising' : s < 0.6 ? 'High' : 'Severe';
  // The wire carries the curve since contract v1.3; an older package falls back to the constants above.
  const onset = typeof w.onset === 'number' ? w.onset : WEATHER_UI.onset;
  const full = typeof w.full === 'number' ? w.full : WEATHER_UI.full;
  const text = s <= 0
    ? `No climate stress yet: erratic weather begins when ${Math.round(onset * 100)}% of the carbon budget is left.`
    : `Weather is more erratic and fires and droughts are likelier. Full stress at ${Math.round(full * 100)}% budget left.`;
  return { stress: s, percent: Math.round(s * 100), remaining: snap.indicators.budgetShare, onset, full, word, text };
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
  else if (fireRisk === 'high') headline = 'Elevated fire risk';
  else if (droughtRisk === 'high') headline = 'Dry spell likely';
  else if (fireRisk === 'medium') headline = 'Fire risk is building';
  else if (droughtRisk === 'medium') headline = 'A dry spell is possible';
  else headline = 'Weather calm';
  const detail = w.stress <= 0 ? 'No climate stress yet, so no fires or droughts are expected.'
    : level === 'low' ? 'Nothing unusual is expected this quarter.'
    : 'A fire station near the forest and resilience spending both lower the risk.';
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
