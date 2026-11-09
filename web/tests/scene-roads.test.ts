import { describe, expect, it } from 'vitest';
import { roadMask, roadParts, roadPiece } from '../src/scene/roads';
import { rimItems } from '../src/scene/island';
import { route, roadShare, pathTiles, STEP_COST } from '../src/scene/paths';
import type { MapLayout } from '../src/store';

const set = (...p: [number, number][]) => new Set(p.map(([x, y]) => `${x},${y}`));
const at = (roads: Set<string>, x: number, y: number) => roadPiece(roadMask(roads, { x, y }));

describe('road connection masks', () => {
  it('classifies isolated, end, straight, corner, T and cross pieces', () => {
    expect(at(set([3, 3]), 3, 3)).toEqual({ piece: 'isolated', rot: 0 });
    expect(at(set([3, 3], [3, 4]), 3, 3)).toEqual({ piece: 'end', rot: 2 }); // arm to the south
    expect(at(set([3, 3], [3, 2], [3, 4]), 3, 3)).toEqual({ piece: 'straight', rot: 0 });
    expect(at(set([3, 3], [2, 3], [4, 3]), 3, 3)).toEqual({ piece: 'straight', rot: 1 });
    expect(at(set([3, 3], [4, 3], [3, 4]), 3, 3)).toEqual({ piece: 'corner', rot: 1 }); // east + south
    expect(at(set([3, 3], [2, 3], [3, 2]), 3, 3)).toEqual({ piece: 'corner', rot: 3 }); // west + north
    expect(at(set([3, 3], [2, 3], [4, 3], [3, 4]), 3, 3)).toEqual({ piece: 't', rot: 2 }); // stem to the south
    expect(at(set([3, 3], [2, 3], [4, 3], [3, 2], [3, 4]), 3, 3)).toEqual({ piece: 'cross', rot: 0 });
  });
  it('ignores diagonal neighbours', () => {
    expect(at(set([3, 3], [4, 4], [2, 2]), 3, 3).piece).toBe('isolated');
  });
  it('geometry parts grow with connections and stay inside the tile arms', () => {
    const lone = roadParts([{ x: 0, y: 0 }], set([0, 0]), 12);
    const cross = roadParts([{ x: 1, y: 1 }], set([1, 1], [0, 1], [2, 1], [1, 0], [1, 2]), 12);
    expect(cross.length).toBeGreaterThan(lone.length);
    for (const p of cross) { expect(Math.abs(p.p[0] - (1 - 5.5))).toBeLessThanOrEqual(0.51); expect(Math.abs(p.p[2] - (1 - 5.5))).toBeLessThanOrEqual(0.51); }
  });
});

const layout = (roads: [number, number][], blocked: [number, number][] = []): MapLayout => ({
  gridSize: 12, scenery: blocked.map(([x, y]) => ({ kind: 'bank' as const, x, y, label: 'b' })), placed: [], pendingTiles: {},
  roads: roads.map(([x, y]) => ({ x, y })), pendingRoads: [],
});

describe('routing prefers roads', () => {
  it('takes a road detour when it is cheap, and the plain line when there is none', () => {
    const a = { x: 0, y: 5 }, b = { x: 8, y: 5 };
    const plain = route(layout([]), a, b);
    expect(plain).toHaveLength(9);
    // a road along row 4 one step above: a detour of 2 extra tiles that is cheaper per tile than open ground
    const road: [number, number][] = Array.from({ length: 9 }, (_, i) => [i, 4]);
    const l = layout(road);
    const r = route(l, a, b);
    expect(roadShare(l, r)).toBeGreaterThan(0.7);
    expect(r[0]).toEqual(a);
    expect(r.at(-1)).toEqual(b);
  });
  it('does not detour for a road that is far away', () => {
    const far: [number, number][] = Array.from({ length: 12 }, (_, i) => [i, 11]);
    const r = route(layout(far), { x: 0, y: 2 }, { x: 8, y: 2 });
    expect(r).toHaveLength(9);
  });
  it('steps are 4-connected and avoid buildings', () => {
    const l = layout([], [[3, 5], [4, 5], [5, 5]]);
    const r = route(l, { x: 0, y: 5 }, { x: 8, y: 5 });
    for (const t of r) expect([[3, 5], [4, 5], [5, 5]].some(([x, y]) => x === t.x && y === t.y)).toBe(false);
    for (let i = 1; i < r.length; i++) expect(Math.abs(r[i]!.x - r[i - 1]!.x) + Math.abs(r[i]!.y - r[i - 1]!.y)).toBe(1);
  });
  it('road tiles cost less than open ground', () => { expect(STEP_COST.road).toBeLessThan(STEP_COST.open); });
  it('dirt paths never cover a road tile', () => {
    const l: MapLayout = { ...layout([[2, 5], [3, 5], [4, 5]]), scenery: [{ kind: 'home', x: 1, y: 5, label: 'h' }, { kind: 'firm', x: 5, y: 5, label: 'f' }] };
    const p = pathTiles(l);
    for (const k of ['2,5', '3,5', '4,5']) expect(p.has(k)).toBe(false);
  });
});

describe('island rim', () => {
  it('is deterministic, stays outside the grid and has trees and hedges', () => {
    const a = rimItems(12), b = rimItems(12);
    expect(a).toEqual(b);
    expect(a.some((i) => i.kind === 'tree') && a.some((i) => i.kind === 'hedge')).toBe(true);
    for (const i of a) expect(Math.max(Math.abs(i.x), Math.abs(i.z))).toBeGreaterThan(6.1);
  });
});
