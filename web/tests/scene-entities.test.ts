import { describe, expect, it } from 'vitest';
import { createSimPort } from './port';
import { layoutOf } from '../src/store';
import { ok } from './helpers';
import { planEntities, entityAt, factsFor } from '../src/scene/entities';
import { MODELS } from '../src/scene/models';
import { route, pathTiles, blockedTiles } from '../src/scene/paths';
import { CLASS_COLOR } from '../src/scene/palette';
import { tileOf, worldOf, hash01 } from '../src/scene/coords';

const snap = () => ok(createSimPort().init(7));

describe('layout to entity mapping', () => {
  const s = snap();
  const layout = layoutOf(s);
  const free = s.map.tiles.filter((t) => t.kind === 'empty');
  const descs = planEntities(layout, s);

  it('makes one entity per scenery and placed item with unique keys', () => {
    expect(descs).toHaveLength(layout.scenery.length + layout.placed.length);
    expect(new Set(descs.map((d) => d.key)).size).toBe(descs.length);
  });
  it('every model id exists', () => {
    for (const d of descs) expect(MODELS[d.model], d.model).toBeDefined();
  });
  it('homes get a distinct model per class', () => {
    const homes = descs.filter((d) => d.ref.kind === 'home');
    expect(homes).toHaveLength(8);
    // Town v7: the estate house has a model of its own on top of the five class models.
    expect(new Set(homes.map((h) => h.model)).size).toBe(new Set(homes.map((h) => (h.model === 'home_estate' ? 'estate' : h.household?.class))).size);
    const classes = ['capitalist', 'rentier', 'manager', 'worker', 'dependent'] as const;
    for (const c of classes) expect(MODELS[`home_${c}`]).toBeDefined();
    expect(new Set(classes.map((c) => JSON.stringify(MODELS[`home_${c}`].parts.map((p) => p.s + p.z.join()))))).toHaveProperty('size', 5);
  });
  it('firms use factory, farm, or vacant lot from sector and active flag', () => {
    const firms = descs.filter((d) => d.ref.kind === 'firm');
    for (const f of firms) {
      const expected = !f.firm || !f.firm.active ? 'firm_vacant' : f.firm.sector === 'food' ? 'firm_food' : f.firm.sector === 'services' ? 'firm_shop' : 'firm_goods';
      expect(f.model).toBe(expected);
    }
    expect(firms.some((f) => f.model === 'firm_goods')).toBe(true);
    expect(firms.some((f) => f.model === 'firm_food')).toBe(true);
  });
  it('queued builds are pending with turns remaining; finished ones are normal', () => {
    const q = { id: 9, command: { type: 'place' as const, kind: 'renewable_plant' as const, tile: { x: free[4]!.x, y: free[4]!.y } }, submittedTurn: s.turn, effectiveTurn: s.turn + 2, pcCost: 2, moneyCost: 2e6 };
    const s2 = { ...s, pending: [q] };
    const d = planEntities(layoutOf(s2), s2);
    const p = d.find((x) => x.ref.tile.x === free[4]!.x && x.ref.tile.y === free[4]!.y)!;
    expect(p.state).toBe('pending');
    expect(p.turnsLeft).toBe(3); // fires at the start of submit turn+2, so three turns away counting this one
    expect(p.ref.pendingTurns).toBe(3);
    expect(d.filter((x) => x.state === 'pending')).toHaveLength(1);
  });
  it('drafted placements appear as ghosts and are not selectable', () => {
    const draft = [{ id: 1, command: { type: 'place' as const, kind: 'fossil_plant' as const, tile: { x: free[2]!.x, y: free[2]!.y } },
      preview: { command: { type: 'remove' as const, tile: { x: 0, y: 0 } }, pc: 0, money: 0, leadTime: 2, effectiveTurn: 3, opposition: 0, blocked: null, tileValid: true } }];
    const d = planEntities(layout, s, draft);
    expect(d.at(-1)!.state).toBe('draft');
    expect(entityAt(d, { x: free[2]!.x, y: free[2]!.y })).toBeNull();
    expect(entityAt(d, { x: 9, y: 3 })?.ref.kind).toBe('firm');
  });
  it('commons model follows access mode and collapse', () => {
    const open = planEntities(layout, { ...s, nature: { ...s.nature, commons: 0.7, commonsAccess: 'enclosed' } });
    expect(open.find((d) => d.ref.kind === 'commons')!.model).toBe('commons_enclosed');
    const dead = planEntities(layout, { ...s, nature: { ...s.nature, commons: 0.02 } });
    expect(dead.find((d) => d.ref.kind === 'commons')!.model).toBe('commons_collapsed');
  });
  it('facts and class palette', () => {
    expect(factsFor(descs.find((d) => d.household)!).length).toBeGreaterThanOrEqual(3);
    expect(new Set(Object.values(CLASS_COLOR)).size).toBe(5);
  });
});

describe('coords and paths', () => {
  it('tileOf inverts worldOf', () => {
    for (const t of [{ x: 0, y: 0 }, { x: 11, y: 11 }, { x: 5, y: 7 }]) {
      const [x, , z] = worldOf(t, 12);
      expect(tileOf(x, z, 12)).toEqual(t);
    }
    expect(tileOf(99, 0, 12)).toBeNull();
  });
  it('hash is deterministic and in range', () => {
    expect(hash01(5)).toBe(hash01(5));
    for (let i = 0; i < 50; i++) { const h = hash01(i); expect(h).toBeGreaterThanOrEqual(0); expect(h).toBeLessThan(1); }
  });
  it('routes avoid buildings and join both ends', () => {
    const l = layoutOf(snap());
    const home = l.scenery.find((s) => s.kind === 'home')!, firm = l.scenery.find((s) => s.kind === 'firm')!;
    const r = route(l, home, firm);
    expect(r[0]).toEqual({ x: home.x, y: home.y });
    expect(r.at(-1)).toEqual({ x: firm.x, y: firm.y });
    const blocked = blockedTiles(l);
    for (const t of r.slice(1, -1)) expect(blocked.has(`${t.x},${t.y}`)).toBe(false);
    for (let i = 1; i < r.length; i++) expect(Math.abs(r[i]!.x - r[i - 1]!.x) + Math.abs(r[i]!.y - r[i - 1]!.y)).toBe(1);
  });
  it('footpaths never sit under buildings', () => {
    const l = layoutOf(snap());
    const p = pathTiles(l), b = blockedTiles(l);
    expect(p.size).toBeGreaterThan(5);
    for (const k of p) expect(b.has(k)).toBe(false);
  });
});
