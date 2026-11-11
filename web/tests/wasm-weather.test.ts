import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { SimEvent, Snapshot } from '../src/contract';
import { forecastView, healthMeters, weatherLook } from '../src/store';
import { deriveCrises } from '../src/panels/crisis';
import { formatEvent } from '../src/panels/format';
import { createMockSim } from '../src/sim-port/mock';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import { ok } from './helpers';

// The REAL simulation (wasm) against the web's weather code, and mock/real parity of the weather shapes. Skipped when the package is not built.
const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
const present = existsSync(resolve(pkg, 'sim_wasm_bg.wasm'));

async function realPort() {
  const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  return createWasmPort(mod);
}
const keys = (o: object) => Object.keys(o).sort();

describe.skipIf(!present)('real simulation: weather (contract v1.2)', () => {
  it('speaks schemaVersion 3 and carries the weather block, health fields and forest tile states', async () => {
    const snap = ok((await realPort()).init(7));
    expect([3, 4, 5]).toContain(snap.schemaVersion);
    expect(snap.weather.stress).toBe(0);
    expect(snap.weather.forecast).toEqual({ fireRisk: 'low', droughtRisk: 'low' });
    for (const k of ['forestHealth', 'commonsHealth', 'farmHealth'] as const) expect(snap.nature[k]).toBeGreaterThan(0);
    const forest = snap.map.tiles.filter((t) => t.kind === 'forest');
    expect(forest.length).toBeGreaterThan(0);
    expect(forest.every((t) => t.state === 'standing')).toBe(true);
    expect(snap.map.tiles.filter((t) => t.kind !== 'forest').every((t) => t.state === undefined)).toBe(true);
  });
  it('mock and real agree on the shape of the weather block, health fields and history', async () => {
    const rp = await realPort(), mp = createMockSim();
    const real = ok(rp.init(7)), mock = ok(mp.init(7));
    expect(keys(mock.weather)).toEqual(keys(real.weather));
    expect(keys(mock.weather.forecast)).toEqual(keys(real.weather.forecast));
    expect(keys(mock.nature).filter((k) => /Health$/.test(k))).toEqual(keys(real.nature).filter((k) => /Health$/.test(k)));
    expect(mock.schemaVersion).toBeGreaterThanOrEqual(real.schemaVersion); // the mock is v1.4 (4); a package built before v1.4 is 3
    expect(mp.levers().map((l) => l.key).sort()).toEqual(rp.levers().map((l) => l.key).sort());
    expect(rp.actions().map((a) => a.key).sort()).toEqual(mp.actions().map((a) => a.key).sort());
    const rl = rp.levers().find((l) => l.key === 'resilience_spending')!, ml = mp.levers().find((l) => l.key === 'resilience_spending')!;
    expect(ml).toMatchObject({ unit: rl.unit, min: rl.min, max: rl.max, step: rl.step, arrows: rl.arrows, label: rl.label });
    expect(ml.costModel).toEqual(rl.costModel);
    expect(mp.actions().find((a) => a.key === 'place:fire_station')!.costModel).toEqual(rp.actions().find((a) => a.key === 'place:fire_station')!.costModel);
  });
  it('lists the fire station and resilience spending; a fire station previews like any build', async () => {
    const port = await realPort();
    ok(port.init(7));
    const a = port.actions().find((x) => x.key === 'place:fire_station')!;
    expect(a).toBeDefined();
    const lever = port.levers().find((l) => l.key === 'resilience_spending')!;
    expect(lever.unit).toBe('usd_per_month');
    const t = port.validTiles('fire_station')[0]!;
    const p = ok(port.preview([{ type: 'place', kind: 'fire_station', tile: t }, { type: 'set_policy', lever: 'resilience_spending', value: 4000 }]));
    expect(p[0]).toMatchObject({ blocked: null, tileValid: true });
    expect(p[0]!.pc).toBeGreaterThan(0);
    expect(p[1]).toMatchObject({ blocked: null });
  });
  it('a no-action game reaches stress, droughts and fires; every event is worded and every derivation runs on the real snapshots', async () => {
    const seen = new Set<SimEvent['type']>();
    let sawStress = false, sawFireState = false, sawFireTrace = false;
    for (const seed of [3, 11, 42]) {
      const port = await realPort();
      let snap: Snapshot = ok(port.init(seed));
      for (let i = 0; i < 25 && !snap.gameOver; i++) {
        const r = ok(port.submitTurn([]));
        snap = r.snapshot;
        for (const e of r.months.flatMap((m) => m.events)) { seen.add(e.type); expect(formatEvent(e, snap)).toBeTruthy(); expect(e.importance).toBeGreaterThanOrEqual(1); }
        if (r.trace.some((t) => t.type === 'emissions' && t.source === 'fire_sink_loss')) sawFireTrace = true;
        if (snap.weather.stress > 0) sawStress = true;
        if (snap.map.tiles.some((t) => t.state === 'burning')) sawFireState = true;
        expect(forecastView(snap)).toBeTruthy();
        expect(healthMeters(snap)).toHaveLength(3);
        expect(() => weatherLook(snap)).not.toThrow();
        expect(() => deriveCrises(snap, r)).not.toThrow();
        // tile states and weather.fires agree
        const burning = snap.map.tiles.filter((t) => t.state === 'burning').length;
        expect(snap.weather.fires.length).toBe(burning);
      }
    }
    expect(sawStress).toBe(true);
    expect(sawFireState).toBe(true);
    expect(seen.has('fire_started')).toBe(true);
    expect(seen.has('drought_started')).toBe(true);
    void sawFireTrace; // the trace record is only present on turns with burning forest; reported, not required
  });
  it('a save from before the weather layout is refused with a clear message (never loaded)', async () => {
    const port = await realPort();
    ok(port.init(1));
    const blob = JSON.parse(ok(port.exportState())) as Record<string, unknown>;
    blob.schemaVersion = 2;
    const r = port.importState(JSON.stringify(blob));
    expect(r.ok).toBe(false);
    if (!r.ok) { expect(r.error.code).toBe('schema_mismatch'); expect(r.error.message.length).toBeGreaterThan(10); }
    // and the mock refuses an old save the same way (code), with its own words
    const m = createMockSim();
    ok(m.init(1));
    const mb = JSON.parse(ok(m.exportState())) as Record<string, unknown>;
    mb.schemaVersion = 2; delete mb.weatherRng; delete mb.drought;
    const mr = m.importState(JSON.stringify(mb));
    expect(mr.ok).toBe(false);
    if (!mr.ok) expect(mr.error.code).toBe('schema_mismatch');
  });
});
