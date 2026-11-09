import type { HistoryPoint, Snapshot, TurnResult } from '../contract';
import { quarterLabel } from '../store/config';
import { emissionsBars } from './why';
import { pct } from './format';

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

function budgetTurnLine(h: HistoryPoint[], snap: Snapshot): string | null {
  const half = h.find((p) => p.budgetShare <= 0.5);
  const quarter = h.find((p) => p.budgetShare <= 0.25);
  if (!half) return h.length ? `The carbon budget was never more than half used (${pct(1 - h[h.length - 1]!.budgetShare)} used at the end).` : null;
  const parts = [`Half the carbon budget was gone by ${when(half.turn, snap)}`];
  if (quarter) parts.push(`three quarters by ${when(quarter.turn, snap)}`);
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
  if (snap.system.current !== 'market' || snap.system.transition) {
    p.push(`The town ended as a ${snap.system.current === 'regulated' ? 'regulated capitalist' : snap.system.current === 'coop' ? 'worker cooperative' : 'market'} economy${snap.system.transition ? ` (still switching to ${snap.system.transition.to})` : ''}.`);
  }
  return { outcome, headline, summary, paragraphs: p };
}
