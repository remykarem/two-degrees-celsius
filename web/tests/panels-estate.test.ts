// Town v7 panels (contract v1.9): the estate card, inspector rows, Gini table, finance row, emissions line, crisis and wording. Real wire fixtures and the mock.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { Snapshot } from '../src/contract';
import { createSimPort } from '../src/sim-port';
import { createGame } from '../src/store';
import EstateCard from '../src/panels/EstateCard.svelte';
import GiniView from '../src/panels/GiniView.svelte';
import IndustryPanel from '../src/panels/IndustryPanel.svelte';
import PoliciesPanel from '../src/panels/PoliciesPanel.svelte';
import { estateCard, giniRows, pilotView, topHouseholdLine, wealthTaxShortfall } from '../src/panels/estate';
import { emissionsView } from '../src/panels/emissions';
import { ownershipText } from '../src/panels/industry';
import { financeSections } from '../src/panels/finance';
import { deriveCrises } from '../src/panels/crisis';
import { formatEvent } from '../src/panels/format';
import { resolveSelection } from '../src/panels/inspect';
import { peopleGroups, unemploymentCount } from '../src/panels/people';
import { generateStory } from '../src/panels/story';
import { isBarePercent } from '../src/panels/backing';
import { layoutOf } from '../src/store';
import { ok } from './helpers';

const real = (): Snapshot => JSON.parse(readFileSync(resolve(__dirname, '../../docs/fixtures/snapshot-turn1.json'), 'utf8')) as Snapshot;
const html = (c: unknown, props: Record<string, unknown>) => render(c as Component<Record<string, unknown>>, { props }).body;
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

describe('estate card', () => {
  it('names the owner, the band, the counts and the pilot from the wire', () => {
    const s = real();
    const c = estateCard(s)!;
    expect(c.tag).toBe('Estate owner');
    expect(c.band).toBe('Ultra-rich');
    expect(c.topLine).toMatch(/Estate owner \(1 of 9 households, 1 of 22 people\) holds \$.+ of the town's \$.+ net worth\./);
    expect(c.wealthParts.map((p) => p.label)).toContain('Private jet');
    expect(c.pilot?.statusText).toBe('Employed');
    expect(c.pilot?.rows.find((r) => r.label === 'Wage')?.value).toBe('$1.8k/month');
    expect(c.flights).toBe('No flights yet this game');
    for (const t of [c.topLine ?? '', c.flights]) expect(isBarePercent(t)).toBe(false);
  });
  it('shows flights beside the town total, the unpaid warning and the grounded note', () => {
    const s = clone(real());
    s.estate!.flightsTotal = 3; s.estate!.tonnesTotal = 6;
    expect(estateCard(s)!.flights).toContain('Flights this game: 3, 6 t CO2');
    s.estate!.pilot!.unpaidMonths = 2;
    expect(pilotView(s)!.warning).toContain('2 of 3 months');
    s.estate!.pilot!.status = 'let_go';
    expect(pilotView(s)!.warning).toBeNull();
    expect(pilotView(s)!.grounded).toContain('grounded');
    expect(deriveCrises(s, null).map((x) => x.id)).toContain('pilot_let_go');
    expect(html(EstateCard, { game: createGame(createSimPort()), snap: s })).toContain('pilot-grounded');
  });
  it('shows a wealth tax shortfall only when the savings fall short of the wire\'s owed amount', () => {
    const s = clone(real());
    s.estate!.finance.wealthTaxOwed = 5000;
    s.households.find((h) => h.id === s.estate!.household)!.savings = 10;
    const x = wealthTaxShortfall(s)!;
    expect(x.owed).toBeGreaterThan(x.paid);
    expect(estateCard(s)!.shortfall).toContain('paid in part');
    s.households.find((h) => h.id === s.estate!.household)!.savings = 1e9;
    expect(wealthTaxShortfall(s)).toBeNull();
  });
});

describe('inspector, people and counts', () => {
  it('the pilot and the estate owner have their own rows; the pilot is employed, not unemployed', () => {
    const s = real();
    const layout = layoutOf(s);
    const pilot = s.people.find((p) => p.id === s.estate!.pilot!.person)!;
    const pm = resolveSelection(s, layout, { kind: 'person', id: pilot.id }, [])!;
    expect(pm.sections.flatMap((x) => x.rows).find((r) => r.label === 'Job')!.value).toBe('Employer: Estate owner (household)');
    expect(pm.sections.flatMap((x) => x.rows).map((r) => r.label)).toEqual(expect.arrayContaining(['Status', 'Short-paid months', 'Flights flown (estate jet)']));
    const owner = s.people.find((p) => p.household === s.estate!.household)!;
    const om = resolveSelection(s, layout, { kind: 'person', id: owner.id }, [])!;
    expect(om.subtitle).toContain('Estate owner');
    expect(om.sections[0]!.rows[0]!.value).toContain('Does not work');
    expect(om.sections.flatMap((x) => x.rows).some((r) => r.label === 'Employs')).toBe(true);
    expect(peopleGroups(s).flatMap((g) => g.people).find((p) => p.id === pilot.id)!.detail).toBe('Employer: Estate owner (household)');
    expect(unemploymentCount(s).unemployed).toBe(s.people.filter((p) => p.inLabourForce && !p.job && !p.employer).length);
  });
  it('the Gini table is richest first with the tag, the band and counts, not bare percentages', () => {
    const s = real();
    const rows = giniRows(s);
    expect(rows[0]!.tag).toBe('Estate owner');
    expect(rows[0]!.shareText).toMatch(/^\$.+ of \$.+$/);
    expect(rows[0]!.parts).toContain('Private jet');
    const out = html(GiniView, { snap: s });
    expect(out).toContain('Estate owner');
    expect(out).toContain('gini-top-line');
    expect(topHouseholdLine(s)).toContain('1 of 9 households');
  });
  it('industry names the estate household that owns the Farm', () => {
    const mock = ok(createSimPort().init(1));
    const farm = mock.industry!.firms.find((f) => f.ownership.some((o) => o.household === mock.estate!.household))!;
    expect(farm).toBeTruthy();
    expect(ownershipText(farm, mock.households)).toMatch(/Estate owner 0?\.\d+/);
    expect(html(IndustryPanel, { game: createGame(createSimPort()) })).toBeDefined();
  });
});

describe('money, emissions, events and the mock', () => {
  it('finance has the domestic wages memo row once wages are paid', () => {
    const s = clone(real());
    for (const x of s.finance.households.income) { if (x.id === 'wages') x.amount = 5000; if (x.id === 'domestic_wages') x.amount = 1800; }
    const row = financeSections(s).flatMap((x) => x.rows).find((r) => r.id === 'inc-domestic_wages');
    expect(row?.month).toBe(1800);
    expect(row?.label).toContain('domestic wages');
  });
  it('the emissions table adds the aviation line and still reconciles', () => {
    const s = clone(real());
    const e = s.breakdowns!.emissions;
    for (const l of [e.month, e.turn]) { l.sources = l.sources.filter((x) => x.id !== 'aviation').concat({ id: 'aviation', label: 'Private jet flight', category: 'aviation', tonnes: 2 }); l.grossTotal += 2; l.net += 2; l.headline += 2; }
    e.reconciles = true;
    const v = emissionsView(e);
    expect(v.rows.some((r) => r.id === 'aviation')).toBe(true);
    expect(v.reconciliation.ok).toBe(true);
  });
  it('events and the story are worded from content', () => {
    const s = real();
    expect(formatEvent({ type: 'private_jet_flight', household: 8, tonnes: 2, importance: 2 }, s)).toBe('Private jet flight from the estate: 2 t CO2');
    expect(formatEvent({ type: 'pilot_let_go', person: 18, importance: 3 }, s)).toContain('let them go');
    expect(formatEvent({ type: 'pilot_hired', person: 18, importance: 2 }, s)).toContain('hired its pilot back');
    const withHistory = clone(s); withHistory.estate!.flightsTotal = 4; withHistory.estate!.tonnesTotal = 8;
    withHistory.history = [clone(s.history[0] ?? ({ turn: 1, gini: 0.5, unemployment: 0.1, legitimacy: 0.5, gdpRealAnnual: 1, budgetShare: 1, emissions: 1 } as never)), clone(s.history[0] ?? ({ turn: 2, gini: 0.6, unemployment: 0.1, legitimacy: 0.5, gdpRealAnnual: 1, budgetShare: 1, emissions: 1 } as never))];
    const story = generateStory(withHistory, null).paragraphs.join(' ');
    expect(story).toContain('Estate owner (1 of 9 households');
    expect(story).toContain('Flights this game: 4');
  });
  it('the mock carries what the panels render: parts, estate finance, domestic wages and the policies note', () => {
    const port = createSimPort();
    const s = ok(port.init(2));
    expect(s.breakdowns!.gini.byHousehold.find((h) => h.household === s.estate!.household)!.parts?.length).toBeGreaterThan(0);
    expect(s.estate!.finance.last12).toBeDefined();
    expect(s.finance.households.income.some((x) => x.id === 'domestic_wages')).toBe(true);
    expect(s.breakdowns!.emissions.month.sources.some((x) => x.id === 'households')).toBe(true);
    expect(html(PoliciesPanel, { game: createGame(port) })).toBeDefined();
  });
});
