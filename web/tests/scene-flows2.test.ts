import { describe, expect, it } from 'vitest';
import type { FrameInput } from '../src/contract';
import { createSimPort } from '../src/sim-port';
import { layoutOf } from '../src/store';
import { ok } from './helpers';
import { centreOf } from '../src/scene/coords';
import { ctxFromSnapshot, planEmitters, planFrame } from '../src/scene/flows';

const sim = createSimPort();
ok(sim.init(9));
const res = ok(sim.submitTurn([]));
const snap = res.snapshot;
const layout = layoutOf(snap);
const ctx = ctxFromSnapshot(layout, snap);
const at = (t: { x: number; y: number }) => centreOf(t, 12);

describe('flow planner, Town v2', () => {
  const buyer = snap.households.find((h) => h.home !== null)!;
  const shop = snap.firms.find((f) => f.sector === 'services' && f.active);
  const firmTile = (id: number) => snap.map.tiles.find((t) => t.kind === 'firm' && t.ref === id)!;
  const homeOf = (hid: number) => snap.homes.find((h) => h.id === snap.households.find((x) => x.id === hid)!.home)!;

  it('purchases go household -> firm as coins, firm -> household as goods (service tickets for a shop)', () => {
    const firm = shop ?? snap.firms.find((f) => f.active)!;
    const frame: FrameInput = { month: 0, events: [], flows: { purchases: [{ household: buyer.id, firm: firm.id, sector: firm.sector, amount: 300 }] } };
    const plan = planFrame(frame, ctx);
    const coin = plan.tracks.find((t) => t.kind === 'token' && t.style === 2)!;
    const good = plan.tracks.find((t) => t.kind === 'good')!;
    expect(coin.path[0]).toEqual(at(homeOf(buyer.id).tile));
    expect(coin.path.at(-1)).toEqual(at(firmTile(firm.id)));
    expect(good.path[0]).toEqual(at(firmTile(firm.id)));
    expect(good.path.at(-1)).toEqual(at(homeOf(buyer.id).tile));
    expect(good.style).toBe(firm.sector === 'services' ? 2 : firm.sector === 'food' ? 1 : 0);
  });
  it('rent goes to the home\'s owner, or the town hall for public housing', () => {
    const priv = snap.homes.find((h) => h.owner !== 'public' && h.household !== null && h.owner !== h.household);
    if (priv && priv.household !== null) {
      const plan = planFrame({ month: 0, events: [], flows: { rent: [{ household: priv.household, home: priv.id, amount: 200 }] } }, ctx);
      const owner = homeOf(priv.owner as number);
      expect(plan.tracks.find((t) => t.style === 1)!.path.at(-1)).toEqual(at(owner.tile));
    }
    const pub = snap.homes.find((h) => h.owner === 'public' && h.household !== null);
    const hall = snap.map.tiles.find((t) => t.kind === 'town_hall')!;
    if (pub && pub.household !== null) {
      const plan = planFrame({ month: 0, events: [], flows: { rent: [{ household: pub.household, home: pub.id, amount: 200 }] } }, ctx);
      expect(plan.tracks.find((t) => t.style === 1)!.path.at(-1)).toEqual(at(hall));
    }
  });
  it('plant emission entries with a tile drive that plant\'s smoke', () => {
    const fossil = snap.plants.filter((p) => p.kind === 'fossil');
    expect(fossil.length).toBeGreaterThan(0);
    const em = planEmitters([{ source: 'plant', tile: fossil[0]!.tile, tonnes: 10 }, ...fossil.slice(1).map((p) => ({ source: 'plant' as const, tile: p.tile, tonnes: 0 }))], { ...ctx, plants: [] });
    expect(em.filter((e) => e.id.startsWith('p')).map((e) => e.id)).toEqual([`p${fossil[0]!.tile.x},${fossil[0]!.tile.y}`]);
  });
  it('degrades gracefully when every array is empty or missing', () => {
    expect(planFrame({ month: 0, events: [], flows: {} }, ctx).tracks).toEqual([]);
    expect(planFrame({ month: 0, events: [], flows: { purchases: [], rent: [], wages: [], commutes: [], emissions: [] } }, ctx).tracks).toEqual([]);
  });
});
