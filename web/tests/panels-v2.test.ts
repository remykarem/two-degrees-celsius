import { describe, expect, it } from 'vitest';
import type { ActionInfo, TraceRecord, TurnResult } from '../src/contract';
import { costLine, estimateDisagrees } from '../src/panels/costs';
import { briefingLines, formatEvent, lobbyingDiffers } from '../src/panels/format';
import { resolveSelection } from '../src/panels/inspect';
import { METRICS, metricById } from '../src/panels/metrics';
import { householdRows, peopleGroups, selectedHouseholdId, selectionAnnouncement, unemploymentCount } from '../src/panels/people';
import { generateStory, townFacts } from '../src/panels/story';
import { emissionsBars } from '../src/panels/why';
import { createSimPort } from './port';
import { createGame, layoutOf } from '../src/store';

function played(turns: number) {
  const g = createGame(createSimPort());
  g.newGame(7);
  for (let i = 0; i < turns; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); }
  return g;
}

describe('household and person lists', () => {
  const snap = played(0).state.get().snapshot!;
  it('has one row per household with a home label or "no home"', () => {
    const rows = householdRows(snap);
    expect(rows).toHaveLength(snap.households.length);
    expect(rows.filter((r) => !r.housed).every((r) => r.homeText === 'no home')).toBe(true);
    expect(rows.some((r) => !r.housed)).toBe(true);
  });
  it('sorts by net worth and income, highest first', () => {
    const w = householdRows(snap, 'netWorth').map((r) => r.netWorth);
    expect(w).toEqual([...w].sort((a, b) => b - a));
    const i = householdRows(snap, 'income').map((r) => r.income);
    expect(i).toEqual([...i].sort((a, b) => b - a));
  });
  it('groups people by household and can narrow to one', () => {
    expect(peopleGroups(snap).flatMap((g) => g.people)).toHaveLength(snap.people.length);
    expect(peopleGroups(snap, 0)).toHaveLength(1);
  });
  it('maps person, home and household selections to one household', () => {
    const p = snap.people[0]!;
    expect(selectedHouseholdId(snap, { kind: 'person', id: p.id })).toBe(p.household);
    expect(selectedHouseholdId(snap, { kind: 'household', id: 3 })).toBe(3);
    const home = snap.homes.find((h) => h.household !== null)!;
    expect(selectedHouseholdId(snap, { kind: 'home', id: home.id })).toBe(home.household);
    expect(selectedHouseholdId(snap, { kind: 'tile', x: 0, y: 0 })).toBeNull();
  });
  it('announces a selection with labels, not ids', () => {
    expect(selectionAnnouncement(snap, { kind: 'person', id: snap.people[0]!.id })).toContain(snap.households[0]!.label);
  });
  it('inspects a person and a household, naming the unhoused one', () => {
    const layout = layoutOf(snap);
    const un = snap.households.find((h) => h.home === null)!;
    const hm = resolveSelection(snap, layout, { kind: 'household', id: un.id }, [])!;
    expect(JSON.stringify(hm)).toMatch(/UNHOUSED/);
    expect(hm.sections.map((s) => s.heading)).toEqual(expect.arrayContaining(['Home', 'Money (per month)']));
    expect(hm.statement?.groups.length).toBe(2); // Town v8: the Wealth rows became the net worth statement
    expect(hm.members!.length).toBe(un.members.length);
    const pm = resolveSelection(snap, layout, { kind: 'person', id: snap.people[0]!.id }, [])!;
    expect(pm.household?.selection).toEqual({ kind: 'household', id: snap.people[0]!.household });
    expect(pm.sections.flatMap((s) => s.rows.map((r) => r.label))).toEqual(expect.arrayContaining(['Job', 'Wage', 'Skill', 'Wellbeing']));
  });
});

describe('metrics derivation', () => {
  it('has avg, median, top wealth share and unemployment as N of M', () => {
    const s = played(1).state.get().snapshot!;
    for (const id of ['avg_income', 'median_income', 'top_wealth_share']) {
      const m = metricById(id)!;
      expect(m.fmt(m.value(s))).not.toMatch(/NaN|undefined/);
    }
    const u = unemploymentCount(s);
    expect(metricById('unemployment')!.detail!(s)).toBe(`${u.unemployed} of ${u.labourForce} unemployed`);
    expect(u.unemployed).toBeLessThanOrEqual(u.labourForce);
    expect(METRICS.filter((m) => m.series).every((m) => s.history.every((h) => Number.isFinite(m.series!(h))))).toBe(true);
  });
});

describe('cost display', () => {
  const action: ActionInfo = { key: 'place:fossil_plant', label: 'Fossil plant', baseCostPc: 12, leadTime: 4, moneyCost: 40000, availableIn: ['market'], costModel: { base: 12, unit: 1, minSize: 1 }, needsTile: true };
  it('prefers the preview over the action table and marks estimates', () => {
    expect(costLine(action, { pc: 12, money: 2_000_000, leadTime: 4 })).toEqual({ pc: 12, money: 2_000_000, leadTime: 4, exact: true });
    expect(costLine(action, null)).toMatchObject({ money: 40000, exact: false });
  });
  it('flags a table that disagrees with the preview', () => {
    expect(estimateDisagrees(action, { money: 2_000_000 })).toBe(true);
    expect(estimateDisagrees(action, { money: 40000 })).toBe(false);
    expect(estimateDisagrees(action, null)).toBe(false);
  });
  it('game.estimateBuild agrees with preview() for the same tile', () => {
    const g = played(0);
    const e = g.estimateBuild('fossil_plant')!;
    expect(e).not.toBeNull();
    g.enterPlan();
    g.draftCommand({ type: 'place', kind: 'fossil_plant', tile: e.tile });
    const d = g.state.get().draft[0]!;
    expect(d.preview.money).toBe(e.money);
    expect(d.preview.pc).toBe(e.pc);
  });
});

describe('briefing and events', () => {
  const g = played(2);
  const st = g.state.get();
  const base = st.lastResult!;
  it('mentions lobbying only when the value in force differs', () => {
    const lever = st.levers[0]!.key;
    const same: TurnResult = { ...base, activated: [], rejected: [], months: [], policyGaps: [{ lever, nominal: 50, effective: 50 }] };
    expect(briefingLines(same, st.levers, st.actions).join(' ')).not.toMatch(/Lobbying weakened/);
    const diff: TurnResult = { ...same, policyGaps: [{ lever, nominal: 50, effective: 30 }] };
    expect(briefingLines(diff, st.levers, st.actions).join(' ')).toMatch(/Lobbying weakened/);
    expect(lobbyingDiffers(1, 1 + 1e-9)).toBe(false);
  });
  it('words the Town v2 events with labels and no raw ids', () => {
    const s = st.snapshot!;
    const hh = s.households[0]!, p = s.people.find((x) => x.household === hh.id)!, f = s.firms[0]!;
    const texts = [
      formatEvent({ type: 'household_moved', household: hh.id, to: s.homes[0]!.id, importance: 2 }, s),
      formatEvent({ type: 'household_evicted', household: hh.id, importance: 3 }, s),
      formatEvent({ type: 'person_hired', person: p.id, firm: f.id, importance: 1 }, s),
      formatEvent({ type: 'person_fired', person: p.id, firm: f.id, importance: 2 }, s),
      formatEvent({ type: 'purchase', household: hh.id, firm: f.id, amount: 120, importance: 1 }, s),
      formatEvent({ type: 'built', kind: 'road', tile: { x: 1, y: 2 }, importance: 2 }, s),
      formatEvent({ type: 'removed', tile: { x: 1, y: 2 }, importance: 2 }, s),
      formatEvent({ type: 'electricity_rationed', served: 0.8, importance: 3 }, s),
    ];
    texts.forEach((t) => expect(t).not.toMatch(/undefined|NaN|person \d|household \d/));
    expect(texts[1]).toContain(hh.label);
    expect(texts[2]).toContain(p.label);
    expect(texts[7]).toContain('80%');
  });
  it('names the plant behind an emissions trace record', () => {
    const s = st.snapshot!;
    const pl = s.plants[0];
    if (!pl) return;
    const trace: TraceRecord[] = [{ type: 'emissions', source: 'plant', id: pl.id, tile: pl.tile, tonnes: 5 }];
    expect(emissionsBars(trace, s)[0]!.label).toContain(`(${pl.tile.x},${pl.tile.y})`);
  });
});

describe('Town v2 story facts', () => {
  it('adds median income and top wealth share from history, nothing without history', () => {
    const s = played(4).state.get().snapshot!;
    const facts = townFacts(s, null);
    expect(facts.join(' ')).toMatch(/median/);
    expect(facts.join(' ')).toMatch(/richest household/);
    expect(generateStory(s, null).paragraphs.join(' ')).toMatch(/median/);
    expect(townFacts({ ...s, history: [] }, null)).toEqual([]);
  });
  it('reports evictions from the final turn', () => {
    const g = played(3);
    const s = g.state.get().snapshot!;
    const hh = s.households[0]!;
    const r = { ...g.state.get().lastResult!, months: [{ month: 0 as const, events: [{ type: 'household_evicted' as const, household: hh.id, importance: 3 as const }], flows: { commutes: [], purchases: [], rent: [], wages: [], emissions: [] } }] };
    expect(townFacts({ ...s, housing: { ...s.housing, unhousedHouseholds: 2 } }, r).join(' ')).toMatch(/2 households were without a home, 1 of them evicted/);
  });
});
