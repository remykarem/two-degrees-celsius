import type { ClassKey, FirmView, HouseholdView, Snapshot } from '../contract';
import type { DraftItem, MapLayout, Tile } from '../store';
import type { EntityRef } from '../store/scene-bus';
import { tileKey, worldOf } from './coords';
import type { ModelId } from './models';
import { homeTint, mix, WALL } from './palette';

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
  /** 0..1 forest or commons stock where relevant. */
  stock?: number;
}

const KIND_TINT: Record<string, number> = {
  firm: 0xd5d8de, bank: WALL, government: WALL, forest: 0x7fb871, commons: 0xb4d891,
  public_housing: mix(0xf2c14e, WALL, 0.5), fossil_plant: 0xa4a4a8, renewable_plant: 0xcfe3cc,
};

type Building = 'public_housing' | 'fossil_plant' | 'renewable_plant';
const INFRA_MODEL: Record<Building, ModelId> = { public_housing: 'public_housing', fossil_plant: 'fossil_plant', renewable_plant: 'renewable_plant' };
const INFRA_LABEL: Record<Building, string> = { public_housing: 'Public housing', fossil_plant: 'Fossil plant', renewable_plant: 'Renewable plant' };

export const infraModel = (k: Building): ModelId => INFRA_MODEL[k];

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
        out.push({ ...base, ref: ref(hh ? hh.label : sc.label), model: `home_${cls}` as ModelId, tint: homeTint(cls), household: hh });
        break;
      }
      case 'firm': {
        const f = snap?.firms.find((x) => x.id === sc.ref);
        const model: ModelId = !f || !f.active ? 'firm_vacant' : f.sector === 'food' ? 'firm_food' : 'firm_goods'; // services share the goods model for now
        const label = f && f.active ? f.name : f ? `${f.name} (closed)` : sc.label;
        out.push({ ...base, ref: ref(label), model, tint: KIND_TINT.firm!, firm: f });
        break;
      }
      case 'bank': out.push({ ...base, ref: ref(), model: 'bank', tint: KIND_TINT.bank! }); break;
      case 'government': out.push({ ...base, ref: ref(), model: 'government', tint: KIND_TINT.government! }); break;
      case 'forest':
        out.push({ ...base, ref: ref(), model: `forest_${(sc.x * 7 + sc.y * 3) % 3}` as ModelId, tint: KIND_TINT.forest!, stock: snap?.nature.forest });
        break;
      case 'commons': {
        const nat = snap?.nature;
        const model: ModelId = !nat ? 'commons_open' : nat.commons < 0.12 ? 'commons_collapsed' : (`commons_${nat.commonsAccess === 'enclosed' ? 'enclosed' : nat.commonsAccess}` as ModelId);
        out.push({ ...base, ref: ref(), model, tint: KIND_TINT.commons!, stock: nat?.commons });
        break;
      }
    }
  }

  for (const p of layout.placed) {
    out.push({
      key: `p:${tileKey(p)}`, ...at(p), state: 'normal', turnsLeft: 0, model: INFRA_MODEL[p.kind], tint: KIND_TINT[p.kind]!,
      ref: { kind: p.kind, id: null, tile: { x: p.x, y: p.y }, label: INFRA_LABEL[p.kind] },
    });
  }
  for (const q of snap?.pending ?? []) {
    const tile = layout.pendingTiles[q.id];
    if (q.command.type !== 'place' || !tile || q.command.kind === 'road') continue;
    const tl = turnsLeft(q.effectiveTurn, turn);
    out.push({
      key: `q:${q.id}`, ...at(tile), state: 'pending', turnsLeft: tl, model: INFRA_MODEL[q.command.kind], tint: KIND_TINT[q.command.kind]!,
      ref: { kind: q.command.kind, id: null, tile: { x: tile.x, y: tile.y }, label: INFRA_LABEL[q.command.kind], pendingTurns: tl },
    });
  }

  for (const d of draft) {
    const c = d.command;
    if (c.type !== 'place' || c.kind === 'road') continue;
    out.push({
      key: `d:${tileKey(c.tile)}`, ...at(c.tile), state: 'draft', turnsLeft: Math.max(1, d.preview.leadTime), model: INFRA_MODEL[c.kind], tint: KIND_TINT[c.kind]!,
      ref: { kind: c.kind, id: null, tile: { ...c.tile }, label: `${INFRA_LABEL[c.kind]} (planned)`, pendingTurns: d.preview.leadTime },
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
  if (h) f.push({ label: 'Class', value: h.class }, { label: 'People', value: String(h.members.length) }, { label: 'Net worth', value: `$${num(h.netWorth)}` });
  const fm = d.firm;
  if (fm) f.push({ label: 'Sector', value: fm.sector }, { label: 'Status', value: fm.active ? 'trading' : 'closed' }, ...(fm.active ? [{ label: 'Workers', value: String(fm.workers) }, { label: 'Emissions', value: `${num(fm.emissions)} t/month` }] : []));
  if (d.stock !== undefined) f.push({ label: d.ref.kind === 'forest' ? 'Forest stock' : 'Commons stock', value: pct(d.stock) });
  return f;
}
