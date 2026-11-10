import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { createWasmPort, type WasmModuleLike, type WasmSimInstance } from '../src/sim-port/wasm';

const fakeModule = (extra: Partial<WasmSimInstance>): WasmModuleLike => ({
  install_panic_hook() {}, last_panic: () => '',
  WasmSim: class { constructor() { Object.assign(this, extra); } } as unknown as WasmModuleLike['WasmSim'],
});

describe('wasm wrapper: live-turn calls', () => {
  it('reports Pressure mode unavailable when the package lacks the exports', () => {
    const port = createWasmPort(fakeModule({}));
    expect(port.liveTurn).toBe(false);
    const r = port.beginTurn([]);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.code).toBe('unsupported');
  });
  it('forwards to begin_turn / advance_month / finish_turn when present', () => {
    const calls: string[] = [];
    const port = createWasmPort(fakeModule({
      begin_turn: (c) => { calls.push(`begin ${c}`); return '{"ok":true,"value":{"turn":1}}'; },
      advance_month: () => { calls.push('advance'); return '{"ok":true,"value":{"month":0}}'; },
      finish_turn: () => { calls.push('finish'); return '{"ok":false,"error":{"code":"turn_state","message":"x"}}'; },
    }));
    expect(port.liveTurn).toBe(true);
    port.beginTurn([]); port.advanceMonth();
    const f = port.finishTurn();
    expect(calls).toEqual(['begin []', 'advance', 'finish']);
    expect(f.ok).toBe(false);
  });
});

const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
const present = existsSync(resolve(pkg, 'sim_wasm_bg.wasm'));
describe.skipIf(!present)('real wasm package: split turn equals submit_turn', () => {
  it('begin + 3 x advance + finish equals submit_turn and enforces turn_state', async () => {
    const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
    mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
    const probe = new mod.WasmSim();
    if (!probe.begin_turn) return; // an older package: nothing to check
    const plan = JSON.stringify([{ type: 'set_policy', lever: 'carbon_price', value: 40 }]);
    const a = createWasmPort(mod); const b = createWasmPort(mod);
    a.init(11); b.init(11);
    expect(a.liveTurn).toBe(true);
    for (let i = 0; i < 3; i++) {
      const cmds = i === 0 ? JSON.parse(plan) : [];
      const sub = b.submitTurn(cmds);
      expect(a.beginTurn(cmds).ok).toBe(true);
      expect(a.finishTurn().ok).toBe(false); // 0 of 3 months
      for (let m = 0; m < 3; m++) expect(a.advanceMonth().ok).toBe(true);
      const err = a.advanceMonth();
      expect(!err.ok && err.error.code).toBe('turn_state');
      const fin = a.finishTurn();
      expect(JSON.stringify(fin)).toBe(JSON.stringify(sub));
    }
    const ea = a.exportState(); const eb = b.exportState();
    expect(ea.ok && eb.ok && ea.value === eb.value).toBe(true);
  });
});
