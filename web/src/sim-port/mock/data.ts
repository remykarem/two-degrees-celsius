import generated from './data.generated.json';
import mockOnly from './mock-only.json';
import type { ActionInfo, AgentsCatalog, AgeBand, ClassKey, CostModel, InfraKind, LeverInfo, LeverKey, Sector, SystemKey } from '../../contract';

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
  /** Contract v1.7: the agents catalogue resolved from content/. */
  agents: AgentsCatalog;
  /** Only when content/params.json has a `weather` block (Town v4). */
  weather?: { onset: number; full: number; curve: number; coverageRadius?: number; smokeRadius?: number; regrowMonths?: number; durationMonths?: number; fireStationCostUsd?: number; fireStationUpkeepUsd?: number };
  /** Only when content/levers.json has `resilience_spending`. */
  weatherLever?: Pick<LeverInfo, 'label' | 'unit' | 'min' | 'max' | 'step' | 'lobbied' | 'arrows' | 'availableIn'>;
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
  fallbackCosts: Record<string, { baseCost: number; leadTime: number; unit: number; minSize: number }>;
}
/** Stress parameters: the real content (params.weather) when it exists, else the labelled mock defaults. */
/** The mock's weather numbers: its own labelled defaults, with the real content's values winning where content/params.json has them. */
export const WX: WeatherMock = {
  ...MOCK.weather,
  ...(GEN.weather?.coverageRadius !== undefined ? { coverageRadius: GEN.weather.coverageRadius } : {}),
  ...(GEN.weather?.regrowMonths !== undefined ? { regrowMonths: GEN.weather.regrowMonths } : {}),
  ...(GEN.weather?.durationMonths !== undefined ? { fireMonths: GEN.weather.durationMonths } : {}),
  ...(GEN.weather?.fireStationCostUsd !== undefined ? { fireStationCostUsd: GEN.weather.fireStationCostUsd } : {}),
  ...(GEN.weather?.fireStationUpkeepUsd !== undefined ? { fireStationUpkeepUsd: GEN.weather.fireStationUpkeepUsd } : {}),
};
export const STRESS = { onset: GEN.weather?.onset ?? MOCK.weather.onset, full: GEN.weather?.full ?? MOCK.weather.full, curve: GEN.weather?.curve ?? MOCK.weather.curve };

export type PolicyValue = number | boolean | string;
export interface LeverData extends LeverInfo { default: PolicyValue; effect: Effect }

export const INFRA_KINDS: InfraKind[] = ['public_housing', 'fossil_plant', 'renewable_plant', 'road', 'fire_station'];
export const INFRA_LABEL: Record<InfraKind, string> = { public_housing: 'Public housing', fossil_plant: 'Fossil plant', renewable_plant: 'Renewable plant', road: 'Road', fire_station: 'Fire station' };
/** Action labels as the real wire sends them (docs/fixtures/actions.json). */
const PLACE_LABEL: Record<InfraKind, string> = { public_housing: 'Build public housing', fossil_plant: 'Build fossil power plant', renewable_plant: 'Build renewable power plant', road: 'Build road', fire_station: 'Build fire station' };
const REMOVE_LABEL: Record<InfraKind, string> = { public_housing: 'Demolish public housing', fossil_plant: 'Decommission fossil power plant', renewable_plant: 'Decommission renewable power plant', road: 'Remove road', fire_station: 'Demolish fire station' };

const cost = (key: string): CostModel & { leadTime: number } => {
  // The real content may not have the Town v4 entries yet: fall back to the labelled mock numbers.
  const c = GEN.turnCosts[key] ?? MOCK.weather.fallbackCosts[key];
  if (!c) throw new Error(`content/params.json turn.levers has no entry for ${key}; run npm run sync-mock`);
  return { base: c.baseCost, unit: c.unit, minSize: c.minSize, leadTime: c.leadTime };
};

export const LEVERS: LeverData[] = MOCK.levers.map((mock) => {
  const m: LeverMeta = mock.key === 'resilience_spending' && GEN.weatherLever ? { ...mock, ...GEN.weatherLever } : mock; // real presentation wins when content has it
  const c = cost(m.key);
  const def = GEN.policyStart[m.key] ?? (m.key === 'resilience_spending' ? 0 : undefined); // mock default 0 until content/initial_town.json has it
  if (def === undefined) throw new Error(`content/initial_town.json has no starting policy for ${m.key}`);
  const { effect, ...info } = m;
  return { ...info, baseCostPc: c.base, costModel: { base: c.base, unit: c.unit, minSize: c.minSize }, leadTime: c.leadTime, default: def, effect };
});
export const LEVER_BY_KEY = new Map<LeverKey, LeverData>(LEVERS.map((l) => [l.key, l]));

export const costKey = (type: 'place' | 'remove', kind: InfraKind) => `${type}_${kind}`;
const ALL: SystemKey[] = ['market', 'regulated', 'coop'];
const moneyFor = (kind: InfraKind) => (kind === 'public_housing' ? GEN.build.publicHousingCostUsd : kind === 'road' ? GEN.build.roadCostUsd : kind === 'fire_station' ? WX.fireStationCostUsd : GEN.build.plantCostUsd);

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
