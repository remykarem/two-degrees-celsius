import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { layoutOf } from '../src/store';
import { aviationTonnes, estateRoles, estateTilesOf, hasWork, isEstateTile, jetFlightIn, staffCommutesOf } from '../src/store/scene-estate';
import { frameToWorld, placeInFrame, placeOnTile, runwayFrame } from '../src/scene/estateLayout';
import { FLIGHT, jetPoseAt, jetRestPose, jetTrail, LANE_V, PARK_U } from '../src/scene/jetPath';
import { MODELS } from '../src/scene/models';
import { planEntities } from '../src/scene/entities';
import { HOUSEHOLD_COLORS, MARKER_SHAPES, planCommutes, planStands } from '../src/scene/people';
import { employmentState, entityTints, extremes, personTints, tileTints, emitterWeights } from '../src/scene/lenses';
import { pathTiles } from '../src/scene/paths';
import { ok } from './helpers';

const sim = createSimPort();
const snap = ok(sim.init(7));
const layout = layoutOf(snap);
const tiles = estateTilesOf(snap)!;
const ctx = (s = snap, estate: typeof tiles | null = tiles) => {
  const l = layoutOf(s);
  const descs = planEntities(l, s);
  return { descs, snapshot: s, layout: l, roads: pathTiles(l), asphalt: new Set(l.roads.map((t) => `${t.x},${t.y}`)), estate };
};

describe('estate selectors', () => {
  it('reads the estate from the snapshot', () => {
    expect(tiles.runway.length).toBeGreaterThanOrEqual(2);
    expect(tiles.grounds.length).toBe(3);
    expect(tiles.house).toEqual(snap.homes.find((h) => h.id === snap.estate!.home)!.tile);
    expect(estateTilesOf({ ...snap, estate: undefined })).toBeNull();
    expect(estateTilesOf(null)).toBeNull();
  });
  it('gives every grounds tile one distinct role, the drive being the one nearest a road', () => {
    const roles = Object.values(tiles.roles);
    expect(roles.length).toBe(3);
    expect(new Set(roles.map((t) => `${t!.x},${t!.y}`)).size).toBe(3);
    const roads = snap.map.tiles.filter((t) => t.kind === 'road');
    const d = (t: { x: number; y: number }) => Math.min(...roads.map((r) => Math.abs(r.x - t.x) + Math.abs(r.y - t.y)));
    for (const g of tiles.grounds) expect(d(tiles.roles.drive!)).toBeLessThanOrEqual(d(g));
  });
  it('roles are deterministic and tolerate fewer tiles or no roads', () => {
    expect(estateRoles([], null, [], { x: 0, y: 0 })).toEqual({});
    const two = estateRoles([{ x: 1, y: 1 }, { x: 0, y: 1 }], { x: 0, y: 0 }, [], { x: 3, y: 3 });
    expect(Object.keys(two).sort()).toEqual(['drive', 'pool']);
    expect(estateRoles([{ x: 1, y: 1 }, { x: 0, y: 1 }], { x: 0, y: 0 }, [], { x: 3, y: 3 })).toEqual(two);
  });
  it('knows its own tiles', () => {
    expect(isEstateTile(tiles, tiles.runway[0]!)).toBe(true);
    expect(isEstateTile(tiles, tiles.grounds[0]!)).toBe(true);
    expect(isEstateTile(tiles, { x: 6, y: 6 })).toBe(false);
    expect(isEstateTile(null, { x: 0, y: 0 })).toBe(false);
  });
  it('reads flights, aviation tonnes and staff commutes from frames', () => {
    expect(jetFlightIn(undefined)).toBeNull();
    expect(jetFlightIn([{ type: 'private_jet_flight', household: 8, tonnes: 2, importance: 2 }])).toEqual({ tonnes: 2 });
    expect(aviationTonnes({ emissions: [{ source: 'aviation', tonnes: 2 }, { source: 'firm', tonnes: 5 }] })).toBe(2);
    expect(aviationTonnes(undefined)).toBe(0);
    expect(staffCommutesOf({ flows: {} })).toEqual([]);
    const sc = { person: 1, household: 6, employer: 8, from: null, to: { x: 2, y: 11 }, tiles: 6, minutes: 24, wage: 1800 };
    expect(staffCommutesOf({ flows: { staffCommutes: [sc] } }).length).toBe(1);
    // the real wire currently spells it staff_commutes (docs/web-notes-v7.md)
    expect(staffCommutesOf({ flows: { staff_commutes: [sc] } as never }).length).toBe(1);
  });
  it('staff count as working', () => {
    const pilot = snap.people.find((p) => p.employer);
    expect(pilot).toBeDefined();
    expect(hasWork(pilot!)).toBe(true);
    expect(employmentState(pilot!)).toBe('employed');
  });
});

describe('runway frame', () => {
  const f = runwayFrame(tiles, 12);
  it('runs from the hangar tile along the runway', () => {
    expect(f.len).toBeCloseTo(tiles.runway.length - 1);
    expect(Math.hypot(...f.axis)).toBeCloseTo(1);
    const [x, z] = frameToWorld(f, f.len, 0);
    const far = tiles.runway[tiles.runway.length - 1]!;
    expect(x).toBeCloseTo(far.x - 5.5);
    expect(z).toBeCloseTo(far.y - 5.5);
  });
  it('rotates parts into place and keeps tile parts where they are put', () => {
    const p = placeInFrame([{ s: 'box', p: [1, 0, 0.5], z: [1, 1, 1], c: 0 }], { origin: [0, 0], axis: [0, 1], yaw: -Math.PI / 2, len: 1 });
    expect(p[0]!.p[0]).toBeCloseTo(-0.5);
    expect(p[0]!.p[2]).toBeCloseTo(1);
    expect(placeOnTile([{ s: 'box', p: [0.1, 0, 0.2], z: [1, 1, 1], c: 0 }], [3, 4])[0]!.p).toEqual([3.1, 0, 4.2]);
  });
});

describe('jet flight', () => {
  const L = 2;
  it('is parked before and after a flight that returns, and away after one that does not', () => {
    expect(jetPoseAt(0, L, true).phase).toBe('parked');
    expect(jetPoseAt(1, L, true).phase).toBe('parked');
    expect(jetPoseAt(1, L, true).u).toBeCloseTo(PARK_U);
    expect(jetPoseAt(1, L, false).visible).toBe(false);
    expect(jetPoseAt((FLIGHT.depart[1] + FLIGHT.back[0]) / 2, L, true).visible).toBe(false);
  });
  it('takes off along the runway and climbs away', () => {
    let prevU = -1, maxY = 0;
    for (let t = FLIGHT.depart[0]; t < FLIGHT.depart[1]; t += 0.005) {
      const p = jetPoseAt(t, L, true);
      if (!p.visible) break;
      expect(p.u).toBeGreaterThanOrEqual(prevU - 1e-9);
      prevU = p.u; maxY = Math.max(maxY, p.y);
    }
    expect(maxY).toBeGreaterThan(1);
    expect(jetPoseAt(FLIGHT.depart[0] + 0.1, L, true).y).toBe(0);
  });
  it('lands from the far end, rolls back toward the hangar and parks, without leaving the ground', () => {
    const touch = jetPoseAt(FLIGHT.back[0] + (FLIGHT.back[1] - FLIGHT.back[0]) * 0.6, L, true);
    expect(touch.y).toBe(0);
    expect(touch.v).toBeCloseTo(LANE_V);
    expect(touch.u).toBeLessThan(L + 0.3);
    expect(jetPoseAt(FLIGHT.back[1] - 1e-6, L, true).u).toBeCloseTo(PARK_U, 1);
  });
  it('is a pure function of time (a paused clock freezes it)', () => {
    expect(jetPoseAt(0.2, L, true)).toEqual(jetPoseAt(0.2, L, true));
    expect(jetTrail(0.3, L, true)).toEqual(jetTrail(0.3, L, true));
  });
  it('leaves a trail while it moves under power and none while parked or away', () => {
    expect(jetTrail(0.02, L, true)).toEqual([]);
    expect(jetTrail(0.3, L, true).length).toBeGreaterThan(3);
    expect(jetTrail(0.5, L, true).length).toBeGreaterThan(0); // the vapour lingers after the jet has gone
    expect(jetTrail(0.59, L, true).length).toBe(0);
    expect(jetTrail(0.3, L, true).some((p) => p.exhaust)).toBe(true);
    expect(jetTrail(0.38, L, true).some((p) => !p.exhaust)).toBe(true);
  });
  it('reduced motion: stands on the runway for a moment with a puff, no flight path', () => {
    const mid = (FLIGHT.depart[0] + FLIGHT.back[0]) / 2;
    for (let t = 0; t <= 1; t += 0.01) expect(jetPoseAt(t, L, true, true).y).toBe(0);
    expect(jetPoseAt(mid, L, true, true).phase).toBe('static');
    expect(jetTrail(FLIGHT.depart[0] + 0.05, L, true, true).length).toBe(1);
    expect(jetPoseAt(1, L, true, true).phase).toBe('parked');
    expect(jetPoseAt(1, L, false, true).visible).toBe(false);
  });
  it('rest pose follows flyingThisMonth', () => {
    expect(jetRestPose(false).visible).toBe(true);
    expect(jetRestPose(true).visible).toBe(false);
  });
});

describe('estate in the scene data', () => {
  it('the estate home gets the estate model and cream walls; other homes keep theirs', () => {
    const descs = planEntities(layout, snap);
    const est = descs.find((d) => d.household?.id === snap.estate!.household)!;
    expect(est.model).toBe('home_estate');
    expect(MODELS.home_estate.height).toBeGreaterThan(Math.max(...(['capitalist', 'rentier', 'manager', 'worker', 'dependent'] as const).map((c) => MODELS[`home_${c}`].height)) * 0.9);
    expect(descs.filter((d) => d.model === 'home_estate').length).toBe(1);
  });
  it('the ninth household has its own colour and marker shape', () => {
    expect(snap.households.length).toBe(9);
    const idx = planStands(layout, snap).map((s) => s.hIndex);
    expect(new Set(idx.map((i) => HOUSEHOLD_COLORS[i % HOUSEHOLD_COLORS.length])).size).toBe(9);
    expect(new Set(idx.map((i) => MARKER_SHAPES[i % MARKER_SHAPES.length])).size).toBe(9);
  });
  it('the pilot walks to the hangar along the staff commute', () => {
    const stands = planStands(layout, snap);
    const pilot = snap.people.find((p) => p.employer)!;
    const st = stands.find((s) => s.id === pilot.id)!;
    const frame = { month: 0 as const, flows: { staffCommutes: [{ person: pilot.id, household: pilot.household, employer: snap.estate!.household, from: null, to: tiles.hangar, tiles: 6, minutes: 4, wage: 1800 }] } };
    const plan = planCommutes(frame, layout, stands, 1).find((c) => c.person === pilot.id)!;
    expect(plan).toBeDefined();
    expect(plan.path.length).toBeGreaterThan(2);
    expect(plan.path[0]).toEqual(st.at);
  });
  it('lenses: the pilot is employed, the estate home shows the footprint, the jet heats the runway', () => {
    const c = ctx();
    const stands = planStands(layout, snap);
    const pilot = snap.people.find((p) => p.employer)!;
    expect(personTints('employment', snap, stands).get(pilot.id)).toBe(personTints('employment', snap, stands).get(snap.people.find((p) => p.job)!.id));
    const est = c.descs.find((d) => d.model === 'home_estate')!;
    expect(entityTints('wealth', c).get(est.key)).toBeDefined();
    expect(entityTints('class', c).get(est.key)).toBeDefined();
    const withJet = { ...snap, breakdowns: { ...snap.breakdowns!, emissions: { ...snap.breakdowns!.emissions, month: { ...snap.breakdowns!.emissions.month, aviation: 2 } } } };
    const before = emitterWeights(ctx(snap)).length;
    expect(emitterWeights({ ...ctx(withJet) }).length).toBe(before + 1);
    expect(emitterWeights({ ...ctx(withJet), estate: null }).length).toBe(before);
    for (const lens of ['none', 'class', 'wealth', 'emissions', 'employment', 'commute', 'flows', 'fire'] as const) {
      expect(tileTints(lens, c).length).toBe(144);
      expect(entityTints(lens, c).size).toBe(c.descs.length);
    }
    expect(Number.isFinite(extremes(snap).maxCommute)).toBe(true);
  });
});
