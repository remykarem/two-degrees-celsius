/* eslint-disable @typescript-eslint/no-explicit-any -- the mock state blob is edited as untyped JSON on purpose */
import { describe, expect, it } from 'vitest';
import type { Command, SimEvent, Snapshot } from '../src/contract';
import { createMockSim } from '../src/sim-port/mock';
import { GEN, LEVERS, STRESS, WX } from '../src/sim-port/mock/data';
import { initialState } from '../src/sim-port/mock/state';
import { budgetShareOf, covered, forecastOf, resilienceEffect, stepWeather, stressOf } from '../src/sim-port/mock/weather';
import { ok } from './helpers';

// MOCK weather (src/sim-port/mock/weather.ts): invented dynamics, labelled so. These tests pin the documented stress curve and the shape of the behaviour
// (determinism, lifecycles, directions of effects), not any number the real simulation will have.

/** A fresh mock state with the carbon budget `share` left (stress follows from the documented curve). */
function stateAt(seed: number, share: number) {
  const s = initialState(seed);
  s.cumulativeTonnes = GEN.carbon.budget * (1 - share);
  return s;
}
const months = (s: ReturnType<typeof initialState>, n: number) => {
  const events: SimEvent[] = [];
  for (let i = 0; i < n; i++) events.push(...stepWeather(s).events);
  return events;
};
const tiles = (s: ReturnType<typeof initialState>, state: string) => s.grid.filter((c) => c.kind === 'forest' && (c.state ?? 'standing') === state).length;

describe('mock weather: the stress curve (documented, not invented)', () => {
  it('is 0 at and above onset, 1 at and below full, and rises smoothly in between with the documented exponent', () => {
    const p = { onset: 0.5, full: 0.1, curve: 1.5 };
    expect(stressOf(1, p)).toBe(0);
    expect(stressOf(0.5, p)).toBe(0);
    expect(stressOf(0.1, p)).toBe(1);
    expect(stressOf(0, p)).toBe(1);
    expect(stressOf(0.3, p)).toBeCloseTo(0.5 ** 1.5, 12);
    let prev = 0;
    for (let r = 0.5; r >= 0.1; r -= 0.01) { const v = stressOf(r, p); expect(v).toBeGreaterThanOrEqual(prev); prev = v; }
  });
  it('uses the real content values when content/params.json has a weather block', () => {
    expect(STRESS.onset).toBe(GEN.weather?.onset ?? 0.5);
    expect(STRESS.full).toBe(GEN.weather?.full ?? 0.1);
    expect(STRESS.curve).toBe(GEN.weather?.curve ?? 1.5);
  });
  it('the snapshot reports the stress for the budget left', () => {
    const sim = createMockSim();
    const snap = ok(sim.init(3));
    expect(snap.weather.stress).toBe(stressOf(snap.indicators.budgetShare));
    expect(snap.weather.stress).toBe(0);
    expect(snap.weather.drought).toBeNull();
    expect(snap.weather.fires).toEqual([]);
    expect(snap.weather.forecast).toEqual({ fireRisk: 'low', droughtRisk: 'low' });
  });
});

describe('mock weather: no hazards before onset, determinism', () => {
  it('never starts a fire or a drought at stress 0, over many months and seeds', () => {
    for (let seed = 1; seed <= 12; seed++) {
      const s = stateAt(seed, 0.8);
      expect(months(s, 120)).toEqual([]);
      expect(tiles(s, 'standing')).toBe(GEN.map.forest.length);
    }
  });
  it('is deterministic per seed and differs between seeds', () => {
    const run = (seed: number) => { const s = stateAt(seed, 0.15); return JSON.stringify(months(s, 60)); };
    expect(run(5)).toBe(run(5));
    const all = new Set([1, 2, 3, 4, 5, 6].map(run));
    expect(all.size).toBeGreaterThan(1);
  });
  it('uses its own random stream: an economy that never reaches onset is identical with or without weather steps', () => {
    const a = createMockSim(), b = createMockSim();
    ok(a.init(9)); ok(b.init(9));
    const ra = ok(a.submitTurn([])), rb = ok(b.submitTurn([]));
    expect(ra.snapshot.indicators.gdpAnnual).toBe(rb.snapshot.indicators.gdpAnnual);
  });
});

describe('mock weather: drought and fire lifecycles', () => {
  const collect = (share: number, n: number, seeds: number[]) => seeds.flatMap((seed) => { const s = stateAt(seed, share); return months(s, n).map((e) => e.type); });
  it('at high stress droughts start and end, and fires start, burn out and leave regrowing tiles', () => {
    const types = new Set(collect(0.12, 80, [1, 2, 3, 4, 5, 6, 7, 8]));
    for (const t of ['drought_started', 'drought_ended', 'fire_started', 'fire_out', 'forest_burned']) expect(types.has(t as SimEvent['type']), t).toBe(true);
  });
  it('a drought counts its months down to the end event; severity is 0..1', () => {
    for (let seed = 1; seed <= 20; seed++) {
      const s = stateAt(seed, 0.12);
      for (let i = 0; i < 80; i++) {
        const before = s.drought ? { ...s.drought } : null;
        const ev = stepWeather(s).events;
        const started = ev.find((e) => e.type === 'drought_started');
        if (started && started.type === 'drought_started') { expect(started.severity).toBeGreaterThan(0); expect(started.severity).toBeLessThanOrEqual(1); expect(started.months).toBeGreaterThanOrEqual(1); }
        if (before && s.drought) expect(s.drought.monthsLeft).toBe(before.monthsLeft - 1);
        if (before && !s.drought) expect(ev.some((e) => e.type === 'drought_ended')).toBe(true);
      }
    }
  });
  it('a burning tile ends as regrowing and later stands again; a tile is in exactly one state; forest stays in 0..1', () => {
    let sawRegrow = false, sawBack = false;
    for (let seed = 1; seed <= 30 && !sawBack; seed++) {
      const s = stateAt(seed, 0.1);
      const seen = new Map<string, string>();
      for (let i = 0; i < 90; i++) {
        stepWeather(s);
        expect(s.forest).toBeGreaterThanOrEqual(0);
        expect(s.forest).toBeLessThanOrEqual(1);
        s.grid.forEach((c, k) => {
          if (c.kind !== 'forest') { expect(c.state).toBeUndefined(); return; }
          const st = c.state ?? 'standing';
          expect(['standing', 'burning', 'regrowing']).toContain(st);
          const prev = seen.get(String(k));
          if (prev === 'burning' && st === 'regrowing') sawRegrow = true;
          if (prev === 'regrowing' && st === 'standing') sawBack = true;
          if (prev === 'standing') expect(st).not.toBe('regrowing'); // standing never skips burning
          seen.set(String(k), st);
        });
      }
    }
    expect(sawRegrow).toBe(true);
    expect(sawBack).toBe(true);
  });
  it('burning forest lowers the sink: net emissions rise as the forest share falls', () => {
    const sim = createMockSim();
    ok(sim.init(4));
    const blob = JSON.parse(ok(sim.exportState()));
    const before = ok(sim.importState(JSON.stringify(blob)));
    blob.forest = blob.forest * 0.5;
    const after = ok(sim.importState(JSON.stringify(blob)));
    expect(after.nature.sinkMonthly).toBeLessThan(before.nature.sinkMonthly);
    expect(after.nature.netEmissionsMonthly).toBeGreaterThan(before.nature.netEmissionsMonthly);
  });
});

describe('mock weather: adaptation (fire station, resilience spending)', () => {
  const burned = (seed: number, build: (s: ReturnType<typeof initialState>) => void) => {
    const s = stateAt(seed, 0.12);
    build(s);
    let total = 0;
    for (let i = 0; i < 80; i++) { stepWeather(s); total += tiles(s, 'burning'); }
    return total;
  };
  const seeds = Array.from({ length: 40 }, (_, i) => i + 1);
  /** Stations on empty tiles within reach of most forest tiles. */
  const stationsNearForest = (s: ReturnType<typeof initialState>) => {
    const cells = GEN.map.forest;
    const cx = Math.round(cells.reduce((a, c) => a + c.x, 0) / cells.length), cy = Math.round(cells.reduce((a, c) => a + c.y, 0) / cells.length);
    const empty = s.grid.map((c, k) => ({ c, x: k % 12, y: Math.floor(k / 12) })).filter((q) => q.c.kind === 'empty').sort((a, b) => Math.hypot(a.x - cx, a.y - cy) - Math.hypot(b.x - cx, b.y - cy)).slice(0, 3);
    for (const q of empty) q.c.kind = 'fire_station';
  };
  it('fire stations reduce the area burnt (MOCK: coverage shortens and slows fires)', () => {
    const without = seeds.reduce((a, sd) => a + burned(sd, () => undefined), 0);
    const withStation = seeds.reduce((a, sd) => a + burned(sd, stationsNearForest), 0);
    expect(withStation).toBeLessThan(without);
  });
  it('a station covers tiles within the radius only', () => {
    const s = initialState(1);
    const e = s.grid.findIndex((c) => c.kind === 'empty');
    s.grid[e]!.kind = 'fire_station';
    const x = e % 12, y = Math.floor(e / 12);
    expect(covered(s, x, y)).toBe(true);
    expect(covered(s, Math.min(11, x + Math.ceil(WX.coverageRadius) + 1), y) || covered(s, Math.max(0, x - Math.ceil(WX.coverageRadius) - 1), y)).toBe(false);
  });
  it('resilience spending lowers fires, with diminishing returns', () => {
    const base = seeds.reduce((a, sd) => a + burned(sd, () => undefined), 0);
    const spent = seeds.reduce((a, sd) => a + burned(sd, (s) => { s.policy.resilience_spending = 20000; }), 0);
    expect(spent).toBeLessThan(base);
    const e = (v: number) => { const s = initialState(1); s.policy.resilience_spending = v; return resilienceEffect(s); };
    expect(e(0)).toBe(0);
    expect(e(4000) - e(0)).toBeGreaterThan(e(20000) - e(16000));
    expect(e(1e9)).toBeLessThanOrEqual(WX.resilienceMaxEffect + 1e-9);
  });
  it('the forecast is a pure function of the state: calling it does not move the random stream', () => {
    const s = stateAt(2, 0.2);
    const rng = s.weatherRng.state;
    const a = forecastOf(s), b = forecastOf(s);
    expect(a).toEqual(b);
    expect(s.weatherRng.state).toBe(rng);
  });
  it('forecast rises with stress and is high during a drought', () => {
    const lvl = { low: 0, medium: 1, high: 2 };
    const f = (share: number) => forecastOf(stateAt(1, share));
    expect(f(0.8)).toEqual({ fireRisk: 'low', droughtRisk: 'low' });
    expect(lvl[f(0.1).droughtRisk]).toBeGreaterThanOrEqual(lvl[f(0.3).droughtRisk]);
    expect(lvl[f(0.1).fireRisk]).toBeGreaterThanOrEqual(lvl[f(0.3).fireRisk]);
    const s = stateAt(1, 0.3);
    s.drought = { severity: 0.5, monthsLeft: 2 };
    expect(forecastOf(s).droughtRisk).toBe('high');
  });
});

describe('mock weather through the port', () => {
  const place = (kind: 'fire_station', tile: { x: number; y: number }): Command => ({ type: 'place', kind, tile });
  it('lists the fire station as a build and resilience spending as a lever, with the same lead-time and PC rules as before', () => {
    const sim = createMockSim();
    ok(sim.init(1));
    const a = sim.actions().find((x) => x.key === 'place:fire_station')!;
    expect(a).toBeDefined();
    const lever = sim.levers().find((l) => l.key === 'resilience_spending')!;
    expect(lever.unit).toBe('usd_per_month');
    expect(LEVERS.find((l) => l.key === 'resilience_spending')!.default).toBe(0);
    const t = sim.validTiles('fire_station')[0]!;
    const p = ok(sim.preview([place('fire_station', t), { type: 'set_policy', lever: 'resilience_spending', value: 5000 }]));
    expect(p[0]).toMatchObject({ blocked: null, tileValid: true, leadTime: a.leadTime });
    expect(p[0]!.pc).toBeGreaterThan(0);
    expect(p[1]).toMatchObject({ blocked: null, leadTime: lever.leadTime });
    expect(p[1]!.pc).toBeGreaterThan(0);
    // not on forest, and not above the lever's range
    const forest = GEN.map.forest[0]!;
    expect(ok(sim.preview([place('fire_station', forest)]))[0]!.blocked).toBe('tile_invalid');
    expect(ok(sim.preview([{ type: 'set_policy', lever: 'resilience_spending', value: lever.max + 1 }]))[0]!.blocked).toBe('invalid_command');
  });
  it('a built fire station shows as a tile kind, and removing it works', () => {
    const sim = createMockSim();
    ok(sim.init(1));
    const t = sim.validTiles('fire_station')[0]!;
    const a = sim.actions().find((x) => x.key === 'place:fire_station')!;
    let snap: Snapshot = ok(sim.init(1));
    ok(sim.submitTurn([place('fire_station', t)]));
    for (let i = 0; i < a.leadTime + 1; i++) snap = ok(sim.submitTurn([])).snapshot;
    expect(snap.map.tiles.find((x) => x.x === t.x && x.y === t.y)!.kind).toBe('fire_station');
    expect(ok(sim.preview([{ type: 'remove', tile: t }]))[0]!.blocked).toBeNull();
  });
  it('forest tiles carry a state, the nature block carries health, history carries the new fields, the trace names the sink lost to fire', () => {
    const sim = createMockSim();
    const s0 = ok(sim.init(2));
    for (const t of s0.map.tiles) expect(t.kind === 'forest' ? t.state : undefined).toBe(t.kind === 'forest' ? 'standing' : undefined);
    expect(s0.nature.forestHealth).toBe(s0.nature.forest);
    expect(s0.nature.farmHealth).toBeCloseTo((s0.nature.forest + s0.nature.commons) / 2, 10);
    // a burnt tile and sink lost to fire, by editing the state at the start of a turn
    const blob = JSON.parse(ok(sim.exportState()));
    blob.cumulativeTonnes = GEN.carbon.budget * 0.9;
    const f = blob.grid.findIndex((c: any) => c.kind === 'forest');
    blob.grid[f].state = 'burning'; blob.grid[f].left = 5;
    ok(sim.importState(JSON.stringify(blob)));
    const r = ok(sim.submitTurn([]));
    const h = r.snapshot.history.at(-1)!;
    for (const k of ['climateStress', 'forestHealth', 'commonsHealth', 'farmHealth'] as const) expect(Number.isFinite(h[k])).toBe(true);
    expect(r.trace.some((t) => t.type === 'emissions' && t.source === 'fire_sink_loss' && t.tonnes > 0)).toBe(true);
    expect(r.snapshot.weather.fires.length).toBeGreaterThan(0);
    expect(r.snapshot.map.tiles[f]!.state).toBe('burning');
    expect(r.months.flatMap((m) => m.events).every((e) => e.importance >= 1 && e.importance <= 3)).toBe(true);
  });
  it('saves round-trip the weather, and a save from before weather is refused with a clear message', () => {
    const sim = createMockSim();
    ok(sim.init(6));
    const blob = JSON.parse(ok(sim.exportState()));
    expect(blob.weatherRng).toBeDefined();
    expect(sim.importState(JSON.stringify(blob)).ok).toBe(true);
    delete blob.weatherRng; delete blob.drought;
    const r = sim.importState(JSON.stringify(blob));
    expect(r.ok).toBe(false);
    if (!r.ok) { expect(r.error.code).toBe('schema_mismatch'); expect(r.error.message).toMatch(/weather/); }
  });
  it('the weather states are reported the same in live (Pressure) turns as in submitTurn', () => {
    const a = createMockSim(), b = createMockSim();
    ok(a.init(8)); ok(b.init(8));
    const mid = (sim: ReturnType<typeof createMockSim>) => { const x = JSON.parse(ok(sim.exportState())); x.cumulativeTonnes = GEN.carbon.budget * 0.88; return ok(sim.importState(JSON.stringify(x))); };
    mid(a); mid(b);
    const ra = ok(a.submitTurn([]));
    ok(b.beginTurn([])); for (let i = 0; i < 3; i++) ok(b.advanceMonth());
    const rb = ok(b.finishTurn());
    expect(rb.snapshot.weather).toEqual(ra.snapshot.weather);
    expect(rb.months.map((m) => m.events)).toEqual(ra.months.map((m) => m.events));
    expect(budgetShareOf(JSON.parse(ok(b.exportState())))).toBeLessThan(0.9);
  });
});
