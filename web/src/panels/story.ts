import type { HistoryPoint, Snapshot, TurnResult } from '../contract';
import { quarterLabel } from '../store/config';
import { fill, text } from '../store/catalog';
import { emissionsBars } from './why';
import { money, pct } from './format';

export interface Story {
  outcome: 'win' | 'lose';
  headline: string;
  summary: string;
  /** One sentence per fact, in a fixed order. Every number comes from history, the last trace or the final snapshot. */
  paragraphs: string[];
}

const when = (turn: number, snap: Snapshot) => quarterLabel(turn, snap.game);

function outcomeText(snap: Snapshot): { outcome: 'win' | 'lose'; headline: string; summary: string } {
  const o = snap.gameOver?.outcome;
  const at = snap.gameOver ? when(Math.min(snap.gameOver.turn, snap.game.lastTurn), snap) : '';
  if (o === 'survived') return { outcome: 'win', headline: 'The town made it', summary: `The town reached the end of the horizon (${when(snap.game.lastTurn, snap)}) with carbon budget to spare.` };
  if (o === 'budget_exhausted') return { outcome: 'lose', headline: 'The carbon budget ran out', summary: `Cumulative emissions used up the whole carbon budget by ${at}.` };
  if (o && typeof o === 'object') {
    const why = o.collapse.replaceAll('_', ' ');
    return { outcome: 'lose', headline: 'The town collapsed', summary: `The town collapsed around ${at}: ${why}.` };
  }
  return { outcome: 'lose', headline: 'The game is not over', summary: 'No ending has been reached yet.' };
}

const dirWord = (from: number, to: number, up = 'rose', down = 'fell', flat = 'stayed about the same') =>
  Math.abs(to - from) < 0.005 ? flat : to > from ? up : down;

function emissionsLine(h: HistoryPoint[], snap: Snapshot): string | null {
  if (h.length < 2) return null;
  let peak = h[0]!;
  for (const p of h) if (p.emissions > peak.emissions) peak = p;
  const last = h[h.length - 1]!;
  if (peak.emissions <= 0) return null;
  if (peak.turn === last.turn) return `Emissions were still at their peak at the end (${when(last.turn, snap)}).`;
  const drop = (peak.emissions - last.emissions) / peak.emissions;
  return drop > 0.005
    ? `Emissions peaked in ${when(peak.turn, snap)} and had fallen ${pct(drop)} from that peak by the end.`
    : `Emissions peaked in ${when(peak.turn, snap)} and never came down.`;
}

/** Milestones are the simulation's own: the budget share where climate stress starts (weather.onset) and where it is fullest (weather.full). */
function budgetTurnLine(h: HistoryPoint[], snap: Snapshot): string | null {
  const w = snap.weather;
  const pctOf = (v: number) => `${Math.round(v * 100)}%`;
  const reached = h.find((p) => p.budgetShare <= w.onset);
  if (!reached) return h.length ? fill(text('story', 'budget_never'), { used: pctOf(1 - h[h.length - 1]!.budgetShare) }) : null;
  const parts = [fill(text('story', 'budget_onset'), { onset: pctOf(w.onset), when: when(reached.turn, snap) })];
  const full = h.find((p) => p.budgetShare <= w.full);
  if (full) parts.push(fill(text('story', 'budget_full'), { full: pctOf(w.full), when: when(full.turn, snap) }));
  return parts.join(', ') + '.';
}

function lobbyingLine(last: TurnResult | null, snap: Snapshot): string | null {
  if (!last) return null;
  const gaps = last.policyGaps;
  if (!gaps.length) return snap.gameOver ? 'In the final turn no lobbied policy was being watered down.' : null;
  const lost = gaps.filter((g) => g.nominal > 0).map((g) => (g.nominal - g.effective) / g.nominal);
  if (!lost.length) return null;
  const avg = lost.reduce((a, b) => a + b, 0) / lost.length;
  const worst = gaps.reduce((w, g) => ((g.nominal - g.effective) / (g.nominal || 1) > (w.nominal - w.effective) / (w.nominal || 1) ? g : w));
  return `Lobbying watered down ${gaps.length} polic${gaps.length === 1 ? 'y' : 'ies'} at the end, by ${pct(Math.max(0, avg))} on average (the biggest gap was on ${worst.lever.replaceAll('_', ' ')}).`;
}

/** Town v2 facts: who earned what, who held the wealth, who lost their home. Skips a fact when the history lacks the field. */
export function townFacts(snap: Snapshot, last: TurnResult | null): string[] {
  const h = snap.history, out: string[] = [];
  const fin = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
  if (h.length >= 2) {
    const a = h[0]!, z = h[h.length - 1]!;
    if (fin(a.medianIncomePerPerson) && fin(z.medianIncomePerPerson) && fin(a.avgIncomePerPerson) && fin(z.avgIncomePerPerson)) {
      out.push(`The median person's income ${dirWord(a.medianIncomePerPerson / (a.medianIncomePerPerson || 1), z.medianIncomePerPerson / (a.medianIncomePerPerson || 1), 'rose', 'fell', 'stayed about the same')} from ${money(a.medianIncomePerPerson)} to ${money(z.medianIncomePerPerson)} a month, while the average went from ${money(a.avgIncomePerPerson)} to ${money(z.avgIncomePerPerson)}.`);
    }
    if (fin(a.topHouseholdWealthShare) && fin(z.topHouseholdWealthShare)) {
      out.push(`The richest household held ${pct(a.topHouseholdWealthShare)} of the town's wealth at the start and ${pct(z.topHouseholdWealthShare)} at the end.`);
    }
  }
  if (!h.length) return out;
  const unhoused = snap.housing.unhousedHouseholds;
  const evictions = (last?.months ?? []).flatMap((m) => m.events).filter((e) => e.type === 'household_evicted').length;
  if (unhoused > 0) out.push(`At the end ${unhoused} household${unhoused === 1 ? ' was' : 's were'} without a home${evictions ? `, ${evictions} of them evicted in the final turn` : ''}.`);
  else if (evictions) out.push(`${evictions} household${evictions === 1 ? ' was' : 's were'} evicted in the final turn.`);
  return out;
}

/** Town v4 facts: the climate stress, and the fires and droughts that moved the forest, commons or farm. Templates only; numbers come from history. Empty for a package without the v1.2 history fields. */
export function weatherFacts(snap: Snapshot, last: TurnResult | null): string[] {
  const h = snap.history, out: string[] = [];
  const fin = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
  if (h.length < 2 || !h.every((p) => fin(p.climateStress) && fin(p.forestHealth) && fin(p.commonsHealth) && fin(p.farmHealth))) return out;
  const peak = h.reduce((m, x) => (x.climateStress > m.climateStress ? x : m));
  if (peak.climateStress > 0.01) out.push(`Climate stress began to bite as the carbon budget ran down; it peaked at ${pct(peak.climateStress)} in ${when(peak.turn, snap)}.`);
  else out.push(fill(text('story', 'stress_never'), { onset: `${Math.round(snap.weather.onset * 100)}%` }));
  const steps = h.slice(1).map((p, i) => ({ turn: p.turn, forest: h[i]!.forestHealth - p.forestHealth, from: h[i]!.forestHealth, to: p.forestHealth, commons: h[i]!.commonsHealth - p.commonsHealth, farm: h[i]!.farmHealth - p.farmHealth }));
  const fires = steps.filter((x) => x.forest > 0.03).sort((a, b) => b.forest - a.forest);
  if (fires.length) {
    const f = fires[0]!;
    out.push(`Forest fires mattered most in ${when(f.turn, snap)}, when forest health fell from ${pct(f.from)} to ${pct(f.to)} in a single quarter${fires.length > 1 ? `; fires cost forest in ${fires.length - 1} other quarter${fires.length === 2 ? '' : 's'} too` : ''}.`);
  }
  const dry = steps.filter((x) => x.forest <= 0.03 && (x.commons > 0.02 || x.farm > 0.04)).sort((a, b) => b.farm + b.commons - (a.farm + a.commons));
  if (dry.length) out.push(`Droughts hit the farm and the commons, worst in ${when(dry[0]!.turn, snap)}, when farm health fell ${pct(Math.max(0, dry[0]!.farm))} in one quarter.`);
  const end = h[h.length - 1]!, low = h.reduce((m, x) => (x.forestHealth < m.forestHealth ? x : m));
  if (low.forestHealth < 0.95 || low.commonsHealth < 0.95) out.push(`Forest health ended at ${pct(end.forestHealth)} (lowest ${pct(low.forestHealth)} in ${when(low.turn, snap)}); commons health ended at ${pct(end.commonsHealth)} and farm health at ${pct(end.farmHealth)}.`);
  const w = snap.weather;
  const nowFires = last?.months.flatMap((m) => m.events).filter((e) => e.type === 'fire_started').length ?? 0;
  if (nowFires) out.push(`${nowFires === 1 ? 'A fire' : `${nowFires} fires`} started in the final turn.`);
  else if (w && w.fires.length) out.push(`${w.fires.length === 1 ? 'A fire was' : `${w.fires.length} fires were`} still burning at the end.`);
  return out;
}

export function generateStory(snap: Snapshot, last: TurnResult | null): Story {
  const { outcome, headline, summary } = outcomeText(snap);
  const h = snap.history;
  const p: string[] = [];

  const bars = emissionsBars(last?.trace ?? [], snap).filter((b) => b.value > 0);
  const total = bars.reduce((a, b) => a + b.value, 0);
  if (bars.length && total > 0) p.push(`The biggest source of emissions at the end was ${bars[0]!.label}, at ${pct(bars[0]!.value / total)} of what was emitted.`);
  const e = emissionsLine(h, snap);
  if (e) p.push(e);
  const b = budgetTurnLine(h, snap);
  if (b) p.push(b);
  const l = lobbyingLine(last, snap);
  if (l) p.push(l);

  if (h.length >= 2) {
    const first = h[0]!, end = h[h.length - 1]!;
    p.push(`Inequality ${dirWord(first.gini, end.gini)}: the Gini went from ${first.gini.toFixed(2)} to ${end.gini.toFixed(2)}.`);
    const peakU = h.reduce((m, x) => (x.unemployment > m.unemployment ? x : m));
    p.push(`Unemployment ${dirWord(first.unemployment, end.unemployment)} from ${pct(first.unemployment, 1)} to ${pct(end.unemployment, 1)}`
      + (peakU.turn !== end.turn && peakU.unemployment - end.unemployment > 0.005 ? `, after peaking at ${pct(peakU.unemployment, 1)} in ${when(peakU.turn, snap)}.` : '.'));
    const minL = h.reduce((m, x) => (x.legitimacy < m.legitimacy ? x : m));
    p.push(`Legitimacy ${dirWord(first.legitimacy, end.legitimacy)} overall; its lowest point was ${pct(minL.legitimacy)} in ${when(minL.turn, snap)}.`);
    p.push(`Real GDP ${dirWord(first.gdpRealAnnual / (first.gdpRealAnnual || 1), end.gdpRealAnnual / (first.gdpRealAnnual || 1), 'grew', 'shrank', 'was flat')} by ${pct(Math.abs(end.gdpRealAnnual / (first.gdpRealAnnual || 1) - 1))} over the run.`);
  }
  p.push(...townFacts(snap, last));
  p.push(...weatherFacts(snap, last));
  if (snap.system.current !== 'market' || snap.system.transition) {
    p.push(`The town ended as a ${snap.system.current === 'regulated' ? 'regulated capitalist' : snap.system.current === 'coop' ? 'worker cooperative' : 'market'} economy${snap.system.transition ? ` (still switching to ${snap.system.transition.to})` : ''}.`);
  }
  return { outcome, headline, summary, paragraphs: p };
}
