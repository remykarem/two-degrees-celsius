import type { ClassKey, FirmOwner, InfraKind, Snapshot } from '../contract';
import { turnsUntil } from './format';
import type { DraftItem, MapLayout, Placed, Scenery, Tile } from '../store';
import { occupant, pendingAt, sameTile } from '../store';
import { compact, money, pct, tonnes } from './format';

export interface Row { label: string; value: string; hint?: string }
export interface Section { heading: string; rows: Row[] }
export interface IncomePart { label: string; value: number; text: string }

export interface InspectModel {
  kind: 'household' | 'firm' | 'bank' | 'government' | 'forest' | 'commons' | 'building' | 'planned' | 'empty';
  title: string;
  subtitle: string;
  sections: Section[];
  income?: IncomePart[];
  /** Set for a finished, removable building. */
  removable?: { kind: InfraKind; already: boolean; reason: string | null };
  note?: string;
}

export const CLASS_LABEL: Record<ClassKey, string> = { capitalist: 'Capitalist', rentier: 'Rentier (landlord)', manager: 'Manager', worker: 'Worker', dependent: 'Dependent' };
export const INFRA_LABEL: Record<InfraKind, string> = { public_housing: 'Public housing', fossil_plant: 'Fossil plant', renewable_plant: 'Renewable plant' };
const OWNER_TEXT = (o: FirmOwner, s: Snapshot) =>
  o.kind === 'household' ? `${s.households.find((h) => h.id === o.id)?.label ?? `Household ${o.id}`}` : o.kind === 'workers' ? 'Its workers' : o.kind === 'state' ? 'The state' : 'The bank';

/** Finished buildings of a kind: the snapshot's count is the truth; the map just draws it. */
export const builtCount = (snap: Snapshot | null, kind: InfraKind): number =>
  !snap ? 0 : kind === 'public_housing' ? snap.buildings.publicHousing : kind === 'fossil_plant' ? snap.buildings.fossilPlant : snap.buildings.renewablePlant;

export function resolveInspect(snap: Snapshot, layout: MapLayout, tile: Tile, draft: DraftItem[]): InspectModel {
  const o = occupant(layout, tile);
  const at = `Tile (${tile.x}, ${tile.y}). Position is cosmetic: it has no effect on the economy.`;
  const queued = o ? null : pendingAt(layout, tile);
  const q = queued === null ? undefined : snap.pending.find((x) => x.id === queued);
  if (q && q.command.type === 'place') {
    const left = turnsUntil(q.effectiveTurn, snap.turn);
    return { kind: 'planned', title: `Under construction: ${INFRA_LABEL[q.command.kind]}`, subtitle: at, sections: [{ heading: 'Queued', rows: [
      { label: 'Takes effect', value: left === 1 ? 'next turn' : `in ${left} turns` }, { label: 'PC paid', value: String(Math.round(q.pcCost * 10) / 10) }] }],
      note: 'Cancel it from the pending list to get half the political capital back.' };
  }
  if (!o) {
    const planned = draft.find((d) => d.command.type === 'place' && d.command.tile && sameTile(d.command.tile, tile));
    if (planned && planned.command.type === 'place') {
      return { kind: 'planned', title: `Planned: ${INFRA_LABEL[planned.command.kind]}`, subtitle: at, sections: [{ heading: 'Plan', rows: [
        { label: 'Takes effect', value: planned.preview.leadTime <= 1 ? 'next turn' : `in ${planned.preview.leadTime} turns` }, { label: 'PC', value: String(Math.round(planned.preview.pc * 10) / 10) }] }],
        note: 'Cancel it from the plan tray.' };
    }
    return { kind: 'empty', title: 'Empty tile', subtitle: at, sections: [], note: 'Pick a building in the build palette to place one here.' };
  }
  if (!('label' in o)) return building(snap, layout, o as Placed, at, draft);
  const sc = o as Scenery;
  switch (sc.kind) {
    case 'home': return household(snap, sc, at);
    case 'firm': return firm(snap, sc, at);
    case 'bank': return bank(snap, at);
    case 'government': return government(snap, at);
    case 'forest': return { kind: 'forest', title: 'Forest', subtitle: at, sections: [{ heading: 'Forest', rows: [
      { label: 'Stock', value: pct(snap.nature.forest) }, { label: 'Absorbs', value: `${tonnes(snap.nature.sinkMonthly)} per month` },
      { label: 'Net emissions', value: `${tonnes(snap.nature.netEmissionsMonthly)} per month` }] }], note: 'Forests are fixed scenery in this version.' };
    case 'commons': return { kind: 'commons', title: 'Commons', subtitle: at, sections: [{ heading: 'Commons', rows: [
      { label: 'Health', value: pct(snap.nature.commons) }, { label: 'Access', value: snap.nature.commonsAccess },
      { label: 'Harvest policy', value: String(snap.policy.commons_harvest.nominal) }] }], note: 'Change access and harvest in the policy drawer.' };
  }
}

function household(s: Snapshot, sc: Scenery, at: string): InspectModel {
  const h = s.households.find((x) => x.id === sc.ref);
  if (!h) return { kind: 'household', title: sc.label, subtitle: at, sections: [] };
  const inc = h.income;
  const parts: IncomePart[] = [
    { label: 'Wages', value: inc.wages, text: money(inc.wages) }, { label: 'Dividends', value: inc.dividends, text: money(inc.dividends) },
    { label: 'Rent', value: inc.rent, text: money(inc.rent) }, { label: 'Interest', value: inc.interest, text: money(inc.interest) },
    { label: 'Benefits', value: inc.benefits, text: money(inc.benefits) },
  ];
  return { kind: 'household', title: h.label, subtitle: `${CLASS_LABEL[h.class]} cohort. ${at}`, income: parts, sections: [
    { heading: 'People', rows: [
      { label: 'People', value: compact(h.people) },
      { label: 'Employment', value: h.labourForce > 0 ? `${compact(h.employed)} of ${compact(h.labourForce)} (${pct(h.employed / h.labourForce)})` : 'no labour force' },
      { label: 'Housed', value: pct(h.housedShare) }, { label: 'Food needs met', value: pct(h.foodMet) }] },
    { heading: 'Money (per month)', rows: [
      { label: 'Tax paid', value: money(inc.taxPaid) }, { label: 'Rent paid', value: money(inc.rentPaid) },
      { label: 'Loan payments', value: money(inc.loanPayments) }, { label: 'Spending', value: money(inc.spending) },
      { label: 'Savings', value: money(h.savings) }, { label: 'Debt', value: money(h.debt) }, { label: 'Wealth per person', value: money(h.wealthPerCapita) }] },
    { heading: 'Life', rows: [
      { label: 'Wellbeing', value: h.wellbeing.toFixed(2) }, { label: 'Political power', value: h.politicalPower.toFixed(2) },
      { label: 'Footprint', value: tonnes(h.footprintTonnes) }] },
  ] };
}

function firm(s: Snapshot, sc: Scenery, at: string): InspectModel {
  const f = s.firms.find((x) => x.id === sc.ref);
  if (!f) return { kind: 'firm', title: sc.label, subtitle: at, sections: [] };
  if (!f.active) return { kind: 'firm', title: f.name, subtitle: at, sections: [], note: 'No firm has been founded here yet. Firms open when households move up in class.' };
  return { kind: 'firm', title: f.name, subtitle: `${f.sector === 'food' ? 'Food' : 'Goods'} firm. ${at}`, sections: [
    { heading: 'Firm', rows: [{ label: 'Owner', value: OWNER_TEXT(f.owner, s) }, { label: 'Workers', value: compact(f.workers) }, { label: 'Wage', value: money(f.wage) }, { label: 'Price', value: money(f.price) }] },
    { heading: 'Output', rows: [{ label: 'Output', value: compact(f.output) }, { label: 'Emissions', value: `${tonnes(f.emissions)} per month` }, { label: 'Green tech', value: pct(f.greenTech) }] },
    { heading: 'Books (per month)', rows: [{ label: 'Revenue', value: money(f.revenue) }, { label: 'Profit', value: money(f.profit) }, { label: 'Capital', value: money(f.capital) }, { label: 'Debt', value: money(f.debt) }] },
  ] };
}

const bank = (s: Snapshot, at: string): InspectModel => ({ kind: 'bank', title: 'Bank', subtitle: s.bank.failed ? `FAILED. ${at}` : at, sections: [{ heading: 'Balance sheet', rows: [
  { label: 'Status', value: s.bank.failed ? 'Failed' : 'Operating' }, { label: 'Equity', value: money(s.bank.equity) }, { label: 'Loans', value: money(s.bank.loans) },
  { label: 'Deposits', value: money(s.bank.deposits) }, { label: 'Default rate', value: pct(s.bank.defaultRate, 1) }] }] });

const government = (s: Snapshot, at: string): InspectModel => ({ kind: 'government', title: 'Town hall', subtitle: at, sections: [{ heading: 'Government', rows: [
  { label: 'Legitimacy', value: pct(s.government.legitimacy, 1) }, { label: 'Cash', value: money(s.government.cash) }, { label: 'Debt', value: money(s.government.debt) },
  { label: 'Debt / GDP', value: pct(s.government.debtToGdp, 1) }, { label: 'Receipts', value: `${money(s.government.receipts)} per month` }, { label: 'Spending', value: `${money(s.government.spending)} per month` }] },
  { heading: 'Politics', rows: [{ label: 'Net lobbying pressure', value: pct(s.netLobby) }, { label: 'Union density', value: pct(s.unionDensity) },
  { label: 'Political capital', value: `${Math.round(s.pc.current)} of ${s.pc.cap}` }] }] });

function building(s: Snapshot, _layout: MapLayout, p: Placed, at: string, draft: DraftItem[]): InspectModel {
  const built = true;
  const sections: Section[] = [];
  if (p.kind === 'public_housing') sections.push({ heading: 'Housing (whole town)', rows: [
    { label: 'Public capacity', value: compact(s.housing.publicCap) }, { label: 'Private capacity', value: compact(s.housing.privateCap) },
    { label: 'Public rent', value: money(s.housing.rentPublic) }, { label: 'Private rent', value: money(s.housing.rentPrivate) },
    { label: 'Occupancy', value: pct(s.housing.occupancy) }, { label: 'Housed', value: pct(s.housing.housedShare) }] });
  else sections.push({ heading: 'Energy (whole town)', rows: [
    { label: 'Fossil capacity', value: compact(s.energy.fossilCap) }, { label: 'Renewable capacity', value: compact(s.energy.renewableCap) }, { label: 'Fossil share', value: pct(s.energy.fossilShare) }] });
  const already = draft.some((d) => d.command.type === 'remove' && d.command.kind === p.kind);
  const count = builtCount(s, p.kind);
  return {
    kind: 'building', title: INFRA_LABEL[p.kind], subtitle: `Built. ${at}`, sections,
    removable: built ? { kind: p.kind, already, reason: count < 1 ? 'Nothing of that kind to remove' : null } : undefined,
    note: built ? 'Removal is queued like any move and costs political capital. The sim removes one of this kind; which tile is cosmetic.' : 'Cancel it from the pending list to get half the political capital back.',
  };
}
