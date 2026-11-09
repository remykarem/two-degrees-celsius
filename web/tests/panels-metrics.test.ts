import { describe, expect, it } from 'vitest';
import { createSimPort } from '../src/sim-port';
import { METRICS, historyChange, metricById, runway, runwayText, toneOf } from '../src/panels/metrics';
import { createGame } from '../src/store';

function played(turns: number) {
  const g = createGame(createSimPort());
  g.newGame(7);
  for (let i = 0; i < turns; i++) {
    g.endTurn();
    g.finishPlayback();
    g.continueFromResults();
  }
  return g.state.get().snapshot!;
}

describe('metrics', () => {
  it('has a value and formatted text for every definition', () => {
    const s = played(0);
    for (const m of METRICS) expect(m.fmt(m.value(s))).not.toMatch(/NaN|undefined/);
  });
  it('tone depends on direction and what is good', () => {
    expect(toneOf('up', false)).toBe('bad');
    expect(toneOf('down', false)).toBe('good');
    expect(toneOf('up', null)).toBe('neutral');
    expect(toneOf('flat', true)).toBe('neutral');
  });
  it('historyChange needs two points and a series', () => {
    expect(historyChange(metricById('gdp')!, played(1).history)).toBeNull();
    expect(historyChange(metricById('gdp')!, played(3).history)).not.toBeNull();
    expect(historyChange(metricById('temperature')!, played(3).history)).toBeNull();
  });
  it('runway projects run-out at the current rate', () => {
    const s = played(3);
    const r = runway(s)!;
    expect(r.total).toBeCloseTo(s.indicators.remainingBudget / s.indicators.budgetShare, 3);
    expect(r.actual).toHaveLength(3);
    expect(r.perTurn).toBeCloseTo(s.nature.netEmissionsMonthly * 3, 6);
    expect(r.runOutTurn).toBeCloseTo(s.turn + r.remaining / r.perTurn, 6);
    expect(r.projection[0]!.remaining).toBe(r.remaining);
    expect(runwayText(r, (t) => `T${t}`)).toMatch(/runs out|lasts past/);
  });
  it('runway with zero emissions never runs out', () => {
    const s = structuredClone(played(1));
    s.nature.netEmissionsMonthly = 0;
    const r = runway(s)!;
    expect(r.runOutTurn).toBeNull();
    expect(runwayText(r, String)).toMatch(/do not use up/);
  });
});
