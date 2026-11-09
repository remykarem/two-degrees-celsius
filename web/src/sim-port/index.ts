import type { SimPort } from './port';
import { createMockSim } from './mock';
import { loadWasmPort, wasmPackagePresent } from './wasm';

export type { SimPort } from './port';

export type SimKind = 'mock' | 'wasm';
export interface SimSelection {
  port: SimPort;
  kind: SimKind;
  /** Set when the mock runs although the real simulation was wanted or could have been built. Shown to the player. */
  notice?: string;
}

/** The mock simulation, synchronously (tests, and the fallback below). */
export function createSimPort(): SimPort {
  return createMockSim();
}

/**
 * The only place that decides which implementation runs.
 * `?sim=wasm|mock` in the URL wins, then `VITE_SIM`; the default is wasm when the package was built, else the mock.
 */
export async function selectSimPort(search: string = '', env: string | undefined = import.meta.env.VITE_SIM): Promise<SimSelection> {
  const asked = new URLSearchParams(search).get('sim') ?? env ?? 'auto';
  if (asked === 'mock') return { port: createMockSim(), kind: 'mock' };
  if (!wasmPackagePresent()) {
    const notice = 'The real simulation is not built, so this is MOCK data. Run "npm run build:wasm" in web/ (needs Rust and wasm-bindgen-cli; see web/src/sim-port/wasm/README.md).';
    console.warn(notice);
    return { port: createMockSim(), kind: 'mock', notice };
  }
  try {
    return { port: await loadWasmPort(), kind: 'wasm' };
  } catch (e) {
    const notice = `The real simulation could not be loaded (${e instanceof Error ? e.message : String(e)}), so this is MOCK data.`;
    console.error(notice);
    return { port: createMockSim(), kind: 'mock', notice };
  }
}
