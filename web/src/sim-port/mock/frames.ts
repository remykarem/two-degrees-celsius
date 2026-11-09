import type { MonthFlows, MonthFrame, SimEvent, SimEventType } from '../../contract';
import { DATA } from './data';
import { noise, nextFloat, pickInt } from './rng';
import type { MockState } from './state';

const fill = (t: string, vars: Record<string, string | number>) =>
  t.replace(/\{(\w+)\}/g, (_, k: string) => String(vars[k] ?? ''));

export function text(type: SimEventType, vars: Record<string, string | number> = {}): string {
  return fill(DATA.eventTexts[type] ?? type, vars);
}

export function makeFlows(s: MockState, emissions: number): MonthFlows {
  const hh = DATA.households;
  const firms = DATA.firms.filter((_, i) => s.firmActive[i]);
  const j = (x: number) => Math.max(0, x * (1 + noise(s.rng, 0.1)));
  const workers = hh.filter((h) => h.cls === 'worker' || h.cls === 'manager');
  return {
    commutes: workers.map((h) => ({ household: h.id, firm: firms[pickInt(s.rng, firms.length)]!.id, people: Math.round(j(h.people * (1 - s.unemployment))) })),
    purchases: hh.flatMap((h) => firms.map((f) => ({ household: h.id, sector: f.sector, amount: Math.round(j(h.people * 90)) }))),
    rent: hh.filter((h) => h.cls !== 'rentier').map((h) => ({ household: h.id, amount: Math.round(j(h.people * 40)) })),
    wages: workers.map((h) => ({ household: h.id, firm: firms[0]!.id, amount: Math.round(j(h.people * 160)) })),
    emissions: [
      ...firms.map((f) => ({ source: 'firm' as const, id: f.id, tonnes: +(emissions * 0.4).toFixed(1) })),
      { source: 'households' as const, tonnes: +(emissions * 0.2).toFixed(1) },
      { source: 'sink' as const, tonnes: +(-emissions * 0.1 * s.forest).toFixed(1) },
    ],
  };
}

export function makeEvents(s: MockState, uDelta: number, carry: SimEvent[]): SimEvent[] {
  const ev: SimEvent[] = [...carry];
  const n = Math.round(Math.abs(uDelta) * 1000);
  if (n > 0) ev.push({ type: uDelta < 0 ? 'hired' : 'fired', importance: 1, text: text(uDelta < 0 ? 'hired' : 'fired', { n }), amount: n });
  ev.push({ type: 'commuted', importance: 1, text: text('commuted') });
  if (nextFloat(s.rng) < 0.15) ev.push({ type: 'rent_shortfall', importance: 2, text: text('rent_shortfall') });
  if (nextFloat(s.rng) < 0.08) ev.push({ type: 'forest_shrank', importance: 2, text: text('forest_shrank') });
  return ev;
}

export function frame(month: 0 | 1 | 2, events: SimEvent[], flows: MonthFlows): MonthFrame {
  return { month, events, flows };
}
