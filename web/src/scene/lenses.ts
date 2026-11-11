import type { ClassKey, HouseholdView, Snapshot } from '../contract';
import { incomeTotal } from '../store/sources';
import { fossilShareOf, text, type Lens, type MapLayout } from '../store';
import type { LensLegend } from '../store/scene-bus';
import { centreOf, tileKey } from './coords';
import type { EntityDesc } from './entities';
import type { PersonStand } from './people';
import { tileFireRisk, weatherLook } from '../store/scene-weather';
import { fireLensGround, rampFire, weatherGround, FIRE_RAMP } from './weatherLook';
import { aviationOfSnapshot, hasWork, type EstateTiles } from '../store/scene-estate';
import { CLASS_COLOR, CLASS_LABEL, GROUND, hex, mix, PATH } from './palette';

/** Map lenses as pure colour functions: snapshot in, hex colours out. No Three.js. */
export interface LensCtx { descs: EntityDesc[]; snapshot: Snapshot | null; layout: MapLayout; roads: Set<string>; /** Built roads (tile keys), drawn as road meshes, not by the ground tiles. */ asphalt: Set<string>;
  /** Town v7: the estate's tiles (optional: lenses without an estate behave as before). */ estate?: EstateTiles | null }

export const RAMP = {
  emissions: [0xfff3c4, 0xf6a54a, 0xd55e00, 0x8a3a22],
  wealth: [0xe6f1fa, 0x8ec5ea, 0x2f7fc1, 0x15385e],
  employment: [0xd55e00, 0xf4c590, 0xcfe3ef, 0x2f7fc1],
  flows: [0xd55e00, 0xeee9dd, 0x0072b2],
  commute: [0xe5f0e6, 0x9bd3ae, 0x3f9d8f, 0x16405b],
} as const;
export const EMPLOYED = 0x2f7fc1, UNEMPLOYED = 0xd55e00, NOT_WORKING = 0xcfcfcf;
const NEUTRAL = 0xe9e5db;
const clamp01 = (v: number) => Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0));

/** Piecewise-linear colour ramp, t in 0..1. */
export function ramp(stops: readonly number[], t: number): number {
  const c = clamp01(t) * (stops.length - 1);
  const i = Math.min(stops.length - 2, Math.floor(c));
  return mix(stops[i]!, stops[i + 1]!, c - i);
}

const dim = (base: number) => mix(base, NEUTRAL, 0.55);

export interface Extremes { maxEmission: number; maxFirmEmissions: number; maxCommute: number; wealthLo: number; wealthHi: number; maxWorkers: number }
export function extremes(s: Snapshot | null): Extremes {
  const w = s?.households.map((h) => h.netWorth / Math.max(1, h.members.length)) ?? [0];
  const firmE = s?.firms.map((f) => (f.active ? f.emissions : 0)) ?? [0];
  return {
    maxFirmEmissions: Math.max(1e-9, ...firmE),
    maxEmission: Math.max(1e-9, ...firmE, ...(s?.plants.map((p) => p.emissionsMonthly) ?? [0])),
    maxCommute: Math.max(1, ...(s?.people.filter((p) => hasWork(p)).map((p) => p.commuteMinutes) ?? [0])),
    wealthLo: Math.min(...w), wealthHi: Math.max(...w),
    maxWorkers: Math.max(1, ...(s?.firms.map((f) => f.workers) ?? [0])),
  };
}

const logT = (v: number, lo: number, hi: number) => {
  const f = (x: number) => Math.log(1 + Math.max(0, x));
  return hi > lo ? (f(v) - f(lo)) / (f(hi) - f(lo)) : 0.5;
};
export const employmentShare = (h: { employed: number; labourForce: number }) => (h.labourForce > 0 ? h.employed / h.labourForce : 1);
/** Employed over the labour force for a household's members (1 when nobody in it is looking for work: owners, retired, children). */
export function householdEmployment(s: Snapshot | null, h: { id: number; class: string }): number {
  if (!s) return 1;
  const lf = s.people.filter((p) => p.household === h.id && p.inLabourForce);
  return employmentShare({ employed: lf.filter((p) => hasWork(p)).length, labourForce: lf.length });
}
export type EmploymentState = 'employed' | 'unemployed' | 'out';
/** Per person: has a job, is looking for one, or is outside the labour force (child, retired, owner). */
export const employmentState = (p: { job: unknown; employer?: unknown; inLabourForce: boolean }): EmploymentState => (p.job || p.employer ? 'employed' : p.inLabourForce ? 'unemployed' : 'out');
export const netMoney = (h: Pick<HouseholdView, 'income'>) => {
  const i = h.income;
  return incomeTotal(i.sources) - i.taxPaid - i.rentPaid - i.loanPayments - i.spending;
};

/** Emitter weights per tile owner: active firms and plants by their emissions over the heaviest emitter. */
export function emitterWeights(ctx: LensCtx): { x: number; z: number; w: number }[] {
  const ex = extremes(ctx.snapshot);
  const out: { x: number; z: number; w: number }[] = [];
  for (const d of ctx.descs) {
    if (d.state === 'draft') continue;
    if (d.firm?.active) out.push({ x: d.x, z: d.z, w: d.firm.emissions / ex.maxEmission });
    else if (d.ref.kind === 'fossil_plant' && d.state === 'normal') out.push({ x: d.x, z: d.z, w: d.plant ? d.plant.emissionsMonthly / ex.maxEmission : 0.35 + 0.65 * (ctx.snapshot ? fossilShareOf(ctx.snapshot.energy) : 0.5) });
  }
  // Town v7: the jet's tonnes show as a heat weight on the runway (the same scale as firms and plants, with a floor so a flight is visible).
  const av = aviationOfSnapshot(ctx.snapshot);
  const e = ctx.estate;
  if (e && av > 0 && e.runway.length) {
    const mid = e.runway[Math.floor(e.runway.length / 2)]!;
    const [x, z] = centreOf(mid, ctx.layout.gridSize);
    out.push({ x, z, w: Math.max(0.3, Math.min(1, av / ex.maxEmission)) });
  }
  return out;
}

/** Tint (wall colour) for every entity under a lens. Keys are EntityDesc.key. */
export function entityTints(lens: Lens, ctx: LensCtx): Map<string, number> {
  const ex = extremes(ctx.snapshot);
  const out = new Map<string, number>();
  const w = lens === 'emissions' ? emitterWeights(ctx) : [];
  const look = weatherLook(ctx.snapshot);
  for (const d of ctx.descs) {
    let c = d.tint;
    const h = d.household;
    if (lens === 'none') {
      // Forest and commons show their stock: lush green when healthy, dry straw when depleted.
      if (d.stock !== undefined) c = d.ref.kind === 'forest' ? mix(0xc9a85a, 0x6fb061, clamp01((d.stock - 0.15) / 0.7)) : mix(0xc9b27a, 0xb4d891, clamp01(d.stock / 0.6));
    } else if (lens === 'emissions') {
      const estateHome = !!h && ctx.snapshot?.estate?.household === h.id;
      const e = estateHome ? (h?.footprintTonnes ?? 0) / Math.max(1e-9, ...(ctx.snapshot?.households.map((q) => q.footprintTonnes) ?? [0])) : d.firm?.active ? d.firm.emissions / ex.maxEmission : d.ref.kind === 'fossil_plant' ? w.find((q) => q.x === d.x && q.z === d.z)?.w : d.ref.kind === 'renewable_plant' ? 0 : undefined;
      c = e === undefined ? dim(d.tint) : ramp(RAMP.emissions, e);
    } else if (lens === 'class') {
      c = h ? CLASS_COLOR[h.class] : dim(d.tint);
    } else if (lens === 'wealth') {
      c = h ? ramp(RAMP.wealth, logT(h.netWorth / Math.max(1, h.members.length), ex.wealthLo, ex.wealthHi)) : dim(d.tint);
    } else if (lens === 'employment') {
      if (h) c = householdEmployment(ctx.snapshot, h) >= 1 && !ctx.snapshot?.people.some((p) => p.household === h.id && p.inLabourForce) ? dim(d.tint) : ramp(RAMP.employment, (householdEmployment(ctx.snapshot, h) - 0.5) / 0.5);
      else if (d.firm?.active) c = ramp(RAMP.wealth, d.firm.workers / ex.maxWorkers);
      else c = dim(d.tint);
    } else if (lens === 'commute') {
      const m = d.commuteMinutes;
      c = h ? (m && m.length ? ramp(RAMP.commute, m[0]! / ex.maxCommute) : dim(d.tint)) : dim(d.tint);
    } else if (lens === 'fire') {
      if (d.forestState === 'standing') c = rampFire(tileFireRisk(look, d.ref.tile));
      else if (d.ref.kind === 'fire_station') c = 0xb9d3e8;
      else if (d.forestState) c = d.tint;
      else c = dim(d.tint);
    } else if (lens === 'flows') {
      if (h) c = ramp(RAMP.flows, 0.5 + 0.5 * Math.tanh(netMoney(h) / Math.max(1, h.income.spending)));
      else if (d.firm?.active) c = ramp(RAMP.flows, 0.5 + 0.5 * Math.tanh(d.firm.profit / Math.max(1, Math.abs(d.firm.revenue))));
      else c = dim(d.tint);
    }
    out.set(d.key, c);
  }
  return out;
}

/** Ground tile colours (row-major, size*size). Emissions lens paints a heat haze around emitters. */
export function tileTints(lens: Lens, ctx: LensCtx): number[] {
  const n = ctx.layout.gridSize;
  const out: number[] = [];
  const em = lens === 'emissions' ? emitterWeights(ctx) : [];
  const look = weatherLook(ctx.snapshot);
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const plain = ctx.roads.has(tileKey({ x, y })) ? PATH : GROUND[(x + y) % 2]!;
    const base = lens === 'fire' ? fireLensGround(look, x, y, plain) : weatherGround(look, x, y, plain);
    if (lens !== 'emissions') { out.push(base); continue; }
    const [cx, cz] = centreOf({ x, y }, n);
    let heat = 0;
    for (const e of em) heat += e.w * Math.exp(-((cx - e.x) ** 2 + (cz - e.z) ** 2) / (2 * 1.5 * 1.5));
    out.push(heat < 0.04 ? base : mix(base, ramp(RAMP.emissions, 0.15 + heat * 0.75), clamp01(heat * 0.6)));
  }
  return out;
}

/** Body colour for every person under a lens (keys are person ids). `none` returns an empty map: figures keep their household colour. */
export function personTints(lens: Lens, snap: Snapshot | null, stands: readonly PersonStand[]): Map<number, number> {
  const out = new Map<number, number>();
  if (!snap || lens === 'none') return out;
  const ex = extremes(snap);
  for (const st of stands) {
    const p = snap.people.find((x) => x.id === st.id);
    const h = snap.households.find((x) => x.id === st.household);
    if (!p || !h) continue;
    let c = NOT_WORKING;
    switch (lens) {
      case 'class': c = CLASS_COLOR[h.class]; break;
      case 'wealth': c = ramp(RAMP.wealth, logT(h.netWorth / Math.max(1, h.members.length), ex.wealthLo, ex.wealthHi)); break;
      case 'employment': { const e = employmentState(p); c = e === 'employed' ? EMPLOYED : e === 'unemployed' ? UNEMPLOYED : NOT_WORKING; break; }
      case 'commute': c = hasWork(p) ? ramp(RAMP.commute, p.commuteMinutes / ex.maxCommute) : NOT_WORKING; break;
      case 'flows': c = ramp(RAMP.flows, 0.5 + 0.5 * Math.tanh(netMoney(h) / Math.max(1, h.income.spending))); break;
      case 'emissions': c = ramp(RAMP.emissions, Math.min(1, h.footprintTonnes / Math.max(1e-9, ...snap.households.map((q) => q.footprintTonnes)))); break;
    }
    out.set(st.id, c);
  }
  return out;
}

const grad = (stops: readonly number[], labels: string[]) => stops.map((s, i) => ({ color: hex(s), label: labels[i] ?? '' }));
const money = (v: number) => (Math.abs(v) >= 1000 ? `$${(v / 1000).toFixed(v >= 10000 ? 0 : 1)}k` : `$${Math.round(v)}`);

/** Legend description for the panels. */
export function legendFor(lens: Lens, ctx: LensCtx): LensLegend {
  const ex = extremes(ctx.snapshot);
  switch (lens) {
    case 'emissions':
      return { lens, label: 'Emissions heat', scale: 'gradient', stops: grad(RAMP.emissions, ['none', 'low', 'high', 'highest']),
        lowLabel: 'Clean', highLabel: 'Heavy emitter',
        meaning: 'Firms and power plants are coloured by their share of the heaviest emitter; the ground glows around them. Smoke shows the same thing: a fossil plant smokes, a renewable plant does not.',
        notes: ['The heat haze on the ground is cosmetic.', ...(ctx.snapshot ? [`Heaviest firm: ${ex.maxFirmEmissions.toFixed(1)} t/month.`] : [])] };
    case 'class':
      return { lens, label: 'Class', scale: 'categories',
        stops: (Object.keys(CLASS_COLOR) as ClassKey[]).map((k) => ({ color: hex(CLASS_COLOR[k]), label: CLASS_LABEL[k] })),
        meaning: 'Homes and people are coloured by the class of their household. Every class also has its own house shape and rooftop marker, and every household its own marker shape above its people, so colour is not the only cue.',
        notes: ['Colours are the colour-blind-safe Okabe-Ito set.'] };
    case 'wealth':
      return { lens, label: 'Wealth', scale: 'gradient', stops: grad(RAMP.wealth, [money(ex.wealthLo), '', '', money(ex.wealthHi)]),
        lowLabel: `Poorest ${money(ex.wealthLo)} per person`, highLabel: `Richest ${money(ex.wealthHi)} per person`,
        meaning: 'Homes and people are shaded by their household\'s savings per person (log scale, relative to this town).', notes: ['Non-home buildings are greyed out.'] };
    case 'employment':
      return { lens, label: 'Employment', scale: 'gradient', stops: grad(RAMP.employment, ['50% or less employed', '', '', 'everyone employed']),
        lowLabel: 'Jobless', highLabel: 'Fully employed',
        meaning: 'People: blue = has a job, orange = looking for work, grey = not in the labour force (children, retired, owners). Homes: share of their labour force with a job. Firms: how many workers they employ.',
        notes: ['Each figure keeps its household marker shape above its head, so households stay recognisable without colour.'] };
    case 'commute':
      return { lens, label: 'Commute', scale: 'gradient', stops: grad(RAMP.commute, ['short', '', '', `${Math.round(ex.maxCommute)} min`]),
        lowLabel: 'Short commute', highLabel: `Longest ${Math.round(ex.maxCommute)} min`,
        meaning: 'People who work are coloured by their one-way commute in minutes; homes by their longest commuter. Grey = no commute. Roads shorten commutes.',
        notes: ['During playback longer commutes take longer to walk.'] };
    case 'flows':
      return { lens, label: 'Money flows', scale: 'gradient', stops: grad(RAMP.flows, ['losing money', 'break-even', 'making money']),
        lowLabel: 'Net outflow', highLabel: 'Net inflow',
        meaning: 'Buildings are shaded by whether households or firms ended the quarter with more or less money coming in than going out. Gold tokens show wages, rent and purchases moving.',
        notes: ['Orange and blue are used so the lens works for colour-blind players; arrows of tokens show direction.'] };
    case 'fire': {
      const fc = ctx.snapshot ? ctx.snapshot.weather?.forecast : undefined;
      return { lens, label: text('weather', 'lens_title'), scale: 'gradient', stops: grad(FIRE_RAMP, ['low', '', '', 'burning or very high']),
        lowLabel: 'Low risk', highLabel: 'High risk',
        meaning: `${text('weather', 'lens_note')}${fc ? ` Forecast: fire risk ${fc.fireRisk}, drought risk ${fc.droughtRisk}.` : ''} Under a drought the ground turns straw-coloured; blue-grey ground is covered by a fire station.`,
        notes: ['Burning tiles are the darkest red; burnt, regrowing tiles are not at risk.'] };
    }
    default:
      return { lens: 'none', label: 'No lens', scale: 'none', stops: [], meaning: 'The town as it is. Home colours hint at class; use a lens for detail.', notes: [] };
  }
}
