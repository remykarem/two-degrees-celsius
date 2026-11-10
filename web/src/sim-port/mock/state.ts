import type { AgeBand, ClassKey, GameOutcome, HistoryPoint, InfraKind, PendingItem, SystemKey, TileKind } from '../../contract';
import { GEN, LEVERS, MOCK, type PolicyValue } from './data';
import { initialLedger, type MockLedger } from './ledger';
import type { Rng } from './rng';

/** The mock's own save format. Equal to the real sim's schemaVersion (4 since the ledger joined the saved blob, Town v5), so saves from before the ledger are refused by both. */
export const SCHEMA_VERSION = 4;
export const GRID = 12;

export interface PersonState { id: number; label: string; household: number; ageBand: AgeBand; skill: number; labourForce: boolean; job: number | null }
export interface HouseholdState { id: number; label: string; class: ClassKey; members: number[]; home: number | null; savings: number; debt: number; arrears: number }
export interface HomeState { id: number; x: number; y: number; owner: number | 'public'; household: number | null }
/** `state` and `left` are MOCK-ONLY weather bookkeeping on forest cells: 'burning' (months of fire left) or 'regrowing' (months until standing again). */
export interface GridCell { kind: TileKind; ref?: number; state?: 'standing' | 'burning' | 'regrowing'; left?: number }

/**
 * Mock state. Fields marked (rules) follow the real Rust turn layer; the economy fields (gdp, gini, wellbeing, prices, ...) are
 * invented mock dynamics and are labelled so in the UI. The map, people, households and homes are owned here, as the real sim owns them.
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
  /** 12 x 12, index y * 12 + x. (rules) */
  grid: GridCell[];
  homes: HomeState[];
  nextHomeId: number;
  people: PersonState[];
  households: HouseholdState[];
  /** MOCK weather (see weather.ts): own random stream, so weather never disturbs the economy's draws; the active drought; tonnes of sink lost to fire this turn. */
  weatherRng: Rng;
  drought: null | { severity: number; monthsLeft: number };
  fireSinkLossTurn: number;
  gdp: number; unemployment: number; legitimacy: number; gini: number; gini0: number; wellbeing: number;
  cash: number; debt: number; netLobby: number; forest: number; commons: number;
  bank: { equity: number; loans: number; failed: boolean };
  cumulativeTonnes: number; lastMonthlyEmissions: number; lastPlantEmissions: number; prices: number; wage: number;
  firmActive: boolean[];
  /** MOCK transaction ledger (ledger.ts): double-entry bookings of the mock's invented money. */
  ledger: MockLedger;
  history: HistoryPoint[];
  gameOver: null | { turn: number; outcome: GameOutcome };
}

export const cellAt = (s: Pick<MockState, 'grid'>, x: number, y: number): GridCell | undefined => (x >= 0 && y >= 0 && x < GRID && y < GRID ? s.grid[y * GRID + x] : undefined);

export function buildingCounts(s: Pick<MockState, 'grid'>): Record<InfraKind, number> {
  const c: Record<InfraKind, number> = { public_housing: 0, fossil_plant: 0, renewable_plant: 0, road: 0, fire_station: 0 };
  for (const cell of s.grid) if (cell.kind in c) c[cell.kind as InfraKind]++;
  return c;
}

export function initialState(seed: number): MockState {
  const policy: Record<string, PolicyValue> = {};
  for (const l of LEVERS) policy[l.key] = l.default;
  const t = GEN.town;
  const grid: GridCell[] = Array.from({ length: GRID * GRID }, () => ({ kind: 'empty' as TileKind }));
  const put = (p: { x: number; y: number }, kind: TileKind, ref?: number) => { grid[p.y * GRID + p.x] = ref === undefined ? { kind } : { kind, ref }; };
  for (const p of GEN.map.forest) { put(p, 'forest'); grid[p.y * GRID + p.x]!.state = 'standing'; }
  for (const p of GEN.map.commons) put(p, 'commons');
  for (const p of GEN.map.roads) put(p, 'road');
  put(GEN.map.townHall, 'town_hall');
  put(GEN.map.bank, 'bank');
  for (const h of GEN.homes) put(h.tile, 'home', h.id);
  for (const f of GEN.firms) put(f.tile, 'firm', f.id);
  for (const p of GEN.plants) put(p.tile, p.kind === 'fossil' ? 'fossil_plant' : 'renewable_plant');

  const people: PersonState[] = [];
  const households: HouseholdState[] = GEN.households.map((h) => {
    const members = h.people.map((p) => {
      const id = people.length;
      people.push({ id, label: p.label, household: h.id, ageBand: p.ageBand, skill: p.skill, labourForce: p.labourForce, job: p.job });
      return id;
    });
    return { id: h.id, label: h.label, class: h.class, members, home: h.home, savings: h.savings, debt: h.debt, arrears: 0 };
  });
  const homes: HomeState[] = GEN.homes.map((h) => ({ id: h.id, x: h.tile.x, y: h.tile.y, owner: h.owner, household: households.find((x) => x.home === h.id)?.id ?? null }));
  const s: MockState = {
    schemaVersion: SCHEMA_VERSION,
    rng: { state: seed >>> 0 },
    turn: GEN.game.firstTurn, tick: 0, pc: GEN.pc.start, system: 'market', transition: null,
    nextPendingId: 1, pending: [], policy, grid, homes, nextHomeId: homes.length, people, households,
    weatherRng: { state: (seed ^ 0x9e3779b9) >>> 0 }, drought: null, fireSinkLossTurn: 0,
    gdp: GEN.derived.monthlyOutputValue * 12, unemployment: t.unemployment, legitimacy: t.legitimacy, gini: 0.7, gini0: 0.7, wellbeing: 0.6,
    cash: t.govCash, debt: t.govDebt, netLobby: MOCK.extra.startingNetLobby!, forest: t.forest / t.forestMax, commons: t.commons / t.commonsMax,
    bank: { equity: t.bank.equity, loans: t.bank.loans, failed: false },
    cumulativeTonnes: 0, lastMonthlyEmissions: baseEmissions(), lastPlantEmissions: GEN.build.start.fossilPlant * MOCK.extra.plantEmissionPerFossil!, prices: 1, wage: GEN.firms[0]!.wage,
    firmActive: GEN.firms.map((f) => f.active), ledger: { opening: {}, openingDebt: {}, balance: {}, debt: {}, detail: [], summary: {}, months: [], personEvents: [], lastTurn: 0 }, history: [], gameOver: null,
  };
  s.ledger = initialLedger(s);
  return s;
}

/** Firm emissions from content plus a mock household share (the plants' part is separate: see plantEmissions in dynamics.ts). */
export function baseNonPlant(): number {
  return GEN.derived.monthlyFirmEmissions * (1 + MOCK.extra.householdEmissionShare!);
}
export const baseEmissions = (): number => baseNonPlant() + GEN.build.start.fossilPlant * MOCK.extra.plantEmissionPerFossil!;
