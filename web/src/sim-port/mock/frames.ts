import type { MonthFlows, MonthFrame, SimEvent } from '../../contract';
import { GEN } from './data';
import { nextFloat, noise, pickInt } from './rng';
import type { MockState } from './state';

const hh = () => GEN.households;

export function makeFlows(s: MockState, emissions: number): MonthFlows {
  const firms = GEN.firms.filter((_, i) => s.firmActive[i]);
  const j = (x: number) => Math.max(0, x * (1 + noise(s.rng, 0.1)));
  const workers = hh().filter((h) => h.labourShare >= 0.8);
  const goods = firms.filter((f) => f.sector === 'goods');
  const share = (f: (typeof firms)[number]) => (f.output * f.intensity) / Math.max(1e-9, firms.reduce((a, x) => a + x.output * x.intensity, 0));
  return {
    commutes: workers.map((h) => ({ household: h.id, firm: firms[pickInt(s.rng, firms.length)]!.id, people: Math.round(j(h.people * h.labourShare * (1 - s.unemployment))) })),
    purchases: hh().flatMap((h) => firms.map((f) => ({ household: h.id, sector: f.sector, amount: Math.round(j(h.people * 90)) }))),
    rent: hh().filter((h) => h.class !== 'rentier').map((h) => ({ household: h.id, amount: Math.round(j(h.people * h.renterShare * GEN.town.rentPrivate)) })),
    wages: workers.map((h) => ({ household: h.id, firm: (goods[0] ?? firms[0]!).id, amount: Math.round(j(h.people * h.labourShare * s.wage * h.wagePremium)) })),
    emissions: [
      ...firms.map((f) => ({ source: 'firm' as const, id: f.id, tonnes: +(emissions * share(f) * 0.8).toFixed(1) })),
      { source: 'households' as const, tonnes: +(emissions * 0.2).toFixed(1) },
      { source: 'sink' as const, tonnes: -+(emissions * 0.1 * s.forest).toFixed(1) },
    ],
  };
}

/** Random-ish monthly events, derived from the flows so animation and log agree. */
export function makeEvents(s: MockState, uDelta: number, flows: MonthFlows, carry: SimEvent[]): SimEvent[] {
  const ev: SimEvent[] = [...carry];
  const firm = flows.wages?.[0]?.firm ?? 0;
  const people = Math.round(Math.abs(uDelta) * GEN.town.labourForce);
  if (people > 0) ev.push({ type: uDelta < 0 ? 'hired' : 'fired', importance: 1, firm, people });
  const c = flows.commutes?.[0];
  if (c) ev.push({ type: 'commuted', importance: 1, household: c.household, firm: c.firm, people: c.people });
  if (nextFloat(s.rng) < 0.15) {
    const dep = hh().find((h) => h.class === 'dependent') ?? hh()[0]!;
    ev.push({ type: 'rent_shortfall', importance: 2, household: dep.id, amount: Math.round(500 + nextFloat(s.rng) * 1500) });
  }
  if (nextFloat(s.rng) < 0.08) ev.push({ type: 'forest_shrank', importance: 2, stock: +s.forest.toFixed(3) });
  return ev;
}

export function frame(month: 0 | 1 | 2, events: SimEvent[], flows: MonthFlows): MonthFrame {
  return { month, events, flows };
}
