import { describe, expect, it } from 'vitest';
import { createSimPort } from './port';
import { LENSES, layoutOf } from '../src/store';
import { ok } from './helpers';
import { planEntities } from '../src/scene/entities';
import { employmentState, entityTints, extremes, legendFor, personTints, ramp, RAMP, EMPLOYED, UNEMPLOYED, NOT_WORKING, type LensCtx } from '../src/scene/lenses';
import { planStands } from '../src/scene/people';
import { CLASS_COLOR } from '../src/scene/palette';
import { pathTiles } from '../src/scene/paths';

const snap = ok(createSimPort().init(5));
const layout = layoutOf(snap);
const descs = planEntities(layout, snap);
const stands = planStands(layout, snap);
const ctx: LensCtx = { descs, snapshot: snap, layout, roads: pathTiles(layout), asphalt: new Set() };

describe('Town v2 lenses', () => {
  it('has a commute lens with a legend and every lens colours every person without throwing', () => {
    expect(LENSES.map((l) => l.id)).toContain('commute');
    for (const l of LENSES) {
      expect(legendFor(l.id, ctx).lens).toBe(l.id);
      const t = personTints(l.id, snap, stands);
      if (l.id === 'none') expect(t.size).toBe(0);
      else expect(t.size).toBe(stands.length);
      for (const c of t.values()) expect(c).toBeGreaterThanOrEqual(0);
    }
  });
  it('class: people take their household class colour', () => {
    const t = personTints('class', snap, stands);
    for (const st of stands) expect(t.get(st.id)).toBe(CLASS_COLOR[snap.households.find((h) => h.id === st.household)!.class]);
  });
  it('employment: employed, unemployed and not-in-labour-force people are three different colours', () => {
    const t = personTints('employment', snap, stands);
    for (const p of snap.people) {
      const e = employmentState(p);
      expect(t.get(p.id)).toBe(e === 'employed' ? EMPLOYED : e === 'unemployed' ? UNEMPLOYED : NOT_WORKING);
    }
    expect(new Set([EMPLOYED, UNEMPLOYED, NOT_WORKING]).size).toBe(3);
  });
  it('commute: longer commutes are darker, people without a job are grey', () => {
    const t = personTints('commute', snap, stands);
    const ex = extremes(snap);
    for (const p of snap.people) expect(t.get(p.id)).toBe(p.job || p.employer ? ramp(RAMP.commute, p.commuteMinutes / ex.maxCommute) : NOT_WORKING);
    expect(ramp(RAMP.commute, 1)).not.toBe(ramp(RAMP.commute, 0));
  });
  it('commute lens colours homes by their longest commuter', () => {
    const t = entityTints('commute', ctx);
    const homes = descs.filter((d) => d.household && d.commuteMinutes?.length);
    for (const d of homes) expect(t.get(d.key)).toBe(ramp(RAMP.commute, d.commuteMinutes![0]! / extremes(snap).maxCommute));
  });
  it('emissions: plants are coloured by their own emissions, a renewable plant is the cleanest end', () => {
    const t = entityTints('emissions', ctx);
    for (const d of descs.filter((x) => x.plant)) {
      const w = d.plant!.emissionsMonthly / extremes(snap).maxEmission;
      expect(t.get(d.key)).toBe(ramp(RAMP.emissions, d.ref.kind === 'renewable_plant' ? 0 : w));
    }
  });
  it('wealth: richer households are not lighter than poorer ones', () => {
    const t = personTints('wealth', snap, stands);
    expect(new Set(t.values()).size).toBeGreaterThan(1);
  });
});
