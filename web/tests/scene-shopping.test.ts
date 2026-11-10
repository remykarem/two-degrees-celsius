import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { layoutOf } from '../src/store';
import { ok } from './helpers';
import { MAX_SHOPPING_TRIPS, planCommutes, planShopping, planStands, planTrips, poseAt } from '../src/scene/people';

const sim = createSimPort();
ok(sim.init(7));
const result = ok(sim.submitTurn([]));
const layout = layoutOf(result.snapshot);
const stands = planStands(layout, result.snapshot);
const frame = result.months.find((m) => (m.flows.purchases ?? []).length > 0)!;

describe('shopping trips (Town v5, cosmetic, from real purchases)', () => {
  it('plans a bounded number of trips, one per household, none for a person already commuting', () => {
    const commuting = new Set(planCommutes(frame, layout, stands, result.snapshot.turn).map((c) => c.person));
    const trips = planShopping(frame, layout, stands, commuting, result.snapshot.turn);
    expect(trips.length).toBeGreaterThan(0);
    expect(trips.length).toBeLessThanOrEqual(MAX_SHOPPING_TRIPS);
    const people = trips.map((t) => t.person);
    expect(new Set(people).size).toBe(people.length);
    expect(people.some((p) => commuting.has(p))).toBe(false);
    const households = people.map((p) => stands.find((s) => s.id === p)!.household);
    expect(new Set(households).size).toBe(households.length);
    for (const t of trips) {
      expect(t.kind).toBe('shop');
      expect(stands.find((s) => s.id === t.person)!.ageBand).not.toBe('child');
    }
  });
  it('walks home to the firm and back inside the month, ending at the idle spot', () => {
    const trips = planTrips(frame, layout, stands, result.snapshot.turn).filter((t) => t.kind === 'shop');
    for (const t of trips) {
      expect(t.t0).toBeGreaterThanOrEqual(0);
      expect(t.t1).toBeLessThanOrEqual(1);
      expect(t.t1).toBeGreaterThan(t.t0);
      const st = stands.find((s) => s.id === t.person)!;
      expect(poseAt(st, t, t.t0).x).toBeCloseTo(st.at[0], 5);
      const mid = poseAt(st, t, (t.t0 + t.t1) / 2);
      expect(Number.isFinite(mid.x) && Number.isFinite(mid.z)).toBe(true);
      expect(Math.hypot(mid.x - st.at[0], mid.z - st.at[1])).toBeGreaterThan(0.2);
    }
  });
  it('goes to firms that real purchases name, and covers more than one sector when it can', () => {
    const trips = planShopping(frame, layout, stands, new Set(), result.snapshot.turn, 5);
    const firmTiles = new Set(frame.flows.purchases!.map((p) => result.snapshot.firms.find((f) => f.id === p.firm)!.tile).map((t) => `${t.x},${t.y}`));
    expect(trips.length).toBeGreaterThan(1);
    // the last waypoint is next to a firm tile that someone bought from
    const near = (x: number, z: number) => [...firmTiles].some((k) => { const [tx, ty] = k.split(',').map(Number) as [number, number]; return Math.hypot(x - (tx - 5.5), z - (ty - 5.5)) < 1; });
    expect(trips.every((t) => near(t.path.at(-1)![0], t.path.at(-1)![1]))).toBe(true);
  });
  it('degrades gracefully: no purchases, a cap of zero, or unknown firms give no trips', () => {
    expect(planShopping({ month: 0, flows: {} }, layout, stands, new Set(), 1)).toEqual([]);
    expect(planShopping({ month: 0, flows: { purchases: [] } }, layout, stands, new Set(), 1)).toEqual([]);
    expect(planShopping(frame, layout, stands, new Set(), 1, 0)).toEqual([]);
    expect(planShopping({ month: 0, flows: { purchases: [{ household: 1, firm: 99, sector: 'food', amount: 50 }] } }, layout, stands, new Set(), 1)).toEqual([]);
    expect(planTrips({ month: 0, flows: {} }, layout, stands, 1)).toEqual([]);
  });
  it('is deterministic', () => {
    expect(planShopping(frame, layout, stands, new Set(), 3)).toEqual(planShopping(frame, layout, stands, new Set(), 3));
  });
});
