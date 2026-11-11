// The wire contract. Types only: no logic. `generated.ts` is produced from the Rust wire types in
// crates/sim-api (ts-rs; `npm run gen:contract`) and is the source of truth for shapes. This file
// re-exports it and adds only (a) the names the web has always used for generated types and
// (b) the few types that are not wire shapes. Semantics: docs/snapshot-contract.md.
import type {
  AgeBandKey,
  ClassKey,
  Finance,
  EstateView,
  ForestTileState,
  InfraKindKey,
  LedgerKindKey,
  LegitTerm,
  MapView,
  OwnerShare,
  PolicyScalar,
  Policies,
  PublicGood,
  RiskKey,
  SectorKey,
  Infrastructure,
  SimEventKind,
  MonthFlows as GMonthFlows,
  SimEvent as GSimEvent,
  Snapshot as GSnapshot,
  Tile as WireTile,
  TileView,
  TileKindKey,
  TurnPhaseView,
  WeatherView,
  Group,
  Bands,
} from './generated';

export * from './generated';

/** The mock sets `mock: true` (the UI shows a badge). The real wire never sends it; goes away with the mock. */
export type Snapshot = GSnapshot & { mock?: boolean };

export type SchemaVersion = number;
/** A lever's value as displayed (the policy board, drafts): `PolicyScalar`. A command carries `CommandValue`. */
export type PolicyValue = PolicyScalar;
export type TileRef = WireTile;
export type Tile = TileView;
export type TileKind = TileKindKey;
export type ForestState = ForestTileState;
export type AgeBand = AgeBandKey;
export type Sector = SectorKey;
export type RiskLevel = RiskKey;
export type LedgerKind = LedgerKindKey;
export type Estate = EstateView;
export type FinanceView = Finance;
export type GameMap = MapView;
export type TurnPhase = TurnPhaseView;
export type Weather = WeatherView;
export type PolicyBoard = Policies;
export type LegitimacyTerm = LegitTerm;
export type OwnershipSlice = OwnerShare;
export type IndustryPublicGood = PublicGood;
export type IndustryInfra = Infrastructure;
export type UiBands = Bands;
export type AssumptionGroup = Group;
export type { ClassKey, InfraKindKey, SimEventKind };

/** A month's flows as the planners accept them: any list may be missing (the real wire omits `staffCommutes` when empty; tests and the mock may omit more). */
export type FrameInput = { month: 0 | 1 | 2; flows: Partial<GMonthFlows>; events?: GSimEvent[] };

export type SimEventType = GSimEvent['type'];
export type ActionKey = 'switch_system' | `place:${InfraKindKey}` | `remove:${InfraKindKey}`;

/** Every fallible SimPort call returns this; Rust violations become error results, never exceptions. */
export interface SimError { code: string; message: string }
export type Result<T> = { ok: true; value: T } | { ok: false; error: SimError };
