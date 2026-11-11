import { emissionOf } from './sources';
import type { Estate, FrameInput, MonthFlows, PersonView, SimEvent, Snapshot, TileRef } from '../contract';

/**
 * Scene-facing selectors for the Town v7 estate (contract v1.9). Pure functions over wire data: no Three.js, no clock, no numbers
 * from the model. The scene draws what these return.
 */

export type EstateRole = 'drive' | 'pool' | 'garden';
export interface EstateTiles {
  estate: Estate;
  /** The house tile (the estate household's home). Null when the home is not on the map. */
  house: TileRef | null;
  /** What each grounds tile is drawn as. A role missing from the wire's grounds list is absent. */
  roles: Partial<Record<EstateRole, TileRef>>;
  grounds: TileRef[];
  runway: TileRef[];
  hangar: TileRef;
}

const same = (a: TileRef, b: TileRef) => a.x === b.x && a.y === b.y;
const dist = (a: TileRef, b: TileRef) => Math.abs(a.x - b.x) + Math.abs(a.y - b.y);

/**
 * The wire lists grounds tiles without a role. The scene draws the one nearest a road as the gravel drive and gate, the one nearest the
 * house (after that) as the pool lawn, and the rest as the garden. Ties break by y then x, so the result is deterministic.
 */
export function estateRoles(grounds: readonly TileRef[], house: TileRef | null, roads: readonly TileRef[], hangar: TileRef): Partial<Record<EstateRole, TileRef>> {
  const order = (a: TileRef, b: TileRef) => a.y - b.y || a.x - b.x;
  const left = [...grounds].sort(order);
  const out: Partial<Record<EstateRole, TileRef>> = {};
  const take = (role: EstateRole, score: (t: TileRef) => number) => {
    if (!left.length) return;
    let bi = 0;
    for (let i = 1; i < left.length; i++) if (score(left[i]!) < score(left[bi]!)) bi = i;
    out[role] = left.splice(bi, 1)[0]!;
  };
  const nearRoad = (t: TileRef) => (roads.length ? Math.min(...roads.map((r) => dist(r, t))) : dist(t, hangar));
  take('drive', nearRoad);
  take('pool', (t) => (house ? dist(t, house) : 0) * 10 + dist(t, hangar));
  take('garden', () => 0);
  return out;
}

/** The estate's tiles, or null when the town has no estate (an older package, or a town without one). */
export function estateTilesOf(snap: Snapshot | null): EstateTiles | null {
  const e = snap?.estate;
  if (!snap || !e) return null;
  const house = snap.homes.find((h) => h.id === e.home)?.tile ?? null;
  const roads = snap.map.tiles.filter((t) => t.kind === 'road').map((t) => ({ x: t.x, y: t.y }));
  return { estate: e, house: house ? { ...house } : null, roles: estateRoles(e.groundsTiles, house, roads, e.hangarTile), grounds: e.groundsTiles.map((t) => ({ ...t })), runway: e.runwayTiles.map((t) => ({ ...t })), hangar: { ...e.hangarTile } };
}

/** Is this tile one of the estate's own tiles (grounds or runway)? */
export const isEstateTile = (t: EstateTiles | null, p: TileRef): boolean => !!t && (t.grounds.some((g) => same(g, p)) || t.runway.some((r) => same(r, p)));

/** The estate household's id, or null. */
export const estateHousehold = (snap: Snapshot | null): number | null => snap?.estate?.household ?? null;

/** A `private_jet_flight` event in a month's events: tonnes of that flight, or null when the jet did not fly. */
export function jetFlightIn(events: readonly SimEvent[] | undefined): { tonnes: number } | null {
  let t = 0, seen = false;
  for (const e of events ?? []) if (e.type === 'private_jet_flight') { seen = true; t += Math.max(0, e.tonnes); }
  return seen ? { tonnes: t } : null;
}

/** Tonnes of aviation emissions in a month's flows (0 when none). */
export function aviationTonnes(flows: Partial<MonthFlows> | undefined): number {
  return (flows?.emissions ?? []).filter((e) => e.source === 'aviation').reduce((a, e) => a + Math.max(0, e.tonnes), 0);
}

/** The latest month's aviation tonnes from the snapshot's emissions breakdown (0 when missing). */
export const aviationOfSnapshot = (snap: Snapshot | null): number => Math.max(0, emissionOf(snap?.breakdowns?.emissions.month, 'aviation'));

/** Staff commutes of a frame, shaped like ordinary commutes so the people layer can walk them. Empty when none. */
export function staffCommutesOf(frame: Pick<FrameInput, 'flows'>): { person: number; from: TileRef | null; to: TileRef; minutes: number }[] {
  const flows = frame.flows;
  return (flows?.staffCommutes ?? []).map((c) => ({ person: c.person, from: c.from ? { ...c.from } : null, to: { ...c.to }, minutes: c.minutes }));
}

/** Domestic staff (the pilot) have an employer household and no firm job; they still go to work. */
export const isStaff = (p: Pick<PersonView, 'employer'>): boolean => !!p.employer;
/** Has a job of any kind: a firm job or a household employer. */
export const hasWork = (p: Pick<PersonView, 'job' | 'employer'>): boolean => !!p.job || !!p.employer;
