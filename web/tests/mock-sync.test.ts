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
    expect(GEN.build.start).toEqual({ publicHousing: 0, fossilPlant: 5, renewablePlant: 0 });
    expect(GEN.carbon.budget).toBe(400000);
    expect(GEN.policyStart.tax_wage).toBe(0.28);
    expect(GEN.policyStart.min_wage).toBe(1200);
    expect(GEN.game).toEqual({ startYear: 2031, firstTurn: 1, lastTurn: 160, turnsPerYear: 4 });
  });
  it('every mock lever has a cost entry and a starting policy, and every cost entry is a lever or an action', () => {
    const levers = new Set<string>(LEVERS.map((l) => l.key));
    for (const k of levers) {
      expect(GEN.turnCosts[k], `turn.levers.${k}`).toBeDefined();
      expect(GEN.policyStart[k], `initial policy ${k}`).toBeDefined();
    }
    const actions = /^(switch_system|place_|remove_)/;
    for (const k of Object.keys(GEN.turnCosts)) expect(levers.has(k) || actions.test(k), k).toBe(true);
    expect(generate()).toBeTruthy();
  });
});
