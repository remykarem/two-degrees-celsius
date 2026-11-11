import { MOCK } from './data';

/** One month of emissions by source, kept in MockState.emLog (the last 3 months make the turn). */
export interface EmMonth { firms: number; households: number; plants: number; sink: number; /** Town v7: private jet flights this month. */ aviation?: number }
export function emissionsMonth(total: number, plants: number, forest: number): EmMonth {
  const rest = Math.max(0, total - plants);
  const hh = rest * (MOCK.extra.householdEmissionShare! / (1 + MOCK.extra.householdEmissionShare!));
  return { firms: rest - hh, households: hh, plants, sink: -total * 0.1 * forest };
}
