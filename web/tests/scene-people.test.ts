import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { layoutOf } from '../src/store';
import { ok } from './helpers';
import { campTiles, HOUSEHOLD_COLORS, MARKER_SHAPES, planCommutes, planStands, poseAt } from '../src/scene/people';
import { planEntities } from '../src/scene/entities';
import { centreOf } from '../src/scene/coords';

const sim = createSimPort();
const snap = ok(sim.init(7));
const layout = layoutOf(snap);
const stands = planStands(layout, snap);
const result = ok(sim.submitTurn([]));
const rl = layoutOf(result.snapshot);

describe('people stands', () => {
  it('draws one figure per person', () => {
    expect(snap.people.length).toBe(22);
    expect(stands.map((s) => s.id).sort((a, b) => a - b)).toEqual(snap.people.map((p) => p.id).sort((a, b) => a - b));
  });
  it('household cue: every household has its own index; colours and marker shapes are distinct up to the 8 the palette has', () => {
    const idx = new Set(stands.map((s) => s.hIndex));
    expect(idx.size).toBe(snap.households.length);
    // Town v7 has 9 households: the ninth (the estate) wraps to the first colour and shape until the scene gets a ninth.
    expect(new Set([...idx].map((i) => MARKER_SHAPES[i % MARKER_SHAPES.length])).size).toBe(Math.min(idx.size, MARKER_SHAPES.length));
    expect(new Set([...idx].map((i) => HOUSEHOLD_COLORS[i % HOUSEHOLD_COLORS.length])).size).toBe(Math.min(idx.size, HOUSEHOLD_COLORS.length));
  });
  it('idle people stand within a tile of their home', () => {
    for (const s of stands.filter((q) => !q.unhoused)) {
      const [cx, cz] = centreOf(s.tile, 12);
      expect(Math.hypot(s.at[0] - cx, s.at[1] - cz)).toBeLessThan(0.75);
    }
  });
  it('the unhoused household camps on a free tile near the town hall', () => {
    const camps = campTiles(layout, snap);
    const un = snap.households.filter((h) => h.home === null);
    expect(camps.length).toBe(un.length);
    for (const c of camps) {
      expect(snap.map.tiles.find((t) => t.x === c.tile.x && t.y === c.tile.y)!.kind).toBe('empty');
      expect(stands.filter((s) => s.household === c.household).every((s) => s.unhoused)).toBe(true);
    }
  });
});

describe('commutes', () => {
  const frame = result.months.find((m) => (m.flows.commutes ?? []).length > 0);
  it('walk home -> work -> home with a window inside the month, longer commutes taking longer', () => {
    if (!frame) return;
    const st = planStands(rl, result.snapshot);
    const plans = planCommutes(frame, rl, st, result.snapshot.turn);
    expect(plans.length).toBeGreaterThan(0);
    for (const p of plans) {
      expect(p.t0).toBeGreaterThanOrEqual(0);
      expect(p.t1).toBeLessThanOrEqual(1);
      const stand = st.find((s) => s.id === p.person)!;
      expect(poseAt(stand, p, p.t0).x).toBeCloseTo(stand.at[0], 5);
      expect(poseAt(stand, p, p.t1).x).toBeCloseTo(stand.at[0], 5);
      const mid = poseAt(stand, p, (p.t0 + p.t1) / 2);
      expect(Number.isFinite(mid.x) && Number.isFinite(mid.z)).toBe(true);
      expect(poseAt(stand, p, 0.999 > p.t1 ? 0.999 : 0).x).toBeCloseTo(stand.at[0], 5);
    }
    const sorted = [...plans].sort((a, b) => a.minutes - b.minutes);
    expect(sorted.at(-1)!.t1 - sorted.at(-1)!.t0).toBeGreaterThanOrEqual(sorted[0]!.t1 - sorted[0]!.t0 - 1e-9);
  });
  it('plans nothing for an empty frame', () => {
    expect(planCommutes({ month: 0, flows: {} }, rl, stands, 1)).toEqual([]);
  });
});

describe('tile to mesh mapping', () => {
  it('maps every tile kind to its own model', () => {
    const d = planEntities(layout, snap);
    const model = (kind: string) => d.filter((x) => x.ref.kind === kind).map((x) => x.model);
    expect(model('fossil_plant').every((m) => m === 'fossil_plant')).toBe(true);
    expect(model('bank')).toEqual(['bank']);
    expect(model('government')).toEqual(['government']);
    expect(model('forest').every((m) => m.startsWith('forest_'))).toBe(true);
    expect(model('commons').every((m) => m.startsWith('commons_'))).toBe(true);
    const shops = d.filter((x) => x.firm?.sector === 'services' && x.firm.active);
    expect(shops.every((x) => x.model === 'firm_shop')).toBe(true);
    expect(d.filter((x) => x.firm?.sector === 'goods' && x.firm.active).every((x) => x.model === 'firm_goods')).toBe(true);
  });
  it('plants carry their own emissions for the smoke and the lens', () => {
    const d = planEntities(layout, snap).filter((x) => x.ref.kind === 'fossil_plant' || x.ref.kind === 'renewable_plant');
    expect(d.every((x) => x.plant !== undefined)).toBe(true);
  });
});
