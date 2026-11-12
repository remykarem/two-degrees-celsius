import type { NetWorthLine, NetWorthStatement, Snapshot } from '../contract';
import { fill, text } from '../store/catalog';
import { money } from './format';

/**
 * Pure views of the net worth statement (Town v8, contract v2.2). Every line, label, category, count and reconcile flag is the wire's
 * (`breakdowns.gini.byHousehold[].statement`, `.town`); nothing here adds up net worth or decides what is an asset. The web only places
 * the given figures side by side and words the totals.
 */
const t = (key: string, vars?: Record<string, string | number>): string => (vars ? fill(text('networth', key), vars) : text('networth', key));

export interface StatementRow { id: string; label: string; value: string; amount: number; count: string | null; how: string | null }
export interface StatementGroup { category: 'asset' | 'liability'; heading: string; total: string; rows: StatementRow[] }
export interface StatementModel {
  title: string;
  groups: StatementGroup[];
  netWorth: string;
  /** "Net worth" row text, with "per person" when the card is for one person. */
  netLabel: string;
  reconciles: boolean;
  reconcileText: string;
  /** A sentence about equal shares, for a person or a household card. */
  note: string;
  unpaidRentNote: string;
}

const countText = (l: NetWorthLine): string | null => {
  if (l.count === undefined) return null;
  if (l.id === 'homes') return l.count === 1 ? t('count_home') : t('count_homes', { count: l.count });
  return l.count === 1 ? t('count_firm') : t('count_firms', { count: l.count });
};

function build(st: NetWorthStatement, perPerson: boolean, note: string): StatementModel {
  const pick = (l: NetWorthLine) => (perPerson ? l.perPerson : l.value);
  const div = perPerson ? Math.max(1, st.people) : 1;
  const group = (category: 'asset' | 'liability', total: number): StatementGroup => ({
    category, heading: t(category === 'asset' ? 'assets' : 'liabilities'), total: money(total / div),
    rows: st.lines.filter((l) => l.category === category).map((l) => ({ id: l.id, label: l.label, value: money(pick(l)), amount: pick(l), count: countText(l), how: l.howId ?? null })),
  });
  return {
    title: t('title'), groups: [group('asset', st.assets), group('liability', st.liabilities)], netWorth: money(st.netWorth / div),
    netLabel: perPerson ? `${t('net_worth')} (${t('per_person')})` : t('net_worth'),
    reconciles: st.reconciles,
    reconcileText: st.reconciles ? t('reconciles_ok', { assets: money(st.assets), liabilities: money(st.liabilities), net: money(st.netWorth) })
      : t('reconciles_bad', { net: money(st.netWorth), headline: money(st.headline) }),
    note, unpaidRentNote: t('unpaid_rent_note'),
  };
}

/** The statement of a household (every line in dollars), or of one person of it (each line per person) when `person` is given. */
export function statementModel(s: Snapshot, household: number, person?: { name: string }): StatementModel | null {
  const w = s.breakdowns?.gini.byHousehold.find((x) => x.household === household);
  if (!w) return null;
  const st = w.statement;
  return build(st, !!person, person ? t('person_note', { name: person.name, people: st.people }) : t('household_note', { people: st.people }));
}

export interface HolderRow { id: string; label: string; note: string; counts: string; assets: string; liabilities: string; netWorth: string; memo: boolean; how: string | null; amount: number }
export interface TownModel { title: string; intro: string; holders: HolderRow[]; total: string; reconciles: boolean; reconcileText: string; memoLabel: string }

/** "Who holds what": the wire's holder list (the headline groups add up to the Gini's total; the memo rows do not). */
export function townModel(s: Snapshot): TownModel | null {
  const g = s.breakdowns?.gini;
  if (!g) return null;
  const tw = g.town;
  const allHouseholds = s.households.length, allPeople = s.people.length;
  return {
    title: t('town_title'), intro: t('town_intro'), memoLabel: t('memo'),
    holders: tw.holders.map((h) => ({
      id: h.id, label: h.label, note: h.note, memo: !h.inHeadline, how: h.howId ?? null, amount: h.netWorth,
      counts: h.inHeadline ? (h.households ? t('holder_counts', { households: h.households, all_households: allHouseholds, people: h.people, all_people: allPeople }) : t('holder_none')) : '',
      assets: money(h.assets), liabilities: money(h.liabilities), netWorth: money(h.netWorth),
    })),
    total: money(tw.holdersTotal), reconciles: tw.reconciles,
    reconcileText: t(tw.reconciles ? 'holders_ok' : 'holders_bad', { total: money(tw.holdersTotal), headline: money(tw.headline) }),
  };
}
