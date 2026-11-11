// The real simulation for tests that need a SimPort: the wasm package, loaded once in Node. These tests need `npm run build:wasm` (CI builds it before `npm test`);
// without the package, vite.config.ts leaves the files that import this module out of the run and says so.
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import type { SimPort } from '../src/sim-port/port';

const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
export const wasmBuilt = existsSync(resolve(pkg, 'sim_wasm_bg.wasm'));
let mod: WasmModuleLike | null = null;
if (wasmBuilt) {
  const m = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  m.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  mod = m;
}

/** A fresh real simulation (its own state), already started with `seed`. */
export function createSimPort(seed = 42): SimPort {
  if (!mod) throw new Error('The wasm package is not built: run npm run build:wasm in web/.');
  const port = createWasmPort(mod);
  port.init(seed);
  return port;
}
