import type { EnergyView, Snapshot } from '../contract';
import { compact, pct } from './format';

/** Pure derivations for the energy panel (docs/town-v5.md section 5). */

/** "MWh/month" from the wire's unit ('MWh' per the contract; a unit that already has a "/" is kept as given). */
export const unitLabel = (e: Pick<EnergyView, 'unit'>): string => (!e.unit ? 'MWh/month' : e.unit.includes('/') ? e.unit : `${e.unit}/month`);
export const energyText = (v: number, e: Pick<EnergyView, 'unit'>): string => `${compact(v)} ${unitLabel(e)}`;
export const UNIT_HELP = (e: Pick<EnergyView, 'unit'>): string =>
  `${unitLabel(e)}: energy per month, in megawatt-hours. A person uses roughly 0.1 to 0.25 of it a month; a plant has the capacity shown. The unit is a label from the game's settings; the numbers are the model's own.`;

export interface PlantRow {
  tile: { x: number; y: number }; kind: 'fossil' | 'renewable'; kindLabel: string; capacity: number; capacityText: string;
  /** Share of delivered energy this plant supplies (capacity-proportional). */
  outputShare: number; emissions: number;
}
export function plantRows(e: EnergyView): PlantRow[] {
  const total = e.supply.reduce((a, p) => a + p.capacity, 0);
  return e.supply.map((p) => ({
    tile: p.tile, kind: p.kind, kindLabel: p.kind === 'fossil' ? 'Fossil' : 'Renewable', capacity: p.capacity, capacityText: energyText(p.capacity, e),
    outputShare: total > 0 ? p.capacity / total : 0, emissions: e.emissionsByPlant.find((x) => x.tile.x === p.tile.x && x.tile.y === p.tile.y)?.tonnes ?? 0,
  }));
}
/** The plant on a tile, for the plant inspector. */
export const plantAt = (snap: Snapshot, t: { x: number; y: number }): PlantRow | undefined => plantRows(snap.energy).find((p) => p.tile.x === t.x && p.tile.y === t.y);

export interface EnergyBalance { capacity: number; demand: number; served: number; servedText: string; rationed: boolean; warning: string | null; firmsShare: number; householdsShare: number }
export function energyBalance(e: EnergyView): EnergyBalance {
  const capacity = e.supply.reduce((a, p) => a + p.capacity, 0);
  const rationed = e.served < 0.999;
  return {
    capacity, demand: e.demand.total, served: e.served, servedText: pct(e.served), rationed,
    warning: rationed ? `Demand is above capacity: only ${pct(e.served)} of it is served. Firms and households are being rationed. Build a plant.` : null,
    firmsShare: e.demand.total > 0 ? e.demand.firms / e.demand.total : 0, householdsShare: e.demand.total > 0 ? e.demand.households / e.demand.total : 0,
  };
}
export const intensityText = (e: EnergyView): string => `${e.intensity.toFixed(2)} t CO2 per ${e.unit || 'MWh'}`;
