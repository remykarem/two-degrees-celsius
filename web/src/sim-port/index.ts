import type { SimPort } from './port';
import { loadReplayPort } from './replay';
import { loadWasmPort, wasmPackagePresent } from './wasm';

export type { SimPort, DemoInfo } from './port';

export type SimKind = 'wasm' | 'replay';
export interface SimSelection {
  port: SimPort;
  kind: SimKind;
  /** Set when the recorded demo runs although the real simulation was wanted or could have been built. Shown to the player. */
  notice?: string;
}

/**
 * The only place that decides which implementation runs. The app always uses the real simulation (wasm); when the package is missing or
 * fails to load it replays a recorded run of that simulation, badged as such. `?sim=replay` (or `VITE_SIM=replay`) asks for the recording.
 */
export async function selectSimPort(search: string = '', env: string | undefined = import.meta.env.VITE_SIM): Promise<SimSelection> {
  const asked = new URLSearchParams(search).get('sim') ?? env ?? 'auto';
  if (asked === 'replay') return { port: await loadReplayPort(), kind: 'replay' };
  if (!wasmPackagePresent()) {
    const notice = 'The real simulation is not built, so this is a RECORDED DEMO (a replay of one real game; your choices cannot change it). Run "npm run build:wasm" in web/ (needs Rust and wasm-bindgen-cli; see web/src/sim-port/wasm/README.md).';
    console.warn(notice);
    return { port: await loadReplayPort(), kind: 'replay', notice };
  }
  try {
    return { port: await loadWasmPort(), kind: 'wasm' };
  } catch (e) {
    const notice = `The real simulation could not be loaded (${e instanceof Error ? e.message : String(e)}), so this is a RECORDED DEMO (a replay of one real game; your choices cannot change it).`;
    console.error(notice);
    return { port: await loadReplayPort(), kind: 'replay', notice };
  }
}
