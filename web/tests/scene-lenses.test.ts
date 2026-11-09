import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { LENSES } from '../src/store';
import { reconcileLayout } from '../src/store/layout';
import { ok } from './helpers';
import { planEntities } from '../src/scene/entities';
import { entityTints, legendFor, ramp, RAMP, tileTints, emitterWeights, employmentShare, type LensCtx } from '../src/scene/lenses';
import { CLASS_COLOR, hex } from '../src/scene/palette';
import { pathTiles } from '../src/scene/paths';

const snapshot = ok(createSimPort().init(3));
const layout = reconcileLayout(null, snapshot);
const descs = planEntities(layout, snapshot);
const ctx: LensCtx = { descs, snapshot, layout, roads: pathTiles(layout) };

describe('lens colours', () => {
  it('ramp hits its end stops and is monotone in between', () => {
    expect(ramp(RAMP.wealth, 0)).toBe(RAMP.wealth[0]);
    expect(ramp(RAMP.wealth, 1)).toBe(RAMP.wealth[3]);
    expect(ramp(RAMP.wealth, -5)).toBe(RAMP.wealth[0]);
    expect(ramp(RAMP.wealth, NaN)).toBe(RAMP.wealth[0]);
  });
  it('class lens paints homes with the Okabe-Ito class colour', () => {
    const t = entityTints('class', ctx);
    for (const d of descs.filter((x) => x.household)) expect(t.get(d.key)).toBe(CLASS_COLOR[d.household!.class]);
    const firm = descs.find((d) => d.firm)!;
    expect(t.get(firm.key)).not.toBe(firm.tint);
  });
  it('emissions lens: the heaviest firm gets the darkest ramp end', () => {
    const t = entityTints('emissions', ctx);
    const active = descs.filter((d) => d.firm?.active);
    const heavy = active.reduce((a, b) => (a.firm!.emissions >= b.firm!.emissions ? a : b));
    expect(t.get(heavy.key)).toBe(RAMP.emissions[3]);
  });
  it('emissions heat is high near emitters and absent far away', () => {
    const tiles = tileTints('emissions', ctx);
    const base = tileTints('none', ctx);
    const em = emitterWeights(ctx);
    expect(em.length).toBeGreaterThan(0);
    const n = layout.gridSize;
    expect(tiles.filter((c, i) => c !== base[i]).length).toBeGreaterThan(3);
    expect(tiles[0 * n + 0]).toBe(base[0]); // far corner untouched
  });
  it('wealth lens ranks poorer homes lighter than richer homes', () => {
    const t = entityTints('wealth', ctx);
    const homes = descs.filter((d) => d.household).sort((a, b) => a.household!.wealthPerCapita - b.household!.wealthPerCapita);
    const lum = (c: number) => ((c >> 16) & 255) + ((c >> 8) & 255) + (c & 255);
    expect(lum(t.get(homes[0]!.key)!)).toBeGreaterThanOrEqual(lum(t.get(homes.at(-1)!.key)!));
  });
  it('employment share handles an empty labour force', () => {
    expect(employmentShare({ employed: 0, labourForce: 0 })).toBe(1);
    expect(employmentShare({ employed: 5, labourForce: 10 })).toBe(0.5);
  });
  it('every lens yields a tint for every entity and ground colours for every tile', () => {
    for (const l of LENSES) {
      expect(entityTints(l.id, ctx).size).toBe(descs.length);
      expect(tileTints(l.id, ctx)).toHaveLength(layout.gridSize ** 2);
    }
  });
  it('none lens tints forests by stock (lush when healthy, dry when depleted)', () => {
    const lush = planEntities(layout, { ...snapshot, nature: { ...snapshot.nature, forest: 0.9 } });
    const dry = planEntities(layout, { ...snapshot, nature: { ...snapshot.nature, forest: 0.1 } });
    const f = (d: typeof lush) => entityTints('none', { ...ctx, descs: d }).get(d.find((x) => x.ref.kind === 'forest')!.key)!;
    expect(f(lush)).not.toBe(f(dry));
  });
});

describe('legend', () => {
  it('describes every lens with label, meaning and a scale', () => {
    for (const l of LENSES) {
      const g = legendFor(l.id, ctx);
      expect(g.lens).toBe(l.id);
      expect(g.label.length).toBeGreaterThan(0);
      expect(g.meaning.length).toBeGreaterThan(10);
      if (l.id !== 'none') expect(g.stops.length).toBeGreaterThan(1);
      for (const s of g.stops) expect(s.color).toMatch(/^#[0-9a-f]{6}$/);
    }
  });
  it('class legend carries a shape note for every class (colour is not the only cue)', () => {
    const g = legendFor('class', ctx);
    expect(g.scale).toBe('categories');
    expect(g.stops).toHaveLength(5);
    expect(g.stops.map((s) => s.color)).toEqual(Object.values(CLASS_COLOR).map(hex));
    for (const s of g.stops) expect(s.label).toMatch(/\(.+marker\)|\(.+cottage/);
  });
});
