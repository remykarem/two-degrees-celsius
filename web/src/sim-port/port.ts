import type { AccountRef, AgentsCatalog, Catalog, AssumptionTopic, ActionInfo, Command, LedgerPage, LedgerQuery, MarketTrades, TradesQuery, PersonLedger, CommandPreview, InfraKind, LeverInfo, Result, Snapshot, MonthResult, TileRef, TurnResult, TurnStart } from '../contract';

/**
 * Set by a port that replays a recorded run instead of running the simulation (platform cleanup phase 2). The UI badges the game as a
 * recorded demo and disables what cannot change the outcome (policies, building, saving, ledgers, previews).
 */
export interface DemoInfo {
  /** The badge text. */
  label: string;
  /** What the player is looking at and why the controls are off (the badge tooltip and the notes beside disabled controls). */
  note: string;
  seed: number;
  /** The last turn number that can be played: the recording ends after it. */
  lastTurn: number;
}

/**
 * The UI's only window onto the simulation (contract v1.0, docs/snapshot-contract.md sections 10.7 and 11). Every fallible call returns a Result:
 * a Rust Violation becomes { ok: false }, never a thrown exception.
 */
export interface SimPort {
  init(seed: number): Result<Snapshot>;
  levers(): LeverInfo[];
  actions(): ActionInfo[];
  preview(commands: Command[]): Result<CommandPreview[]>;
  /** Tiles that can take a new `kind` right now (empty and buildable), to shade the grid. preview() stays the authority per command. */
  validTiles(kind: InfraKind): TileRef[];
  submitTurn(commands: Command[]): Result<TurnResult>;
  /**
   * Contract v1.1 live turn (section 12), used by Pressure mode. In order, beginTurn + 3 x advanceMonth + finishTurn equal submitTurn.
   * Out-of-order calls return { ok: false, error: { code: 'turn_state' } } and change nothing.
   */
  beginTurn(commands: Command[]): Result<TurnStart>;
  advanceMonth(): Result<MonthResult>;
  finishTurn(): Result<TurnResult>;
  /** False when the implementation lacks the v1.1 calls (an older wasm package): Pressure mode is then unavailable. Absent means true. */
  readonly liveTurn?: boolean;
  /** Present only on the replay port: this is a recorded run, not the simulation. */
  readonly demo?: DemoInfo;
  cancelPending(id: number): Result<{ refundPc: number; snapshot: Snapshot }>;
  /** Contract v1.4 (section 14): the transaction ledger of one account. Legal at any time. A package without it returns { ok: false, code: 'unsupported' }. */
  ledger(account: AccountRef, query?: LedgerQuery): Result<LedgerPage>;
  /** Contract v1.4: a person's wages earned and employment events (people have no purse; money is household-level). */
  personLedger(person: number): Result<PersonLedger>;
  /** Contract v2.4 (section 24): the list of individual trades behind one market (default: the latest month). A package without it, and the recorded demo, return { ok: false, code: 'unsupported' }. */
  marketTrades(market: string, query?: TradesQuery): Result<MarketTrades>;
  /** Contract v1.5 (section 15): the model explorer's topics. Values and examples come from the model; the web never hard-codes them. */
  assumptions(): Result<AssumptionTopic[]>;
  /** Contract v1.7: who decides what, how, and where chance enters. */
  agents(): Result<AgentsCatalog>;
  /** Contract v1.8: UI bands, labels, explanatory text and the economic system cards (content/ui.json). */
  catalog(): Result<Catalog>;
  exportState(): Result<string>;
  importState(blob: string): Result<Snapshot>;
}
