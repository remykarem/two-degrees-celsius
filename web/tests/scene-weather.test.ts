import { describe, expect, it } from 'vitest';
import type { Snapshot } from '../src/contract';
import { entityTints, legendFor, tileTints, type LensCtx } from '../src/scene/lenses';
import { planEntities } from '../src/scene/entities';
import { MODELS } from '../src/scene/models';
import { pathTiles } from '../src/scene/paths';
import { GROUND } from '../src/scene/palette';
import { planFire, planHaze, WEATHER_CAPS } from '../src/scene/weather';
import { BURNT_GROUND, COVER_GROUND, DRY_GROUND, droughtLook, EMBER_GROUND, FIRE_RAMP, fireLensGround, forestModelFor, rampFire, weatherGround } from '../src/scene/weatherLook';
import { layoutOf, LENSES, parseSave, tileFireRisk, weatherLook, isCovered, nearFire } from '../src/store';
import { createMockSim } from '../src/sim-port/mock';
import { ok } from './helpers';

const base = (): Snapshot => ok(createMockSim().init(2));
const forests = (s: Snapshot) => s.map.tiles.filter((t) => t.kind === 'forest');
/** A copy of the mock snapshot with chosen forest tile states, a drought and fire stations. */
function scene(opts: { burning?: number[]; regrowing?: number[]; drought?: number; stations?: { x: number; y: number }[]; fireRisk?: 'low' | 'medium' | 'high'; stress?: number } = {}): Snapshot {
  const s = structuredClone(base());
  const f = forests(s);
  for (const i of opts.burning ?? []) f[i]!.state = 'burning';
  for (const i of opts.regrowing ?? []) f[i]!.state = 'regrowing';
  for (const st of opts.stations ?? []) s.map.tiles[st.y * 12 + st.x]!.kind = 'fire_station';
  s.weather = {
    stress: opts.stress ?? 0.5, drought: opts.drought ? { severity: opts.drought, monthsLeft: 3 } : null,
    fires: (opts.burning ?? []).map((i) => ({ tile: { x: f[i]!.x, y: f[i]!.y }, monthsLeft: 2 })), forecast: { fireRisk: opts.fireRisk ?? 'medium', droughtRisk: 'low' },
  };
  return s;
}
const ctxOf = (snapshot: Snapshot): LensCtx => { const layout = layoutOf(snapshot); return { descs: planEntities(layout, snapshot), snapshot, layout, roads: pathTiles(layout), asphalt: new Set() }; };

describe('scene: tile state to mesh', () => {
  it('maps forest states to models, standing keeps the three varied trees', () => {
    expect(forestModelFor('burning', 1, 1)).toBe('forest_burning');
    expect(forestModelFor('regrowing', 1, 1)).toBe('forest_regrowing');
    expect(forestModelFor('standing', 2, 5)).toMatch(/^forest_[012]$/);
    expect(forestModelFor(null, 0, 0)).toMatch(/^forest_[012]$/);
    for (const id of ['forest_burning', 'forest_regrowing', 'fire_station'] as const) expect(MODELS[id].parts.length).toBeGreaterThan(3);
  });
  it('entities take the model of their tile state and say so in their facts', () => {
    const snap = scene({ burning: [0], regrowing: [1] });
    const f = forests(snap);
    const descs = planEntities(layoutOf(snap), snap);
    const at = (t: { x: number; y: number }) => descs.find((d) => d.ref.tile.x === t.x && d.ref.tile.y === t.y)!;
    expect(at(f[0]!)).toMatchObject({ model: 'forest_burning', forestState: 'burning' });
    expect(at(f[0]!).ref.label).toBe('Burning forest');
    expect(at(f[1]!)).toMatchObject({ model: 'forest_regrowing', forestState: 'regrowing' });
    expect(at(f[2]!).model).toMatch(/^forest_[012]$/);
  });
  it('a fire station is an entity with its own model', () => {
    const snap = scene({ stations: [{ x: 0, y: 0 }] });
    const d = planEntities(layoutOf(snap), snap).find((x) => x.ref.kind === 'fire_station')!;
    expect(d.model).toBe('fire_station');
    expect(d.ref.label).toBe('Fire station');
  });
  it('a package with no tile state or weather draws every forest tile as standing', () => {
    const old = structuredClone(base()) as unknown as { weather?: unknown; map: { tiles: { state?: string }[] } };
    delete old.weather; old.map.tiles.forEach((t) => delete t.state);
    const descs = planEntities(layoutOf(old as unknown as Snapshot), old as unknown as Snapshot);
    expect(descs.filter((d) => d.ref.kind === 'forest').every((d) => /^forest_[012]$/.test(d.model))).toBe(true);
    expect(weatherLook(old as unknown as Snapshot).fires).toEqual([]);
  });
});

describe('scene: derived look', () => {
  it('collects fires, regrowing tiles, stations and drought from the snapshot (memoised per snapshot)', () => {
    const snap = scene({ burning: [0, 2], regrowing: [1], drought: 0.6, stations: [{ x: 0, y: 0 }] });
    const l = weatherLook(snap);
    expect(l.fires).toHaveLength(2);
    expect(l.regrowing).toHaveLength(1);
    expect(l.stations).toEqual([{ x: 0, y: 0 }]);
    expect(l.drought).toBe(0.6);
    expect(weatherLook(snap)).toBe(l);
    expect(weatherLook(null).fires).toEqual([]);
  });
  it('a burning tile missing from weather.fires still burns', () => {
    const snap = scene({ burning: [0] });
    snap.weather.fires = [];
    expect(weatherLook(snap).fires).toHaveLength(1);
  });
  it('fire risk: higher with a worse forecast, halved under a station, 1 when burning, low when regrowing', () => {
    const t = (s: Snapshot) => forests(s)[3]!;
    const lo = scene({ fireRisk: 'low' }), hi = scene({ fireRisk: 'high' });
    expect(tileFireRisk(weatherLook(hi), t(hi))).toBeGreaterThan(tileFireRisk(weatherLook(lo), t(lo)));
    const f = forests(hi)[3]!;
    const covered = scene({ fireRisk: 'high', stations: [{ x: f.x, y: f.y === 0 ? 1 : f.y - 1 }] });
    const ct = forests(covered)[3]!;
    expect(isCovered(weatherLook(covered), ct)).toBe(true);
    expect(tileFireRisk(weatherLook(covered), ct)).toBeCloseTo(tileFireRisk(weatherLook(hi), f) * 0.5, 6);
    const b = scene({ burning: [3] });
    expect(tileFireRisk(weatherLook(b), forests(b)[3]!)).toBe(1);
    const r = scene({ regrowing: [3] });
    expect(tileFireRisk(weatherLook(r), forests(r)[3]!)).toBeLessThan(0.1);
    const home = hi.map.tiles.find((x) => x.kind === 'home')!;
    expect(tileFireRisk(weatherLook(hi), home)).toBe(0); // not forest
  });
});

describe('scene: ground, drought and lens colours', () => {
  it('no drought and no fire leaves the ground exactly as before', () => {
    const calm = weatherLook(scene({ stress: 0 }));
    expect(weatherGround(calm, 0, 0, GROUND[0]!)).toBe(GROUND[0]);
    expect(droughtLook(0)).toMatchObject({ ground: 0, fog: null, sun: 0xfff0d8, sunIntensity: 1.9 });
  });
  it('drought look scales smoothly with severity: more tint, nearer fog, warmer and dimmer sun', () => {
    const a = droughtLook(0.2), b = droughtLook(0.9);
    expect(b.ground).toBeGreaterThan(a.ground);
    expect(b.fog!.near).toBeLessThan(a.fog!.near);
    expect(b.sunIntensity).toBeLessThan(a.sunIntensity);
    expect(droughtLook(5).ground).toBe(droughtLook(1).ground);
    expect(droughtLook(Number.NaN).fog).toBeNull();
    const dry = weatherLook(scene({ drought: 0.9 }));
    expect(weatherGround(dry, 0, 0, GROUND[0]!)).not.toBe(GROUND[0]);
    expect(weatherGround(dry, 0, 0, DRY_GROUND)).toBe(DRY_GROUND);
  });
  it('burning ground goes to embers, regrowing ground to burnt brown, the neighbours scorch a little', () => {
    const snap = scene({ burning: [0], regrowing: [1] });
    const l = weatherLook(snap), f = forests(snap);
    const dark = (c: number) => ((c >> 16) & 255) + ((c >> 8) & 255) + (c & 255);
    const burn = weatherGround(l, f[0]!.x, f[0]!.y, GROUND[0]!);
    expect(dark(burn)).toBeLessThan(dark(GROUND[0]!));
    expect(Math.abs(burn - EMBER_GROUND)).toBeLessThan(Math.abs(GROUND[0]! - EMBER_GROUND));
    expect(weatherGround(l, f[1]!.x, f[1]!.y, GROUND[0]!)).not.toBe(GROUND[0]);
    expect(BURNT_GROUND).not.toBe(EMBER_GROUND);
  });
  it('tileTints applies the weather under every lens, and the fire lens paints forest by risk and stations by cover', () => {
    const snap = scene({ burning: [0], drought: 0.7, fireRisk: 'high', stations: [{ x: 3, y: 3 }] });
    const ctx = ctxOf(snap);
    const plain = tileTints('none', ctx), emissions = tileTints('emissions', ctx), fire = tileTints('fire', ctx);
    const f0 = forests(snap)[0]!;
    expect(plain[f0.y * 12 + f0.x]).not.toBe(GROUND[(f0.x + f0.y) % 2]);
    expect(emissions).toHaveLength(144);
    const near = fire[3 * 12 + 4];
    expect(near).toBe(fireLensGround(weatherLook(snap), 4, 3, ctx.roads.has('4,3') ? 0xd6c99f : GROUND[(4 + 3) % 2]!));
    expect(isCovered(weatherLook(snap), { x: 4, y: 3 })).toBe(true);
    expect(COVER_GROUND).toBeDefined();
  });
  it('the fire lens colours standing forest on a low-to-high ramp and keeps burning and regrowing models as they are', () => {
    const snap = scene({ burning: [0], regrowing: [1], fireRisk: 'high' });
    const ctx = ctxOf(snap);
    const tints = entityTints('fire', ctx);
    const key = (t: { x: number; y: number }) => ctx.descs.find((d) => d.ref.tile.x === t.x && d.ref.tile.y === t.y)!.key;
    const f = forests(snap);
    expect(tints.get(key(f[2]!))).toBe(rampFire(tileFireRisk(weatherLook(snap), f[2]!)));
    expect(tints.get(key(f[0]!))).toBe(ctx.descs.find((d) => d.key === key(f[0]!))!.tint);
    expect(rampFire(0)).toBe(FIRE_RAMP[0]);
    expect(rampFire(1)).toBe(FIRE_RAMP[FIRE_RAMP.length - 1]);
  });
  it('the lens exists in the lens list, in saves and has a legend with a gradient and a plain-language meaning', () => {
    expect(LENSES.map((l) => l.id)).toContain('fire');
    const lg = legendFor('fire', ctxOf(scene({ fireRisk: 'high' })));
    expect(lg).toMatchObject({ lens: 'fire', label: 'Fire and drought risk', scale: 'gradient' });
    expect(lg.stops).toHaveLength(FIRE_RAMP.length);
    expect(lg.meaning).toMatch(/fire risk high/);
    const save = JSON.stringify({ format: 'ecotown-save', saveVersion: 2, schemaVersion: 2, simState: '{}', lastResult: null, previous: null, uiPrefs: { lens: 'fire', camera: 0 }, savedAt: '' });
    expect(parseSave(save).ok).toBe(true);
  });
});

describe('scene: flames, smoke, haze (capped, deterministic)', () => {
  it('plans flames and smoke columns for every burning tile, within the caps', () => {
    const snap = scene({ burning: [0, 1, 2] });
    const p = planFire(weatherLook(snap), 12);
    expect(p.flames.length).toBe(9);
    expect(p.puffs.length).toBe(21);
    expect(planFire(weatherLook(snap), 12)).toEqual(p);
    const all = scene({ burning: forests(base()).map((_, i) => i) });
    const q = planFire(weatherLook(all), 12);
    expect(q.flames.length).toBeLessThanOrEqual(WEATHER_CAPS.flames);
    expect(q.puffs.length).toBeLessThanOrEqual(WEATHER_CAPS.smoke);
    expect(planFire(weatherLook(scene()), 12)).toEqual({ flames: [], puffs: [] });
  });
  it('haze appears over people near a fire only, capped', () => {
    const snap = scene({ burning: [0] });
    const f = forests(snap)[0]!;
    const stands = [{ id: 1, tile: { x: f.x + 1, y: f.y }, at: [0, 0] as const }, { id: 2, tile: { x: f.x + 9, y: f.y + 9 }, at: [1, 1] as const }];
    expect(nearFire(weatherLook(snap), stands[0]!.tile)).toBe(true);
    expect(planHaze(weatherLook(snap), stands).map((h) => h.id)).toEqual([1]);
    expect(planHaze(weatherLook(scene()), stands)).toEqual([]);
    const many = Array.from({ length: 100 }, (_, i) => ({ id: i, tile: { x: f.x, y: f.y }, at: [0, 0] as const }));
    expect(planHaze(weatherLook(snap), many).length).toBe(WEATHER_CAPS.haze);
  });
});
