import type { EnergyView } from '../contract';

/** Fossil share of plant capacity, 0..1 (0 with no plants). The scene's old `energy.fossilShare`, derived from the v1.4 per-plant supply. */
export function fossilShareOf(e: Pick<EnergyView, 'supply'> | null | undefined): number {
  if (!e || !e.supply.length) return 0;
  const total = e.supply.reduce((a, p) => a + p.capacity, 0);
  return total > 0 ? e.supply.filter((p) => p.kind === 'fossil').reduce((a, p) => a + p.capacity, 0) / total : 0;
}
