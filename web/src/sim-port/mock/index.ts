import type { Command, HistoryPoint, PendingItem, Result, SimEvent, TraceRecord, TurnResult } from '../../contract';
import type { SimPort } from '../port';
import { actionList, GEN, LEVERS, MOCK } from './data';
import { checkOutcome, effectiveValue, remainingBudget, stepMonth, updateLobby } from './dynamics';
import { frame, makeEvents, makeFlows } from './frames';
import { activate, evaluate } from './rules';
import { buildSnapshot } from './snapshot';
import { initialState, SCHEMA_VERSION, type MockState } from './state';

const fail = (code: string, message: string): { ok: false; error: { code: string; message: string } } => ({ ok: false, error: { code, message } });
const ok = <T>(value: T): Result<T> => ({ ok: true, value });
const GAP_LEVERS = ['carbon_price', 'wealth_tax', 'tax_profit', 'emissions_cap'] as const; // as in the Rust policy_gaps

export function createMockSim(): SimPort {
  let s: MockState = initialState(1);
  const snap = () => buildSnapshot(s);

  /** Rust activate_due: runs at the START of a submit, before the ticks. */
  function activateDue(out: PendingItem[], events: SimEvent[]) {
    const due = s.pending.filter((p) => p.effectiveTurn <= s.turn);
    s.pending = s.pending.filter((p) => p.effectiveTurn > s.turn);
    for (const p of due) {
      const blocked = activate(s, p);
      out.push(p);
      if (blocked) events.push({ type: 'policy_blocked', importance: 2, reason: blocked });
      else if (p.command.type === 'switch_system') events.push({ type: 'system_switch_started', importance: 3, to: p.command.to });
      events.push({ type: 'command_activated', importance: 2, id: p.id });
    }
  }

  function historyPoint(): HistoryPoint {
    return {
      turn: s.turn, gdpRealAnnual: s.gdp / s.prices, unemployment: s.unemployment, gini: s.gini,
      emissions: s.lastMonthlyEmissions * GEN.ticksPerTurn, budgetShare: remainingBudget(s) / GEN.carbon.budget,
      legitimacy: s.legitimacy, avgWellbeing: s.wellbeing, gpiAnnual: snap().indicators.gpiAnnual, debtToGdp: s.debt / s.gdp,
    };
  }

  return {
    init(seed) {
      s = initialState(seed);
      return ok(snap());
    },
    levers: () => LEVERS.map(({ default: _d, effect: _e, ...info }) => structuredClone(info)),
    actions: actionList,
    preview(commands) {
      if (!Array.isArray(commands)) return fail('invalid_command', 'commands must be a list');
      return ok(evaluate(s, commands).previews);
    },

    submitTurn(commands: Command[]): Result<TurnResult> {
      if (s.gameOver) return fail('game_over', 'The game is over: no more turns can be played. Start a new game or load a save.');
      if (!Array.isArray(commands)) return fail('invalid_command', 'commands must be a list');
      const turn = s.turn;
      const pcBefore = s.pc;
      const ev = evaluate(s, commands);
      const accepted = ev.accepted;
      const rejected: TurnResult['rejected'] = [];
      const month0: SimEvent[] = [];
      ev.previews.forEach((p, index) => {
        if (p.blocked) {
          rejected.push({ index, command: p.command, reason: p.blocked });
          month0.push({ type: 'policy_blocked', importance: 2, reason: p.blocked });
        }
      });
      s.pending.push(...accepted);
      s.nextPendingId += accepted.length;
      s.pc = ev.pcLeft;
      const activated: PendingItem[] = [];
      activateDue(activated, month0);

      let halted = false;
      const months = ([0, 1, 2] as const).map((m) => {
        if (halted) return frame(m, [], {});
        const sysBefore = s.system;
        const out = stepMonth(s);
        if (turn === MOCK.dynamics.firmFoundedTurn && m === 0) {
          const i = s.firmActive.findIndex((a) => !a);
          if (i >= 0) { s.firmActive[i] = true; month0.push({ type: 'firm_founded', importance: 2, firm: GEN.firms[i]!.id }); }
        }
        const flows = makeFlows(s, out.emissions);
        const events = makeEvents(s, out.unemploymentDelta, flows, m === 0 ? month0 : []);
        if (out.bankFailedNow) events.push({ type: 'bank_failed', importance: 3 });
        if (out.bailoutAmount > 0) events.push({ type: 'bank_bailout', importance: 3, amount: Math.round(out.bailoutAmount) });
        if (s.system !== sysBefore) events.push({ type: 'system_switch_completed', importance: 3, to: s.system });
        const end = checkOutcome(s);
        if (end && !s.gameOver) {
          s.gameOver = { turn, outcome: end.outcome };
          events.push(end.event === 'budget_exhausted' ? { type: 'budget_exhausted', importance: 3 } : { type: 'collapse', importance: 3, reason: end.event });
          halted = true;
        }
        return frame(m, events, flows);
      });

      s.history.push(historyPoint());
      const policyGaps = GAP_LEVERS.map((k) => {
        const l = LEVERS.find((x) => x.key === k)!;
        return { lever: k, nominal: Number(s.policy[l.key]), effective: effectiveValue(s, k) };
      });
      const trace: TraceRecord[] = [
        ...policyGaps.map((g) => ({ type: 'policy_gap' as const, ...g, netLobby: s.netLobby })),
        ...(months[2]!.flows.emissions ?? months[1]!.flows.emissions ?? []).map((e) => ({ type: 'emissions' as const, source: e.source, ...(e.id !== undefined ? { id: e.id } : {}), tonnes: e.tonnes })),
        ...(months[2]!.flows.wages ?? months[1]!.flows.wages ?? []).map((w) => ({ type: 'income' as const, household: w.household, kind: 'wages' as const, amount: w.amount })),
      ];

      // End of turn (Rust: regenerate, then turn += 1). Nothing activates here: items due next turn fire at the start of the next submit.
      s.pc = Math.min(GEN.pc.cap, s.pc + GEN.pc.regenBase * s.legitimacy);
      s.turn++;
      updateLobby(s);
      if (!s.gameOver && turn >= GEN.game.lastTurn) s.gameOver = { turn, outcome: 'survived' };
      return ok({ turn, accepted, rejected, activated, pcBefore, pcAfter: s.pc, months, policyGaps, trace, snapshot: snap() });
    },

    cancelPending(id) {
      const p = s.pending.find((x) => x.id === id);
      if (!p) return fail('unknown_pending', `There is no pending item #${id}.`);
      s.pending = s.pending.filter((x) => x.id !== id);
      const refundPc = p.pcCost * GEN.pc.cancelRefund; // PC only: money is charged when the item takes effect
      s.pc = Math.min(GEN.pc.cap, s.pc + refundPc);
      return ok({ refundPc, snapshot: snap() });
    },

    exportState: () => ok(JSON.stringify(s)),
    importState(blob) {
      let parsed: MockState;
      try { parsed = JSON.parse(blob) as MockState; } catch { return fail('bad_save', 'The saved game data is not valid JSON.'); }
      if (!parsed || typeof parsed !== 'object' || typeof parsed.turn !== 'number' || !parsed.rng || !parsed.policy || !parsed.buildings || !Array.isArray(parsed.pending)) {
        return fail('bad_save', 'The saved game data is incomplete.');
      }
      if (parsed.schemaVersion !== SCHEMA_VERSION) return fail('schema_mismatch', `The save was made by a different version (schema ${String(parsed.schemaVersion)}, expected ${SCHEMA_VERSION}).`);
      s = parsed;
      return ok(snap());
    },
  };
}
