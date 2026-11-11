import { describe, expect, it } from 'vitest';
import type { SimEvent, Snapshot, TurnResult } from '../src/contract';
import { deriveCrises } from '../src/panels/crisis';
import { bands, catalog } from '../src/store';
const LEGITIMACY_WARNING = bands().crisis.legitimacyBelow, BUDGET_WARNING = bands().crisis.budgetBelow;
import { arrowFlip, estimatePc, leverCostHint } from '../src/panels/costs';
import { leadRangeText, turnsUntil } from '../src/panels/format';
import { arrowsShown, groupLevers, leverStatus, lobbyNote, toCommandValue, toNum } from '../src/panels/policy';
import { comingNextTurn, eventLines, lobbyLines, metricDeltas, rankEvents } from '../src/panels/results';
import { generateStory } from '../src/panels/story';
import { emissionsBars, incomeByType, lobbyingBars, whyData } from '../src/panels/why';
import { resolveSelection, resolveTile, builtCount } from '../src/panels/inspect';
import { switchOptions } from '../src/panels/systemSwitch';
import { createSimPort } from '../src/sim-port';
import { createGame, layoutOf, quarterLabel, shownTurn, type Game } from '../src/store';

function playTurns(g: Game, n: number, cmd?: () => void) {
  for (let i = 0; i < n && !g.state.get().snapshot!.gameOver; i++) {
    cmd?.();
    g.endTurn();
    g.finishPlayback();
    g.continueFromResults();
  }
}
const fresh = (turns = 0) => {
  const g = createGame(createSimPort());
  g.newGame(11);
  playTurns(g, turns);
  return g;
};
const ev = (e: Partial<SimEvent> & { type: SimEvent['type'] }, importance: 1 | 2 | 3 = 1) => ({ importance, ...e }) as SimEvent;
const result = (events: SimEvent[], extra: Partial<TurnResult> = {}): TurnResult => ({
  turn: 1, accepted: [], rejected: [], activated: [], pcBefore: 10, pcAfter: 10, months: [{ month: 0, events, flows: { commutes: [], purchases: [], rent: [], wages: [], emissions: [] } }],
  policyGaps: [], trace: [], snapshot: {} as Snapshot, ...extra,
});

describe('crisis derivation', () => {
  it('is empty for a healthy start', () => {
    expect(deriveCrises(fresh().state.get().snapshot, null)).toEqual([]);
  });
  it('turns bank failure and commons collapse events into critical banners', () => {
    const s = fresh().state.get().snapshot!;
    const c = deriveCrises(s, result([ev({ type: 'bank_failed' }, 3), ev({ type: 'commons_collapsed' }, 3), ev({ type: 'policy_blocked', reason: 'insufficient_funds' }, 2)]));
    expect(c.map((x) => x.id)).toEqual(['bank_failed', 'commons_collapsed', 'policy_blocked:insufficient_funds']);
    expect(c[0]!.severity).toBe('critical');
    expect(c[2]!.severity).toBe('warning');
  });
  it('warns at the documented thresholds only', () => {
    const s = structuredClone(fresh().state.get().snapshot!);
    s.government.legitimacy = LEGITIMACY_WARNING;
    s.indicators.budgetShare = BUDGET_WARNING;
    expect(deriveCrises(s, null)).toEqual([]);
    s.government.legitimacy = LEGITIMACY_WARNING - 0.01;
    s.indicators.budgetShare = BUDGET_WARNING - 0.01;
    expect(deriveCrises(s, null).map((x) => x.id).sort()).toEqual(['low_budget', 'low_legitimacy']);
  });
  it('a failed bank in the snapshot persists', () => {
    const s = structuredClone(fresh().state.get().snapshot!);
    s.bank.failed = true;
    expect(deriveCrises(s, null)[0]!.id).toBe('bank_failed');
  });
});

describe('event ranking', () => {
  it('ranks by importance then order and hides noise', () => {
    const events = [ev({ type: 'commuted', person: 1, firm: 1, tiles: 3, minutes: 18 }), ev({ type: 'person_hired', person: 1, firm: 1 }, 1), ev({ type: 'bank_failed' }, 3),
      ev({ type: 'command_activated', id: 1 }, 2), ev({ type: 'firm_closed', firm: 2 }, 2)];
    const ranked = rankEvents(events);
    expect(ranked.map((e) => e.type)).toEqual(['bank_failed', 'firm_closed', 'person_hired']);
  });
  it('limits and counts the rest, wording from the payload', () => {
    const many = Array.from({ length: 12 }, (_, i) => ev({ type: 'firm_closed', firm: i }, 2));
    const { lines, more } = eventLines(many, null, 8);
    expect(lines).toHaveLength(8);
    expect(more).toBe(4);
    expect(lines[0]!.text).toBe('firm 0 closed');
  });
});

describe('results sections', () => {
  it('lists lobbying gaps and rejected commands', () => {
    const g = fresh();
    const levers = g.state.get().levers, actions = g.state.get().actions;
    const r = result([], {
      policyGaps: [{ lever: 'carbon_price', nominal: 50, effective: 31 }],
      rejected: [{ index: 0, command: { type: 'switch_system', to: 'coop' }, reason: 'insufficient_political_capital' }],
    });
    const lines = lobbyLines(r, levers, actions);
    // Not enough political capital is NOT lobbying: it gets its own kind.
    expect(lines[0]).toMatchObject({ kind: 'unaffordable' });
    expect(lines[0]!.text).toMatch(/Not enough political capital/);
    expect(lines[1]).toMatchObject({ kind: 'weakened' });
    expect(lines[1]!.text).toBe('Carbon price: you set $50/t, about $31/t in force (38% lost to lobbying)');
  });
  it('rule blocks are separate from lobbying and from unaffordable, and zero gaps are not reported', () => {
    const g = fresh();
    const levers = g.state.get().levers, actions = g.state.get().actions;
    const r = result([], {
      policyGaps: [{ lever: 'carbon_price', nominal: 50, effective: 50 }, { lever: 'wealth_tax', nominal: 0, effective: 0 }, { lever: 'tax_profit', nominal: 0.3, effective: 0.25 }],
      rejected: [{ index: 0, command: { type: 'set_policy', lever: 'campaign_cap', value: 0.5 }, reason: 'lever_unavailable' }, { index: 1, command: { type: 'switch_system', to: 'coop' }, reason: 'insufficient_funds' }],
    });
    const lines = lobbyLines(r, levers, actions);
    expect(lines.map((l) => l.kind)).toEqual(['blocked', 'unaffordable', 'weakened']);
    expect(lines.filter((l) => l.kind === 'weakened')).toHaveLength(1);
  });
  it('computes metric deltas with arrows and tones from two snapshots', () => {
    const g = fresh();
    const before = g.state.get().snapshot!;
    const after = structuredClone(before);
    after.indicators.unemployment += 0.05;
    after.indicators.avgWellbeing += 0.1;
    const rows = metricDeltas(before, after);
    expect(rows.find((r) => r.def.id === 'unemployment')!.change).toMatchObject({ trend: 'up', tone: 'bad' });
    expect(rows.find((r) => r.def.id === 'wellbeing')!.change).toMatchObject({ trend: 'up', tone: 'good' });
    expect(rows.find((r) => r.def.id === 'gdp')!.change.trend).toBe('flat');
  });
  it('coming next turn splits pending by effective turn', () => {
    const s = structuredClone(fresh().state.get().snapshot!);
    const item = (id: number, t: number) => ({ id, command: { type: 'remove' as const, tile: { x: 10, y: 2 } }, submittedTurn: 1, effectiveTurn: t, pcCost: 1, moneyCost: 0 });
    // Items fire at the START of the next submit: effectiveTurn == snapshot.turn is "coming next turn".
    s.pending = [item(1, s.turn), item(2, s.turn + 2)];
    const c = comingNextTurn(s);
    expect(c.next.map((p) => p.id)).toEqual([1]);
    expect(c.later).toBe(1);
  });
});

describe('why panels', () => {
  const snap = fresh().state.get().snapshot!;
  it('groups emissions by source, sink negative', () => {
    const bars = emissionsBars([
      { type: 'emissions', source: 'firm', id: snap.firms[0]!.id, tonnes: 90 }, { type: 'emissions', source: 'households', tonnes: 10 }, { type: 'emissions', source: 'sink', tonnes: 20 },
    ], snap);
    expect(bars[0]).toMatchObject({ label: snap.firms[0]!.name, value: 90, sub: '90 t of 100 t emitted' });
    expect(bars.at(-1)).toMatchObject({ value: -20, tone: 'sink' });
  });
  it('sums income by type', () => {
    const bars = incomeByType([{ type: 'income', household: 1, kind: 'wages', amount: 100 }, { type: 'income', household: 2, kind: 'wages', amount: 50 }, { type: 'income', household: 1, kind: 'rent', amount: 50 }]);
    expect(bars.map((b) => [b.label, b.value])).toEqual([['Wages', 150], ['Rent', 50]]);
  });
  it('lobbying bars carry set and in-force values', () => {
    const levers = createSimPort().levers();
    const { bars, netLobby } = lobbyingBars([{ type: 'policy_gap', lever: 'carbon_price', nominal: 80, effective: 40, netLobby: 0.3 }], levers);
    expect(bars[0]).toMatchObject({ value: 80, pair: 40, sub: '50% lost to lobbying' });
    expect(netLobby).toBe(0.3);
    expect(whyData('lobbying', [], snap, levers).empty).toMatch(/No lobbied policy/);
    expect(whyData('emissions', [], snap, levers).empty).toMatch(/End a turn/);
  });
});

describe('causal story', () => {
  it('is deterministic and uses only data from the run', () => {
    const g = fresh();
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 120 });
    playTurns(g, 6);
    const s = g.state.get().snapshot!, last = g.state.get().lastResult;
    const a = generateStory(s, last), b = generateStory(s, last);
    expect(a).toEqual(b);
    expect(a.paragraphs.join(' ')).toMatch(/Gini went from/);
    expect(a.paragraphs.join(' ')).toMatch(/Legitimacy/);
  });
  it('names the biggest emissions source from the last trace and the lobbying gap', () => {
    const g = fresh(2);
    const s = structuredClone(g.state.get().snapshot!);
    const last = result([], {
      trace: [{ type: 'emissions', source: 'firm', id: s.firms[0]!.id, tonnes: 75 }, { type: 'emissions', source: 'households', tonnes: 25 }],
      policyGaps: [{ lever: 'carbon_price', nominal: 100, effective: 60 }, { lever: 'wealth_tax', nominal: 0.04, effective: 0.03 }],
    });
    const story = generateStory(s, last).paragraphs.join('\n');
    expect(story).toContain(`${s.firms[0]!.name}, at 75% of what was emitted`);
    expect(story).toContain('watered down 2 policies at the end, by 33% on average');
  });
  it('win and lose outcomes map to the game outcome', () => {
    const s = structuredClone(fresh(2).state.get().snapshot!);
    s.gameOver = { turn: 161, outcome: 'survived' };
    expect(generateStory(s, null).outcome).toBe('win');
    s.gameOver = { turn: 40, outcome: 'budget_exhausted' };
    expect(generateStory(s, null)).toMatchObject({ outcome: 'lose', headline: 'The carbon budget ran out' });
    s.gameOver = { turn: 40, outcome: { collapse: 'legitimacy_zero' } };
    expect(generateStory(s, null).summary).toMatch(/legitimacy zero/);
  });
  it('does not crash with no history', () => {
    const s = structuredClone(fresh().state.get().snapshot!);
    s.gameOver = { turn: 1, outcome: 'budget_exhausted' };
    expect(generateStory(s, null).paragraphs).toEqual([]);
  });
});

describe('policy logic', () => {
  const g = fresh();
  const { levers, snapshot } = g.state.get();
  it('groups all 20 levers exactly once', () => {
    const groups = groupLevers(levers);
    expect(groups.flatMap((x) => x.levers.map((l) => l.key)).sort()).toEqual(levers.map((l) => l.key).sort());
    expect(groups.map((x) => x.group.id)).not.toContain('other');
    expect(levers).toHaveLength(20);
  });
  it('converts between numbers and command values', () => {
    const bool = levers.find((l) => l.key === 'bailout')!, en = levers.find((l) => l.key === 'commons_access')!;
    expect(toCommandValue(bool, 1)).toBe(true);
    expect(toCommandValue(en, 1)).toEqual({ mode: 'community', cap: 0.5 });
    expect(toCommandValue(en, 0)).toEqual({ mode: 'open' });
    expect(toNum(en, 'community')).toBe(1);
  });
  it('disables levers unavailable in the system; cost never locks a lever (preview() decides)', () => {
    const cap = levers.find((l) => l.key === 'campaign_cap')!;
    expect(leverStatus(cap, snapshot!, []).disabled).toMatch(/Not available in Market capitalism. Needs Regulated capitalism or Worker cooperatives/);
    const poor = structuredClone(snapshot!);
    poor.pc.current = 3;
    expect(leverStatus(levers.find((l) => l.key === 'carbon_price')!, poor, []).disabled).toBeNull();
    expect(leverStatus(levers.find((l) => l.key === 'carbon_price')!, snapshot!, []).disabled).toBeNull();
  });
  it('lobby note states what you set and what to expect', () => {
    const cp = levers.find((l) => l.key === 'carbon_price')!;
    const pv = { command: { type: 'set_policy' as const, lever: 'carbon_price' as const, value: 50 }, pc: 14, money: 0, leadTime: 2, effectiveTurn: 3, opposition: 0.38, expectedEffective: 31, blocked: null, tileValid: true };
    expect(lobbyNote(cp, 0, 50, pv)).toBe('You set $50/t; lobbying is expected to leave about $31/t in force (this opposes the change).');
    expect(lobbyNote(cp, 0, 50, { ...pv, expectedEffective: undefined })).toBeNull(); // no expectation from the sim: no claim
    expect(lobbyNote(cp, 0, 50, { ...pv, opposition: 0 })).toBeNull();
    expect(lobbyNote(levers.find((l) => l.key === 'tax_wage')!, 0, 0.3, pv)).toBeNull();
  });
});

describe('system switch', () => {
  it('lists costs from actions() and reasons it cannot be queued', () => {
    const g = fresh();
    const { snapshot, actions } = g.state.get();
    const act = actions.find((a) => a.key === 'switch_system');
    const opts = switchOptions(snapshot!, act, 100);
    expect(opts.find((o) => o.to === 'market')!.disabled).toMatch(/current/);
    const reg = opts.find((o) => o.to === 'regulated')!;
    expect(reg.disabled).toBeNull();
    expect(reg.pcCost).toBe(act!.baseCostPc);
    expect(reg.summary).toMatch(/12 month transition, 5% less output/);
    expect(switchOptions(snapshot!, act, 0).find((o) => o.to === 'coop')!.disabled).toMatch(/Needs about/);
  });
  it('marks copy the design doc is silent on as placeholder, and takes numbers from the wire cards', () => {
    const card = (k: string) => catalog().systems.find((c) => c.key === k)!;
    expect(card('market').resistancePlaceholder).toBe(true);
    expect(card('coop').resistancePlaceholder).toBe(true);
    expect(card('regulated').resistancePlaceholder).toBe(false);
    expect(card('coop').transitionMonths).toBe(24);
  });
});

describe('inspector model', () => {
  it('resolves people, households, homes, firms, scenery and buildings, and offers removal per tile', () => {
    const g = fresh();
    const { snapshot } = g.state.get();
    const snap = snapshot!;
    const layout = layoutOf(snap);
    const home = layout.scenery.find((s) => s.kind === 'home')!;
    const hm = resolveTile(snap, layout, home, []);
    expect(hm.kind).toBe('household');
    expect(hm.income!.map((p) => p.label)).toEqual(['Wages', 'Dividends', 'Rent', 'Interest', 'Benefits']);
    expect(hm.members!.length).toBeGreaterThanOrEqual(2);
    const person = resolveSelection(snap, layout, hm.members![0]!.selection, [])!;
    expect(person.kind).toBe('person');
    expect(person.household!.selection).toEqual({ kind: 'household', id: 0 });
    const worker = snap.people.find((p) => p.job)!;
    const wm = resolveSelection(snap, layout, { kind: 'person', id: worker.id }, [])!;
    expect(wm.sections.map((x) => x.heading)).toEqual(['Work', 'Commute', 'Life']);
    expect(wm.sections[1]!.rows[1]!.value).toMatch(/minutes/);
    const unhoused = resolveSelection(snap, layout, { kind: 'household', id: 7 }, [])!;
    expect(unhoused.sections[0]!.rows[0]!.value).toMatch(/unhoused/i);
    const firm = layout.scenery.find((s) => s.kind === 'firm')!;
    expect(resolveTile(snap, layout, firm, []).sections.length).toBeGreaterThan(0);
    const dormant = layout.scenery.filter((s) => s.kind === 'firm').map((f) => resolveTile(snap, layout, f, [])).find((m) => m.note?.includes('No firm'));
    expect(dormant).toBeDefined();
    for (const k of ['bank', 'government', 'forest', 'commons'] as const) expect(resolveTile(snap, layout, layout.scenery.find((s) => s.kind === k)!, []).kind).toBe(k);
    const fossil = layout.placed.find((p) => p.kind === 'fossil_plant')!;
    const bm = resolveTile(snap, layout, fossil, []);
    expect(bm.removable).toEqual({ kind: 'fossil_plant', tile: { x: fossil.x, y: fossil.y }, already: false });
    expect(builtCount(snap, 'fossil_plant')).toBe(2);
    expect(builtCount(snap, 'road')).toBe(11);
    const queued = resolveTile(snap, layout, fossil, [{ id: 1, command: { type: 'remove', tile: { x: fossil.x, y: fossil.y } }, preview: {} as never }]);
    expect(queued.removable!.already).toBe(true);
    const road = layout.roads[0]!;
    expect(resolveTile(snap, layout, road, []).removable).toMatchObject({ kind: 'road', already: false });
    const free = snap.map.tiles.find((t) => t.kind === 'empty')!;
    const q = { id: 4, command: { type: 'place' as const, kind: 'renewable_plant' as const, tile: { x: free.x, y: free.y } }, submittedTurn: snap.turn, effectiveTurn: snap.turn + 2, pcCost: 2, moneyCost: 2e6 };
    const withQ = { ...snap, pending: [q] };
    const under = resolveTile(withQ, layoutOf(withQ), free, []);
    expect(under.removable).toBeUndefined();
    expect(under.title).toMatch(/Under construction/);
    expect(under.sections[0]!.rows[0]!.value).toBe('in 3 turns');
    expect(resolveTile(snap, layout, free, []).kind).toBe('empty');
  });
  it('a queued removal goes through the store as a draft and takes effect', () => {
    const g = fresh();
    const plant = layoutOf(g.state.get().snapshot).placed.find((p) => p.kind === 'fossil_plant')!;
    expect(g.removeAt(plant)).toBeNull();
    const d = g.state.get().draft[0]!;
    expect(d.preview.blocked).toBeNull();
    g.endTurn();
    g.finishPlayback();
    g.continueFromResults();
    playTurns(g, 4);
    expect(layoutOf(g.state.get().snapshot).placed.filter((p) => p.kind === 'fossil_plant')).toHaveLength(1);
  });
});

describe('v0.3 panels logic', () => {
  const g = fresh();
  const { levers, snapshot } = g.state.get();
  const lv = (k: string) => levers.find((l) => l.key === k)!;
  it('policy arrows describe raising and flip when the slider goes down', () => {
    const cp = lv('carbon_price');
    expect(arrowsShown(cp, 0, undefined)).toMatchObject({ basis: 'raise', arrows: cp.arrows });
    expect(arrowsShown(cp, 50, 100).basis).toBe('raised');
    const down = arrowsShown(cp, 100, 20);
    expect(down.basis).toBe('lowered');
    expect(down.arrows[0]).toMatchObject({ metric: 'emissions', direction: 'up' });
    expect(arrowsShown(cp, 50, 50).basis).toBe('raise');
  });
  it('the emissions cap treats "no cap" as the loosest setting', () => {
    const cap = lv('emissions_cap');
    expect(arrowFlip(cap, 0, 200)).toBe(-1); // none -> a cap is TIGHTER
    expect(arrowFlip(cap, 200, 0)).toBe(1);
    expect(arrowFlip(cap, 300, 100)).toBe(-1);
    // arrows come from content/levers.json (none for the cap today); the flip logic is checked on a lever that has one
    const withArrow = { ...cap, arrows: [{ metric: 'emissions' as const, direction: 'up' as const }] }; // raising the cap raises emissions
    expect(arrowsShown(withArrow, 0, 200).arrows[0]).toMatchObject({ metric: 'emissions', direction: 'down' });
  });
  it('only causal-model arrows (content/levers.json): none for campaign_cap, nothing on prices for bailout', () => {
    expect(lv('campaign_cap').arrows).toEqual([]);
    expect(lv('bailout').arrows.map((a) => a.metric)).not.toContain('prices');
    for (const l of levers) for (const a of l.arrows) expect(['gdp', 'unemployment', 'gini', 'emissions', 'legitimacy', 'debt', 'wage_share', 'money', 'commons', 'forest_health', 'commons_health', 'farm_health']).toContain(a.metric);
  });
  it('cost hints are estimates built from the cost model', () => {
    expect(leverCostHint(lv('carbon_price'))).toBe('About 2 PC for a small change, 8 PC per $50/t of change');
    expect(leverCostHint(lv('bailout'))).toBe('About 15 PC to switch');
    expect(estimatePc({ base: 8, unit: 50, minSize: 0.25 }, 2)).toBe(16);
  });
  it('lead ranges read naturally', () => {
    expect(leadRangeText([2, 3])).toBe('in 2 to 3 turns');
    expect(leadRangeText([1, 3])).toBe('next turn to 3 turns');
    expect(leadRangeText([2, 2])).toBe('in 2 turns');
    expect(leadRangeText([])).toBe('');
  });
  it('turnsUntil counts a lead-1 item as next turn', () => {
    expect(turnsUntil(snapshot!.turn, snapshot!.turn)).toBe(1);
    expect(turnsUntil(snapshot!.turn + 2, snapshot!.turn)).toBe(3);
  });
  it('commons community option is greyed in Market with a reason', () => {
    expect(lv('commons_access').optionAvailableIn).toEqual({ community: ['regulated', 'coop'] });
  });
  it('system copy does not claim the emissions cap is unlocked only by Regulated', () => {
    expect(JSON.stringify(catalog().systems.find((c) => c.key === 'regulated'))).not.toMatch(/Unlocks the emissions cap/);
    expect(lv('emissions_cap').availableIn).toContain('market');
  });
  it('crisis text makes no unsupported causal claim', () => {
    const snap = structuredClone(snapshot!);
    snap.bank.failed = true;
    snap.government.legitimacy = 0.1;
    const cs = deriveCrises(snap, null);
    expect(cs.find((c) => c.id === 'bank_failed')!.detail).not.toMatch(/frozen/);
    expect(cs.find((c) => c.id === 'low_legitimacy')!.why).toBeUndefined();
  });
  it('shownTurn is the same for the top bar and the end screen at game over', () => {
    const over = { ...snapshot!, turn: 41, gameOver: { turn: 40, outcome: 'survived' as const } };
    expect(shownTurn(over)).toBe(40);
    expect(quarterLabel(shownTurn(over), over.game)).toBe('Q4 2035');
    expect(shownTurn({ ...snapshot!, turn: 7 })).toBe(7);
  });
});
