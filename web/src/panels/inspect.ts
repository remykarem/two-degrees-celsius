import type { ClassKey, FirmOwner, InfraKind, PersonView, Snapshot } from '../contract';
import type { DraftItem, MapLayout, Placed, Scenery, Selection, Tile } from '../store';
import { occupant, pendingAt, sameTile, tileAt } from '../store';
import { compact, money, pct, tonnes, turnsUntil } from './format';

export interface Row { label: string; value: string; hint?: string }
export interface Section { heading: string; rows: Row[] }
export interface IncomePart { label: string; value: number; text: string }
/** A clickable line in a household card. */
export interface Link { label: string; detail: string; selection: Selection }

export interface InspectModel {
  kind: 'person' | 'household' | 'home' | 'firm' | 'bank' | 'government' | 'forest' | 'commons' | 'building' | 'road' | 'planned' | 'empty';
  title: string;
  subtitle: string;
  sections: Section[];
  income?: IncomePart[];
  /** People of a household (opens the person card). */
  members?: Link[];
  /** Person card: the household they belong to. */
  household?: Link;
  /** Set for a finished, removable building or road: the tile it stands on. */
  removable?: { kind: InfraKind; tile: Tile; already: boolean };
  note?: string;
}

export const CLASS_LABEL: Record<ClassKey, string> = { capitalist: 'Capitalist', rentier: 'Rentier (landlord)', manager: 'Manager', worker: 'Worker', dependent: 'Dependent' };
export const INFRA_LABEL: Record<InfraKind, string> = { public_housing: 'Public housing', fossil_plant: 'Fossil plant', renewable_plant: 'Renewable plant', road: 'Road' };
const AGE_LABEL = { child: 'Child', adult: 'Adult', retired: 'Retired' } as const;
const OWNER_TEXT = (o: FirmOwner, s: Snapshot) =>
  o.kind === 'household' ? `${s.households.find((h) => h.id === o.id)?.label ?? `Household ${o.id}`}` : o.kind === 'workers' ? 'Its workers' : o.kind === 'state' ? 'The state' : 'The bank';

/** Finished buildings of a kind (summary counts; the tiles are the truth). */
export const builtCount = (snap: Snapshot | null, kind: InfraKind): number =>
  !snap ? 0 : kind === 'public_housing' ? snap.buildings.publicHousing : kind === 'fossil_plant' ? snap.buildings.fossilPlant : kind === 'renewable_plant' ? snap.buildings.renewablePlant : snap.buildings.road ?? snap.map.tiles.filter((t) => t.kind === 'road').length;

const at = (t: Tile) => `Tile (${t.x}, ${t.y})`;

/** Inspector content for a selection (a person, household, home, firm or tile). */
export function resolveSelection(snap: Snapshot, layout: MapLayout, sel: Selection, draft: DraftItem[]): InspectModel | null {
  switch (sel.kind) {
    case 'person': return personModel(snap, sel.id);
    case 'household': return householdModel(snap, sel.id);
    case 'home': {
      const home = snap.homes.find((h) => h.id === sel.id);
      if (!home) return null;
      return home.household !== null ? householdModel(snap, home.household, home) : vacantHome(snap, home.id);
    }
    case 'firm': {
      const f = snap.firms.find((x) => x.id === sel.id);
      const t = snap.map.tiles.find((x) => x.kind === 'firm' && x.ref === sel.id);
      return f ? firm(snap, f.id, t ? at(t) : '') : null;
    }
    case 'tile': return resolveTile(snap, layout, sel, draft);
  }
}

function personModel(s: Snapshot, id: number): InspectModel | null {
  const p = s.people.find((x) => x.id === id);
  if (!p) return null;
  const h = s.households.find((x) => x.id === p.household);
  const firmName = p.job ? s.firms.find((f) => f.id === p.job!.firm)?.name ?? `Firm ${p.job.firm}` : null;
  const works = p.ageBand === 'adult' && (p.job !== null || h?.class === 'worker' || h?.class === 'manager' || h?.class === 'dependent');
  return {
    kind: 'person', title: `${h?.label ?? 'Household'}, ${p.label}`, subtitle: `${AGE_LABEL[p.ageBand]}. ${h ? CLASS_LABEL[h.class] : ''} household`,
    ...(h ? { household: { label: h.label, detail: `${h.members.length} people`, selection: { kind: 'household' as const, id: h.id } } } : {}),
    sections: [
      { heading: 'Work', rows: [
        { label: 'Job', value: firmName ?? (works ? 'looking for work' : p.ageBand === 'child' ? 'child' : 'not in the labour force') },
        { label: 'Wage', value: p.job ? `${money(p.wage)}/month` : 'none' }, { label: 'Skill', value: pct(p.skill) }] },
      { heading: 'Commute', rows: [
        { label: 'Distance', value: p.job ? `${compact(p.commuteTiles)} tiles` : 'none' }, { label: 'Time', value: p.job ? `${Math.round(p.commuteMinutes)} minutes` : 'none' }] },
      { heading: 'Life', rows: [{ label: 'Wellbeing', value: p.wellbeing.toFixed(2) }, { label: 'Home', value: h?.home != null ? `home ${h.home}` : 'no home' }] },
    ],
  };
}

function householdModel(s: Snapshot, id: number, home?: Snapshot['homes'][number]): InspectModel | null {
  const h = s.households.find((x) => x.id === id);
  if (!h) return null;
  const inc = h.income;
  const parts: IncomePart[] = [
    { label: 'Wages', value: inc.wages, text: money(inc.wages) }, { label: 'Dividends', value: inc.dividends, text: money(inc.dividends) },
    { label: 'Rent', value: inc.rent, text: money(inc.rent) }, { label: 'Interest', value: inc.interest, text: money(inc.interest) },
    { label: 'Benefits', value: inc.benefits, text: money(inc.benefits) },
  ];
  const members: PersonView[] = h.members.flatMap((m) => s.people.filter((p) => p.id === m));
  const homeId = home?.id ?? h.home;
  const theHome = home ?? s.homes.find((x) => x.id === homeId);
  return {
    kind: 'household', title: h.label, subtitle: `${CLASS_LABEL[h.class]} household. ${theHome ? at(theHome.tile) : 'No home'}`, income: parts,
    members: members.map((p) => ({ label: p.label, detail: p.job ? 'works' : AGE_LABEL[p.ageBand].toLowerCase(), selection: { kind: 'person' as const, id: p.id } })),
    sections: [
      { heading: 'Home', rows: [
        { label: 'Lives in', value: theHome ? `home ${theHome.id}` : 'no home (unhoused)' },
        { label: 'Owner', value: theHome ? (theHome.owner === 'public' ? 'public housing' : theHome.owner === h.id ? 'this household' : s.households.find((x) => x.id === theHome.owner)?.label ?? 'a landlord') : 'none' },
        { label: 'Months behind on rent', value: String(h.arrearsMonths) }] },
      { heading: 'Money (per month)', rows: [
        { label: 'Tax paid', value: money(inc.taxPaid) }, { label: 'Rent paid', value: money(inc.rentPaid) },
        { label: 'Loan payments', value: money(inc.loanPayments) }, { label: 'Spending', value: money(inc.spending) },
        { label: 'Savings', value: money(h.savings) }, { label: 'Debt', value: money(h.debt) }, { label: 'Net worth', value: money(h.netWorth) }] },
      { heading: 'Politics', rows: [{ label: 'Political power', value: h.politicalPower.toFixed(2) }] },
    ],
  };
}

function vacantHome(s: Snapshot, id: number): InspectModel | null {
  const home = s.homes.find((h) => h.id === id);
  if (!home) return null;
  const owner = home.owner === 'public' ? 'public housing' : s.households.find((h) => h.id === home.owner)?.label ?? 'a landlord';
  const tile = tileAt(s, home.tile);
  return { kind: 'home', title: `Vacant home ${home.id}`, subtitle: at(home.tile), sections: [{ heading: 'Home', rows: [{ label: 'Owner', value: owner }, { label: 'Rent', value: `${money(home.rent)}/month` }] }],
    ...(tile?.kind === 'public_housing' ? { removable: { kind: 'public_housing' as const, tile: { ...home.tile }, already: false } } : {}) };
}

/** Inspector content for a tile (hover, or a selected tile with no entity of its own). */
export function resolveTile(snap: Snapshot, layout: MapLayout, tile: Tile, draft: DraftItem[]): InspectModel {
  const o = occupant(layout, tile);
  const where = `${at(tile)}.`;
  const queued = o ? null : pendingAt(layout, tile);
  const q = queued === null ? undefined : snap.pending.find((x) => x.id === queued);
  if (q && q.command.type === 'place') {
    const left = turnsUntil(q.effectiveTurn, snap.turn);
    return { kind: 'planned', title: `Under construction: ${INFRA_LABEL[q.command.kind]}`, subtitle: where, sections: [{ heading: 'Queued', rows: [
      { label: 'Takes effect', value: left === 1 ? 'next turn' : `in ${left} turns` }, { label: 'PC paid', value: String(Math.round(q.pcCost * 10) / 10) }] }],
      note: 'Cancel it from the pending list to get half the political capital back.' };
  }
  const draftHere = draft.find((d) => sameTile(d.command.type === 'set_policy' || d.command.type === 'switch_system' ? { x: -1, y: -1 } : d.command.tile, tile));
  if (!o) {
    const t = tileAt(snap, tile);
    if (t?.kind === 'road') return roadModel(where, tile, draft);
    if (draftHere && draftHere.command.type === 'place') {
      return { kind: 'planned', title: `Planned: ${INFRA_LABEL[draftHere.command.kind]}`, subtitle: where, sections: [{ heading: 'Plan', rows: [
        { label: 'Takes effect', value: draftHere.preview.leadTime <= 1 ? 'next turn' : `in ${draftHere.preview.leadTime} turns` }, { label: 'PC', value: String(Math.round(draftHere.preview.pc * 10) / 10) }] }],
        note: 'Cancel it from the plan tray.' };
    }
    return { kind: 'empty', title: 'Empty tile', subtitle: where, sections: [], note: 'Pick a building or a road in the build palette to place one here.' };
  }
  if (!('label' in o)) return building(snap, o as Placed, where, draft);
  const sc = o as Scenery;
  switch (sc.kind) {
    case 'home': return (sc.ref !== undefined ? resolveSelection(snap, layout, { kind: 'home', id: sc.ref }, draft) : null) ?? { kind: 'home', title: sc.label, subtitle: where, sections: [] };
    case 'firm': return firm(snap, sc.ref ?? -1, where) ?? { kind: 'firm', title: sc.label, subtitle: where, sections: [] };
    case 'bank': return bank(snap, where);
    case 'government': return government(snap, where);
    case 'forest': return { kind: 'forest', title: 'Forest', subtitle: where, sections: [{ heading: 'Forest', rows: [
      { label: 'Stock', value: pct(snap.nature.forest) }, { label: 'Absorbs', value: `${tonnes(snap.nature.sinkMonthly)} per month` },
      { label: 'Net emissions', value: `${tonnes(snap.nature.netEmissionsMonthly)} per month` }] }], note: 'Forests are fixed scenery in this version.' };
    case 'commons': return { kind: 'commons', title: 'Commons', subtitle: where, sections: [{ heading: 'Commons', rows: [
      { label: 'Health', value: pct(snap.nature.commons) }, { label: 'Access', value: snap.nature.commonsAccess },
      { label: 'Harvest policy', value: String(snap.policy.commons_harvest.nominal) }] }], note: 'Change access and harvest in the policy drawer.' };
  }
}

function roadModel(where: string, tile: Tile, draft: DraftItem[]): InspectModel {
  const already = draft.some((d) => d.command.type === 'remove' && sameTile(d.command.tile, tile));
  return { kind: 'road', title: 'Road', subtitle: where, sections: [], removable: { kind: 'road', tile: { ...tile }, already }, note: 'A commute over a road tile takes less time than over open ground.' };
}

function firm(s: Snapshot, id: number, where: string): InspectModel | null {
  const f = s.firms.find((x) => x.id === id);
  if (!f) return null;
  if (!f.active) return { kind: 'firm', title: f.name, subtitle: where, sections: [], note: 'No firm has been founded here yet. Firms open when households move up in class.' };
  const sector = f.sector === 'food' ? 'Food' : f.sector === 'services' ? 'Services' : 'Goods';
  return { kind: 'firm', title: f.name, subtitle: `${sector} firm. ${where}`, sections: [
    { heading: 'Firm', rows: [{ label: 'Owner', value: OWNER_TEXT(f.owner, s) }, { label: 'Workers', value: compact(f.workers) }, { label: 'Wage', value: money(f.wage) }, { label: 'Price', value: money(f.price) }] },
    { heading: 'Output', rows: [{ label: 'Output', value: compact(f.output) }, { label: 'Emissions', value: `${tonnes(f.emissions)} per month` }, { label: 'Green tech', value: pct(f.greenTech) }] },
    { heading: 'Books (per month)', rows: [{ label: 'Revenue', value: money(f.revenue) }, { label: 'Profit', value: money(f.profit) }, { label: 'Capital', value: money(f.capital) }, { label: 'Debt', value: money(f.debt) }] },
  ] };
}

const bank = (s: Snapshot, where: string): InspectModel => ({ kind: 'bank', title: 'Bank', subtitle: s.bank.failed ? `FAILED. ${where}` : where, sections: [{ heading: 'Balance sheet', rows: [
  { label: 'Status', value: s.bank.failed ? 'Failed' : 'Operating' }, { label: 'Equity', value: money(s.bank.equity) }, { label: 'Loans', value: money(s.bank.loans) },
  { label: 'Deposits', value: money(s.bank.deposits) }, { label: 'Default rate', value: pct(s.bank.defaultRate, 1) }] }] });

const government = (s: Snapshot, where: string): InspectModel => ({ kind: 'government', title: 'Town hall', subtitle: where, sections: [{ heading: 'Government', rows: [
  { label: 'Legitimacy', value: pct(s.government.legitimacy, 1) }, { label: 'Cash', value: money(s.government.cash) }, { label: 'Debt', value: money(s.government.debt) },
  { label: 'Debt / GDP', value: pct(s.government.debtToGdp, 1) }, { label: 'Receipts', value: `${money(s.government.receipts)} per month` }, { label: 'Spending', value: `${money(s.government.spending)} per month` }] },
  { heading: 'Politics', rows: [{ label: 'Net lobbying pressure', value: pct(s.netLobby) }, { label: 'Union density', value: pct(s.unionDensity) },
  { label: 'Political capital', value: `${Math.round(s.pc.current)} of ${s.pc.cap}` }] }] });

function building(s: Snapshot, p: Placed, where: string, draft: DraftItem[]): InspectModel {
  const sections: Section[] = [];
  const t = tileAt(s, p);
  if (p.kind === 'public_housing') sections.push({ heading: 'Housing (whole town)', rows: [
    { label: 'Homes in town', value: compact(s.housing.homes) }, { label: 'Unhoused households', value: compact(s.housing.unhousedHouseholds) },
    { label: 'Public rent', value: money(s.housing.rentPublic) }, { label: 'Private rent', value: money(s.housing.rentPrivate) },
    { label: 'Occupancy', value: pct(s.housing.occupancy) }] });
  else sections.push({ heading: 'This plant', rows: [{ label: 'Emits', value: `${tonnes(t?.emissionsMonthly ?? 0)} per month` }] },
    { heading: 'Energy (whole town)', rows: [
    { label: 'Fossil capacity', value: compact(s.energy.fossilCap) }, { label: 'Renewable capacity', value: compact(s.energy.renewableCap) }, { label: 'Fossil share', value: pct(s.energy.fossilShare) }] });
  const already = draft.some((d) => d.command.type === 'remove' && sameTile(d.command.tile, p));
  return {
    kind: 'building', title: INFRA_LABEL[p.kind], subtitle: `Built. ${where}`, sections,
    removable: { kind: p.kind, tile: { x: p.x, y: p.y }, already },
    note: 'Removal is queued like any move and costs political capital.',
  };
}
