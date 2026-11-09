import type { ClassKey, Snapshot } from '../contract';
import type { Lens, MapLayout } from '../store';
import type { LensLegend } from '../store/scene-bus';
import { centreOf, tileKey } from './coords';
import type { EntityDesc } from './entities';
import { CLASS_COLOR, CLASS_LABEL, GROUND, hex, mix, PATH } from './palette';

/** Map lenses as pure colour functions: snapshot in, hex colours out. No Three.js. */
export interface LensCtx { descs: EntityDesc[]; snapshot: Snapshot | null; layout: MapLayout; roads: Set<string> }

export const RAMP = {
  emissions: [0xfff3c4, 0xf6a54a, 0xd55e00, 0x8a3a22],
  wealth: [0xe6f1fa, 0x8ec5ea, 0x2f7fc1, 0x15385e],
  employment: [0xd55e00, 0xf4c590, 0xcfe3ef, 0x2f7fc1],
  flows: [0xd55e00, 0xeee9dd, 0x0072b2],
} as const;
const NEUTRAL = 0xe9e5db;
const clamp01 = (v: number) => Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0));

/** Piecewise-linear colour ramp, t in 0..1. */
export function ramp(stops: readonly number[], t: number): number {
  const c = clamp01(t) * (stops.length - 1);
  const i = Math.min(stops.length - 2, Math.floor(c));
  return mix(stops[i]!, stops[i + 1]!, c - i);
}

const dim = (base: number) => mix(base, NEUTRAL, 0.55);

export interface Extremes { maxFirmEmissions: number; wealthLo: number; wealthHi: number; maxWorkers: number }
export function extremes(s: Snapshot | null): Extremes {
  const w = s?.households.map((h) => h.wealthPerCapita) ?? [0];
  return {
    maxFirmEmissions: Math.max(1e-9, ...(s?.firms.map((f) => (f.active ? f.emissions : 0)) ?? [0])),
    wealthLo: Math.min(...w), wealthHi: Math.max(...w),
    maxWorkers: Math.max(1, ...(s?.firms.map((f) => f.workers) ?? [0])),
  };
}

const logT = (v: number, lo: number, hi: number) => {
  const f = (x: number) => Math.log(1 + Math.max(0, x));
  return hi > lo ? (f(v) - f(lo)) / (f(hi) - f(lo)) : 0.5;
};
export const employmentShare = (h: { employed: number; labourForce: number }) => (h.labourForce > 0 ? h.employed / h.labourForce : 1);
export const netMoney = (h: { income: { wages: number; dividends: number; rent: number; interest: number; benefits: number; taxPaid: number; rentPaid: number; loanPayments: number; spending: number } }) => {
  const i = h.income;
  return i.wages + i.dividends + i.rent + i.interest + i.benefits - i.taxPaid - i.rentPaid - i.loanPayments - i.spending;
};

/** Emitter weights per tile owner: active firms by their emissions, fossil plants by the fossil share of energy. */
export function emitterWeights(ctx: LensCtx): { x: number; z: number; w: number }[] {
  const ex = extremes(ctx.snapshot);
  const out: { x: number; z: number; w: number }[] = [];
  for (const d of ctx.descs) {
    if (d.state === 'draft') continue;
    if (d.firm?.active) out.push({ x: d.x, z: d.z, w: d.firm.emissions / ex.maxFirmEmissions });
    else if (d.ref.kind === 'fossil_plant' && d.state === 'normal') out.push({ x: d.x, z: d.z, w: 0.35 + 0.65 * (ctx.snapshot?.energy.fossilShare ?? 0.5) });
  }
  return out;
}

/** Tint (wall colour) for every entity under a lens. Keys are EntityDesc.key. */
export function entityTints(lens: Lens, ctx: LensCtx): Map<string, number> {
  const ex = extremes(ctx.snapshot);
  const out = new Map<string, number>();
  const w = emitterWeights(ctx);
  for (const d of ctx.descs) {
    let c = d.tint;
    const h = d.household;
    if (lens === 'none') {
      // Forest and commons show their stock: lush green when healthy, dry straw when depleted.
      if (d.stock !== undefined) c = d.ref.kind === 'forest' ? mix(0xc9a85a, 0x6fb061, clamp01((d.stock - 0.15) / 0.7)) : mix(0xc9b27a, 0xb4d891, clamp01(d.stock / 0.6));
    } else if (lens === 'emissions') {
      const e = d.firm?.active ? d.firm.emissions / ex.maxFirmEmissions : d.ref.kind === 'fossil_plant' ? w.find((q) => q.x === d.x && q.z === d.z)?.w : undefined;
      c = e === undefined ? dim(d.tint) : ramp(RAMP.emissions, e);
    } else if (lens === 'class') {
      c = h ? CLASS_COLOR[h.class] : dim(d.tint);
    } else if (lens === 'wealth') {
      c = h ? ramp(RAMP.wealth, logT(h.wealthPerCapita, ex.wealthLo, ex.wealthHi)) : dim(d.tint);
    } else if (lens === 'employment') {
      if (h) c = ramp(RAMP.employment, (employmentShare(h) - 0.5) / 0.5);
      else if (d.firm?.active) c = ramp(RAMP.wealth, d.firm.workers / ex.maxWorkers);
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
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const base = ctx.roads.has(tileKey({ x, y })) ? PATH : GROUND[(x + y) % 2]!;
    if (lens !== 'emissions') { out.push(base); continue; }
    const [cx, cz] = centreOf({ x, y }, n);
    let heat = 0;
    for (const e of em) heat += e.w * Math.exp(-((cx - e.x) ** 2 + (cz - e.z) ** 2) / (2 * 1.5 * 1.5));
    out.push(heat < 0.04 ? base : mix(base, ramp(RAMP.emissions, 0.15 + heat * 0.75), clamp01(heat * 0.6)));
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
        meaning: 'Firms and power plants are coloured by their share of the heaviest emitter; the ground glows around them. Smoke shows the same thing.',
        notes: ['Position on the map has no gameplay effect: the heat haze is cosmetic.', ...(ctx.snapshot ? [`Heaviest firm: ${ex.maxFirmEmissions.toFixed(1)} t/month.`] : [])] };
    case 'class':
      return { lens, label: 'Class', scale: 'categories',
        stops: (Object.keys(CLASS_COLOR) as ClassKey[]).map((k) => ({ color: hex(CLASS_COLOR[k]), label: CLASS_LABEL[k] })),
        meaning: 'Each home is coloured by the class of its households. Every class also has its own house shape and rooftop marker, so colour is not the only cue.',
        notes: ['Colours are the colour-blind-safe Okabe-Ito set.'] };
    case 'wealth':
      return { lens, label: 'Wealth', scale: 'gradient', stops: grad(RAMP.wealth, [money(ex.wealthLo), '', '', money(ex.wealthHi)]),
        lowLabel: `Poorest ${money(ex.wealthLo)} per person`, highLabel: `Richest ${money(ex.wealthHi)} per person`,
        meaning: 'Homes are shaded by savings per person (log scale, relative to this town).', notes: ['Non-home buildings are greyed out.'] };
    case 'employment':
      return { lens, label: 'Employment', scale: 'gradient', stops: grad(RAMP.employment, ['50% or less employed', '', '', 'everyone employed']),
        lowLabel: 'Jobless', highLabel: 'Fully employed',
        meaning: 'Homes: share of the labour force with a job. Firms: how many workers they employ (lighter blue = fewer, deeper blue = more).', notes: [] };
    case 'flows':
      return { lens, label: 'Money flows', scale: 'gradient', stops: grad(RAMP.flows, ['losing money', 'break-even', 'making money']),
        lowLabel: 'Net outflow', highLabel: 'Net inflow',
        meaning: 'Buildings are shaded by whether households or firms ended the quarter with more or less money coming in than going out. Gold tokens show wages, rent and purchases moving.',
        notes: ['Orange and blue are used so the lens works for colour-blind players; arrows of tokens show direction.'] };
    default:
      return { lens: 'none', label: 'No lens', scale: 'none', stops: [], meaning: 'The town as it is. Home colours hint at class; use a lens for detail.', notes: [] };
  }
}
