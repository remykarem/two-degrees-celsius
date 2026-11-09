import type { HouseholdView, PersonView, SimEvent } from '../../contract';
import { GEN } from './data';
import { commute, type Route } from './space';
import { cellAt, type HouseholdState, type MockState, type PersonState } from './state';

// MOCK ONLY: person and household money and wellbeing are invented toy rules, shaped like the real sim's outputs.

export const personWage = (s: MockState, p: PersonState): number => (p.job === null ? 0 : Math.round(Math.max(Number(s.policy.min_wage), s.wage * (0.7 + 0.6 * p.skill))));
const homeOf = (s: MockState, h: HouseholdState) => (h.home === null ? undefined : s.homes.find((x) => x.id === h.home));

export function personRoute(s: MockState, p: PersonState): Route | null {
  if (p.job === null) return null;
  return commute(s, s.households[p.household]!.home, p.job);
}

/** Close to a fossil plant or the goods firm (within 2 tiles): the optional local pollution term of docs/town-v2.md section 6. */
function polluted(s: MockState, h: HouseholdState): boolean {
  const home = homeOf(s, h);
  if (!home) return false;
  const near = (x: number, y: number) => Math.abs(home.x - x) + Math.abs(home.y - y) <= 2;
  for (let y = 0; y < 12; y++) for (let x = 0; x < 12; x++) if (cellAt(s, x, y)!.kind === 'fossil_plant' && near(x, y)) return true;
  return GEN.firms.some((f) => f.sector === 'goods' && s.firmActive[f.id] && near(f.tile.x, f.tile.y));
}

export function wellbeingOf(s: MockState, p: PersonState, route: Route | null): number {
  const h = s.households[p.household]!;
  let w = s.wellbeing;
  if (p.labourForce) w += p.job !== null ? 0.08 : -0.1;
  if (p.ageBand === 'child') w += 0.02;
  if (h.home === null) w -= 0.12;
  if (route) w -= 0.002 * Math.max(0, route.minutes - GEN.space.freeMinutes);
  if (polluted(s, h)) w -= 0.03;
  if (h.savings <= 0 && h.debt > 0) w -= 0.05;
  return Math.min(1, Math.max(0, w));
}

export function peopleViews(s: MockState): PersonView[] {
  return s.people.map((p) => {
    const r = personRoute(s, p);
    return {
      id: p.id, label: p.label, household: p.household, ageBand: p.ageBand, skill: p.skill, job: p.job === null ? null : { firm: p.job },
      wage: personWage(s, p), commuteTiles: r?.tiles ?? 0, commuteMinutes: r?.minutes ?? 0, wellbeing: wellbeingOf(s, p, r),
      inLabourForce: p.labourForce, jobGuarantee: false,
      effectiveHours: r ? Math.max(GEN.space.minEffectiveHours, 1 - GEN.space.kCommute * Math.max(0, r.minutes - GEN.space.freeMinutes)) : 1,
    };
  });
}

export interface Economy { monthlyOutput: number; wageBill: number; profitFirms: number; activeFirms: number }
export function economy(s: MockState): Economy {
  const monthlyOutput = s.gdp / 12;
  const wageBill = s.people.reduce((a, p) => a + personWage(s, p), 0);
  return { monthlyOutput, wageBill, profitFirms: Math.max(0, monthlyOutput - wageBill), activeFirms: s.firmActive.filter(Boolean).length };
}

export type Income = HouseholdView['income'];
export function householdIncome(s: MockState, h: HouseholdState, e: Economy): Income {
  const members = h.members.map((id) => s.people[id]!);
  const taxWage = Number(s.policy.tax_wage), taxProfit = Number(s.policy.tax_profit);
  const wages = members.reduce((a, p) => a + personWage(s, p), 0);
  const owned = GEN.firms.filter((f) => f.founder === h.id && s.firmActive[f.id]).length;
  const dividends = e.activeFirms ? (e.profitFirms * 0.6 * (1 - taxProfit) * owned) / e.activeFirms : 0;
  const rent = s.homes.filter((x) => x.owner === h.id && x.household !== null && x.household !== h.id).length * GEN.town.rentPrivate;
  const home = homeOf(s, h);
  const rentPaid = home && home.owner !== h.id ? (home.owner === 'public' ? GEN.town.rentPublic : GEN.town.rentPrivate) : 0;
  const poor = h.class === 'worker' || h.class === 'dependent';
  const claimants = members.filter((p) => (p.labourForce && p.job === null) || (p.ageBand === 'retired' && poor)).length;
  const benefits = claimants * Number(s.policy.benefit) + members.length * Number(s.policy.ubi);
  const interest = (h.savings * Number(s.policy.base_rate)) / 12;
  const gross = wages + dividends + rent + interest + benefits;
  const taxPaid = wages * taxWage;
  const loanPayments = h.debt * 0.003;
  const spending = Math.max(0, 0.85 * (gross - taxPaid - rentPaid - loanPayments));
  return { wages, dividends, rent, interest, benefits, taxPaid, rentPaid, loanPayments, spending };
}

export const totalIncome = (i: Income) => i.wages + i.dividends + i.rent + i.interest + i.benefits;

/** Wealth: savings plus owned homes and founded firms, minus debt (mock valuation of firms: capital x 1,500). */
export function netWorth(s: MockState, h: HouseholdState): number {
  const homes = s.homes.filter((x) => x.owner === h.id).length * GEN.homes[0]!.valueUsd;
  const firms = GEN.firms.filter((f) => f.founder === h.id && f.active).reduce((a, f) => a + f.capital * 1500, 0);
  return h.savings + homes + firms - h.debt;
}

export function householdViews(s: MockState): HouseholdView[] {
  const e = economy(s);
  const people = peopleViews(s);
  return s.households.map((h) => {
    const mine = people.filter((p) => p.household === h.id);
    const power = h.class === 'capitalist' ? 0.4 : h.class === 'rentier' ? 0.3 : h.class === 'manager' ? 0.1 : 0.03;
    return { id: h.id, label: h.label, members: [...h.members], home: h.home, class: h.class, savings: h.savings, debt: h.debt, netWorth: netWorth(s, h), income: householdIncome(s, h, e), politicalPower: power, arrearsMonths: h.arrears,
      wellbeing: mine.reduce((a, p) => a + p.wellbeing, 0) / Math.max(1, mine.length), footprintTonnes: 0.1 * (s.lastMonthlyEmissions / s.households.length), foodMet: h.savings > 0 || h.home !== null ? 1 : 0.7 };
  });
}

export const median = (xs: number[]): number => {
  const a = [...xs].sort((x, y) => x - y);
  const m = a.length >> 1;
  return a.length === 0 ? 0 : a.length % 2 ? a[m]! : (a[m - 1]! + a[m]!) / 2;
};
export function gini(xs: number[]): number {
  const a = xs.map((x) => Math.max(0, x)).sort((x, y) => x - y);
  const n = a.length, total = a.reduce((p, q) => p + q, 0);
  if (!n || total <= 0) return 0;
  return a.reduce((acc, x, i) => acc + (2 * (i + 1) - n - 1) * x, 0) / (n * total);
}

/** Per-person income: each member carries an equal share of the household's income. */
export function personIncomes(s: MockState): number[] {
  const e = economy(s);
  return s.households.flatMap((h) => h.members.map(() => totalIncome(householdIncome(s, h, e)) / h.members.length));
}
export function topWealthShare(s: MockState): number {
  const w = s.households.map((h) => Math.max(0, netWorth(s, h)));
  const total = w.reduce((a, b) => a + b, 0);
  return total > 0 ? Math.max(...w) / total : 0;
}

/** Monthly household money: savings absorb the surplus, debt the shortfall; three months of unpaid rent mean eviction. Also moves. */
export function stepHouseholds(s: MockState, events: SimEvent[]): void {
  const e = economy(s);
  for (const h of s.households) {
    const inc = householdIncome(s, h, e);
    const disposable = totalIncome(inc) - inc.taxPaid - inc.rentPaid - inc.loanPayments;
    h.savings += disposable - inc.spending;
    if (h.savings < 0) { h.debt += -h.savings; h.savings = 0; }
    if (inc.rentPaid > 0 && disposable < inc.rentPaid) {
      h.arrears++;
      events.push({ type: 'rent_shortfall', importance: 2, household: h.id, amount: Math.round(inc.rentPaid - Math.max(0, disposable)) });
    } else h.arrears = 0;
    if (h.arrears >= 3 && h.home !== null) {
      const home = s.homes.find((x) => x.id === h.home);
      if (home) home.household = null;
      h.home = null;
      h.arrears = 0;
      events.push({ type: 'household_evicted', importance: 3, household: h.id });
    }
  }
  // Moves: an unhoused household takes the cheapest vacant home whose rent is at most half of its income.
  for (const h of [...s.households].filter((x) => x.home === null).sort((a, b) => b.savings - a.savings)) {
    const income = totalIncome(householdIncome(s, h, e));
    const cost = (x: { owner: number | 'public' }) => (x.owner === 'public' ? GEN.town.rentPublic : GEN.town.rentPrivate);
    const vacant = s.homes.filter((x) => x.household === null && cost(x) * 2 <= income).sort((a, b) => cost(a) - cost(b) || a.id - b.id)[0];
    if (!vacant) continue;
    vacant.household = h.id;
    h.home = vacant.id;
    h.arrears = 0;
    events.push({ type: 'household_moved', importance: 2, household: h.id, to: vacant.id });
  }
}

/** Match the number of employed people to the mock's aggregate unemployment: hire the most skilled jobless, fire the least skilled. */
export function syncJobs(s: MockState, events: SimEvent[]): void {
  const lf = s.people.filter((p) => p.labourForce);
  const target = Math.round(lf.length * (1 - s.unemployment));
  const active = GEN.firms.filter((f) => s.firmActive[f.id]);
  if (!active.length) return;
  const staff = (id: number) => s.people.filter((p) => p.job === id).length;
  let employed = lf.filter((p) => p.job !== null);
  while (employed.length > target) {
    const p = [...employed].sort((a, b) => a.skill - b.skill || b.id - a.id)[0]!;
    events.push({ type: 'person_fired', importance: 2, person: p.id, firm: p.job! });
    p.job = null;
    employed = lf.filter((x) => x.job !== null);
  }
  while (employed.length < target) {
    const p = lf.filter((x) => x.job === null).sort((a, b) => b.skill - a.skill || a.id - b.id)[0];
    if (!p) break;
    const f = [...active].sort((a, b) => staff(a.id) - staff(b.id) || a.id - b.id)[0]!;
    p.job = f.id;
    events.push({ type: 'person_hired', importance: 2, person: p.id, firm: f.id });
    employed = lf.filter((x) => x.job !== null);
  }
  // Jobs at a firm that is not active any more (not reachable in the mock, kept for safety).
  for (const p of s.people) if (p.job !== null && !s.firmActive[p.job]) p.job = null;
}
