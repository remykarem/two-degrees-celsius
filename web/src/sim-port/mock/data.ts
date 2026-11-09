import raw from './data.json';
import type { LeverInfo, LeverKey } from '../../contract';

export type LeverValue = number | boolean;
export interface LeverData extends LeverInfo {
  default: LeverValue;
  effect: Partial<Record<'gdp' | 'unemployment' | 'gini' | 'emissions' | 'legitimacy', number>>;
  moneyPerUnit: number;
}
export interface InfraData {
  label: string; pc: number; money: number; leadTime: number;
  effect: Partial<Record<'emissions' | 'gdp' | 'wellbeing' | 'housing', number>>;
}

export const DATA = raw as unknown as {
  game: { startYear: number; firstTurn: number; lastTurn: number; turnsPerYear: number };
  schemaVersion: number;
  pc: { start: number; cap: number; regenBase: number; cancelRefund: number };
  start: Record<string, number>;
  dynamics: Record<string, number>;
  infra: Record<string, InfraData>;
  system: { pc: number; money: number; leadTime: number; transitionMonths: number };
  households: { id: number; label: string; cls: 'capitalist' | 'rentier' | 'manager' | 'worker' | 'dependent'; people: number }[];
  firms: { id: number; sector: 'goods' | 'food'; name: string; active: boolean }[];
  levers: LeverData[];
};

export const LEVER_BY_KEY = new Map<LeverKey, LeverData>(DATA.levers.map((l) => [l.key, l]));
