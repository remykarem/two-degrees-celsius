import type { GameOutcome, HistoryPoint, InfraKind, PendingItem, SystemKey } from '../../contract';
import { GEN, LEVERS, MOCK, type PolicyValue } from './data';
import type { Rng } from './rng';

export const SCHEMA_VERSION = 3;

/**
 * Mock state. Fields marked (rules) follow the real Rust turn layer; the economy fields (gdp, gini, wellbeing, prices, ...) are
 * invented mock dynamics and are labelled so in the UI.
 */
export interface MockState {
  schemaVersion: number;
  rng: Rng;
  turn: number;
  tick: number;
  pc: number; // (rules)
  system: SystemKey; // (rules)
  transition: null | { to: SystemKey; monthsLeft: number; total: number }; // (rules)
  nextPendingId: number;
  pending: PendingItem[]; // (rules)
  policy: Record<string, PolicyValue>; // (rules)
  buildings: Record<InfraKind, number>; // (rules)
  gdp: number; unemployment: number; legitimacy: number; gini: number; wellbeing: number;
  cash: number; debt: number; netLobby: number; forest: number; commons: number;
  bank: { equity: number; loans: number; failed: boolean };
  cumulativeTonnes: number; lastMonthlyEmissions: number; prices: number; wage: number;
  firmActive: boolean[];
  history: HistoryPoint[];
  gameOver: null | { turn: number; outcome: GameOutcome };
}

export function initialState(seed: number): MockState {
  const policy: Record<string, PolicyValue> = {};
  for (const l of LEVERS) policy[l.key] = l.default;
  const t = GEN.town;
  const b = GEN.build.start;
  return {
    schemaVersion: SCHEMA_VERSION,
    rng: { state: seed >>> 0 },
    turn: GEN.game.firstTurn, tick: 0, pc: GEN.pc.start, system: 'market', transition: null,
    nextPendingId: 1, pending: [], policy,
    buildings: { public_housing: b.publicHousing, fossil_plant: b.fossilPlant, renewable_plant: b.renewablePlant },
    gdp: GEN.derived.monthlyOutputValue * 12, unemployment: t.unemployment, legitimacy: t.legitimacy, gini: 0.4, wellbeing: 0.6,
    cash: t.govCash, debt: t.govDebt, netLobby: MOCK.extra.startingNetLobby!, forest: t.forest / t.forestMax, commons: t.commons / t.commonsMax,
    bank: { equity: t.bank.equity, loans: t.bank.loans, failed: false },
    cumulativeTonnes: 0, lastMonthlyEmissions: baseEmissions(), prices: 1, wage: GEN.firms[0]!.wage,
    firmActive: GEN.firms.map((f) => f.active), history: [], gameOver: null,
  };
}

/** Firm emissions from content plus a mock household share. */
export function baseEmissions(): number {
  return GEN.derived.monthlyFirmEmissions * (1 + MOCK.extra.householdEmissionShare!);
}
