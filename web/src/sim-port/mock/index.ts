import type { Command, HistoryPoint, PendingItem, SimEvent, TurnResult } from '../../contract';
import type { SimPort } from '../port';
import { DATA } from './data';
import { remainingBudget, stepMonth } from './dynamics';
import { frame, makeEvents, makeFlows, text } from './frames';
import { activate, evaluate, toPending } from './rules';
import { buildSnapshot } from './snapshot';
import { initialState, type MockState } from './state';

export function createMockSim(): SimPort {
  let s: MockState = initialState(1);

  const snap = () => buildSnapshot(s);

  function activateDue(upTo: number, out: PendingItem[], events: SimEvent[]) {
    const due = s.pending.filter((p) => p.effectiveTurn <= upTo);
    s.pending = s.pending.filter((p) => p.effectiveTurn > upTo);
    for (const p of due) {
      const what = activate(s, p);
      out.push(p);
      events.push({ type: 'command_activated', importance: 2, text: text('command_activated', { what }) });
    }
  }

  function historyPoint(): HistoryPoint {
    return {
      turn: s.turn, gdpRealAnnual: s.gdp / s.prices, unemployment: s.unemployment, gini: s.gini,
      emissions: s.lastMonthlyEmissions * 3, budgetShare: remainingBudget(s) / DATA.dynamics.carbonBudget!,
      legitimacy: s.legitimacy, avgWellbeing: s.wellbeing, gpiAnnual: buildSnapshot(s).indicators.gpiAnnual,
      debtToGdp: s.debt / s.gdp,
    };
  }

  return {
    init(seed) {
      s = initialState(seed);
      return snap();
    },
    levers: () => DATA.levers.map(({ default: _d, effect: _e, moneyPerUnit: _m, oneOff: _o, ...info }) => info),
    preview: (commands) => evaluate(s, commands),

    submitTurn(commands: Command[]): TurnResult {
      if (s.gameOver) throw new Error('game is over');
      const turn = s.turn;
      const pcBefore = s.pc;
      const previews = evaluate(s, commands);
      const accepted: PendingItem[] = [];
      const rejected: TurnResult['rejected'] = [];
      const month0: SimEvent[] = [];
      previews.forEach((p, index) => {
        if (p.blocked) {
          rejected.push({ index, command: p.command, reason: p.blocked });
          month0.push({ type: 'policy_blocked', importance: 2, text: text('policy_blocked', { reason: p.blocked }) });
          return;
        }
        s.pc -= p.pc;
        const item = toPending(s, p);
        s.pending.push(item);
        accepted.push(item);
      });
      const activated: PendingItem[] = [];
      activateDue(turn, activated, month0);

      const months = ([0, 1, 2] as const).map((m) => {
        const { emissions, unemploymentDelta } = stepMonth(s);
        if (turn === DATA.dynamics.firmFoundedTurn && m === 0) {
          s.firmActive = s.firmActive.map(() => true);
          month0.push({ type: 'firm_founded', importance: 2, text: text('firm_founded') });
        }
        const events = makeEvents(s, unemploymentDelta, m === 0 ? month0 : []);
        return frame(m, events, makeFlows(s, emissions));
      });

      s.history.push(historyPoint());
      const trace = DATA.levers
        .filter((l) => l.lobbied && Number(s.policy[l.key]) !== Number(l.default))
        .map((l) => ({ type: 'policy_gap' as const, lever: l.key, nominal: Number(s.policy[l.key]), effective: snap().policy[l.key].effective ?? 0, reason: 'lobbying' }));
      const policyGaps = trace.map(({ lever, nominal, effective }) => ({ lever, nominal, effective }));

      // advance to next turn
      s.turn++;
      s.pc = Math.min(DATA.pc.cap, s.pc + Math.round(DATA.pc.regenBase * s.legitimacy));
      s.netLobby = Math.min(1, Math.max(0, s.netLobby + (s.rng.state % 7 - 3) * 0.004));
      activateDue(s.turn, activated, months[2]!.events);

      if (remainingBudget(s) <= 0) {
        s.gameOver = { turn: s.turn, outcome: 'budget_exhausted' };
        months[2]!.events.push({ type: 'budget_exhausted', importance: 3, text: text('budget_exhausted') });
      } else if (s.legitimacy <= 0) {
        s.gameOver = { turn: s.turn, outcome: { collapse: 'legitimacy_zero' } };
        months[2]!.events.push({ type: 'collapse', importance: 3, text: text('collapse') });
      } else if (s.turn > DATA.totalTurns) {
        s.gameOver = { turn: s.turn, outcome: 'survived' };
      }
      return { turn, accepted, rejected, activated, pcBefore, pcAfter: s.pc, months, policyGaps, trace, snapshot: snap() };
    },

    cancelPending(id) {
      const p = s.pending.find((x) => x.id === id);
      if (!p) return { refundPc: 0, snapshot: snap() };
      s.pending = s.pending.filter((x) => x.id !== id);
      const refundPc = Math.floor(p.pcCost * DATA.pc.cancelRefund);
      s.pc = Math.min(DATA.pc.cap, s.pc + refundPc);
      return { refundPc, snapshot: snap() };
    },

    exportState: () => JSON.stringify(s),
    importState(blob) {
      const parsed = JSON.parse(blob) as MockState;
      if (parsed.schemaVersion !== DATA.schemaVersion) throw new Error(`schemaVersion mismatch: ${parsed.schemaVersion}`);
      s = parsed;
      return snap();
    },
  };
}
