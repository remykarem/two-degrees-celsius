import type { ClassKey, HouseholdView, PersonView, Snapshot } from '../contract';
import type { Selection } from '../store';
import { money } from './format';
import { classShort } from '../store/catalog';
import { incomeTotal } from '../store/sources';
import { employerLine } from './estate';

export type HouseholdSortKey = 'id' | 'netWorth' | 'income';

export interface HouseholdRow {
  id: number;
  label: string;
  class: ClassKey;
  people: number;
  /** 'Home 3' or 'no home'. */
  homeText: string;
  housed: boolean;
  netWorth: number;
  /** Monthly income from every source. */
  income: number;
  netWorthText: string;
  incomeText: string;
}

export const CLASS_SHORT = classShort;

/** Monthly income of a household from all five sources. */
export const householdIncome = (h: HouseholdView) => incomeTotal(h.income.sources);

export function householdRows(snap: Snapshot, sort: HouseholdSortKey = 'id'): HouseholdRow[] {
  const rows = snap.households.map((h): HouseholdRow => {
    const income = householdIncome(h);
    return {
      id: h.id, label: h.label, class: h.class, people: h.members.length, housed: h.home !== null,
      homeText: h.home !== null ? `Home ${h.home}` : 'no home', netWorth: h.netWorth, income,
      netWorthText: money(h.netWorth), incomeText: money(income),
    };
  });
  if (sort === 'netWorth') rows.sort((a, b) => b.netWorth - a.netWorth || a.id - b.id);
  else if (sort === 'income') rows.sort((a, b) => b.income - a.income || a.id - b.id);
  else rows.sort((a, b) => a.id - b.id);
  return rows;
}

export interface PersonRow { id: number; label: string; household: number; detail: string }
export interface PersonGroup { household: number; label: string; people: PersonRow[] }

const AGE = { child: 'child', adult: 'adult', retired: 'retired' } as const;

/** One line about a person: their job, or what they do instead. */
export function personDetail(p: PersonView, snap: Snapshot): string {
  if (p.employer) return employerLine(p)!;
  if (p.job) return `works at ${snap.firms.find((f) => f.id === p.job!.firm)?.name ?? `firm ${p.job.firm}`}`;
  if (p.ageBand !== 'adult') return AGE[p.ageBand];
  return p.inLabourForce ? 'looking for work' : 'not in the labour force';
}

/** People grouped by household, in household order. */
export function peopleGroups(snap: Snapshot, onlyHousehold?: number | null): PersonGroup[] {
  return snap.households
    .filter((h) => onlyHousehold == null || h.id === onlyHousehold)
    .map((h) => ({
      household: h.id, label: h.label,
      people: h.members.flatMap((id) => snap.people.filter((p) => p.id === id)).map((p) => ({ id: p.id, label: p.label, household: h.id, detail: personDetail(p, snap) })),
    }));
}

/** The household a selection belongs to (a person's, a home's occupant, or the household itself), or null. */
export function selectedHouseholdId(snap: Snapshot | null, sel: Selection | null): number | null {
  if (!snap || !sel) return null;
  switch (sel.kind) {
    case 'household': return snap.households.some((h) => h.id === sel.id) ? sel.id : null;
    case 'person': return snap.people.find((p) => p.id === sel.id)?.household ?? null;
    case 'home': return snap.homes.find((h) => h.id === sel.id)?.household ?? null;
    default: return null;
  }
}

/** Screen-reader sentence announced when the selection changes through a list. */
export function selectionAnnouncement(snap: Snapshot | null, sel: Selection | null): string {
  if (!snap || !sel) return '';
  if (sel.kind === 'household') { const h = snap.households.find((x) => x.id === sel.id); return h ? `Selected ${h.label}, ${h.members.length} people${h.home === null ? ', no home' : ''}.` : ''; }
  if (sel.kind === 'person') {
    const p = snap.people.find((x) => x.id === sel.id);
    const h = p && snap.households.find((x) => x.id === p.household);
    return p ? `Selected ${h ? `${h.label}, ` : ''}${p.label}, ${personDetail(p, snap)}.` : '';
  }
  return '';
}

/** Unemployed and labour force counts from the people (the sim's own counts when present). */
export function unemploymentCount(snap: Snapshot): { unemployed: number; labourForce: number } {
  const lf = snap.people.filter((p) => p.inLabourForce).length;
  const un = snap.people.filter((p) => p.inLabourForce && !p.job && !p.employer).length;
  return lf > 0 ? { unemployed: un, labourForce: lf } : { unemployed: Math.round(snap.indicators.unemployment * snap.indicators.labourForce), labourForce: Math.round(snap.indicators.labourForce) };
}
