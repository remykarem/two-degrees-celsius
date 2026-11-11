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

type Sim = InstanceType<WasmModuleLike['WasmSim']>;
/** The ledger calls of the plan (`ledgerCalls`, `personLedgerCalls`) hashed as the native test does: `{ name: fnv(reply) }`. Every page must reconcile. */
function ledgerHashes(sim: Sim, plan: { ledgerCalls?: Record<string, { account: unknown; query?: unknown }>; personLedgerCalls?: number[] }): Record<string, string> {
  const h: Record<string, string> = {};
  for (const [name, call] of Object.entries(plan.ledgerCalls ?? {})) {
    const reply = sim.ledger!(JSON.stringify(call.account), call.query == null ? '' : JSON.stringify(call.query));
    const v = JSON.parse(reply);
    expect(v.ok, `${name}: ${reply.slice(0, 200)}`).toBe(true);
    expect(v.value.reconciles, `${name} does not reconcile`).toBe(true);
    h[name] = fnv(reply);
  }
  for (const p of plan.personLedgerCalls ?? []) {
    const reply = sim.person_ledger!(p);
    expect(reply).toContain('"ok":true');
    h[`person${p}`] = fnv(reply);
  }
  return h;
}

describe.skipIf(!present)('wasm build against the native golden run', () => {
  it('produces the same output as the native build, per turn', async () => {
    const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
    mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
    const plan = JSON.parse(readFileSync(resolve(golden, 'plan.json'), 'utf8'));
    const want = JSON.parse(readFileSync(resolve(golden, 'seed7.json'), 'utf8'));
    const sim = new mod.WasmSim();
    const got: Record<string, unknown> = { init: fnv(sim.init(plan.seed, null)), levers: fnv(sim.levers()), actions: fnv(sim.actions()), assumptions: fnv(sim.assumptions!()), agents: fnv(sim.agents!()), catalog: fnv(sim.catalog!()) };
    const turns: unknown[] = [];
    const ledgers: Record<string, Record<string, string>> = {};
    let ms = 0;
    plan.turns.forEach((cmds: unknown, i: number) => {
      const turn = i + 1;
      const id = plan.cancelBeforeTurn[String(turn)];
      if (id !== undefined) got[`cancel${turn}`] = fnv(sim.cancel_pending(id));
      const t0 = performance.now();
      const reply = sim.submit_turn(JSON.stringify(cmds));
      ms += performance.now() - t0;
      if ((plan.ledgerAfterTurns as number[] | undefined)?.includes(turn)) ledgers[String(turn)] = ledgerHashes(sim, plan);
      const v = JSON.parse(reply);
      const snap = v.value?.snapshot; // undefined after game over (the 600 t budget runs out near turn 23)
      const bd = snap?.breakdowns;
      const results: Record<string, unknown> = {};
      if (bd) { for (const k of ['gdp', 'gpi', 'wageShare', 'profitShare', 'debtToGdp', 'gini', 'legitimacy', 'capitalRatio']) results[k] = bd[k].result.value; results.emissionsCumulative = bd.emissions.cumulative.tonnes; }
      // `breakdownsFnv` hashes the native side's re-serialised JSON (serde prints 5.0 where JSON.parse sees 5), which JS cannot reproduce: it is carried over from the golden after checking it is there.
      const nativeTurn = want.turns[i];
      turns.push({ turn, ok: v.ok, fnv: fnv(reply), gdpRealAnnual: snap?.indicators.gdpRealAnnual ?? null, legitimacy: snap?.government.legitimacy ?? null, temperature: snap?.indicators.temperature ?? null,
        ...(bd ? { breakdownsFnv: nativeTurn?.breakdownsFnv, breakdownResults: results, emissionsReconcile: bd.emissions.reconciles } : {}) });
      if (bd) expect(typeof nativeTurn?.breakdownsFnv, `turn ${turn} has a native breakdowns hash`).toBe('string');
      if (turn === plan.exportAfterTurn) {
        const blob = sim.export_state();
        got.export = fnv(blob);
        const before = ledgerHashes(sim, plan);
        got.import = fnv(sim.import_state(JSON.parse(blob).value));
        expect(ledgerHashes(sim, plan), 'the ledger is unchanged by import').toEqual(before);
      }
    });
    got.turns = turns;
    got.ledger = ledgers;
    got.finalSnapshot = fnv(sim.snapshot());
    console.info(`wasm: ${(ms / plan.turns.length).toFixed(1)} ms per submit_turn (mean of ${plan.turns.length})`);
    // Bit-identical iff every hash matches. The headline floats are compared first so a failure says which turn drifted.
    const headline = (t: { turn: number; gdpRealAnnual: number | null; legitimacy: number | null; temperature: number | null }) => [t.turn, t.gdpRealAnnual, t.legitimacy, t.temperature];
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
    expect(port.levers()).toHaveLength(20);
    expect(port.actions()).toHaveLength(11);
    expect(port.validTiles('road').length).toBeGreaterThan(50);
    expect(port.init(-5).ok).toBe(false);
  });
});
