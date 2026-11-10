import type { RiskLevel, SimEvent, TileRef, Weather } from '../../contract';
import { GEN, STRESS, WX } from './data';
import { nextFloat } from './rng';
import { cellAt, GRID, type MockState } from './state';

// MOCK ONLY. The real weather system is S11 (docs/causal-model.md section 11, Rust). Everything here except the stress formula is invented,
// plausible dynamics so the web UI can be built and tested before (and beside) the real sim. Numbers: mock-only.json `weather`.
const W = WX;
const clamp01 = (x: number) => Math.min(1, Math.max(0, x));

/** The documented stress curve (docs/town-v4.md section 2): s = clamp((onset - r) / (onset - full), 0, 1) ^ curve, r = budget share remaining. */
export function stressOf(budgetShare: number, p: { onset: number; full: number; curve: number } = STRESS): number {
  const lin = Math.min(1, Math.max(0, (p.onset - budgetShare) / (p.onset - p.full)));
  return lin ** p.curve;
}
export const budgetShareOf = (s: Pick<MockState, 'cumulativeTonnes'>): number => Math.max(0, GEN.carbon.budget - s.cumulativeTonnes) / GEN.carbon.budget;
export const stressNow = (s: MockState): number => stressOf(budgetShareOf(s));

/** Diminishing returns on resilience spending: 0 for nothing, approaching resilienceMaxEffect. */
export function resilienceEffect(s: MockState): number {
  const spend = Math.max(0, Number(s.policy.resilience_spending ?? 0));
  return W.resilienceMaxEffect * (1 - Math.exp(-spend / W.resilienceHalfUsd));
}

const stations = (s: MockState): TileRef[] => {
  const out: TileRef[] = [];
  for (let y = 0; y < GRID; y++) for (let x = 0; x < GRID; x++) if (cellAt(s, x, y)!.kind === 'fire_station') out.push({ x, y });
  return out;
};
/** True when a fire station stands within coverageRadius tiles (straight-line distance). */
export const covered = (s: MockState, x: number, y: number): boolean => stations(s).some((t) => Math.hypot(t.x - x, t.y - y) <= W.coverageRadius + 1e-9);

const forestCells = (s: MockState): { x: number; y: number; state: string }[] => {
  const out: { x: number; y: number; state: string }[] = [];
  for (let y = 0; y < GRID; y++) for (let x = 0; x < GRID; x++) { const c = cellAt(s, x, y)!; if (c.kind === 'forest') out.push({ x, y, state: c.state ?? 'standing' }); }
  return out;
};

export interface MockHealth { forest: number; commons: number; farm: number }
export function healthOf(s: MockState): MockHealth {
  const d = s.drought?.severity ?? 0;
  return { forest: s.forest, commons: s.commons, farm: clamp01(((s.forest + s.commons) / 2) * (1 - W.droughtFarmLoss * d)) };
}

const level = (score: number): RiskLevel => (score >= W.forecastHigh ? 'high' : score >= W.forecastMedium ? 'medium' : 'low');

/** Deterministic: from the state only, never from the RNG's next draws (the real contract says the same). */
export function forecastOf(s: MockState): Weather['forecast'] {
  const stress = stressNow(s);
  const cells = forestCells(s);
  const standing = cells.filter((c) => c.state === 'standing');
  const fires = cells.filter((c) => c.state === 'burning');
  const frac = cells.length ? standing.length / cells.length : 0;
  const sev = s.drought?.severity ?? 0;
  const dryness = Math.min(1, (1 + W.fireDroughtDryness * sev + 0.5 * (1 - s.commons)) / 2);
  const cover = standing.length ? standing.filter((c) => covered(s, c.x, c.y)).length / standing.length : 0;
  const fire = stress * dryness * Math.sqrt(frac) * (1 - resilienceEffect(s)) * (1 - 0.3 * cover) + (fires.length ? 0.2 : 0);
  const drought = stress * (1 - 0.3 * resilienceEffect(s));
  return { fireRisk: level(fire), droughtRisk: s.drought ? 'high' : level(drought) };
}

export function weatherView(s: MockState): Weather {
  const fires = forestCells(s).filter((c) => c.state === 'burning').map((c) => ({ tile: { x: c.x, y: c.y }, monthsLeft: cellAt(s, c.x, c.y)!.left ?? 1 }));
  return { stress: stressNow(s), onset: STRESS.onset, full: STRESS.full, curve: STRESS.curve, drought: s.drought ? { ...s.drought } : null, fires, forecast: forecastOf(s) };
}

export interface WeatherMonth { events: SimEvent[] }

/** One month of weather, before the economy step. Uses its own random stream (s.weatherRng). */
export function stepWeather(s: MockState): WeatherMonth {
  const events: SimEvent[] = [];
  const rnd = () => nextFloat(s.weatherRng);
  const stress = stressNow(s);
  const res = resilienceEffect(s);
  const community = s.policy.commons_access === 'community';

  // Drought: an active one counts down and hurts the commons; otherwise a roll that rises with stress (none at stress 0).
  if (s.drought) {
    s.commons = clamp01(s.commons - W.droughtCommonsLoss * s.drought.severity * (1 - (community ? W.communityBuffer : 0)) * (1 - res * 0.5));
    if (--s.drought.monthsLeft <= 0) { s.drought = null; events.push({ type: 'drought_ended', importance: 2 }); }
  } else if (stress > 0 && rnd() < W.droughtBase * (1 + W.droughtStressGain * stress)) {
    const severity = +clamp01(0.25 + 0.7 * stress * rnd() + 0.1 * rnd()).toFixed(2);
    const months = W.droughtMonthsMin + Math.floor(rnd() * (W.droughtMonthsSpan + W.droughtMonthsStress * stress));
    s.drought = { severity, monthsLeft: months };
    events.push({ type: 'drought_started', importance: severity > 0.6 ? 3 : 2, severity, months });
  }
  const sev = s.drought?.severity ?? 0;

  // Fires: burning tiles lose forest, may spread, then end as regrowing tiles; regrowing tiles stand again after regrowMonths.
  const cells = forestCells(s);
  let lost = 0;
  const burning = cells.filter((c) => c.state === 'burning');
  const started = new Set<string>();
  for (const b of burning) {
    const cell = cellAt(s, b.x, b.y)!;
    const dl = Math.min(s.forest, W.forestLossPerBurningTile);
    s.forest -= dl;
    lost += dl;
    for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]] as const) {
      const n = cellAt(s, b.x + dx, b.y + dy);
      if (!n || n.kind !== 'forest' || (n.state ?? 'standing') !== 'standing') continue;
      const p = clamp01(W.spreadBase + W.spreadDrought * sev) * (covered(s, b.x + dx, b.y + dy) ? W.coverageSpread : 1) * (1 - res * 0.3);
      if (rnd() < p) {
        n.state = 'burning';
        n.left = Math.max(1, W.fireMonths - (covered(s, b.x + dx, b.y + dy) ? W.coverageShorten : 0));
        started.add(`${b.x + dx},${b.y + dy}`);
        events.push({ type: 'fire_spread', importance: 2, from: { x: b.x, y: b.y }, to: { x: b.x + dx, y: b.y + dy } });
      }
    }
    cell.left = (cell.left ?? 1) - 1;
    if (cell.left <= 0) {
      cell.state = 'regrowing';
      cell.left = W.regrowMonths;
      events.push({ type: 'fire_out', importance: 2, tile: { x: b.x, y: b.y } }, { type: 'forest_burned', importance: 2, tile: { x: b.x, y: b.y } });
    }
  }
  for (const c of cells) {
    if (c.state !== 'regrowing') continue;
    const cell = cellAt(s, c.x, c.y)!;
    s.forest = clamp01(s.forest + W.regrowGainPerTile);
    if (--cell.left! <= 0) { cell.state = 'standing'; delete cell.left; }
  }
  if (lost > 0) s.fireSinkLossTurn += lost * W.sinkShare * s.lastMonthlyEmissions;

  // Ignition: p = base x (1 + a x stress) x dryness x forest fraction x (1 - resilience); none at stress 0.
  const standing = cells.filter((c) => (cellAt(s, c.x, c.y)!.state ?? 'standing') === 'standing' && !started.has(`${c.x},${c.y}`));
  if (stress > 0 && standing.length) {
    const dryness = 1 + W.fireDroughtDryness * sev + 0.5 * (1 - s.commons);
    const p = W.fireBase * (1 + W.fireStressGain * stress) * dryness * (standing.length / Math.max(1, cells.length)) * (1 - res);
    if (rnd() < p) {
      const t = standing[Math.min(standing.length - 1, Math.floor(rnd() * standing.length))]!;
      const cell = cellAt(s, t.x, t.y)!;
      cell.state = 'burning';
      cell.left = Math.max(1, W.fireMonths - (covered(s, t.x, t.y) ? W.coverageShorten : 0));
      events.push({ type: 'fire_started', importance: 3, tile: { x: t.x, y: t.y } });
    }
  }
  return { events };
}

/** Monthly cost of the weather items (resilience spending and fire station upkeep), in dollars. */
export const weatherSpendMonthly = (s: MockState): number => Math.max(0, Number(s.policy.resilience_spending ?? 0)) + stations(s).length * W.fireStationUpkeepUsd;
