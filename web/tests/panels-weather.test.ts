import { describe, expect, it } from 'vitest';
import type { FrameInput, SimEvent, Snapshot, TraceRecord, TurnResult } from '../src/contract';
import { deriveCrises } from '../src/panels/crisis';
import { formatEvent, severityWord } from '../src/panels/format';
import { METRICS, historyChange, seriesOf } from '../src/panels/metrics';
import { groupLevers } from '../src/panels/policy';
import { emissionsBars, whyData } from '../src/panels/why';
import { generateStory, weatherFacts } from '../src/panels/story';
import { eventLines, rankEvents } from '../src/panels/results';
import { builtCount, resolveTile } from '../src/panels/inspect';
import { fireLabel, fireSelection, forecastView, healthLevel, healthMeters, layoutOf, nearestLandmark, stressGauge, weatherOf } from '../src/store';
import { createMockSim } from '../src/sim-port/mock';
import { ok } from './helpers';

const base = (): Snapshot => ok(createMockSim().init(2));
type Edit = { fires?: { x: number; y: number; left?: number }[]; drought?: { severity: number; monthsLeft: number } | null; stress?: number; fireRisk?: 'low' | 'medium' | 'high'; droughtRisk?: 'low' | 'medium' | 'high'; regrowing?: { x: number; y: number }[]; budgetShare?: number };
/** A copy of the mock snapshot with a chosen weather. */
function withWeather(e: Edit): Snapshot {
  const s = structuredClone(base());
  s.weather = {
    ...s.weather!, stress: e.stress ?? 0.4, drought: e.drought ?? null, fires: (e.fires ?? []).map((f) => ({ tile: { x: f.x, y: f.y }, monthsLeft: f.left ?? 2 })),
    forecast: { fireRisk: e.fireRisk ?? 'low', droughtRisk: e.droughtRisk ?? 'low' },
  };
  for (const f of e.fires ?? []) s.map.tiles[f.y * 12 + f.x]!.state = 'burning';
  for (const f of e.regrowing ?? []) s.map.tiles[f.y * 12 + f.x]!.state = 'regrowing';
  if (e.budgetShare !== undefined) s.indicators.budgetShare = e.budgetShare;
  return s;
}
const forestTile = (s: Snapshot) => s.map.tiles.find((t) => t.kind === 'forest')!;
const ev = (e: SimEvent): SimEvent => e;

describe('stress gauge and health derivation', () => {
  it('the gauge reports percent, a word, the marks and the budget left', () => {
    const g = stressGauge(withWeather({ stress: 0.35, budgetShare: 0.3 }))!;
    expect(g).toMatchObject({ percent: 35, onset: 0.75, full: 0.1, remaining: 0.3, word: 'High' });
    expect(stressGauge(withWeather({ stress: 0 }))!.word).toBe('Calm');
    expect(stressGauge(withWeather({ stress: 0 }))!.text).toMatch(/75%/); // the onset, from the wire
    expect(stressGauge(withWeather({ stress: 0.9 }))!.word).toBe('Severe');
  });
  it('the gauge reads onset and full from the wire (contract v1.3), never from a web constant', () => {
    const wired = withWeather({ stress: 0 });
    wired.weather!.onset = 0.6; wired.weather!.full = 0.2;
    const g = stressGauge(wired)!;
    expect(g).toMatchObject({ onset: 0.6, full: 0.2 });
    expect(g.text).toMatch(/60%/);
    expect(stressGauge(withWeather({ stress: 0.5 }))!.text).toMatch(/10% budget left/);
    // the mock snapshot carries the curve and radii from content/params.json
    expect(base().weather!).toMatchObject({ onset: 0.75, full: 0.1, curve: 1, coverageRadius: 3, smokeRadius: 2 });
  });
  it('three health meters with a number and a word each (colour is never the only cue), from the wire fields', () => {
    const s = structuredClone(base());
    s.nature.forestHealth = 0.82; s.nature.commonsHealth = 0.5; s.nature.farmHealth = 0.1;
    const m = healthMeters(s);
    expect(m.map((x) => x.id)).toEqual(['forest', 'commons', 'farm']);
    expect(m.map((x) => x.word)).toEqual(['Healthy', 'Strained', 'Failing']);
    expect(m.every((x) => x.value >= 0 && x.value <= 1 && x.hint.length > 0)).toBe(true);
    expect([0.9, 0.6, 0.5, 0.2, 0.1].map(healthLevel)).toEqual(['healthy', 'healthy', 'strained', 'poor', 'failing']);
  });
  it('falls back to the stock shares for a package without health fields, and to nothing without a snapshot', () => {
    const s = structuredClone(base()) as unknown as { nature: Record<string, unknown>; weather?: unknown };
    delete s.nature.forestHealth; delete s.nature.commonsHealth; delete s.nature.farmHealth; delete s.weather;
    const m = healthMeters(s as unknown as Snapshot);
    expect(m[0]!.value).toBe((s.nature as { forest: number }).forest);
    expect(weatherOf(s as unknown as Snapshot)).toBeNull();
    expect(forecastView(s as unknown as Snapshot)).toBeNull();
    expect(stressGauge(null)).toBeNull();
    expect(healthMeters(null)).toEqual([]);
  });
});

describe('forecast card derivation', () => {
  it('uses calm wording and a risk level word', () => {
    expect(forecastView(withWeather({ stress: 0, fireRisk: 'low', droughtRisk: 'low' }))).toMatchObject({ headline: 'Weather calm', calm: true, adapt: false, level: 'low' });
    expect(forecastView(withWeather({ droughtRisk: 'high' }))).toMatchObject({ headline: 'Dry spell likely', droughtWord: 'High', adapt: true, level: 'high' });
    expect(forecastView(withWeather({ fireRisk: 'high' }))!.headline).toBe('Elevated fire risk');
    expect(forecastView(withWeather({ fireRisk: 'medium' }))!.headline).toBe('Fire risk is building');
    expect(forecastView(withWeather({ droughtRisk: 'medium' }))!.headline).toBe('A dry spell is possible');
  });
  it('active fires and droughts lead the headline and offer Adapt', () => {
    const t = forestTile(base());
    const f = forecastView(withWeather({ fires: [{ x: t.x, y: t.y }], drought: { severity: 0.5, monthsLeft: 1 } }))!;
    expect(f.headline).toMatch(/Forest fire burning on 1 tile\. Drought under way, 1 month left/);
    expect(f.adapt).toBe(true);
    expect(f.calm).toBe(false);
    expect(f.active).toEqual({ fire: 1, drought: true });
  });
});

describe('event formatting (labels, never ids)', () => {
  const s = (() => { const x = withWeather({}); return x; })();
  const home = s.map.tiles.find((t) => t.kind === 'home')!;
  it('a fire is described as "Forest fire near <nearest building>" or coordinates', () => {
    const near = { x: home.x + 1, y: home.y };
    const name = nearestLandmark(s, near)!;
    expect(name).toBeTruthy();
    expect(formatEvent(ev({ type: 'fire_started', importance: 3, tile: near }), s)).toBe(`Forest fire near ${name} started`);
    expect(fireLabel(s, near)).toBe(`Forest fire near ${name}`);
    expect(formatEvent(ev({ type: 'fire_out', importance: 2, tile: near }), s)).toMatch(/^Forest fire near .* is out$/);
    expect(formatEvent(ev({ type: 'fire_spread', importance: 2, from: near, to: { x: 1, y: 2 } }), s)).toMatch(/spread to \(1,2\)/);
    expect(formatEvent(ev({ type: 'forest_burned', importance: 2, tile: near }), s)).toMatch(/burned down near/);
  });
  it('far from every building it says coordinates', () => {
    const lonely = structuredClone(s);
    lonely.map.tiles.forEach((t) => { if (t.kind !== 'forest' && t.kind !== 'commons' && t.kind !== 'empty') t.kind = 'empty'; });
    expect(formatEvent(ev({ type: 'fire_started', importance: 3, tile: { x: 5, y: 5 } }), lonely)).toBe('Forest fire near (5,5) started');
  });
  it('droughts and damaged homes', () => {
    expect(formatEvent(ev({ type: 'drought_started', importance: 3, severity: 0.8, months: 4 }), s)).toBe('A severe drought began and may last about 4 months');
    expect(formatEvent(ev({ type: 'drought_ended', importance: 2 }), s)).toBe('The drought ended');
    expect([0.1, 0.5, 0.9].map(severityWord)).toEqual(['mild', 'moderate', 'severe']);
    const h = s.homes.find((x) => x.household !== null)!;
    expect(formatEvent(ev({ type: 'home_damaged', importance: 2, home: h.id }), s)).toMatch(/home was damaged by fire$/);
    expect(formatEvent(ev({ type: 'home_damaged', importance: 2, home: 999 }), s)).toBe('Home 999 was damaged by fire');
  });
  it('events rank by importance on the results screen and keep their wording', () => {
    const events: SimEvent[] = [ev({ type: 'fire_out', importance: 2, tile: { x: 1, y: 1 } }), ev({ type: 'fire_started', importance: 3, tile: { x: 1, y: 1 } }), ev({ type: 'drought_ended', importance: 1 })];
    expect(rankEvents(events).map((e) => e.type)).toEqual(['fire_started', 'fire_out', 'drought_ended']);
    expect(eventLines(events, s).lines[0]!.text).toMatch(/Forest fire near .* started/);
  });
});

describe('crisis banner derivation', () => {
  it('no weather crisis when calm', () => {
    expect(deriveCrises(withWeather({}), null).map((c) => c.id)).not.toContain('fire_active');
    expect(deriveCrises(withWeather({}), null).map((c) => c.id)).not.toContain('drought_active');
  });
  it('an active fire and an active drought each get one calm, non-critical banner with the right shortcuts', () => {
    const t = forestTile(base());
    const snap = withWeather({ fires: [{ x: t.x, y: t.y }, { x: t.x, y: t.y }], drought: { severity: 0.7, monthsLeft: 2 } });
    const cs = deriveCrises(snap, null);
    const fire = cs.find((c) => c.id === 'fire_active')!, dry = cs.find((c) => c.id === 'drought_active')!;
    expect(fire).toMatchObject({ severity: 'warning', why: 'emissions', adapt: true });
    expect(fire.title).toMatch(/^Forest fire near .*\(2 tiles burning\)$/);
    expect(fire.show).toEqual(fireSelection(snap));
    expect(dry).toMatchObject({ severity: 'warning', adapt: true });
    expect(dry.title).toBe('A severe drought is under way');
    expect(dry.detail).toMatch(/2 more months/);
  });
});

describe('metrics, why-bars and the end story', () => {
  it('has forest, commons and farm health and climate stress with sparklines from history', () => {
    const snap = ok(createMockSim().init(1));
    for (const id of ['forest_health', 'commons_health', 'farm_health', 'climate_stress']) {
      const m = METRICS.find((x) => x.id === id)!;
      expect(m.tab).toBe('Ecology');
      expect(Number.isFinite(m.value(snap))).toBe(true);
      expect(m.series).not.toBeNull();
    }
    const sim = createMockSim();
    ok(sim.init(1));
    const r1 = ok(sim.submitTurn([])), r2 = ok(sim.submitTurn([]));
    const h = r2.snapshot.history;
    for (const id of ['forest_health', 'climate_stress']) {
      const m = METRICS.find((x) => x.id === id)!;
      expect(seriesOf(m, h)).toHaveLength(2);
      expect(historyChange(m, h)).not.toBeNull();
    }
    expect(r1.snapshot.history).toHaveLength(1);
  });
  it('seriesOf is empty (not NaN) for a package whose history lacks the fields', () => {
    const m = METRICS.find((x) => x.id === 'farm_health')!;
    const old = [{ turn: 1 }, { turn: 2 }] as unknown as Snapshot['history'];
    expect(seriesOf(m, old)).toEqual([]);
    expect(historyChange(m, old)).toBeNull();
  });
  it('the emissions "why" includes sink lost to fire as an emission', () => {
    const trace: TraceRecord[] = [
      { type: 'emissions', source: 'plant', id: 0, tile: { x: 1, y: 1 }, tonnes: 20 }, { type: 'emissions', source: 'sink', tonnes: -3 }, { type: 'emissions', source: 'fire_sink_loss', tonnes: 5 },
    ];
    const bars = emissionsBars(trace, base());
    const fire = bars.find((b) => b.label === 'Sink lost to fire')!;
    expect(fire.value).toBe(5);
    expect(fire.sub).toMatch(/ t emitted/);
    expect(whyData('emissions', trace, base(), []).groups[0]!.bars.map((b) => b.label)).toContain('Sink lost to fire');
  });
  it('the policy drawer has resilience spending in the climate and energy group', () => {
    const sim = createMockSim();
    ok(sim.init(1));
    const g = groupLevers(sim.levers());
    expect(g.find((x) => x.group.label === 'Climate and energy')!.levers.map((l) => l.key)).toContain('resilience_spending');
    expect(g.map((x) => x.group.id)).not.toContain('other');
  });
  const full = (() => { const sim = createMockSim(); ok(sim.init(1)); return ok(sim.submitTurn([])).snapshot.history[0]!; })();
  const hist = (rows: [number, number, number, number, number][]): Snapshot['history'] => rows.map(([turn, climateStress, forestHealth, commonsHealth, farmHealth]) => ({ ...full, turn, climateStress, forestHealth, commonsHealth, farmHealth }));
  it('story mentions the fires and droughts that mattered (templates, numbers from history)', () => {
    const snap = structuredClone(base());
    snap.history = hist([[1, 0, 0.9, 0.9, 0.9], [2, 0.2, 0.9, 0.9, 0.9], [3, 0.5, 0.6, 0.85, 0.7], [4, 0.7, 0.55, 0.6, 0.45], [5, 0.9, 0.55, 0.55, 0.3]]);
    const facts = weatherFacts(snap, null);
    expect(facts[0]).toMatch(/peaked at 90%/);
    expect(facts.join(' ')).toMatch(/Forest fires mattered most in .*forest health fell from 90% to 60%/);
    expect(facts.join(' ')).toMatch(/Droughts hit the farm and the commons/);
    expect(facts.join(' ')).toMatch(/Forest health ended at 55%/);
    snap.gameOver = { turn: 5, outcome: 'budget_exhausted' };
    expect(generateStory(snap, null).paragraphs.join(' ')).toMatch(/Forest fires mattered most/);
  });
  it('story says so when stress never began, and stays silent for a package without the history fields', () => {
    const snap = structuredClone(base());
    snap.history = hist([[1, 0, 0.9, 0.9, 0.9], [2, 0, 0.9, 0.9, 0.9]]);
    expect(weatherFacts(snap, null)[0]).toMatch(/never began/);
    snap.history = [{ turn: 1 }, { turn: 2 }] as unknown as Snapshot['history'];
    expect(weatherFacts(snap, null)).toEqual([]);
  });
  it('counts a fire that started in the final turn', () => {
    const snap = structuredClone(base());
    snap.history = hist([[1, 0.3, 0.9, 0.9, 0.9], [2, 0.5, 0.9, 0.9, 0.9]]);
    const frame: FrameInput = { month: 0, events: [ev({ type: 'fire_started', importance: 3, tile: { x: 1, y: 1 } })], flows: {} };
    const last = { months: [frame] } as unknown as TurnResult;
    expect(weatherFacts(snap, last).join(' ')).toMatch(/A fire started in the final turn/);
  });
});

describe('inspector and palette for the new things', () => {
  it('a burning forest tile says so and says how long; a regrowing one is described', () => {
    const t = forestTile(base());
    const burning = withWeather({ fires: [{ x: t.x, y: t.y, left: 2 }], fireRisk: 'high' });
    const m = resolveTile(burning, layoutOf(burning), { x: t.x, y: t.y }, []);
    expect(m.title).toBe('Forest fire');
    expect(m.sections[0]!.rows.map((r) => `${r.label}: ${r.value}`)).toEqual(expect.arrayContaining(['This tile: BURNING', 'Fire ends in: about 2 months', 'Fire risk: High']));
    const regrow = withWeather({ regrowing: [{ x: t.x, y: t.y }] });
    expect(resolveTile(regrow, layoutOf(regrow), { x: t.x, y: t.y }, []).title).toBe('Burnt forest, regrowing');
  });
  it('a fire station is described and counted from the tiles', () => {
    const snap = structuredClone(base());
    const e = snap.map.tiles.find((t) => t.kind === 'empty')!;
    e.kind = 'fire_station';
    expect(builtCount(snap, 'fire_station')).toBe(1);
    const m = resolveTile(snap, layoutOf(snap), { x: e.x, y: e.y }, []);
    expect(m.title).toBe('Fire station');
    expect(m.removable).toMatchObject({ kind: 'fire_station' });
    expect(m.sections[0]!.rows[0]!.value).toMatch(/3 tiles/);
  });
});
