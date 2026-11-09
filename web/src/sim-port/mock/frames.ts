import type { MonthFlows, MonthFrame, SimEvent } from '../../contract';
import { GEN } from './data';
import { economy, householdIncome, personRoute, personWage } from './people';
import { nextFloat, noise } from './rng';
import { cellAt, type MockState } from './state';

const SPLIT = { food: 0.4, goods: 0.3, services: 0.3 } as const;

/** Plant tiles in grid order. The mock shares the plants' part of the emissions equally among fossil plants. */
export function plantTiles(s: MockState): { x: number; y: number }[] {
  const out: { x: number; y: number }[] = [];
  for (let y = 0; y < 12; y++) for (let x = 0; x < 12; x++) if (cellAt(s, x, y)!.kind === 'fossil_plant') out.push({ x, y });
  return out;
}
export const PLANT_SHARE = 0.4;

export function makeFlows(s: MockState, emissions: number): MonthFlows {
  const j = (x: number) => Math.max(0, x * (1 + noise(s.rng, 0.1)));
  const firms = GEN.firms.filter((f) => s.firmActive[f.id]);
  const e = economy(s);
  const commutes: NonNullable<MonthFlows['commutes']> = [];
  const wages = new Map<string, { household: number; firm: number; amount: number }>();
  for (const p of s.people) {
    if (p.job === null) continue;
    const r = personRoute(s, p)!;
    commutes.push({ person: p.id, household: p.household, firm: p.job, path: r.path });
    const k = `${p.household}:${p.job}`;
    const w = wages.get(k) ?? { household: p.household, firm: p.job, amount: 0 };
    w.amount += Math.round(j(personWage(s, p)));
    wages.set(k, w);
  }
  const purchases: NonNullable<MonthFlows['purchases']> = [];
  for (const h of s.households) {
    const spend = householdIncome(s, h, e).spending; // a month of spending, split over the sectors
    for (const f of firms) purchases.push({ household: h.id, firm: f.id, sector: f.sector, amount: Math.round(j(spend * SPLIT[f.sector] / firms.filter((x) => x.sector === f.sector).length)) });
  }
  const rent = s.households.map((h) => ({ household: h.id, amount: Math.round(j(householdIncome(s, h, e).rentPaid)) })).filter((r) => r.amount > 0);

  // Emissions: the plants' part (fossil plants only) and the rest between firms and households. The sink absorbs a slice of the total.
  const plants = plantTiles(s);
  const plantTotal = plants.length ? emissions * PLANT_SHARE : 0;
  const rest = emissions - plantTotal;
  const weight = (f: (typeof firms)[number]) => (f.output * f.intensity) / Math.max(1e-9, firms.reduce((a, x) => a + x.output * x.intensity, 0));
  return {
    commutes, purchases, rent, wages: [...wages.values()],
    emissions: [
      ...plants.map((tile) => ({ source: 'plant' as const, tile, tonnes: +(plantTotal / plants.length).toFixed(1) })),
      ...firms.map((f) => ({ source: 'firm' as const, id: f.id, tonnes: +(rest * weight(f) * 0.8).toFixed(1) })),
      { source: 'households' as const, tonnes: +(rest * 0.2).toFixed(1) },
      { source: 'sink' as const, tonnes: -+(emissions * 0.1 * s.forest).toFixed(1) },
    ],
  };
}

/** Events derived from the flows so animation and log agree (the job and household events come from people.ts). */
export function makeEvents(s: MockState, flows: MonthFlows, carry: SimEvent[]): SimEvent[] {
  const ev: SimEvent[] = [...carry];
  const c = flows.commutes?.[0];
  if (c) ev.push({ type: 'commuted', importance: 1, person: c.person, household: c.household, firm: c.firm });
  const biggest = [...(flows.purchases ?? [])].sort((a, b) => b.amount - a.amount)[0];
  if (biggest) ev.push({ type: 'purchase', importance: 1, household: biggest.household, firm: biggest.firm, amount: biggest.amount });
  if (nextFloat(s.rng) < 0.08) ev.push({ type: 'forest_shrank', importance: 2, stock: +s.forest.toFixed(3) });
  return ev;
}

export function frame(month: 0 | 1 | 2, events: SimEvent[], flows: MonthFlows): MonthFrame {
  return { month, events, flows };
}
