// The entity registry (content/entities.json, contract v2.1): the web reads labels, roles, scene models and inspector sections from it.
// A facility added only in content (a made-up landfill) appears in the inspector, the scene plan and the lists with no web change.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import type { Catalog, EntityKind, Snapshot } from '../src/contract';
import { resolveTile } from '../src/panels/inspect';
import { infraLines } from '../src/panels/industry';
import { layoutOf, setCatalog, catalog, entityKind, entityOfTile, infraLabel } from '../src/store';
import { planEntities } from '../src/scene/entities';
import { MODELS } from '../src/scene/models';

const real = (): Snapshot => JSON.parse(readFileSync(resolve(__dirname, '../../docs/fixtures/snapshot-turn1.json'), 'utf8')) as Snapshot;
const industry = () => JSON.parse(readFileSync(resolve(__dirname, '../../docs/fixtures/industry.json'), 'utf8')) as Parameters<typeof infraLines>[0];
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
const original = clone(catalog());
afterEach(() => setCatalog(clone(original)));

const LANDFILL = {
  id: 'landfill', kind: 'facility', category: 'safety', label: 'Landfill', plural: 'Landfills', roles: ['located', 'emitter', 'payer', 'buildable', 'labelled'],
  tileKinds: ['landfill'], footprint: [[0, 0]], infraKind: 'landfill', owner: 'public', attributes: {}, sceneModel: { default: ['landfill_pit'] }, labels: {}, inspector: ['emissions'],
} as unknown as EntityKind;

describe('registry on the wire', () => {
  it('names a known scene model for every entity that has one, and every wire tile kind has an entity', () => {
    const c: Catalog = catalog();
    for (const e of c.entities) for (const ids of Object.values(e.sceneModel)) for (const id of ids) expect(Object.keys(MODELS), `${e.id}: ${id}`).toContain(id);
    for (const t of real().map.tiles) expect(entityOfTile(t.kind), t.kind).toBeDefined();
  });
  it('labels come from it, not from the web', () => {
    expect(infraLabel('fossil_plant')).toBe(entityKind('fossil_plant')?.label);
    const plants = catalog().entities.filter((e) => e.group === 'power_plant');
    expect(plants.map((e) => e.attributes.fuel).sort()).toEqual(['fossil', 'renewable']);
  });
  it('the estate owner, the pilot and the estate household are registered as variants', () => {
    for (const id of ['estate_owner', 'pilot', 'estate_household', 'estate_house']) expect(entityKind(id)?.variantOf).toBeTruthy();
  });
});

describe('a facility added only in content', () => {
  const withLandfill = (): Snapshot => {
    setCatalog({ ...clone(original), entities: [...clone(original).entities, LANDFILL] });
    const s = real();
    const i = s.map.tiles.findIndex((t) => t.kind === 'empty');
    const t = s.map.tiles[i]!;
    s.map.tiles[i] = { ...t, kind: 'landfill' as never, buildable: false, emissionsMonthly: 12 };
    return s;
  };
  it('is inspected with its registry label and sections', () => {
    const s = withLandfill();
    const t = s.map.tiles.find((x) => (x.kind as string) === 'landfill')!;
    const m = resolveTile(s, layoutOf(s), { x: t.x, y: t.y }, []);
    expect(m.kind).toBe('building');
    expect(m.title).toBe('Landfill');
    expect(m.sections.map((x) => x.heading)).toEqual(['Emissions']);
    expect(m.sections[0]!.rows[0]!.value).toMatch(/12/);
    expect(m.removable?.kind).toBe('landfill');
  });
  it('is planned into the scene with a fallback model and its label', () => {
    const s = withLandfill();
    const d = planEntities(layoutOf(s), s).find((x) => x.ref.kind === 'landfill')!;
    expect(d.ref.label).toBe('Landfill');
    expect(d.model).toBe('facility_generic');
  });
  it('is named in the registry lists', () => {
    withLandfill();
    expect(infraLabel('landfill')).toBe('Landfill');
    const ind = industry();
    expect(infraLines({ ...ind, infrastructure: [{ ...ind.infrastructure[0]!, kind: 'landfill' as never }] })[0]!.label).toBe('Landfills');
  });
});
