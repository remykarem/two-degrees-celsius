import generated from './data.generated.json';
import mockOnly from './mock-only.json';
import type { ActionInfo, AgentsCatalog, Catalog, AgeBand, ClassKey, CostModel, InfraKind, LeverInfo, LeverKey, Sector, SystemKey } from '../../contract';

interface XY { x: number; y: number }

/** Rules and starting numbers generated from content/ (npm run sync-mock). */
export interface Generated {
  ticksPerTurn: number;
  game: { startYear: number; firstTurn: number; lastTurn: number; turnsPerYear: number; ticksPerTurn: number };
  pc: { start: number; cap: number; regenBase: number; cancelRefund: number };
  turnCosts: Record<string, { baseCost: number; leadTime: number; unit: number; minSize: number }>;
  policyStart: Record<string, number | boolean | string>;
  systems: Record<SystemKey, { name: string; transitionMonths: number; outputPenalty: number; lambdaMax: number }>;
  space: { slowMinutes: number; fastMinutes: number; freeMinutes: number; kCommute: number; minEffectiveHours: number; unhousedTiles: number; unhousedMinutes: number };
  build: {
    plantCapacity: number; publicHousingCostUsd: number; plantCostUsd: number; roadCostUsd: number; roadUpkeepUsd: number;
    start: { publicHousing: number; fossilPlant: number; renewablePlant: number; road: number };
  };
  /** Contract v1.7: the agents catalogue resolved from content/. */
  agents: AgentsCatalog;
  weather: { onset: number; full: number; curve: number; coverageRadius: number; smokeRadius: number; regrowMonths: number; durationMonths: number; fireStationCostUsd: number; fireStationUpkeepUsd: number };
  /** The lever table as the wire sends it (content/levers.json), without costs and lead times. */
  levers: Omit<LeverInfo, 'baseCost' | 'costModel' | 'leadTime' | 'baseCostPc'>[];
  /** Contract v1.8: the real catalog() reply. */
  catalog: Catalog;
  bankCascadeUnemployment: number;
  /** content/levers.json `actions`: the labels and availability the wire sends. */
  actions: Record<string, { label: string; availableIn: SystemKey[]; tileRule?: string }>;
  /** Only when content/levers.json has `resilience_spending`. */
  town: {
    govCash: number; govDebt: number; legitimacy: number; unionDensity: number; forest: number; forestMax: number; commons: number; commonsMax: number;
    rentPrivate: number; rentPublic: number; bankOwner: number;
    bank: { loans: number; deposits: number; equity: number; headroom: number };
    labourForce: number; employed: number; unemployment: number;
  };
  carbon: { budget: number; t0: number; tLimit: number };
  collapseUnemployment: number;
  derived: { monthlyFirmEmissions: number; monthlyOutputValue: number; monthlyWageBill: number };
  map: { width: number; height: number; roads: XY[]; forest: XY[]; commons: XY[]; townHall: XY; bank: XY; estateGrounds: XY[]; runway: XY[] };
  /** Town v7: content/initial_town.json `estate` and content/params.json `estate` (null when the town has none). */
  estate: null | {
    household: number; home: number; hangarTile: XY; jetName: string; jetValueUsd: number;
    flightProbMonthly: number; flightTonnes: number; householdEmissionsT: number; pilotWageUsd: number; pilotUnpaidMonthsMax: number; pilotRehireMonthsOfWages: number;
  };
  homes: { id: number; tile: XY; owner: number; valueUsd: number }[];
  plants: { kind: 'fossil' | 'renewable'; tile: XY; capacity: number }[];
  households: { id: number; label: string; class: ClassKey; savings: number; debt: number; home: number | null; monthlyIncome: number;
    people: { label: string; ageBand: AgeBand; skill: number; labourForce: boolean; reservation: number; job: number | null; staffOf: number | null }[] }[];
  firms: { id: number; sector: Sector; active: boolean; name: string; founder: number; tile: XY; capital: number; cash: number; debt: number; wage: number; price: number;
    output: number; intensity: number; workers: number }[];
}
export const GEN = generated as unknown as Generated;

type Effect = Partial<Record<'gdp' | 'unemployment' | 'gini' | 'emissions' | 'legitimacy', number>>;
/** MOCK ONLY: lever presentation and invented dynamics. */
export const MOCK = mockOnly as unknown as {
  leverEffects: Record<string, Effect>;
  dynamics: Record<string, number>;
  extra: Record<string, number>;
  infraEffect: Partial<Record<InfraKind, { emissions?: number; gdp?: number; wellbeing?: number }>>;
  weather: WeatherMock;
};

/** MOCK ONLY: the invented weather numbers (see mock-only.json `weather`). */
export interface WeatherMock {
  onset: number; full: number; curve: number;
  droughtBase: number; droughtStressGain: number; droughtMonthsMin: number; droughtMonthsSpan: number; droughtMonthsStress: number;
  droughtCommonsLoss: number; droughtFarmLoss: number; communityBuffer: number;
  fireBase: number; fireStressGain: number; fireDroughtDryness: number; fireMonths: number; spreadBase: number; spreadDrought: number;
  forestLossPerBurningTile: number; regrowMonths: number; regrowGainPerTile: number; sinkShare: number; smokeWellbeing: number;
  coverageRadius: number; coverageSpread: number; coverageShorten: number; resilienceMaxEffect: number; resilienceHalfUsd: number;
  fireStationCostUsd: number; fireStationUpkeepUsd: number; forecastMedium: number; forecastHigh: number;
}
/** The mock's weather numbers: its own labelled invented dynamics, with every number content/params.json `weather` has winning. */
export const WX: WeatherMock = {
  ...MOCK.weather,
  onset: GEN.weather.onset, full: GEN.weather.full, curve: GEN.weather.curve, coverageRadius: GEN.weather.coverageRadius, regrowMonths: GEN.weather.regrowMonths,
  fireMonths: GEN.weather.durationMonths, fireStationCostUsd: GEN.weather.fireStationCostUsd, fireStationUpkeepUsd: GEN.weather.fireStationUpkeepUsd,
};
export const STRESS = { onset: GEN.weather.onset, full: GEN.weather.full, curve: GEN.weather.curve };

export type PolicyValue = number | boolean | string;
export interface LeverData extends LeverInfo { default: PolicyValue; effect: Effect }

export const INFRA_KINDS: InfraKind[] = ['public_housing', 'fossil_plant', 'renewable_plant', 'road', 'fire_station'];
export const INFRA_LABEL = Object.fromEntries(GEN.catalog.entities.filter((e: { infraKind?: string }) => e.infraKind).map((e: { infraKind?: string; label: string }) => [e.infraKind, e.label])) as Record<InfraKind, string>;
const cost = (key: string): CostModel & { leadTime: number } => {
  const c = GEN.turnCosts[key];
  if (!c) throw new Error(`content/params.json turn.levers has no entry for ${key}; run npm run sync-mock`);
  return { base: c.baseCost, unit: c.unit, minSize: c.minSize, leadTime: c.leadTime };
};

export const LEVERS: LeverData[] = GEN.levers.map((m) => {
  const c = cost(m.key);
  const def = GEN.policyStart[m.key];
  if (def === undefined) throw new Error(`content/initial_town.json has no starting policy for ${m.key}`);
  const effect = MOCK.leverEffects[m.key];
  if (!effect) throw new Error(`mock-only.json leverEffects has no entry for ${m.key}`);
  return { ...m, baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, default: def, effect };
});
export const LEVER_BY_KEY = new Map<LeverKey, LeverData>(LEVERS.map((l) => [l.key, l]));

export const costKey = (type: 'place' | 'remove', kind: InfraKind) => `${type}_${kind}`;
const tileRule = (key: string) => (GEN.actions[key]?.tileRule ? { tileRule: GEN.actions[key]!.tileRule } : {});
const moneyFor = (kind: InfraKind) => (kind === 'public_housing' ? GEN.build.publicHousingCostUsd : kind === 'road' ? GEN.build.roadCostUsd : kind === 'fire_station' ? WX.fireStationCostUsd : GEN.build.plantCostUsd);

export function actionList(): ActionInfo[] {
  const s = cost('switch_system');
  const out: ActionInfo[] = [{ key: 'switch_system', label: GEN.actions.switch_system!.label, baseCostPc: s.base, costModel: { base: s.base, unit: s.unit, minSize: s.minSize }, leadTime: s.leadTime, moneyCost: 0, availableIn: GEN.actions.switch_system!.availableIn, needsTile: false }];
  for (const kind of INFRA_KINDS) {
    const c = cost(costKey('place', kind));
    out.push({ key: `place:${kind}`, label: GEN.actions[`place:${kind}`]!.label, baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, moneyCost: moneyFor(kind), availableIn: GEN.actions[`place:${kind}`]!.availableIn, needsTile: true, ...tileRule(`place:${kind}`) });
  }
  for (const kind of INFRA_KINDS) {
    const c = cost(costKey('remove', kind));
    out.push({ key: `remove:${kind}`, label: GEN.actions[`remove:${kind}`]!.label, baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, moneyCost: 0, availableIn: GEN.actions[`remove:${kind}`]!.availableIn, needsTile: true, ...tileRule(`remove:${kind}`) });
  }
  return out;
}
export { cost as costOf, moneyFor };
