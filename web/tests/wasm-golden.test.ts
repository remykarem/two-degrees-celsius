import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';

// Replays tests/golden/plan.json in the wasm build and compares with tests/golden/seed7.json, written by the native
// test crates/sim-wasm/tests/golden.rs from the very same JSON strings. Skipped when the package is not built (npm run build:wasm).
const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
const present = existsSync(resolve(pkg, 'sim_wasm_bg.wasm'));
const golden = resolve(__dirname, '..', '..', 'tests', 'golden');

function fnv(s: string): string {
  let h = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(s)) h = ((h ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h.toString(16).padStart(16, '0');
}

describe.skipIf(!present)('wasm build against the native golden run', () => {
  it('produces the same output as the native build, per turn', async () => {
    const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
    mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
    const plan = JSON.parse(readFileSync(resolve(golden, 'plan.json'), 'utf8'));
    const want = JSON.parse(readFileSync(resolve(golden, 'seed7.json'), 'utf8'));
    const sim = new mod.WasmSim();
    const got: Record<string, unknown> = { init: fnv(sim.init(plan.seed, null)), levers: fnv(sim.levers()), actions: fnv(sim.actions()) };
    const turns: unknown[] = [];
    let ms = 0;
    plan.turns.forEach((cmds: unknown, i: number) => {
      const turn = i + 1;
      const id = plan.cancelBeforeTurn[String(turn)];
      if (id !== undefined) got[`cancel${turn}`] = fnv(sim.cancel_pending(id));
      const t0 = performance.now();
      const reply = sim.submit_turn(JSON.stringify(cmds));
      ms += performance.now() - t0;
      const v = JSON.parse(reply);
      const snap = v.value.snapshot;
      turns.push({ turn, ok: v.ok, fnv: fnv(reply), gdpRealAnnual: snap.indicators.gdpRealAnnual, legitimacy: snap.government.legitimacy, temperature: snap.indicators.temperature });
      if (turn === plan.exportAfterTurn) {
        const blob = sim.export_state();
        got.export = fnv(blob);
        got.import = fnv(sim.import_state(JSON.parse(blob).value));
      }
    });
    got.turns = turns;
    got.finalSnapshot = fnv(sim.snapshot());
    console.info(`wasm: ${(ms / plan.turns.length).toFixed(1)} ms per submit_turn (mean of ${plan.turns.length})`);
    // Bit-identical iff every hash matches. The headline floats are compared first so a failure says which turn drifted.
    const headline = (t: { turn: number; gdpRealAnnual: number; legitimacy: number; temperature: number }) => [t.turn, t.gdpRealAnnual, t.legitimacy, t.temperature];
    expect((got.turns as typeof want.turns).map(headline)).toEqual(want.turns.map(headline));
    expect(got).toEqual(want);
  });

  it('the SimPort wrapper returns error results, not exceptions', async () => {
    const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
    mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
    const port = createWasmPort(mod);
    const bad = port.submitTurn([]);
    expect(bad.ok).toBe(false); // not started yet
    const snap = port.init(3);
    expect(snap.ok).toBe(true);
    if (snap.ok) expect(snap.value.nature.forest).toBeLessThanOrEqual(1);
    expect(port.levers()).toHaveLength(19);
    expect(port.actions()).toHaveLength(9);
    expect(port.validTiles('road').length).toBeGreaterThan(50);
    expect(port.init(-5).ok).toBe(false);
  });
});
