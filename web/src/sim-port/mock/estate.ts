import type { SimEvent } from '../../contract';
import { GEN } from './data';
import { nextFloat } from './rng';
import { hKey } from './ledger';
import type { MockState } from './state';

// MOCK ONLY: the estate's jet, pilot and flights (docs/town-v7.md). The numbers come from content/ (GEN.estate); the draw uses the estate's own seeded
// stream so it never disturbs the economy's draws, and the dismissal rule is a labelled copy of crates/sim-model/src/estate.rs, applied to the mock's own money.

export const pilotOf = (s: MockState) => (s.estate?.pilot === null || !s.estate ? null : s.people[s.estate.pilot!] ?? null);
export const pilotEmployed = (s: MockState): boolean => !!pilotOf(s) && pilotOf(s)!.staffOf !== null;

/** Start of a month: dismiss or rehire the pilot, then one flight draw (always taken). Returns the month's events and the flight's tonnes. */
export function stepEstateMonth(s: MockState, month: 0 | 1 | 2): { events: SimEvent[]; tonnes: number } {
  const e = s.estate, g = GEN.estate;
  const events: SimEvent[] = [];
  if (!e || !g) return { events, tonnes: 0 };
  const draw = nextFloat(e.rng) < g.flightProbMonthly;
  const pilot = pilotOf(s);
  const savings = s.ledger.balance[hKey(g.household)] ?? s.households[g.household]!.savings;
  if (pilot) {
    if (pilot.staffOf !== null && e.unpaidMonths >= g.pilotUnpaidMonthsMax) {
      pilot.staffOf = null; e.unpaidMonths = 0;
      events.push({ type: 'pilot_let_go', importance: 3, person: pilot.id });
    } else if (pilot.staffOf === null && pilot.job === null && savings >= g.pilotWageUsd * g.pilotRehireMonthsOfWages) {
      pilot.staffOf = g.household; e.unpaidMonths = 0;
      events.push({ type: 'pilot_hired', importance: 2, person: pilot.id });
    }
  }
  const flies = draw && (!pilot || pilot.staffOf !== null);
  e.flyingThisMonth = flies;
  if (!flies) return { events, tonnes: 0 };
  e.flightsTotal++; e.tonnesTotal += g.flightTonnes; e.lastFlight = { turn: s.turn, month, tonnes: g.flightTonnes };
  events.push({ type: 'private_jet_flight', importance: 2, household: g.household, tonnes: g.flightTonnes });
  return { events, tonnes: g.flightTonnes };
}

/** After the month's money moved: a short payment counts towards dismissal (the wage is paid out of savings plus income, as in income.rs). */
export function settleEstatePay(s: MockState): void {
  const e = s.estate, g = GEN.estate;
  if (!e || !g || !pilotEmployed(s)) return;
  const savings = s.ledger.balance[hKey(g.household)] ?? 0;
  e.unpaidMonths = savings < 0 ? e.unpaidMonths + 1 : 0;
}
