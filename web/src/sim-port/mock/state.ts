import type { Command, HistoryPoint, LeverKey, PendingItem, SystemKey } from '../../contract';
import { DATA, type LeverValue } from './data';
import type { Rng } from './rng';

export interface MockState {
  schemaVersion: number;
  rng: Rng;
  turn: number;
  tick: number;
  pc: number;
  system: SystemKey;
  transition: null | { to: SystemKey; monthsLeft: number; total: number };
  nextPendingId: number;
  pending: PendingItem[];
  policy: Record<string, LeverValue>;
  commonsMode: 'open' | 'community';
  infra: Record<string, number>;
  gdp: number; unemployment: number; legitimacy: number; gini: number; wellbeing: number;
  cash: number; debt: number; netLobby: number; forest: number; commons: number;
  cumulativeTonnes: number; lastMonthlyEmissions: number; prices: number;
  firmActive: boolean[];
  history: HistoryPoint[];
  gameOver: null | { turn: number; outcome: 'budget_exhausted' | 'survived' | { collapse: 'legitimacy_zero' } };
}

export function initialState(seed: number): MockState {
  const s = DATA.start;
  const policy: Record<string, LeverValue> = {};
  for (const l of DATA.levers) policy[l.key] = l.default;
  return {
    schemaVersion: DATA.schemaVersion,
    rng: { state: seed >>> 0 },
    turn: 1, tick: 0, pc: DATA.pc.start, system: 'market', transition: null,
    nextPendingId: 1, pending: [], policy, commonsMode: 'open',
    infra: { public_housing: s.publicHousing!, fossil_plant: s.fossilCap!, renewable_plant: s.renewableCap! },
    gdp: s.gdp!, unemployment: s.unemployment!, legitimacy: s.legitimacy!, gini: s.gini!, wellbeing: s.wellbeing!,
    cash: s.cash!, debt: 0, netLobby: s.netLobby!, forest: s.forest!, commons: s.commons!,
    cumulativeTonnes: 0, lastMonthlyEmissions: DATA.dynamics.baseMonthlyEmissions!, prices: s.prices!,
    firmActive: DATA.firms.map((f) => f.active), history: [], gameOver: null,
  };
}

export type { Command, LeverKey };
