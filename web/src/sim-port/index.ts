import type { SimPort } from './port';
import { createMockSim } from './mock';

export type { SimPort } from './port';

/** The only place that decides which implementation runs. The wasm one will be added here. */
export function createSimPort(): SimPort {
  return createMockSim();
}
