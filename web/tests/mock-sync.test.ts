import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { generate, OUT_PATH, render } from '../scripts/sync-mock.mjs';
import { GEN, LEVERS } from '../src/sim-port/mock/data';

describe('mock data is generated from content/', () => {
  it('data.generated.json is up to date (run `npm run sync-mock` when content/ changes)', () => {
    const onDisk = readFileSync(OUT_PATH, 'utf8');
    expect(JSON.parse(onDisk), 'stale: run npm run sync-mock').toEqual(JSON.parse(render()));
    expect(onDisk).toBe(render());
  });
  it('carries the real numbers, not invented ones', () => {
    expect(GEN.pc).toEqual({ start: 60, cap: 100, regenBase: 8, cancelRefund: 0.5 });
    expect(GEN.turnCosts.carbon_price).toEqual({ baseCost: 8, leadTime: 2, unit: 50, minSize: 0.25 });
    expect(GEN.turnCosts.switch_system!.baseCost).toBe(45);
    expect(GEN.turnCosts.place_fossil_plant).toEqual({ baseCost: 12, leadTime: 4, unit: 1, minSize: 1 });
    expect(GEN.systems.regulated.transitionMonths).toBe(12);
    expect(GEN.systems.coop.transitionMonths).toBe(24);
    expect(GEN.build.start).toEqual({ publicHousing: 0, fossilPlant: 2, renewablePlant: 0, road: 11 });
    expect(GEN.build.roadCostUsd).toBe(2500);
    expect(GEN.space).toMatchObject({ slowMinutes: 6, fastMinutes: 2 });
    expect(GEN.carbon.budget).toBe(900);
    expect(GEN.policyStart.tax_wage).toBe(0.3);
    expect(GEN.policyStart.min_wage).toBe(1200);
    expect(GEN.game).toEqual({ startYear: 2026, firstTurn: 1, lastTurn: 40, turnsPerYear: 4, ticksPerTurn: 3 });
  });
  it('the layout comes from content/initial_town.json: 22 people, 9 households, 8 homes, 4 firms, a 12 x 12 map', () => {
    expect(GEN.households).toHaveLength(9);
    expect(GEN.households.flatMap((h) => h.people)).toHaveLength(22);
    expect(GEN.homes).toHaveLength(8);
    // Town v7: the estate block and the pilot, from content
    expect(GEN.estate).toMatchObject({ household: 8, home: 7, hangarTile: { x: 2, y: 11 } });
    expect(GEN.map.estateGrounds).toHaveLength(3);
    expect(GEN.map.runway).toHaveLength(3);
    expect(GEN.households[6]!.people.filter((p) => p.staffOf === 8)).toHaveLength(1);
    expect(GEN.town.employed).toBe(10); // the pilot counts as employed, as Person::employed does
    expect(GEN.town.labourForce).toBe(11);
    expect(GEN.households.filter((h) => h.home === null)).toHaveLength(1);
    expect(GEN.firms.map((f) => f.sector)).toEqual(['goods', 'food', 'services', 'services']);
    expect(GEN.map).toMatchObject({ width: 12, height: 12 });
    expect(GEN.map.roads).toHaveLength(11); // 4 in the town and the estate's 7
    expect(GEN.plants).toHaveLength(2);
    expect(GEN.turnCosts.place_road).toEqual({ baseCost: 0.5, leadTime: 2, unit: 1, minSize: 1 });
  });
  it('every mock lever has a cost entry and a starting policy, and every cost entry is a lever or an action', () => {
    const levers = new Set<string>(LEVERS.map((l) => l.key));
    // resilience_spending: the real content has its cost but no starting policy yet (the mock starts it at 0, labelled in data.ts).
    for (const k of levers) {
      expect(GEN.turnCosts[k], `turn.levers.${k}`).toBeDefined();
      if (k !== 'resilience_spending') expect(GEN.policyStart[k], `initial policy ${k}`).toBeDefined();
    }
    const actions = /^(switch_system|place_|remove_)/;
    for (const k of Object.keys(GEN.turnCosts)) expect(levers.has(k) || actions.test(k), k).toBe(true);
    expect(generate()).toBeTruthy();
  });
});
