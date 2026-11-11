import type { EntityKind, FirmOwner, FirmView, HomeView, HouseholdView, InfraKind, InspectorSection, PersonView, Snapshot } from '../contract';
import type { DraftItem, MapLayout, Placed, Scenery, Selection, Tile } from '../store';
import { fossilShareOf, occupant, pendingAt, sameTile, tileAt } from '../store';
import { incomeTotal } from '../store/sources';
import { forestStateOf, forecastView, weatherOf } from '../store/weather';
import { classLabel, fill, ownerText, ownerWord, text } from '../store/catalog';
import { entityKind, entityLabel, entityOfTile, hasRole, infraLabel, sectionsOf, stateLabel } from '../store/entities';
import { energyText, plantAt, plantRows, unitHelp } from './energy';
import { debtBacking, forestTiles, legitimacyBacking, pctWith, stockOf } from './backing';
import { compact, money, tonnes, turnsUntil } from './format';
import { doesNotWorkText, employerLine, estateOwnerSections, isEstatePerson, isPilot, pilotSections } from './estate';
import { SECTOR_WORD } from './industry';

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

export const INFRA_LABEL = infraLabel;
const AGE_LABEL = { child: 'Child', adult: 'Adult', retired: 'Retired' } as const;
const OWNER_TEXT = (o: FirmOwner, s: Snapshot) =>
  o.kind === 'household' ? `${s.households.find((h) => h.id === o.id)?.label ?? `${ownerWord('household')} ${o.id}`}` : ownerText(o.kind);

/** Finished buildings of a kind (summary counts; the tiles are the truth). */
export const builtCount = (snap: Snapshot | null, kind: InfraKind): number =>
  !snap ? 0 : kind === 'public_housing' ? snap.buildings.publicHousing : kind === 'fossil_plant' ? snap.buildings.fossilPlant : kind === 'renewable_plant' ? snap.buildings.renewablePlant
    : kind === 'road' ? snap.buildings.road ?? snap.map.tiles.filter((t) => t.kind === 'road').length : snap.map.tiles.filter((t) => t.kind === kind).length; // the summary counts have no fire stations (contract v1.2 adds none): count the tiles

const at = (t: Tile) => `Tile (${t.x}, ${t.y})`;

// ---- Entity resolution and the section builders (Platform cleanup phase 2: the registry, content/entities.json, decides which sections an entity gets) ----

/** What a section builder may read. A builder returns no rows when its subject is missing. */
interface Ctx {
  s: Snapshot;
  entity: EntityKind | undefined;
  person?: PersonView;
  household?: HouseholdView;
  home?: HomeView;
  firm?: FirmView;
  tile?: Tile;
}

/** The registry entity of a person: the pilot and the estate's own people are variants with their own sections. */
const personEntity = (s: Snapshot, p: PersonView): EntityKind | undefined =>
  entityKind(isPilot(s, p) ? 'pilot' : isEstatePerson(s, p) && !p.employer ? 'estate_owner' : 'person');
const householdEntity = (s: Snapshot, h: HouseholdView): EntityKind | undefined => entityKind(s.estate?.household === h.id ? 'estate_household' : 'household');

function personWork({ s, person: p, entity }: Ctx): Section[] {
  if (!p) return [];
  const firmName = p.job ? s.firms.find((f) => f.id === p.job!.firm)?.name ?? `Firm ${p.job.firm}` : null;
  const jobText = firmName ?? employerLine(p) ?? (entity?.id === 'estate_owner' ? doesNotWorkText() : p.ageBand === 'child' ? 'child, not working' : p.ageBand === 'retired' ? 'retired' : p.inLabourForce ? 'looking for work' : 'not in the labour force');
  return [{ heading: 'Work', rows: [
    { label: 'Job', value: jobText }, { label: 'Wage', value: p.job || p.employer ? `${money(p.wage)}/month` : 'none' },
    { label: 'Skill', value: `${p.skill.toFixed(2)} (0 to 1)` }, ...(p.jobGuarantee ? [{ label: 'Job guarantee', value: 'yes' }] : [])] }];
}
const personCommute = ({ person: p }: Ctx): Section[] => !p ? [] : [{ heading: 'Commute', rows: p.job || p.employer ? [
  { label: 'Distance', value: `${compact(p.commuteTiles)} ${p.commuteTiles === 1 ? 'tile' : 'tiles'}` }, { label: 'Time', value: `${Math.round(p.commuteMinutes)} minutes each way` }, ...(p.employer ? [{ label: 'To', value: 'the estate hangar' }] : [])]
  : [{ label: 'Commute', value: 'none (no job)' }] }];
function personLife({ s, person: p, household: h }: Ctx): Section[] {
  if (!p) return [];
  const home = h?.home != null ? s.homes.find((x) => x.id === h.home) : undefined;
  return [{ heading: 'Life', rows: [
    { label: 'Wellbeing', value: p.wellbeing.toFixed(2), hint: '0 to 1' },
    { label: 'Home', value: h?.home != null ? `home ${h.home}${home ? (home.owner === 'public' ? ' (public)' : ' (private)') : ''}` : 'UNHOUSED' },
    ...(home ? [{ label: 'Rent', value: `${money(home.rent)}/month` }] : [])] }];
}

function householdHome({ s, household: h, home }: Ctx): Section[] {
  if (!h) return [];
  const owns = s.homes.filter((x) => x.owner === h.id && x.household !== h.id);
  const theHome = home ?? s.homes.find((x) => x.id === h.home);
  const owner = theHome ? (theHome.owner === 'public' ? 'public housing' : theHome.owner === h.id ? 'this household' : s.households.find((x) => x.id === theHome.owner)?.label ?? 'a landlord') : 'none';
  return [{ heading: 'Home', rows: [
    { label: 'Lives in', value: theHome ? `home ${theHome.id}` : 'UNHOUSED (no home)' },
    { label: 'Owner', value: owner },
    ...(theHome && theHome.owner !== h.id ? [{ label: 'Rent', value: `${money(theHome.rent)}/month` }] : []),
    ...(owns.length ? [{ label: 'Lets out', value: `${owns.length} ${owns.length === 1 ? 'home' : 'homes'}` }] : []),
    { label: 'Rent arrears', value: `${h.arrearsMonths} ${h.arrearsMonths === 1 ? 'month' : 'months'}` }] }];
}
const householdWealth = ({ household: h }: Ctx): Section[] => !h ? [] : [{ heading: 'Wealth', rows: [
  { label: 'Savings', value: money(h.savings) }, { label: 'Debt', value: money(h.debt) }, { label: 'Net worth', value: money(h.netWorth) }] }];
const householdMoney = ({ household: h }: Ctx): Section[] => !h ? [] : [{ heading: 'Money (per month)', rows: [
  { label: 'Income, all sources', value: money(incomeTotal(h.income.sources)) },
  { label: 'Tax paid', value: money(h.income.taxPaid) }, { label: 'Rent paid', value: money(h.income.rentPaid) },
  { label: 'Loan payments', value: money(h.income.loanPayments) }, { label: 'Spending', value: money(h.income.spending) }] }];
const householdLife = ({ household: h }: Ctx): Section[] => !h ? [] : [{ heading: 'Life and politics', rows: [{ label: 'Wellbeing', value: h.wellbeing.toFixed(2) }, { label: 'Political power', value: h.politicalPower.toFixed(2) }] }];

const firmFirm = ({ s, firm: f }: Ctx): Section[] => !f ? [] : [{ heading: 'Firm', rows: [{ label: 'Owner', value: OWNER_TEXT(f.owner, s) }, { label: 'Workers', value: compact(f.workers) }, { label: 'Wage', value: money(f.wage) }, { label: 'Price', value: money(f.price) }] }];
const firmOutput = ({ firm: f }: Ctx): Section[] => !f ? [] : [{ heading: 'Output', rows: [{ label: 'Output', value: compact(f.output) }, { label: 'Emissions', value: `${tonnes(f.emissions)} per month` }, { label: 'Green tech', value: f.greenTech.toFixed(2) }] }];
const firmBooks = ({ firm: f }: Ctx): Section[] => !f ? [] : [{ heading: 'Books (per month)', rows: [{ label: 'Revenue', value: money(f.revenue) }, { label: 'Profit', value: money(f.profit) }, { label: 'Capital', value: money(f.capital) }, { label: 'Debt', value: money(f.debt) }] }];

const bankBalance = ({ s }: Ctx): Section[] => [{ heading: 'Balance sheet', rows: [
  { label: 'Status', value: s.bank.failed ? 'Failed' : 'Operating' }, { label: 'Equity', value: money(s.bank.equity) }, { label: 'Loans', value: money(s.bank.loans) },
  { label: 'Deposits', value: money(s.bank.deposits) }, { label: 'Default rate', value: pctWith(s.bank.defaultRate, `on ${money(s.bank.loans)} of loans`, 1) }] }];
const governmentState = ({ s }: Ctx): Section[] => [{ heading: 'Government', rows: [
  { label: 'Legitimacy', value: pctWith(s.government.legitimacy, legitimacyBacking(s), 1) }, { label: 'Cash', value: money(s.government.cash) }, { label: 'Debt', value: money(s.government.debt) },
  { label: 'Debt / GDP', value: pctWith(s.government.debtToGdp, debtBacking(s), 1) }, { label: 'Receipts', value: `${money(s.government.receipts)} per month` }, { label: 'Spending', value: `${money(s.government.spending)} per month` }] }];
const governmentPolitics = ({ s }: Ctx): Section[] => [{ heading: 'Politics', rows: [{ label: 'Net lobbying pressure', value: `${s.netLobby.toFixed(2)} (0 none, 1 full)` }, { label: 'Union density', value: `${s.unionDensity.toFixed(2)} of workers` },
  { label: 'Political capital', value: `${Math.round(s.pc.current)} of ${s.pc.cap}` }] }];

const housingTown = ({ s }: Ctx): Section[] => [{ heading: 'Housing (whole town)', rows: [
  { label: 'Homes in town', value: compact(s.housing.homes) }, { label: 'Unhoused households', value: compact(s.housing.unhousedHouseholds) },
  { label: 'Public rent', value: money(s.housing.rentPublic) }, { label: 'Private rent', value: money(s.housing.rentPrivate) },
  { label: 'Occupancy', value: pctWith(s.housing.occupancy, `${s.homes.filter((h) => h.household !== null).length} of ${s.housing.homes} homes occupied`) }] }];
const fireCover = ({ s }: Ctx): Section[] => [{ heading: 'Fire cover', rows: [
  { label: 'Covers', value: fill(text('inspect', 'fire_station_covers'), { radius: weatherOf(s)?.coverageRadius ?? 0 }) }, { label: 'Effect', value: text('inspect', 'fire_station_effect') },
  { label: 'Fire risk now', value: forecastView(s)?.fireWord ?? 'unknown' }] }];
function plantThis({ s, tile }: Ctx): Section[] {
  const pl = tile ? plantAt(s, tile) : undefined;
  const t = tile ? tileAt(s, tile) : undefined;
  return [{ heading: 'This plant', rows: [
    ...(pl ? [{ label: 'Capacity', value: pl.capacityText, hint: unitHelp(s.energy) }, { label: 'Share of the grid', value: pctWith(pl.outputShare, `${pl.capacityText} of ${energyText(plantRows(s.energy).reduce((a, r) => a + r.capacity, 0), s.energy)}`) }] : []),
    { label: 'Emits', value: `${tonnes(pl?.emissions ?? t?.emissionsMonthly ?? 0)} per month` }] }];
}
const energyTown = ({ s }: Ctx): Section[] => [{ heading: 'Energy (whole town)', rows: [
  { label: 'Fossil share of capacity', value: pctWith(fossilShareOf(s.energy), `${s.energy.supply.filter((x) => x.kind === 'fossil').length} of ${s.energy.supply.length} plants`) }, { label: 'Renewable share', value: pctWith(s.energy.renewableShare, `${s.energy.supply.filter((x) => x.kind === 'renewable').length} of ${s.energy.supply.length} plants`) },
  { label: 'Served', value: pctWith(s.energy.served, `supply ${energyText(s.energy.supply.reduce((a, x) => a + x.capacity, 0), s.energy)}, demand ${energyText(s.energy.demand.total, s.energy)}`) }, { label: 'Grid intensity', value: `${s.energy.intensity.toFixed(2)} t per ${s.energy.unit || 'MWh'}` }] }];
/** Generic: any emitter standing on a tile reports the tile's monthly emissions. */
const emissionsSection = ({ s, tile }: Ctx): Section[] => [{ heading: 'Emissions', rows: [{ label: 'Emits', value: `${tonnes((tile ? tileAt(s, tile)?.emissionsMonthly : undefined) ?? 0)} per month` }] }];

const STATE_WORD = { standing: 'Standing', burning: 'BURNING', regrowing: 'Regrowing after a fire' } as const;
function forestSection({ s: snap, tile }: Ctx): Section[] {
  if (!tile) return [];
  const st = forestStateOf(snap, tile) ?? 'standing';
  const fire = weatherOf(snap)?.fires.find((f) => f.tile.x === tile.x && f.tile.y === tile.y);
  const fc = forecastView(snap);
  return [{ heading: 'Forest', rows: [{ label: 'This tile', value: STATE_WORD[st] }, ...(fire ? [{ label: 'Fire ends in', value: `about ${fire.monthsLeft} ${fire.monthsLeft === 1 ? 'month' : 'months'}` }] : []),
    { label: 'Forest stock', value: pctWith(snap.nature.forest, stockOf(snap.nature.forestStock, snap.nature.forest) ?? forestTiles(snap)) }, { label: 'Absorbs', value: `${tonnes(snap.nature.sinkMonthly)} per month` },
    { label: 'Net emissions', value: `${tonnes(snap.nature.netEmissionsMonthly)} per month` }, ...(fc ? [{ label: 'Fire risk', value: fc.fireWord }] : [])] }];
}
const commonsSection = ({ s: snap }: Ctx): Section[] => [{ heading: 'Commons', rows: [
  { label: 'Health', value: pctWith(snap.nature.commons, stockOf(snap.nature.commonsStock, snap.nature.commons) ?? 'stock n/a') }, { label: 'Access', value: snap.nature.commonsAccess },
  { label: 'Harvest policy', value: String(snap.policy.commons_harvest.nominal) }] }];

/** One builder per section id of the registry (a closed set: `InspectorSection`). */
const SECTIONS: Record<InspectorSection, (c: Ctx) => Section[]> = {
  emissions: emissionsSection,
  person_work: personWork, person_commute: personCommute, person_life: personLife,
  household_home: householdHome, household_wealth: householdWealth, household_money: householdMoney, household_life: householdLife,
  estate_owner: ({ s }) => estateOwnerSections(s), pilot: ({ s, person }) => (person ? pilotSections(s, person) : []),
  firm_firm: firmFirm, firm_output: firmOutput, firm_books: firmBooks,
  bank_balance: bankBalance, government_state: governmentState, government_politics: governmentPolitics,
  housing_town: housingTown, fire_cover: fireCover, plant_this: plantThis, energy_town: energyTown, forest: forestSection, commons: commonsSection,
};
const sectionsFor = (c: Ctx): Section[] => sectionsOf(c.entity).flatMap((id) => SECTIONS[id](c));

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
    case 'government': return government(snap, '');
    case 'bank': return bank(snap, '');
    case 'tile': return resolveTile(snap, layout, sel, draft);
  }
}

function personModel(s: Snapshot, id: number): InspectModel | null {
  const p = s.people.find((x) => x.id === id);
  if (!p) return null;
  const h = s.households.find((x) => x.id === p.household);
  const c: Ctx = { s, entity: personEntity(s, p), person: p, ...(h ? { household: h } : {}) };
  return {
    kind: 'person', title: `${h?.label ?? 'Household'}, ${p.label}`, subtitle: `${h?.tag ? `${h.tag}. ` : ''}${AGE_LABEL[p.ageBand]}${h ? `. ${classLabel(h.class)} household` : ''}${h?.wealthBand ? `. ${h.wealthBand.label}` : ''}`,
    ...(h ? { household: { label: h.label, detail: `${h.members.length} people`, selection: { kind: 'household' as const, id: h.id } } } : {}),
    sections: sectionsFor(c),
  };
}

function householdModel(s: Snapshot, id: number, home?: HomeView): InspectModel | null {
  const h = s.households.find((x) => x.id === id);
  if (!h) return null;
  const inc = h.income;
  const parts: IncomePart[] = inc.sources.map((x) => ({ label: x.label, value: x.amount, text: money(x.amount) }));
  const members: PersonView[] = h.members.flatMap((m) => s.people.filter((p) => p.id === m));
  const theHome = home ?? s.homes.find((x) => x.id === h.home);
  return {
    kind: 'household', title: h.label, subtitle: `${h.tag ? `${h.tag}. ` : ''}${h.wealthBand ? `${h.wealthBand.label}. ` : ''}${classLabel(h.class)} household. ${theHome ? at(theHome.tile) : 'UNHOUSED: no home'}`, income: parts,
    members: members.map((p) => ({ label: p.label, detail: p.job ? `works${p.wage > 0 ? `, ${money(p.wage)}/month` : ''}` : AGE_LABEL[p.ageBand].toLowerCase(), selection: { kind: 'person' as const, id: p.id } })),
    sections: sectionsFor({ s, entity: householdEntity(s, h), household: h, ...(home ? { home } : {}) }),
  };
}

function vacantHome(s: Snapshot, id: number): InspectModel | null {
  const home = s.homes.find((h) => h.id === id);
  if (!home) return null;
  const owner = home.owner === 'public' ? 'public housing' : s.households.find((h) => h.id === home.owner)?.label ?? 'a landlord';
  const tileEntity = entityOfTile(tileAt(s, home.tile)?.kind ?? 'home');
  return { kind: 'home', title: `${stateLabel('home', 'vacant') ?? entityLabel('home')} ${home.id}`, subtitle: at(home.tile), sections: [{ heading: 'Home', rows: [{ label: 'Owner', value: owner }, { label: 'Rent', value: `${money(home.rent)}/month` }] }],
    ...(tileEntity?.infraKind && hasRole(tileEntity, 'buildable') ? { removable: { kind: tileEntity.infraKind, tile: { ...home.tile }, already: false } } : {}) };
}

/** Inspector content for a tile (hover, or a selected tile with no entity of its own). */
export function resolveTile(snap: Snapshot, layout: MapLayout, tile: Tile, draft: DraftItem[]): InspectModel {
  const o = occupant(layout, tile);
  const where = `${at(tile)}.`;
  const queued = o ? null : pendingAt(layout, tile);
  const q = queued === null ? undefined : snap.pending.find((x) => x.id === queued);
  if (q && q.command.type === 'place') {
    const left = turnsUntil(q.effectiveTurn, snap.turn);
    return { kind: 'planned', title: `Under construction: ${INFRA_LABEL(q.command.kind)}`, subtitle: where, sections: [{ heading: 'Queued', rows: [
      { label: 'Takes effect', value: left === 1 ? 'next turn' : `in ${left} turns` }, { label: 'PC paid', value: String(Math.round(q.pcCost * 10) / 10) }] }],
      note: text('inspect', 'planned_queued') };
  }
  const draftHere = draft.find((d) => sameTile(d.command.type === 'set_policy' || d.command.type === 'switch_system' ? { x: -1, y: -1 } : d.command.tile, tile));
  if (!o) {
    const t = tileAt(snap, tile);
    if (t?.kind === 'road') return roadModel(where, tile, draft);
    if (draftHere && draftHere.command.type === 'place') {
      return { kind: 'planned', title: `Planned: ${INFRA_LABEL(draftHere.command.kind)}`, subtitle: where, sections: [{ heading: 'Plan', rows: [
        { label: 'Takes effect', value: draftHere.preview.leadTime <= 1 ? 'next turn' : `in ${draftHere.preview.leadTime} turns` }, { label: 'PC', value: String(Math.round(draftHere.preview.pc * 10) / 10) }] }],
        note: text('inspect', 'planned_draft') };
    }
    return { kind: 'empty', title: entityLabel('empty'), subtitle: where, sections: [], note: text('inspect', 'empty') };
  }
  if (!('label' in o)) return building(snap, o as Placed, where, draft);
  const sc = o as Scenery;
  switch (sc.kind) {
    case 'home': return (sc.ref !== undefined ? resolveSelection(snap, layout, { kind: 'home', id: sc.ref }, draft) : null) ?? { kind: 'home', title: sc.label, subtitle: where, sections: [] };
    case 'firm': return firm(snap, sc.ref ?? -1, where) ?? { kind: 'firm', title: sc.label, subtitle: where, sections: [] };
    case 'bank': return bank(snap, where);
    case 'government': return government(snap, where);
    case 'forest': return forestModel(snap, tile, where);
    case 'commons': return { kind: 'commons', title: entityLabel('commons'), subtitle: where, sections: sectionsFor({ s: snap, entity: entityKind('commons') }), note: text('inspect', 'commons') };
  }
}

function forestModel(snap: Snapshot, tile: Tile, where: string): InspectModel {
  const st = forestStateOf(snap, tile) ?? 'standing';
  return { kind: 'forest', title: stateLabel('forest', `title_${st}`) ?? entityLabel('forest'), subtitle: where, sections: sectionsFor({ s: snap, entity: entityKind('forest'), tile }),
    note: text('inspect', st === 'burning' ? 'forest_burning' : st === 'regrowing' ? 'forest_regrowing' : 'forest_standing') };
}

function roadModel(where: string, tile: Tile, draft: DraftItem[]): InspectModel {
  const road = entityKind('road');
  const already = draft.some((d) => d.command.type === 'remove' && sameTile(d.command.tile, tile));
  return { kind: 'road', title: entityLabel('road'), subtitle: where, sections: [], ...(road?.infraKind ? { removable: { kind: road.infraKind, tile: { ...tile }, already } } : {}), note: text('inspect', 'road') };
}

function firm(s: Snapshot, id: number, where: string): InspectModel | null {
  const f = s.firms.find((x) => x.id === id);
  if (!f) return null;
  if (!f.active) return { kind: 'firm', title: f.name, subtitle: where, sections: [], note: text('inspect', 'firm_inactive') };
  return { kind: 'firm', title: f.name, subtitle: `${SECTOR_WORD[f.sector]} ${entityLabel('firm').toLowerCase()}. ${where}`, sections: sectionsFor({ s, entity: entityKind('firm'), firm: f }) };
}

const bank = (s: Snapshot, where: string): InspectModel => ({ kind: 'bank', title: entityLabel('bank'), subtitle: s.bank.failed ? `FAILED. ${where}` : where, sections: sectionsFor({ s, entity: entityKind('bank') }) });

const government = (s: Snapshot, where: string): InspectModel => ({ kind: 'government', title: entityLabel('government'), subtitle: where, sections: sectionsFor({ s, entity: entityKind('government') }) });

/** A finished facility the player built (any buildable entity of the registry, including one added only in content). */
function building(s: Snapshot, p: Placed, where: string, draft: DraftItem[]): InspectModel {
  const e = entityOfTile(p.kind);
  const already = draft.some((d) => d.command.type === 'remove' && sameTile(d.command.tile, p));
  return {
    kind: 'building', title: e?.label ?? p.kind, subtitle: `Built. ${where}`, sections: sectionsFor({ s, entity: e, tile: { x: p.x, y: p.y } }),
    ...(e?.infraKind && hasRole(e, 'buildable') ? { removable: { kind: e.infraKind, tile: { x: p.x, y: p.y }, already } } : {}),
    note: text('inspect', 'building'),
  };
}
