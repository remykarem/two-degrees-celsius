import { ownerWord } from '../store/catalog';
import type { HouseholdView, Industry, IndustryFirm, InfraKind } from '../contract';
import { compact, money, tonnes } from './format';

/** Pure derivations for the Industry tab (contract v1.6). */
export const SECTOR_WORD = { goods: 'Goods', food: 'Food', services: 'Services' } as const;
export const INFRA_WORD: Record<InfraKind, string> = { road: 'Roads', public_housing: 'Public housing', fossil_plant: 'Fossil power plants', renewable_plant: 'Renewable power plants', fire_station: 'Fire stations' };

/** A household owner is named by its label and tag ("Estate owner") when the snapshot's households are given, else by id. */
export function ownershipText(f: IndustryFirm, households: HouseholdView[] = []): string {
  return f.ownership.map((o) => {
    const h = o.household !== undefined ? households.find((x) => x.id === o.household) : undefined;
    const who = h ? `${h.label}${h.tag && h.tag !== h.label ? ` (${h.tag})` : ''}` : `${ownerWord(o.owner)}${o.household !== undefined ? ` ${o.household}` : ''}`;
    return `${who} ${o.share.toFixed(2)}`;
  }).join(', ');
}
export const stateShareText = (f: IndustryFirm): string => (f.stateShare > 0 ? `state holds ${f.stateShare.toFixed(2)} of the shares` : 'state holds no shares (0 of 1)');

export interface FirmLine { firm: number; name: string; sector: string; cells: { label: string; text: string }[]; ownership: string; state: string }
export function firmLines(ind: Industry, households: HouseholdView[] = []): FirmLine[] {
  return ind.firms.map((f) => ({
    firm: f.firm, name: f.name, sector: SECTOR_WORD[f.sector],
    cells: [
      { label: 'Output', text: compact(f.output) }, { label: 'Price', text: money(f.price) }, { label: 'Workers', text: String(f.workers) }, { label: 'Wage', text: `${money(f.wage)}/month` },
      { label: 'Revenue', text: `${money(f.revenue)}/month` }, { label: 'Profit', text: `${money(f.profit)}/month` }, { label: 'Emissions', text: `${tonnes(f.emissions)}/month` },
    ], ownership: ownershipText(f, households), state: stateShareText(f),
  }));
}
export interface InfraLine { kind: InfraKind; label: string; count: number; cost: string; upkeep: string; upkeepTotal: string }
export const infraLines = (ind: Industry): InfraLine[] => ind.infrastructure.map((i) => ({
  kind: i.kind, label: INFRA_WORD[i.kind], count: i.count, cost: money(i.buildCost), upkeep: `${money(i.upkeepMonthly)}/month each`, upkeepTotal: `${money(i.upkeepMonthly * i.count)}/month`,
}));
/** True when every firm has no state share (the note says so for the current systems). */
export const noStateOwnership = (ind: Industry): boolean => ind.firms.every((f) => f.stateShare === 0);
