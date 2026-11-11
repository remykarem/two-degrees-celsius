import type { Estate, HouseholdView, PersonView, Snapshot } from '../contract';
import { fill, text } from '../store/catalog';
import { compact, money, tonnes } from './format';

/**
 * Pure views of the elite estate (Town v7, contract v1.9). Every label and sentence is the wire's or `catalog().text.estate`; the numbers are the wire's.
 * Nothing here decides anything: it places given figures side by side (and says so where it has to estimate).
 */
export interface EstateRow { label: string; value: string; hint?: string }
export interface EstateSection { heading: string; rows: EstateRow[] }

const t = (key: string, vars?: Record<string, string | number>): string => (vars ? fill(text('estate', key), vars) : text('estate', key));
const n1 = (v: number): string => String(Math.round(v * 10) / 10);
const plural = (count: number, one: string, many: string): string => `${count} ${count === 1 ? one : many}`;

export const estateHousehold = (s: Snapshot): HouseholdView | null => (s.estate ? s.households.find((h) => h.id === s.estate!.household) ?? null : null);
export const isEstatePerson = (s: Snapshot, p: PersonView): boolean => !!s.estate && p.household === s.estate.household;
export const isPilot = (s: Snapshot, p: PersonView): boolean => !!s.estate?.pilot && s.estate.pilot.person === p.id;
const personName = (s: Snapshot, id: number): string => s.people.find((p) => p.id === id)?.label ?? `person ${id}`;

/** The richest household, named, with counts: "Estate owner (1 of 9 households, 1 of 22 people) holds $2.1M of the town's $3.4M net worth." */
export function topHouseholdLine(s: Snapshot): string | null {
  if (!s.households.length) return null;
  const top = s.households.reduce((a, h) => (h.netWorth > a.netWorth ? h : a), s.households[0]!);
  return t('top_share_line', {
    name: top.tag ?? top.label, households: s.households.length, people: top.members.length, total_people: s.people.length,
    top: money(top.netWorth), total: money(s.finance.households.netWorth),
  });
}

/**
 * The wealth tax the estate was charged against what it owed. The wire sends the amount paid (`estate.finance.month.wealthTax`) but not the amount owed, so
 * owed is estimated here from the policy in force and the household's net worth (rate in force, one twelfth a month, above the threshold): a labelled
 * estimate, shown only when the estate's savings are below it. Returns null when nothing is short.
 */
export function wealthTaxShortfall(s: Snapshot): { owed: number; paid: number; savings: number } | null {
  const e = s.estate, h = estateHousehold(s);
  const rate = s.policy?.wealth_tax, thr = s.policy?.wealth_threshold;
  if (!e || !h || !rate || !thr) return null;
  const rateNow = rate.effective ?? Number(rate.nominal), thresholdNow = thr.effective ?? Number(thr.nominal);
  const monthsPerYear = s.game.turnsPerYear * s.game.ticksPerTurn;
  const owed = Math.max(0, h.netWorth - thresholdNow) * rateNow / monthsPerYear;
  const paid = e.finance.month.wealthTax;
  return owed > 0 && owed > paid && h.savings < owed ? { owed, paid, savings: Math.max(0, h.savings) } : null;
}
export const wealthTaxShortText = (s: Snapshot): string | null => {
  const x = wealthTaxShortfall(s);
  return x ? t('wealth_tax_short', { owed: money(x.owed), paid: money(x.paid), savings: money(x.savings) }) : null;
};

export interface PilotView { name: string; status: 'employed' | 'let_go'; statusText: string; rows: EstateRow[]; warning: string | null; grounded: string | null }
export function pilotView(s: Snapshot): PilotView | null {
  const e = s.estate;
  if (!e?.pilot) return null;
  const p = e.pilot;
  const unpaid = p.unpaidMonths;
  const rows: EstateRow[] = [
    { label: 'Status', value: t(p.status === 'employed' ? 'pilot_status_employed' : 'pilot_status_let_go') },
    { label: 'Wage', value: `${money(p.wage)}/month` },
    { label: 'Short-paid months', value: `${unpaid} of ${p.unpaidMonthsMax} in a row` },
  ];
  return {
    name: personName(s, p.person), status: p.status, statusText: rows[0]!.value, rows,
    warning: p.status === 'employed' && unpaid > 0 ? t('pilot_unpaid_warning', { unpaid, max: p.unpaidMonthsMax }) : null,
    grounded: p.status === 'let_go' ? t('grounded_note') : null,
  };
}

const flightsText = (s: Snapshot, e: Estate): string =>
  e.flightsTotal === 0 ? t('no_flights') : t('flights_line', { flights: e.flightsTotal, tonnes: n1(e.tonnesTotal), town_tonnes: compact(s.breakdowns?.emissions.cumulative.tonnes ?? s.nature.cumulativeTonnes) });
const lastFlightText = (e: Estate): string => (e.lastFlight ? `turn ${e.lastFlight.turn}, month ${e.lastFlight.month + 1} of 3 (${tonnes(e.lastFlight.tonnes)})` : 'none yet');

export interface EstateCardModel {
  title: string; tag: string; band: string;
  /** The richest-household sentence (with counts). */
  topLine: string | null;
  netWorth: string;
  wealthParts: { label: string; value: number; text: string }[];
  income: { label: string; value: number; text: string }[];
  incomeTotal: string;
  jet: EstateRow[]; flights: string;
  pilot: PilotView | null;
  money: { label: string; month: string; last12: string }[]; moneyMonths: number;
  shortfall: string | null;
}

const INCOME_LABELS: [keyof HouseholdView['income'], string][] = [['dividends', 'Dividends'], ['wages', 'Wages'], ['rent', 'Rent'], ['interest', 'Interest'], ['benefits', 'Benefits']];

export function estateCard(s: Snapshot): EstateCardModel | null {
  const e = s.estate, h = estateHousehold(s);
  if (!e || !h) return null;
  const parts = s.breakdowns?.gini.byHousehold.find((x) => x.household === h.id)?.parts ?? [];
  const m = e.finance;
  const mrow = (label: string, k: keyof Estate['finance']['month'], sign = 1): EstateCardModel['money'][number] => ({ label, month: money(sign * m.month[k]), last12: money(sign * m.last12[k]) });
  return {
    title: t('estate_card_title'), tag: h.tag ?? t('home_label'), band: h.wealthBand?.label ?? '',
    topLine: topHouseholdLine(s),
    netWorth: money(h.netWorth),
    wealthParts: parts.map((p) => ({ label: t(`wealth_part_${p.id}`), value: p.value, text: money(p.value) })),
    income: INCOME_LABELS.map(([k, label]) => ({ label, value: h.income[k], text: money(h.income[k]) })).filter((r) => r.value > 0),
    incomeTotal: money(h.income.wages + h.income.dividends + h.income.rent + h.income.interest + h.income.benefits),
    jet: [
      { label: t('jet_label'), value: money(e.jet.value) },
      { label: 'Flew this month', value: e.jet.flyingThisMonth ? 'yes' : 'no' },
      { label: 'One flight', value: tonnes(e.flightTonnes) },
      { label: 'Last flight', value: lastFlightText(e) },
      { label: 'Home footprint', value: `${tonnes(e.householdEmissionsT)}/month` },
    ].map((r) => ({ label: r.label, value: r.value })),
    flights: flightsText(s, e),
    pilot: pilotView(s),
    money: [mrow('Dividends received', 'dividends'), mrow('Domestic wages paid', 'wagePaid'), mrow('Wealth tax paid', 'wealthTax'), mrow('Carbon charge paid (jet)', 'carbonCharge')],
    moneyMonths: m.months,
    shortfall: wealthTaxShortText(s),
  };
}

/** Inspector sections for the estate owner's household or person card. */
export function estateOwnerSections(s: Snapshot): EstateSection[] {
  const c = estateCard(s), e = s.estate;
  if (!c || !e) return [];
  const pilot = c.pilot;
  return [
    { heading: c.title, rows: [
      { label: 'Tag', value: c.tag }, ...(c.band ? [{ label: 'Wealth band', value: c.band }] : []),
      ...(e.pilot ? [{ label: 'Employs', value: `${personName(s, e.pilot.person)}, ${money(e.pilot.wage)}/month${pilot?.status === 'let_go' ? ' (let go)' : ''}`, hint: t('employs_line', { name: personName(s, e.pilot.person) }) }] : []),
    ] },
    { heading: t('jet_label'), rows: [{ label: 'Flights', value: String(e.flightsTotal) }, { label: 'CO2 flown', value: tonnes(e.tonnesTotal) }, { label: 'Last flight', value: lastFlightText(e) }, { label: 'Jet value', value: money(e.jet.value) }] },
    { heading: `Estate money, last ${plural(Math.max(1, e.finance.months), 'month', 'months')}`, rows: c.money.map((r) => ({ label: r.label.replace(' paid', '').replace(' received', ''), value: r.last12, hint: `Last month: ${r.month}` })) },
  ];
}

/** Inspector rows for the pilot (employer line, pay, status, unpaid months, the jet's flights). */
export function pilotSections(s: Snapshot, p: PersonView): EstateSection[] {
  const v = pilotView(s), e = s.estate;
  if (!v || !e || !p.employer) return [];
  return [{ heading: 'Pilot for the estate', rows: [
    ...v.rows,
    { label: 'Flights flown (estate jet)', value: String(e.flightsTotal) },
  ] }];
}

/** The line shown instead of a job for people who work for a household or for nobody on purpose (the estate owner). */
export const doesNotWorkText = (): string => t('does_not_work');
export const employerLine = (p: PersonView): string | null => (p.employer ? t('employer_line', { label: p.employer.label }) : null);

export interface GiniRow { household: number; label: string; tag: string | null; band: string | null; people: number; netWorth: string; perPerson: string; shareText: string; parts: string }
/** The Gini table's rows, richest first, with the tag, wealth band, what the net worth is made of, and the share as a count (dollars of the town's total). */
export function giniRows(s: Snapshot): GiniRow[] {
  const g = s.breakdowns?.gini;
  if (!g) return [];
  const total = g.byHousehold.reduce((a, h) => a + h.netWorth, 0);
  return [...g.byHousehold].sort((a, b) => b.netWorth - a.netWorth || a.household - b.household).map((r) => {
    const h = s.households.find((x) => x.id === r.household);
    return {
      household: r.household, label: h?.label ?? `Household ${r.household}`, tag: h?.tag ?? null, band: h?.wealthBand?.label ?? null, people: r.people,
      netWorth: money(r.netWorth), perPerson: money(r.people ? r.netWorth / r.people : 0), shareText: `${money(r.netWorth)} of ${money(total)}`,
      parts: (r.parts ?? []).map((p) => `${t(`wealth_part_${p.id}`)} ${money(p.value)}`).join(', '),
    };
  });
}
