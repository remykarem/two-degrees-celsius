import { describe, expect, it } from 'vitest';
import type { MonthFrame } from '../src/contract';
import { createSimPort } from '../src/sim-port';
import { layoutOf } from '../src/store';
import { ok } from './helpers';
import { allocate, CAPS, commuteFraction, ctxFromSnapshot, emissionsFromSnapshot, planEmitters, planFlowLoop, planFrame, pointAt, smokeCounts, smokeParticle, trackProgress } from '../src/scene/flows';

const sim = createSimPort();
const s0 = ok(sim.init(11));
const result = ok(sim.submitTurn([]));
const layout = layoutOf(result.snapshot);
const ctx = ctxFromSnapshot(layout, result.snapshot);
const frame = result.months[0]!;

describe('allocate', () => {
  it('never exceeds the cap and keeps heavier flows', () => {
    const w = Array.from({ length: 40 }, (_, i) => i + 1);
    const c = allocate(w, 20);
    expect(c.reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(20);
    expect(c[39]).toBeGreaterThan(0);
    expect(c[0]).toBe(0);
  });
  it('gives each positive flow at least one and ignores zero weights', () => {
    expect(allocate([100, 0, 1], 50, 4)).toEqual([4, 0, 1]);
    expect(allocate([], 5)).toEqual([]);
  });
});

describe('frame planner', () => {
  it('turns flows into tracks without touching the input', () => {
    const before = JSON.stringify(frame);
    const plan = planFrame(frame, ctx);
    expect(JSON.stringify(frame)).toBe(before);
    expect(plan.tracks.length).toBeGreaterThan(0);
    for (const k of ['good', 'token'] as const) expect(plan.tracks.some((t) => t.kind === k), k).toBe(true);
  });
  it('is deterministic', () => {
    expect(planFrame(frame, ctx)).toEqual(planFrame(frame, ctx));
  });
  it('respects the caps even for huge flows', () => {
    const big: MonthFrame = {
      month: 1, events: [],
      flows: {
        commutes: Array.from({ length: 50 }, (_, i) => ({ person: i, household: (i % 5) + 1, firm: (i % 2) + 1, from: { x: 3, y: 2 }, to: { x: 9, y: 3 }, tiles: 7, minutes: 30 })),
        purchases: Array.from({ length: 80 }, (_, i) => ({ household: (i % 5) + 1, firm: i % 2, sector: i % 2 ? 'goods' as const : 'food' as const, amount: 100 })),
        rent: Array.from({ length: 40 }, (_, i) => ({ household: (i % 5) + 1, home: i % 5, amount: 50 })),
        wages: Array.from({ length: 40 }, (_, i) => ({ person: i, household: (i % 5) + 1, firm: 1, amount: 99 })),
        emissions: [],
      },
    };
    const p = planFrame(big, ctx);
    expect(p.tracks.filter((t) => t.kind === 'good').length).toBeLessThanOrEqual(CAPS.goods);
    expect(p.tracks.filter((t) => t.kind === 'token').length).toBeLessThanOrEqual(CAPS.tokens);
  });
  it('every track has a path starting and ending on real tiles and a valid window', () => {
    for (const t of planFrame(frame, ctx).tracks) {
      expect(t.path.length).toBeGreaterThanOrEqual(2);
      expect(t.t0).toBeGreaterThanOrEqual(0);
      expect(t.t1).toBeLessThanOrEqual(1);
      expect(t.t1).toBeGreaterThan(t.t0);
    }
  });
  it('skips flows that name homes or firms the map does not have', () => {
    const odd: MonthFrame = { month: 0, events: [], flows: { commutes: [{ person: 99, household: 99, firm: 99, from: null, to: { x: 0, y: 0 }, tiles: 1, minutes: 6 }], purchases: [], rent: [], wages: [], emissions: [] } };
    expect(planFrame(odd, ctx).tracks).toHaveLength(0);
  });
  it('maps events to effects at the right tile', () => {
    const f: MonthFrame = { month: 2, flows: { commutes: [], purchases: [], rent: [], wages: [], emissions: [] },
      events: [{ type: 'firm_closed', importance: 3, firm: 0 }, { type: 'bank_failed', importance: 3 }, { type: 'policy_blocked', importance: 1, reason: 'invalid_command' }] };
    const fx = planFrame(f, ctx).fx;
    expect(fx.map((x) => x.type).sort()).toEqual(['bank', 'closed']);
    expect(fx.find((x) => x.type === 'closed')!.tile).toMatchObject({ x: 9, y: 3 });
  });
  it('flow loop keeps only money tokens', () => {
    const loop = planFlowLoop(result.months, ctx);
    expect(loop.length).toBeGreaterThan(0);
    expect(loop.every((t) => t.kind === 'token')).toBe(true);
    expect(planFlowLoop([], ctx)).toEqual([]);
  });
});

describe('emitters and smoke', () => {
  it('scale with emissions and skip closed firms', () => {
    const em = planEmitters([{ source: 'firm', id: 0, tonnes: 10 }, { source: 'firm', id: 1, tonnes: 5 }], ctx);
    const f1 = em.find((e) => e.id === 'f0')!, f2 = em.find((e) => e.id === 'f1')!;
    expect(f1.rate).toBe(1);
    expect(f2.rate).toBeCloseTo(0.5);
    const closed = planEmitters([{ source: 'firm', id: 0, tonnes: 10 }], { ...ctx, firms: ctx.firms.map((f) => ({ ...f, active: false })) });
    expect(closed.find((e) => e.id === 'f0')).toBeUndefined();
  });
  it('plant smoke follows each plant\'s own emissions; pending plants and zero emitters do not smoke', () => {
    const fossil = layout.placed.filter((p) => p.kind === 'fossil_plant');
    expect(fossil.length).toBeGreaterThanOrEqual(2);
    const e0 = emissionsFromSnapshot(s0).filter((e) => e.source === 'plant');
    expect(e0.length).toBeGreaterThan(0);
    // make the first plant twice as heavy as the second, the rest silent
    const frame: MonthFrame['flows']['emissions'] = fossil.map((p, i) => ({ source: 'plant' as const, tile: { x: p.x, y: p.y }, tonnes: i === 0 ? 8 : i === 1 ? 4 : 0 }));
    const em = planEmitters(frame, { ...ctx, plants: [] }).filter((e) => e.id.startsWith('p'));
    expect(em).toHaveLength(2);
    expect(em[0]!.rate).toBeCloseTo(1);
    expect(em[1]!.rate).toBeCloseTo(0.5);
    const none = { ...ctx, layout: { ...layout, placed: [] } };
    expect(planEmitters(frame, none).filter((e) => e.id.startsWith('p'))).toHaveLength(0);
  });
  it('ambient emissions come from the snapshot', () => {
    expect(emissionsFromSnapshot(s0).some((e) => e.source === 'firm')).toBe(true);
  });
  it('particles are reproducible and the total count is capped', () => {
    const e = { id: 'f1', kind: 'smoke' as const, at: [1, 2] as [number, number], top: 1, rate: 0.8 };
    expect(smokeParticle(e, 3, 4.2)).toEqual(smokeParticle(e, 3, 4.2));
    expect(smokeParticle(e, 3, 4.2).y).toBeGreaterThanOrEqual(e.top);
    const many = Array.from({ length: 50 }, (_, i) => ({ ...e, id: `x${i}`, rate: 1 }));
    expect(smokeCounts(many, CAPS.smoke).reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(CAPS.smoke);
  });
});

describe('path and timing helpers', () => {
  it('pointAt walks a polyline', () => {
    const p: [number, number][] = [[0, 0], [2, 0], [2, 2]];
    expect(pointAt(p, 0)).toEqual([0, 0]);
    expect(pointAt(p, 0.5)).toEqual([2, 0]);
    expect(pointAt(p, 1)).toEqual([2, 2]);
    expect(pointAt([[1, 1]], 0.4)).toEqual([1, 1]);
  });
  it('commuters go out, wait, and come back', () => {
    expect(commuteFraction(0)).toBe(0);
    expect(commuteFraction(0.5)).toBe(1);
    expect(commuteFraction(1)).toBe(0);
  });
  it('track progress is null outside its window', () => {
    const t = { id: 'a', kind: 'token' as const, style: 0, path: [[0, 0], [1, 0]] as [number, number][], t0: 0.2, t1: 0.6 };
    expect(trackProgress(t, 0.1)).toBeNull();
    expect(trackProgress(t, 0.4)).toBeCloseTo(0.5);
    expect(trackProgress(t, 0.9)).toBeNull();
  });
});

describe('empty and partial flows (contract v0.3 section 10.8)', () => {
  const empty: MonthFrame = { month: 0, events: [], flows: {} };
  it('plans an empty frame without throwing', () => {
    const plan = planFrame(empty, ctx);
    expect(plan.tracks).toEqual([]);
    expect(plan.emitters.every((e) => e.id.startsWith('p'))).toBe(true); // only the fossil plants smoke
    expect(plan.fx).toEqual([]);
  });
  it('plans a frame with only commutes and emissions', () => {
    const h = ctx.households[0]!.id, f = ctx.firms.find((x) => x.active)!.id;
    const partial: MonthFrame = { month: 1, events: [], flows: { commutes: [{ person: 4, household: h, firm: f, from: { x: 3, y: 2 }, to: { x: 9, y: 3 }, tiles: 7, minutes: 30 }], emissions: [{ source: 'firm', id: f, tonnes: 10 }] } };
    const plan = planFrame(partial, ctx);
    expect(plan.tracks).toEqual([]); // commutes are walked by the people layer, not the track planner
    expect(plan.emitters.length).toBeGreaterThan(0);
  });
  it('tolerates empty arrays, no events array, and a loop made only of empty frames', () => {
    const arrays: MonthFrame = { month: 2, events: [], flows: { commutes: [], purchases: [], rent: [], wages: [], emissions: [] } };
    expect(planFrame(arrays, ctx).tracks).toEqual([]);
    expect(planFrame({ month: 0, flows: {} } as unknown as MonthFrame, ctx).fx).toEqual([]);
    expect(planFlowLoop([empty, empty, empty], ctx)).toEqual([]);
    expect(planEmitters(undefined, ctx).filter((e) => e.kind !== 'smoke')).toEqual([]);
  });
  it('the flow loop uses the last frame that has flows', () => {
    const loop = planFlowLoop([result.months[0]!, result.months[1]!, empty], ctx);
    expect(loop.length).toBeGreaterThan(0);
  });
});
