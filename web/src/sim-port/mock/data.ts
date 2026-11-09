import generated from './data.generated.json';
import mockOnly from './mock-only.json';
import type { ActionInfo, AgeBand, ClassKey, CostModel, InfraKind, LeverInfo, LeverKey, Sector, SystemKey } from '../../contract';

interface XY { x: number; y: number }

/** Rules and starting numbers generated from content/ (npm run sync-mock). */
export interface Generated {
  ticksPerTurn: number;
  game: { startYear: number; firstTurn: number; lastTurn: number; turnsPerYear: number };
  pc: { start: number; cap: number; regenBase: number; cancelRefund: number };
  turnCosts: Record<string, { baseCost: number; leadTime: number; unit: number; minSize: number }>;
  policyStart: Record<string, number | boolean | string>;
  systems: Record<SystemKey, { name: string; transitionMonths: number; outputPenalty: number; lambdaMax: number }>;
  space: { slowMinutes: number; fastMinutes: number; freeMinutes: number; kCommute: number; minEffectiveHours: number; unhousedTiles: number; unhousedMinutes: number };
  build: {
    plantCapacity: number; publicHousingCostUsd: number; plantCostUsd: number; roadCostUsd: number; roadUpkeepUsd: number;
    start: { publicHousing: number; fossilPlant: number; renewablePlant: number; road: number };
  };
  town: {
    govCash: number; govDebt: number; legitimacy: number; unionDensity: number; forest: number; forestMax: number; commons: number; commonsMax: number;
    rentPrivate: number; rentPublic: number; bankOwner: number;
    bank: { loans: number; deposits: number; equity: number; headroom: number };
    labourForce: number; employed: number; unemployment: number;
  };
  carbon: { budget: number; t0: number; tLimit: number };
  collapseUnemployment: number;
  derived: { monthlyFirmEmissions: number; monthlyOutputValue: number; monthlyWageBill: number };
  map: { width: number; height: number; roads: XY[]; forest: XY[]; commons: XY[]; townHall: XY; bank: XY };
  homes: { id: number; tile: XY; owner: number; valueUsd: number }[];
  plants: { kind: 'fossil' | 'renewable'; tile: XY; capacity: number }[];
  households: { id: number; label: string; class: ClassKey; savings: number; debt: number; home: number | null; monthlyIncome: number;
    people: { label: string; ageBand: AgeBand; skill: number; labourForce: boolean; reservation: number; job: number | null }[] }[];
  firms: { id: number; sector: Sector; active: boolean; name: string; founder: number; tile: XY; capital: number; cash: number; debt: number; wage: number; price: number;
    output: number; intensity: number; workers: number }[];
}
export const GEN = generated as unknown as Generated;

type Effect = Partial<Record<'gdp' | 'unemployment' | 'gini' | 'emissions' | 'legitimacy', number>>;
interface LeverMeta extends Omit<LeverInfo, 'baseCostPc' | 'costModel' | 'leadTime'> { effect: Effect }
/** MOCK ONLY: lever presentation and invented dynamics. */
export const MOCK = mockOnly as unknown as {
  levers: LeverMeta[];
  dynamics: Record<string, number>;
  extra: Record<string, number>;
  infraEffect: Partial<Record<InfraKind, { emissions?: number; gdp?: number; wellbeing?: number }>>;
};

export type PolicyValue = number | boolean | string;
export interface LeverData extends LeverInfo { default: PolicyValue; effect: Effect }

export const INFRA_KINDS: InfraKind[] = ['public_housing', 'fossil_plant', 'renewable_plant', 'road'];
export const INFRA_LABEL: Record<InfraKind, string> = { public_housing: 'Public housing', fossil_plant: 'Fossil plant', renewable_plant: 'Renewable plant', road: 'Road' };
/** Action labels as the real wire sends them (docs/fixtures/actions.json). */
const PLACE_LABEL: Record<InfraKind, string> = { public_housing: 'Build public housing', fossil_plant: 'Build fossil power plant', renewable_plant: 'Build renewable power plant', road: 'Build road' };
const REMOVE_LABEL: Record<InfraKind, string> = { public_housing: 'Demolish public housing', fossil_plant: 'Decommission fossil power plant', renewable_plant: 'Decommission renewable power plant', road: 'Remove road' };

const cost = (key: string): CostModel & { leadTime: number } => {
  const c = GEN.turnCosts[key];
  if (!c) throw new Error(`content/params.json turn.levers has no entry for ${key}; run npm run sync-mock`);
  return { base: c.baseCost, unit: c.unit, minSize: c.minSize, leadTime: c.leadTime };
};

export const LEVERS: LeverData[] = MOCK.levers.map((m) => {
  const c = cost(m.key);
  const def = GEN.policyStart[m.key];
  if (def === undefined) throw new Error(`content/initial_town.json has no starting policy for ${m.key}`);
  const { effect, ...info } = m;
  return { ...info, baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, default: def, effect };
});
export const LEVER_BY_KEY = new Map<LeverKey, LeverData>(LEVERS.map((l) => [l.key, l]));

export const costKey = (type: 'place' | 'remove', kind: InfraKind) => `${type}_${kind}`;
const ALL: SystemKey[] = ['market', 'regulated', 'coop'];
const moneyFor = (kind: InfraKind) => (kind === 'public_housing' ? GEN.build.publicHousingCostUsd : kind === 'road' ? GEN.build.roadCostUsd : GEN.build.plantCostUsd);

export function actionList(): ActionInfo[] {
  const s = cost('switch_system');
  const out: ActionInfo[] = [{ key: 'switch_system', label: 'Switch economic system', baseCostPc: s.base, costModel: { base: s.base, unit: s.unit, minSize: s.minSize }, leadTime: s.leadTime, moneyCost: 0, availableIn: ALL }];
  for (const kind of INFRA_KINDS) {
    const c = cost(costKey('place', kind));
    out.push({ key: `place:${kind}`, label: PLACE_LABEL[kind], baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, moneyCost: moneyFor(kind), availableIn: ALL });
  }
  for (const kind of INFRA_KINDS) {
    const c = cost(costKey('remove', kind));
    out.push({ key: `remove:${kind}`, label: REMOVE_LABEL[kind], baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, moneyCost: 0, availableIn: ALL });
  }
  return out;
}
export { cost as costOf, moneyFor };
