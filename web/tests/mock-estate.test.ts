import { describe, expect, it } from 'vitest';
import { createMockSim } from '../src/sim-port/mock';
import { GEN } from '../src/sim-port/mock/data';
import { ok } from './helpers';

// MOCK ONLY (Town v7): the estate in the mock mirrors the wire fields of contract v1.9; its numbers come from content/ (npm run sync-mock).
describe('mock estate (contract v1.9)', () => {
  it('the start snapshot carries the estate, its tag and wealth band, and the pilot employed by the estate household', () => {
    const sim = createMockSim();
    const s = ok(sim.init(1));
    const e = s.estate!;
    expect(e.household).toBe(GEN.estate!.household);
    expect(e.groundsTiles).toHaveLength(3);
    expect(e.runwayTiles).toHaveLength(3);
    expect(e.pilot).toMatchObject({ status: 'employed', unpaidMonths: 0, wage: GEN.estate!.pilotWageUsd });
    const h = s.households[e.household]!;
    expect(h.tag).toBe(GEN.catalog.tags!.estate_owner);
    expect(h.wealthBand!.id).toBe('ultra_rich');
    const pilot = s.people[e.pilot!.person]!;
    expect(pilot.job).toBeNull();
    expect(pilot.employer).toMatchObject({ kind: 'household', household: e.household });
    expect(s.map.tiles.filter((t) => t.kind === 'estate_grounds')).toHaveLength(3);
    expect(s.map.tiles.filter((t) => t.kind === 'runway')).toHaveLength(3);
    expect(s.indicators.unemployment).toBeCloseTo(1 / 11, 9);
  });
  it('flights are events with tonnes; the emissions tally includes them and still reconciles', () => {
    const sim = createMockSim();
    ok(sim.init(3));
    let flights = 0, tonnes = 0, staff = 0;
    for (let i = 0; i < 12; i++) {
      const r = ok(sim.submitTurn([]));
      for (const m of r.months) {
        for (const ev of m.events) if (ev.type === 'private_jet_flight') { flights++; tonnes += ev.tonnes; }
        const a = (m.flows.emissions ?? []).filter((x) => x.source === 'aviation').reduce((acc, x) => acc + x.tonnes, 0);
        if (m.events.some((x) => x.type === 'private_jet_flight')) expect(a).toBe(GEN.estate!.flightTonnes);
        staff += m.flows.staffCommutes?.length ?? 0;
      }
      expect(r.snapshot.breakdowns!.emissions.reconciles).toBe(true);
      if (r.snapshot.gameOver) break;
    }
    const e = ok(sim.exportState()) && JSON.parse(ok(sim.exportState())).estate;
    expect(flights).toBeGreaterThan(0);
    expect(e.flightsTotal).toBe(flights);
    expect(e.tonnesTotal).toBeCloseTo(tonnes, 9);
    expect(staff).toBeGreaterThan(0);
  });
});
