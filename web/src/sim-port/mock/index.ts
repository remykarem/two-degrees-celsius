import type { AccountRef, Command, LedgerQuery, HistoryPoint, MonthFrame, MonthResult, PendingItem, Result, SimEvent, TraceRecord, TurnResult, TurnStart } from '../../contract';
import type { SimPort } from '../port';
import { actionList, GEN, LEVERS, MOCK } from './data';
import { checkOutcome, effectiveValue, remainingBudget, stepMonth, updateLobby, wealthGini } from './dynamics';
import { ledgerPage, personLedger, rollUp } from './ledger';
import { frame, makeEvents, makeFlows } from './frames';
import { stepHouseholds, syncJobs } from './people';
import { activate, evaluate, spend, validTiles } from './rules';
import { buildSnapshot } from './snapshot';
import { healthOf, stepWeather, stressNow } from './weather';
import { buildingCounts, initialState, SCHEMA_VERSION, type MockState } from './state';

const fail = (code: string, message: string): { ok: false; error: { code: string; message: string } } => ({ ok: false, error: { code, message } });
const ok = <T>(value: T): Result<T> => ({ ok: true, value });
const GAP_LEVERS = ['carbon_price', 'wealth_tax', 'tax_profit', 'emissions_cap'] as const; // as in the Rust policy_gaps

export function createMockSim(): SimPort {
  let s: MockState = initialState(1);
  /** The turn in progress (contract section 12). Lives outside MockState: a save is only legal at a turn boundary, so it is never serialised. */
  let open: { turn: number; pcBefore: number; accepted: PendingItem[]; rejected: TurnResult['rejected']; activated: PendingItem[]; month0: SimEvent[]; months: MonthFrame[]; halted: boolean } | null = null;
  const snap = () => ({ ...buildSnapshot(s), turnPhase: { open: open !== null, monthsDone: (open ? open.months.length : 0) as 0 | 1 | 2 | 3 } });
  const turnState = (message: string) => fail('turn_state', message);

  /** Rust activate_due: runs at the START of a submit, before the ticks. */
  function activateDue(out: PendingItem[], events: SimEvent[]) {
    const due = s.pending.filter((p) => p.effectiveTurn <= s.turn);
    s.pending = s.pending.filter((p) => p.effectiveTurn > s.turn);
    for (const p of due) {
      const blocked = activate(s, p);
      out.push(p);
      if (blocked) events.push({ type: 'policy_blocked', importance: 2, reason: blocked });
      else if (p.command.type === 'switch_system') events.push({ type: 'system_switch_started', importance: 3, to: p.command.to });
      else if (p.command.type === 'place') events.push({ type: 'built', importance: 2, kind: p.command.kind, tile: { ...p.command.tile } });
      else if (p.command.type === 'remove') events.push({ type: 'removed', importance: 2, tile: { ...p.command.tile } });
      events.push({ type: 'command_activated', importance: 2, id: p.id });
    }
  }

  function historyPoint(): HistoryPoint {
    const sn = snap();
    const ind = sn.indicators;
    const sum = (o: Record<string, number>) => Object.values(o).reduce((a, b) => a + b, 0);
    const gov = { revenue: sum(sn.finance.government.revenue), spending: sum(sn.finance.government.spending) };
    return {
      turn: s.turn, gdpRealAnnual: s.gdp / s.prices, unemployment: s.unemployment, gini: s.gini,
      // net monthly tonnes at the end of the turn, as the real wire (docs/sim-api-notes.md)
      emissions: s.lastMonthlyEmissions * (1 - 0.1 * s.forest), budgetShare: remainingBudget(s) / GEN.carbon.budget,
      legitimacy: s.legitimacy, avgWellbeing: ind.avgWellbeing, gpiAnnual: ind.gpiAnnual, debtToGdp: s.debt / s.gdp,
      avgIncomePerPerson: ind.avgIncomePerPerson, medianIncomePerPerson: ind.medianIncomePerPerson, topHouseholdWealthShare: ind.topHouseholdWealthShare,
      govRevenue: gov.revenue, govSpending: gov.spending, bankCapitalRatio: s.bank.loans > 0 ? s.bank.equity / s.bank.loans : 0, renewableShare: sn.energy.renewableShare, gridServed: sn.energy.served,
      climateStress: stressNow(s), forestHealth: healthOf(s).forest, commonsHealth: healthOf(s).commons, farmHealth: healthOf(s).farm,
    };
  }

  const port: SimPort = {
    init(seed) {
      open = null;
      s = initialState(seed);
      s.gini = s.gini0 = wealthGini(s);
      return ok(snap());
    },
    validTiles: (kind) => validTiles(s, kind),
    levers: () => LEVERS.map(({ default: _d, effect: _e, ...info }) => structuredClone(info)),
    actions: actionList,
    preview(commands) {
      if (!Array.isArray(commands)) return fail('invalid_command', 'commands must be a list');
      return ok(evaluate(s, commands).previews);
    },

    submitTurn(commands: Command[]): Result<TurnResult> {
      // Exactly the live-turn calls in order (contract section 12): one code path, so the two can never drift apart.
      if (open) return turnState('submitTurn needs no turn to be open (a live turn is in progress).');
      const begun = port.beginTurn(commands);
      if (!begun.ok) return begun;
      for (let m = 0; m < 3; m++) {
        const r = port.advanceMonth();
        if (!r.ok) return r;
      }
      return port.finishTurn();
    },

    beginTurn(commands: Command[]): Result<TurnStart> {
      if (open) return turnState('A turn is already open: call advanceMonth and finishTurn first.');
      if (s.gameOver) return fail('game_over', 'The game is over: no more turns can be played. Start a new game or load a save.');
      if (!Array.isArray(commands)) return fail('invalid_command', 'commands must be a list');
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
      const pcBefore = s.pc;
      s.pending.push(...accepted);
      s.nextPendingId += accepted.length;
      s.pc = ev.pcLeft;
      const activated: PendingItem[] = [];
      activateDue(activated, month0);
      open = { turn: s.turn, pcBefore, accepted, rejected, activated, month0, months: [], halted: false };
      return ok({ turn: open.turn, accepted, rejected, activated, pcBefore, snapshot: snap() });
    },

    advanceMonth(): Result<MonthResult> {
      if (!open) return turnState('No turn is open: call beginTurn first.');
      if (open.months.length >= 3) return turnState('All 3 months have run: call finishTurn.');
      const { turn, month0 } = open;
      const m = open.months.length as 0 | 1 | 2;
      let f: MonthFrame;
      if (open.halted) f = frame(m, [], {});
      else {
        const sysBefore = s.system;
        const weather = stepWeather(s); // MOCK weather (weather.ts), before the economy step as S11 runs before nature and production
        const out = stepMonth(s);
        spend(s, buildingCounts(s).road * GEN.build.roadUpkeepUsd); // road maintenance, paid by the government
        if (turn === MOCK.dynamics.firmFoundedTurn && m === 0) {
          const i = s.firmActive.findIndex((a) => !a);
          if (i >= 0) { s.firmActive[i] = true; month0.push({ type: 'firm_founded', importance: 2, firm: GEN.firms[i]!.id }); }
        }
        const jobEvents: SimEvent[] = [];
        syncJobs(s, jobEvents);
        for (const ev of jobEvents) if (ev.type === 'person_hired' || ev.type === 'person_fired') s.ledger.personEvents.push({ person: ev.person, event: structuredClone(ev) });
        stepHouseholds(s, jobEvents);
        const flows = makeFlows(s, out.emissions);
        const events = makeEvents(s, flows, [...(m === 0 ? month0 : []), ...jobEvents]);
        events.push(...weather.events);
        if (out.bankFailedNow) events.push({ type: 'bank_failed', importance: 3 });
        if (out.bailoutAmount > 0) events.push({ type: 'bank_bailout', importance: 3, amount: Math.round(out.bailoutAmount) });
        if (s.system !== sysBefore) events.push({ type: 'system_switch_completed', importance: 3, to: s.system });
        const end = checkOutcome(s);
        if (end && !s.gameOver) {
          s.gameOver = { turn, outcome: end.outcome };
          events.push(end.event === 'budget_exhausted' ? { type: 'budget_exhausted', importance: 3 } : { type: 'collapse', importance: 3, reason: end.event });
          open.halted = true;
        }
        f = frame(m, events, flows);
      }
      open.months.push(f);
      return ok({ month: m, frame: f, snapshot: snap() });
    },

    finishTurn(): Result<TurnResult> {
      if (!open) return turnState('No turn is open: call beginTurn first.');
      if (open.months.length < 3) return turnState(`Only ${open.months.length} of 3 months have run: call advanceMonth.`);
      const { turn, accepted, rejected, activated, pcBefore, months } = open;
      open = null;
      s.history.push(historyPoint());
      const policyGaps = GAP_LEVERS.map((k) => {
        const l = LEVERS.find((x) => x.key === k)!;
        return { lever: k, nominal: Number(s.policy[l.key]), effective: effectiveValue(s, k) };
      });
      const trace: TraceRecord[] = [
        ...policyGaps.map((g) => ({ type: 'policy_gap' as const, ...g, netLobby: s.netLobby })),
        ...(months[2]!.flows.emissions ?? months[1]!.flows.emissions ?? []).map((e) => ({ type: 'emissions' as const, source: e.source, ...(e.id !== undefined ? { id: e.id } : {}), ...(e.tile ? { tile: e.tile } : {}), tonnes: e.tonnes })),
        ...(s.fireSinkLossTurn > 0 ? [{ type: 'emissions' as const, source: 'fire_sink_loss' as const, tonnes: +s.fireSinkLossTurn.toFixed(2) }] : []),
        ...(months[2]!.flows.wages ?? months[1]!.flows.wages ?? []).map((w) => ({ type: 'income' as const, household: w.household, kind: 'wages' as const, amount: w.amount })),
      ];

      // End of turn (Rust: regenerate, then turn += 1). Nothing activates here: items due next turn fire at the start of the next submit.
      s.fireSinkLossTurn = 0;
      s.pc = Math.min(GEN.pc.cap, s.pc + GEN.pc.regenBase * s.legitimacy);
      s.turn++;
      rollUp(s);
      updateLobby(s);
      if (!s.gameOver && turn >= GEN.game.lastTurn) s.gameOver = { turn, outcome: 'survived' };
      return ok({ turn, accepted, rejected, activated, pcBefore, pcAfter: s.pc, months, policyGaps, trace, snapshot: snap() });
    },

    ledger: (account: AccountRef, query?: LedgerQuery) => ledgerPage(s, account, query),
    personLedger: (person: number) => personLedger(s, person),

    cancelPending(id) {
      if (s.gameOver) return fail('game_over', 'The game is over: nothing can be cancelled.');
      const p = s.pending.find((x) => x.id === id);
      if (!p) return fail('unknown_pending', `There is no pending item #${id}.`);
      s.pending = s.pending.filter((x) => x.id !== id);
      const refundPc = p.pcCost * GEN.pc.cancelRefund; // PC only: money is charged when the item takes effect
      s.pc = Math.min(GEN.pc.cap, s.pc + refundPc);
      return ok({ refundPc, snapshot: snap() });
    },

    exportState: () => (open ? turnState('A turn is open: a save is only possible at a turn boundary (finish the turn first).') : ok(JSON.stringify(s))),
    importState(blob) {
      let parsed: MockState;
      try { parsed = JSON.parse(blob) as MockState; } catch { return fail('bad_save', 'The saved game data is not valid JSON.'); }
      if (!parsed || typeof parsed !== 'object' || typeof parsed.turn !== 'number' || !parsed.rng || !parsed.policy || !Array.isArray(parsed.grid) || !Array.isArray(parsed.people) || !Array.isArray(parsed.pending)) {
        return fail('bad_save', 'The saved game data is incomplete.');
      }
      if (!('drought' in parsed) || !parsed.weatherRng) return fail('schema_mismatch', 'The save was made before weather was added to the game (Town v3 or earlier), so it cannot be loaded.');
      if (!parsed.ledger) return fail('schema_mismatch', 'The save was made before transaction ledgers were added to the game (Town v4 or earlier), so it cannot be loaded.');
      if (parsed.schemaVersion !== SCHEMA_VERSION) return fail('schema_mismatch', `The save was made by a different version (schema ${String(parsed.schemaVersion)}, expected ${SCHEMA_VERSION}).`);
      s = parsed;
      open = null;
      return ok(snap());
    },
  };
  return port;
}
