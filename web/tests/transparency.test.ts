import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { Breakdowns, Snapshot } from '../src/contract';
import { createMockSim } from '../src/sim-port/mock';
import { fmtUnit, popoverModel, sumCheck, breakdownOf } from '../src/panels/explain';
import { cumulativeSeries, emissionsView } from '../src/panels/emissions';
import { lorenzPath } from '../src/panels/lorenz';
import { byGroup, pendingLines, policyLines, timelineLines } from '../src/panels/policies';
import { firmLines, infraLines, noStateOwnership, ownershipText } from '../src/panels/industry';
import { anchorsWithoutTopic, groupTopics, relatedOf, searchTopics, topicFor } from '../src/panels/explorer';
import { METRICS, metricText } from '../src/panels/metrics';
import { isBarePercent, pctWith } from '../src/panels/backing';
import { energyBalance } from '../src/panels/energy';
import { financeSections, fmtFin } from '../src/panels/finance';
import { text } from '../src/store';
import { taxLines } from '../src/panels/taxes';
import { relabelLevers, uiLabel } from '../src/store';
import { ok } from './helpers';

const sim = createMockSim();
const mk = () => { const s = createMockSim(); return { s, snap: ok(s.init(3)) }; };
const play = (s: ReturnType<typeof createMockSim>, n: number, cmds: Parameters<typeof s.submitTurn>[0] = []) => { let r = ok(s.submitTurn(cmds)); for (let i = 1; i < n; i++) r = ok(s.submitTurn([])); return r.snapshot; };
const { s: simA, snap: start } = mk();
const played = play(simA, 3);
const bd = played.breakdowns as Breakdowns;

describe('breakdown popover logic', () => {
  it('formats values with units', () => {
    expect(fmtUnit(1200, 'USD')).toBe('$1.2k');
    expect(fmtUnit(0.62, 'share')).toBe('0.620 (62.0%)');
    expect(fmtUnit(1500, 'tCO2')).toBe('1.5k t');
    // units as the real wire spells them
    expect(fmtUnit(386445, '$/year')).toBe('$386k per year');
    expect(fmtUnit(60, '$/t CO2')).toBe('$60 per t CO2');
    expect(fmtUnit(0.694, '0..1')).toBe('0.694');
    expect(fmtUnit(-0.00001, '0..1')).toBe('0.000');
    expect(fmtUnit(0, '0 or 1')).toBe('0');
    expect(fmtUnit(12.5, 't CO2')).toBe('12.5 t');
    expect(fmtUnit(8, 'people')).toBe('8 people');
    expect(fmtUnit(0.001, 't CO2 per month per unit')).toBe('0.001 t CO2 per month per unit');
  });
  it('builds a model with formula, inputs, result and parts that sum', () => {
    const m = popoverModel(bd.gdp);
    expect(m.formula).toMatch(/GDP/);
    expect(m.inputs.length).toBeGreaterThan(1);
    expect(m.parts.length).toBeGreaterThan(0);
    expect(m.sums?.ok).toBe(true);
    expect(m.resultText).toMatch(/^\$/);
  });
  it('every mock breakdown with parts sums to its result and says so', () => {
    for (const k of ['gdp', 'gpi', 'wageShare', 'profitShare', 'debtToGdp', 'gini', 'legitimacy', 'capitalRatio'] as const) {
      const b = bd[k];
      expect(b.inputs.length, k).toBeGreaterThan(0);
      if (b.parts) { expect(sumCheck(b)?.ok, k).toBe(true); expect(b.sumsToResult, k).toBe(true); }
    }
  });
  it('a failing sum is detected and not hidden', () => {
    const c = sumCheck({ parts: [{ label: 'a', value: 1, unit: 'USD' }, { label: 'b', value: 2, unit: 'USD' }], result: { value: 4, unit: 'USD' }, sumsToResult: false });
    expect(c?.ok).toBe(false);
    expect(c?.wireFlag).toBe(false);
    expect(sumCheck({ result: { value: 1, unit: 'USD' } })).toBeNull();
  });
  it('finds a breakdown by key and none on an old package', () => {
    expect(breakdownOf(played, 'gdp')?.id).toBe('gdp');
    expect(breakdownOf({ ...played, breakdowns: undefined }, 'gdp')).toBeNull();
  });
  it('the legitimacy terms add up to the trend and each is weight x (input - threshold)', () => {
    const l = bd.legitimacy;
    expect(l.terms.reduce((a, t) => a + t.contribution, 0)).toBeCloseTo(l.trend, 9);
    for (const t of l.terms) expect(t.contribution).toBeCloseTo(t.weight * (t.input - (t.threshold ?? 0)), 9);
  });
});

describe('emissions reconciliation display', () => {
  it('shows month, turn, sources, sink, net and cumulative from the wire, reconciled', () => {
    const v = emissionsView(bd.emissions);
    expect(v.rows.map((r) => r.id)).toEqual(['firms', 'households', 'plants', 'gross', 'sink', 'net']);
    expect(v.reconciliation.ok).toBe(true);
    expect(v.reconciliation.text).toMatch(/gross/);
    expect(v.cumulative.used).toMatch(/ t$/);
    expect(bd.emissions.turn.net).toBeGreaterThan(bd.emissions.month.net); // 3 months vs 1
    expect(bd.emissions.month.headline).toBe(bd.emissions.month.net);
  });
  it('shows a failed reconciliation when the numbers disagree, even if the flag says true', () => {
    const e = structuredClone(bd.emissions);
    e.month.grossTotal += 10;
    const v = emissionsView(e);
    expect(v.reconciliation.ok).toBe(false);
    expect(v.reconciliation.wireFlag).toBe(true);
    expect(emissionsView({ ...bd.emissions, reconciles: false }).reconciliation.ok).toBe(false);
  });
  it('the metric row uses the reconciled headline, not the old gross figure', () => {
    const m = METRICS.find((x) => x.id === 'emissions')!;
    expect(m.value(played)).toBe(bd.emissions.month.headline);
    expect(m.detail!(played)).toMatch(/accumulated/);
  });
  it('builds the cumulative series against the budget', () => {
    const series = cumulativeSeries(played.history, bd.emissions.cumulative.budget);
    expect(series.length).toBe(played.history.length);
    expect(series.at(-1)!.tonnes).toBeCloseTo(bd.emissions.cumulative.tonnes, 6);
  });
});

describe('Lorenz data to path', () => {
  it('starts at the origin, ends at the top right and is convex (never above equality)', () => {
    const l = (bd.gini as Breakdowns['gini']).lorenz;
    expect(l[0]).toEqual({ populationShare: 0, wealthShare: 0 });
    expect(l.at(-1)!.wealthShare).toBeCloseTo(1, 9);
    for (const p of l) expect(p.wealthShare).toBeLessThanOrEqual(p.populationShare + 1e-9);
  });
  it('turns points into an SVG path with the y axis flipped', () => {
    const p = lorenzPath([{ populationShare: 0, wealthShare: 0 }, { populationShare: 0.5, wealthShare: 0.1 }, { populationShare: 1, wealthShare: 1 }], 100, 100, 0);
    expect(p.curve).toBe('M0.0 100.0 L50.0 90.0 L100.0 0.0');
    expect(p.equality).toBe('M0.0 100.0 L100.0 0.0');
    expect(p.area.endsWith('Z')).toBe(true);
    expect(lorenzPath([], 10, 10).curve).toMatch(/^M/);
  });
  it('lists wealth by household', () => {
    const g = bd.gini as Breakdowns['gini'];
    expect(g.byHousehold.length).toBe(played.households.length);
    expect(g.byHousehold[0]!.netWorth).toBe(played.households[0]!.netWorth);
  });
});

describe('Policies tab derivation', () => {
  const { s, snap } = mk();
  const levers = s.levers();
  it('starts with initial = active and nothing pending or in the timeline', () => {
    const lines = policyLines(snap.policies!, levers);
    expect(lines.length).toBe(levers.length);
    expect(lines.every((l) => !l.changedFromInitial && !l.pending)).toBe(true);
    expect(snap.policies!.timeline).toEqual([]);
  });
  it('queues, activates and cancels with a timeline, and shows pending with its turn', () => {
    const tax = levers.find((l) => l.key === 'tax_wage')!;
    const cur = Number(snap.policy.tax_wage.nominal);
    const turn1 = ok(s.beginTurn([{ type: 'set_policy', lever: 'tax_wage', value: cur + 0.05 }]));
    for (let i = 0; i < 3; i++) ok(s.advanceMonth());
    const after = ok(s.finishTurn()).snapshot;
    const lines = policyLines(after.policies!, levers);
    const row = lines.find((l) => l.lever === 'tax_wage')!;
    expect(turn1.accepted.length).toBe(1);
    expect(row.label).toBe('Income tax');
    expect(tax.label).toBe('Income tax');
    expect(row.pending?.text).toBeTruthy();
    expect(pendingLines(lines).length).toBe(1);
    const queued = timelineLines(after.policies!, levers);
    expect(queued[0]!.kind).toBe('queued');
    expect(ok(s.cancelPending(after.policies!.rows.find((r) => r.lever === 'tax_wage')!.pending!.id)).snapshot.policies!.timeline.some((e) => e.kind === 'cancelled')).toBe(true);
  });
  it('activates a change and records last change, active vs initial, in newest-first order', () => {
    const m = mk();
    const cur = Number(m.snap.policy.tax_profit.nominal);
    play(m.s, 3, [{ type: 'set_policy', lever: 'tax_profit', value: Math.min(1, cur + 0.1) }]);
    const sn = ok(m.s.submitTurn([])).snapshot;
    const r = sn.policies!.rows.find((x) => x.lever === 'tax_profit')!;
    expect(r.initial).toBe(cur);
    expect(Number(r.active)).not.toBe(cur);
    expect(r.lastChangeTurn).not.toBeNull();
    expect(r.pending).toBeNull();
    const tl = timelineLines(sn.policies!, m.s.levers());
    expect(tl.map((e) => e.kind)).toContain('activated');
    for (let i = 1; i < tl.length; i++) expect(tl[i - 1]!.turn).toBeGreaterThanOrEqual(tl[i]!.turn);
    expect(byGroup(policyLines(sn.policies!, m.s.levers())).length).toBeGreaterThan(2);
  });
  it('flags a lever weakened by lobbying with the numbers beside it', () => {
    const rows = played.policies!.rows.filter((r) => r.weakenedBy !== null && r.weakenedBy > 0);
    const lines = policyLines(played.policies!, sim.levers());
    for (const r of rows) expect(lines.find((l) => l.lever === r.lever)!.weakened).toMatch(/in force/);
  });
});

describe('Industry derivation', () => {
  it('lists firms with the ownership split and says the state share is zero', () => {
    const ind = played.industry!;
    expect(noStateOwnership(ind)).toBe(true);
    const lines = firmLines(ind);
    expect(lines.length).toBe(played.firms.filter((f) => f.active).length);
    expect(lines[0]!.cells.map((c) => c.label)).toEqual(['Output', 'Price', 'Workers', 'Wage', 'Revenue', 'Profit', 'Emissions']);
    expect(lines[0]!.state).toMatch(/no shares/);
    expect(ownershipText(ind.firms[0]!)).toMatch(/Household/);
    for (const f of ind.firms) expect(f.ownership.reduce((a, o) => a + o.share, 0)).toBeCloseTo(1, 9);
    expect(ind.note).toMatch(/state/i);
  });
  it('lists infrastructure with count, cost and upkeep, and public goods with their spend', () => {
    const infra = infraLines(played.industry!);
    expect(infra.map((i) => i.kind)).toContain('road');
    expect(infra.find((i) => i.kind === 'road')!.upkeepTotal).toMatch(/month/);
    expect(played.industry!.publicGoods.map((g) => g.id)).toEqual(expect.arrayContaining(['publicSpending', 'jobGuarantee', 'benefits', 'ubi']));
    for (const g of played.industry!.publicGoods) expect(g.buys.length).toBeGreaterThan(5);
  });
  it('selecting a firm and an infrastructure row goes through the store (selectFirstOf)', async () => {
    const { createGame } = await import('../src/store');
    const game = createGame(createMockSim());
    game.newGame(2);
    expect(game.selectFirstOf('fossil_plant')).toBe(true);
    expect(game.state.get().selection).toMatchObject({ kind: 'tile' });
    expect(game.selectFirstOf('fire_station')).toBe(false);
  });
});

describe('explorer', () => {
  const topics = ok(sim.assumptions());
  it('has every metric, lever and the panel anchors covered by a topic (usedBy)', () => {
    const anchors = [...new Set(METRICS.map((m) => m.metric)), ...sim.levers().map((l) => l.key), 'energy', 'stress', 'health', 'ledger', 'inspector', 'taxes', 'capital_ratio', 'industry', 'wellbeing'];
    expect(anchorsWithoutTopic(topics, anchors)).toEqual([]);
  });
  it('has well-formed topics: unique ids, resolvable related, status and group', () => {
    expect(new Set(topics.map((t) => t.id)).size).toBe(topics.length);
    for (const t of topics) { expect(relatedOf(topics, t).length, t.id).toBe(t.related.length); expect(['placeholder', 'tuned', 'rough_real_world']).toContain(t.status); expect(t.summary.length).toBeGreaterThan(10); }
    expect(groupTopics(topics).length).toBeGreaterThan(5);
  });
  it('searches and filters, resolves anchors to topics', () => {
    expect(searchTopics(topics, 'emissions').map((t) => t.id)).toContain('emissions');
    expect(searchTopics(topics, 'gst').length).toBeGreaterThan(0);
    expect(searchTopics(topics, 'zzzznothing')).toEqual([]);
    expect(searchTopics(topics, '').length).toBe(topics.length);
    expect(searchTopics(topics, '', 'tuned').every((t) => t.status === 'tuned')).toBe(true);
    expect(topicFor(topics, 'tax_wage')!.id).toBe('taxes');
    expect(topicFor(topics, 'not-an-anchor')).toBeNull();
  });
});

describe('Income tax label and taxes table', () => {
  it('relabels Wage tax everywhere in the UI, keys unchanged', () => {
    expect(uiLabel('Wage tax')).toBe('Income tax');
    expect(relabelLevers([{ ...sim.levers()[1]!, key: 'tax_wage', label: 'Wage tax' }])[0]).toMatchObject({ key: 'tax_wage', label: 'Income tax' });
    expect(text('finance_label', 'revenue.wageTax')).toBe('Income tax');
    expect(financeSections(played).flatMap((s) => s.rows.map((r) => r.label)).join('|')).not.toMatch(/Wage tax/);
  });
  it('has a GST/VAT row that is not modelled, and tax rows with revenue', () => {
    const t = taxLines(played);
    expect(t.map((x) => x.id)).toEqual(['income', 'profit', 'capital', 'wealth', 'carbon', 'gst']);
    expect(t.find((x) => x.id === 'gst')).toMatchObject({ modelled: false, nominal: 'not modelled' });
    expect(played.breakdowns!.taxes.find((x) => x.id === 'income')!.revenueMonth).toBeGreaterThan(0);
  });
});

describe('no bare percentages', () => {
  it('the rule itself', () => {
    expect(isBarePercent('62%')).toBe(true);
    expect(isBarePercent('62% (equity $5k / loans $40k)')).toBe(false);
    expect(isBarePercent('$5k')).toBe(false);
    expect(pctWith(0.5, '$1 of $2')).toBe('50% ($1 of $2)');
  });
  for (const [name, snap] of [['start', start], ['played', played]] as [string, Snapshot][]) {
    it(`every metric string on ${name} that shows a percent also shows its numbers`, () => {
      for (const m of METRICS) {
        const t = metricText(m, snap);
        expect(isBarePercent(t), `${m.id}: ${t}`).toBe(false);
      }
    });
    it(`finance rows with a percent format carry backing numbers (${name})`, () => {
      for (const sec of financeSections(snap)) for (const r of sec.rows) if (r.fmt === 'pct') expect(isBarePercent(`${fmtFin(r.month, 'pct')} (${r.backing ?? ''})`), r.id).toBe(false);
    });
  }
  it('the served share carries supply and demand', () => {
    expect(isBarePercent(energyBalance(played.energy).servedText)).toBe(false);
    expect(energyBalance(played.energy).servedText).toMatch(/supply/);
  });
});

// Contract fixtures written by the Rust builder (skipped, with a pointer, until they exist).
const dir = resolve(__dirname, '../../docs/fixtures');
const exists = (n: string) => existsSync(resolve(dir, `${n}.json`));
describe.skipIf(!(exists('breakdowns') && exists('policies') && exists('industry')))('real fixtures against contract v1.6', () => {
  const load = (n: string) => { let v = JSON.parse(readFileSync(resolve(dir, `${n}.json`), 'utf8')) as Record<string, unknown>; if (v && 'ok' in v && 'value' in v) v = v.value as Record<string, unknown>; return v; };
  it('breakdowns: parts sum to results, emissions reconcile', () => {
    const raw = load('breakdowns');
    const b = (raw.breakdowns ?? raw) as Breakdowns;
    for (const k of ['gdp', 'gpi', 'wageShare', 'profitShare', 'debtToGdp', 'gini', 'legitimacy', 'capitalRatio'] as const) if (b[k].parts) expect(sumCheck(b[k])?.ok, k).toBe(true);
    expect(emissionsView(b.emissions).reconciliation.ok).toBe(true);
  });
  it('policies and industry parse into the Policies and Industry views', () => {
    const p = load('policies'), i = load('industry');
    const board = (p.policies ?? p) as Snapshot['policies'];
    const ind = (i.industry ?? i) as Snapshot['industry'];
    expect(() => policyLines(board!, sim.levers())).not.toThrow();
    expect(() => timelineLines(board!, sim.levers())).not.toThrow();
    expect(() => firmLines(ind!)).not.toThrow();
  });
});
describe.skipIf(!exists('assumptions'))('real fixture against contract v1.5 (docs/fixtures/assumptions.json)', () => {
  const load = () => JSON.parse(readFileSync(resolve(dir, 'assumptions.json'), 'utf8')) as unknown;
  it('assumptions: every topic has the declared fields and every metric and lever has a topic', () => {
    const raw = load();
    const t = ((raw as { value?: unknown }).value ?? (raw as { assumptions?: unknown }).assumptions ?? raw) as ReturnType<typeof sim.assumptions> extends { ok: true; value: infer V } ? V : never;
    expect(anchorsWithoutTopic(t, [...new Set(METRICS.map((m) => m.metric)), ...sim.levers().map((l) => l.key)])).toEqual([]);
  });
});

describe('Pressure mode pause', () => {
  it('the popover and the explorer are modal dialogs (aria-modal), which the Pressure clock treats as a pause, and both say so', async () => {
    const { readFileSync } = await import('node:fs');
    const { PRESSURE_NOTE } = await import('../src/panels/explain');
    for (const f of ['BreakdownModal', 'ExplorerModal']) {
      const src = readFileSync(resolve(__dirname, `../src/panels/${f}.svelte`), 'utf8');
      expect(src).toMatch(/<Modal /);
      expect(src).toMatch(/PRESSURE_NOTE/);
    }
    expect(PRESSURE_NOTE).toMatch(/paused/);
    expect(readFileSync(resolve(__dirname, '../src/ui-kit/Modal.svelte'), 'utf8')).toMatch(/aria-modal="true"/);
    expect(readFileSync(resolve(__dirname, '../src/app/keys.ts'), 'utf8')).toMatch(/\[aria-modal="true"\]/);
  });
});
