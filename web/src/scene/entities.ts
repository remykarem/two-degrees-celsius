import type { ClassKey, ForestState, FirmView, HouseholdView, PlantView, Snapshot } from '../contract';
import { entityLabel, entityOfInfra, entityOfTile, infraLabel, stateLabel, fill, type DraftItem, type MapLayout, type Tile } from '../store';
import type { EntityRef } from '../store/scene-bus';
import { tileKey, worldOf } from './coords';
import type { ModelId } from './models';
import { homeTint, mix, WALL } from './palette';
import { forestStateOf } from '../store/weather';
import { hasWork } from '../store/scene-estate';
import { ESTATE_STONE } from './estateModels';
import { modelFor } from './entityModel';

export type EntityState = 'normal' | 'pending' | 'draft';

/** Everything the renderer needs to draw one tile's entity. Pure data. */
export interface EntityDesc {
  key: string;
  ref: EntityRef;
  model: ModelId;
  tint: number;
  /** World position of the tile centre. */
  x: number;
  z: number;
  state: EntityState;
  turnsLeft: number;
  household?: HouseholdView;
  firm?: FirmView;
  plant?: PlantView;
  /** Residents' commute minutes (longest first), for the commute lens. */
  commuteMinutes?: number[];
  /** 0..1 forest or commons stock where relevant. */
  stock?: number;
  /** Forest tiles: standing, burning or regrowing (contract v1.2). */
  forestState?: ForestState;
}

const KIND_TINT: Record<string, number> = {
  firm: 0xd5d8de, bank: WALL, government: WALL, forest: 0x7fb871, commons: 0xb4d891,
  public_housing: mix(0xf2c14e, WALL, 0.5), fossil_plant: 0xa4a4a8, renewable_plant: 0xcfe3cc, road: 0xd8d2c6, fire_station: 0xeae3d6,
};

const DEFAULT_TINT = 0xd5d8de;
const tintOf = (kind: string): number => KIND_TINT[kind] ?? DEFAULT_TINT;
/** Scene model of a buildable kind (an `InfraKind`) or of any tile kind the registry knows. */
export const infraModel = (k: string): ModelId => { const e = entityOfInfra(k as never) ?? entityOfTile(k); return modelFor(e?.id ?? k, undefined, 0, 0); };
const INFRA_LABEL = (k: string): string => infraLabel(k);

/** Turns until a queued item takes effect: it fires at the start of the submit of turn `effectiveTurn`, so effectiveTurn == turn is "next turn". */
export function turnsLeft(effectiveTurn: number, turn: number): number {
  return Math.max(1, effectiveTurn - turn + 1);
}

/**
 * Layout + snapshot + draft -> entity descriptions. Scenery comes from the UI-owned layout; what each one looks like comes from
 * the snapshot (household class, firm sector and whether it is active, commons access, forest stock).
 */
export function planEntities(layout: MapLayout, snap: Snapshot | null, draft: DraftItem[] = []): EntityDesc[] {
  const n = layout.gridSize;
  const turn = snap?.turn ?? 1;
  const out: EntityDesc[] = [];
  const at = (t: Tile) => { const [x, , z] = worldOf(t, n); return { x, z }; };

  for (const sc of layout.scenery) {
    const base = { key: `s:${tileKey(sc)}`, x: at(sc).x, z: at(sc).z, state: 'normal' as const, turnsLeft: 0 };
    const ref = (label = sc.label): EntityRef => ({ kind: sc.kind, id: sc.ref ?? null, tile: { x: sc.x, y: sc.y }, label });
    switch (sc.kind) {
      case 'home': {
        const hh = snap ? snap.households.find((h) => h.id === snap.homes.find((x) => x.id === sc.ref)?.household) : undefined;
        const cls: ClassKey = hh?.class ?? 'worker';
        const mins = snap && hh ? snap.people.filter((q) => q.household === hh.id && hasWork(q)).map((q) => q.commuteMinutes).sort((a, b) => b - a) : undefined;
        const estate = !!hh && snap?.estate?.household === hh.id;
        out.push({ ...base, ref: ref(hh ? hh.label : sc.label), model: estate ? modelFor('estate_house', undefined, sc.x, sc.y) : modelFor('home', cls, sc.x, sc.y), tint: estate ? ESTATE_STONE : homeTint(cls), household: hh, commuteMinutes: mins });
        break;
      }
      case 'firm': {
        const f = snap?.firms.find((x) => x.id === sc.ref);
        const model = modelFor('firm', !f || !f.active ? 'vacant' : f.sector, sc.x, sc.y);
        const closed = stateLabel('firm', 'closed');
        const label = f && f.active ? f.name : f ? (closed ? fill(closed, { name: f.name }) : f.name) : sc.label;
        out.push({ ...base, ref: ref(label), model, tint: KIND_TINT.firm!, firm: f });
        break;
      }
      case 'bank': out.push({ ...base, ref: ref(), model: modelFor('bank', undefined, sc.x, sc.y), tint: KIND_TINT.bank! }); break;
      case 'government': out.push({ ...base, ref: ref(), model: modelFor('government', undefined, sc.x, sc.y), tint: KIND_TINT.government! }); break;
      case 'forest': {
        const st = forestStateOf(snap, sc);
        out.push({ ...base, ref: ref((st && stateLabel('forest', st)) || entityLabel('forest')), model: modelFor('forest', st ?? 'standing', sc.x, sc.y), tint: KIND_TINT.forest!, stock: snap?.nature.forest, ...(st ? { forestState: st } : {}) });
        break;
      }
      case 'commons': {
        const nat = snap?.nature;
        const model = modelFor('commons', !nat ? 'open' : nat.commons < 0.12 ? 'collapsed' : nat.commonsAccess, sc.x, sc.y);
        out.push({ ...base, ref: ref(), model, tint: KIND_TINT.commons!, stock: nat?.commons });
        break;
      }
    }
  }

  for (const p of layout.placed) {
    out.push({
      key: `p:${tileKey(p)}`, ...at(p), plant: snap?.plants.find((q) => q.tile.x === p.x && q.tile.y === p.y), state: 'normal', turnsLeft: 0, model: infraModel(p.kind), tint: tintOf(p.kind),
      ref: { kind: p.kind, id: null, tile: { x: p.x, y: p.y }, label: INFRA_LABEL(p.kind) },
    });
  }
  for (const q of snap?.pending ?? []) {
    const tile = layout.pendingTiles[q.id];
    if (q.command.type !== 'place' || !tile) continue;
    const tl = turnsLeft(q.effectiveTurn, turn);
    out.push({
      key: `q:${q.id}`, ...at(tile), state: 'pending', turnsLeft: tl, model: infraModel(q.command.kind), tint: tintOf(q.command.kind),
      ref: { kind: q.command.kind, id: null, tile: { x: tile.x, y: tile.y }, label: INFRA_LABEL(q.command.kind), pendingTurns: tl },
    });
  }

  for (const d of draft) {
    const c = d.command;
    if (c.type !== 'place') continue;
    out.push({
      key: `d:${tileKey(c.tile)}`, ...at(c.tile), state: 'draft', turnsLeft: Math.max(1, d.preview.leadTime), model: infraModel(c.kind), tint: tintOf(c.kind),
      ref: { kind: c.kind, id: null, tile: { ...c.tile }, label: `${INFRA_LABEL(c.kind)} (planned)`, pendingTurns: d.preview.leadTime },
    });
  }
  return out;
}

/** Entity occupying a tile (planned drafts excluded), for hover and selection reporting. */
export function entityAt(descs: EntityDesc[], t: Tile): EntityDesc | null {
  return descs.find((d) => d.state !== 'draft' && d.ref.tile.x === t.x && d.ref.tile.y === t.y) ?? null;
}

const pct = (v: number) => `${Math.round(v * 100)}%`;
const num = (v: number) => (Math.abs(v) >= 1000 ? `${(v / 1000).toFixed(1)}k` : String(Math.round(v * 10) / 10));

/** Short facts about an entity for the inspector (plain strings; the panels decide how to show them). */
export function factsFor(d: EntityDesc): { label: string; value: string }[] {
  const f: { label: string; value: string }[] = [];
  if (d.state !== 'normal') f.push({ label: d.state === 'pending' ? 'Under construction' : 'Planned', value: `${d.turnsLeft} turn${d.turnsLeft === 1 ? '' : 's'} left` });
  const h = d.household;
  if (h) f.push({ label: 'Class', value: h.class }, ...(h.tag ? [{ label: 'Tag', value: h.tag }] : []), { label: 'People', value: String(h.members.length) }, { label: 'Net worth', value: `$${num(h.netWorth)}` });
  const fm = d.firm;
  if (fm) f.push({ label: 'Sector', value: fm.sector }, { label: 'Status', value: fm.active ? 'trading' : 'closed' }, ...(fm.active ? [{ label: 'Workers', value: String(fm.workers) }, { label: 'Emissions', value: `${num(fm.emissions)} t/month` }] : []));
  const pl = d.plant;
  if (pl) f.push({ label: 'Emissions', value: `${num(pl.emissionsMonthly)} t/month` }, { label: 'Output', value: num(pl.output) });
  if (d.forestState) f.push({ label: 'Tile', value: d.forestState === 'burning' ? 'burning' : d.forestState === 'regrowing' ? 'regrowing after a fire' : 'standing' });
  if (d.stock !== undefined) f.push({ label: `${entityLabel(d.ref.kind)} stock`, value: pct(d.stock) });
  return f;
}
