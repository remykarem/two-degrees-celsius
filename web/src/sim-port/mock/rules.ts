import type { BlockReason, Command, CommandPreview, InfraKind, LeverKey, PendingItem, SystemKey } from '../../contract';
import { costKey, costOf, GEN, INFRA_KINDS, LEVER_BY_KEY, moneyFor, type LeverData } from './data';
import { cellAt, GRID, type MockState } from './state';

/** Numeric position of a policy value (bool 0/1, enum option index). null when the value has the wrong shape for the lever. */
export function numericValue(l: LeverData, v: unknown): number | null {
  if (l.unit === 'bool') return typeof v === 'boolean' ? (v ? 1 : 0) : null;
  if (l.unit === 'enum') {
    const mode = typeof v === 'string' ? v : v && typeof v === 'object' && 'mode' in v ? String((v as { mode: unknown }).mode) : null;
    const i = mode === null ? -1 : (l.options ?? []).indexOf(mode);
    return i >= 0 ? i : null;
  }
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

export const activeNumeric = (s: MockState, l: LeverData): number => numericValue(l, s.policy[l.key]) ?? 0;

/** Mirrors PolicyChange::opposed_by_capital in crates/sim-types/src/enums.rs. */
export function opposedByCapital(s: MockState, l: LeverData, next: number): boolean {
  const active = activeNumeric(s, l);
  switch (l.key) {
    case 'carbon_price': case 'wealth_tax': case 'tax_profit': return next > active;
    case 'emissions_cap': return next > 0 && (active <= 0 || next < active);
    default: return false;
  }
}

/** Latest queued value for the lever, else the active one (turn.rs size_of). */
function reference(s: MockState, pending: PendingItem[], l: LeverData): number {
  for (let i = pending.length - 1; i >= 0; i--) {
    const c = pending[i]!.command;
    if (c.type === 'set_policy' && c.lever === l.key) {
      const v = numericValue(l, c.value);
      if (v !== null && l.unit !== 'bool' && l.unit !== 'enum') return v;
    }
  }
  return activeNumeric(s, l);
}

interface Cost { pc: number; money: number; leadTime: number; opposition: number; blocked: BlockReason | null; tileValid: boolean; tileReason?: BlockReason }
const bad = (blocked: BlockReason): Cost => ({ pc: 0, money: 0, leadTime: 0, opposition: 0, blocked, tileValid: true });

/**
 * Why a tile command cannot be carried out (turn.rs tile_problem): off the map, forest or commons for a place (`tile_invalid`);
 * anything already there or something queued on the tile (`tile_occupied`); a remove on an empty tile (`nothing_to_remove`) or on
 * something that cannot be removed (`tile_invalid`).
 */
export function tileProblem(s: MockState, pending: PendingItem[], c: Command): BlockReason | null {
  if (c.type === 'place') {
    const cell = cellAt(s, c.tile.x, c.tile.y);
    if (!cell || cell.kind === 'forest' || cell.kind === 'commons') return 'tile_invalid';
    if (cell.kind !== 'empty') return 'tile_occupied';
    return pending.some((p) => p.command.type === 'place' && p.command.tile.x === c.tile.x && p.command.tile.y === c.tile.y) ? 'tile_occupied' : null;
  }
  if (c.type === 'remove') {
    const cell = cellAt(s, c.tile.x, c.tile.y);
    if (!cell) return 'tile_invalid';
    if (cell.kind === 'empty') return 'nothing_to_remove';
    return REMOVABLE.includes(cell.kind) ? null : 'tile_invalid';
  }
  return null;
}
const REMOVABLE = ['public_housing', 'fossil_plant', 'renewable_plant', 'road'];

/** Tiles that could take a new `kind` now (empty, nothing queued). */
export function validTiles(s: MockState, kind: InfraKind): { x: number; y: number }[] {
  const out: { x: number; y: number }[] = [];
  for (let y = 0; y < GRID; y++) for (let x = 0; x < GRID; x++) if (!tileProblem(s, s.pending, { type: 'place', kind, tile: { x, y } })) out.push({ x, y });
  return out;
}

const isKind = (k: unknown): k is InfraKind => INFRA_KINDS.includes(k as InfraKind);

/** Lending headroom: how much more the bank can lend under the capital ratio (turn.rs available_funds). */
export function headroom(s: MockState): number {
  const kappa = Math.max(Number(s.policy.bank_capital_ratio), 1e-9);
  return Math.max(0, Math.max(0, s.bank.equity) / kappa - s.bank.loans);
}
export const availableFunds = (s: MockState, pending: PendingItem[]) => s.cash + headroom(s) - pending.reduce((a, p) => a + p.moneyCost, 0);

function costOne(s: MockState, pending: PendingItem[], c: Command): Cost {
  if (c.type === 'switch_system') {
    if (!(c.to in GEN.systems)) return bad('invalid_command');
    const cfg = costOf('switch_system');
    if (s.transition || pending.some((p) => p.command.type === 'switch_system')) return bad('transition_in_progress');
    if (c.to === s.system) return bad('same_system');
    return { pc: cfg.base * Math.max(1, cfg.minSize), money: 0, leadTime: cfg.leadTime, opposition: 0, blocked: null, tileValid: true };
  }
  if (c.type === 'place' || c.type === 'remove') {
    if (c.type === 'place' && !isKind(c.kind)) return bad('invalid_command');
    if (!c.tile || !Number.isInteger(c.tile.x) || !Number.isInteger(c.tile.y)) return bad('invalid_command');
    const problem = tileProblem(s, pending, c);
    if (problem) return { pc: 0, money: 0, leadTime: 0, opposition: 0, blocked: problem, tileValid: false, tileReason: problem };
    const kind = c.type === 'place' ? c.kind : (cellAt(s, c.tile.x, c.tile.y)!.kind as InfraKind);
    const cfg = costOf(costKey(c.type, kind));
    return { pc: cfg.base * Math.max(1, cfg.minSize), money: c.type === 'place' ? moneyFor(kind) : 0, leadTime: cfg.leadTime, opposition: 0, blocked: null, tileValid: true };
  }
  const l = LEVER_BY_KEY.get(c.lever);
  if (!l) return bad('invalid_command');
  const v = numericValue(l, c.value);
  if (v === null) return bad('invalid_command');
  if (l.unit !== 'enum' && l.unit !== 'bool' && (v < l.min - 1e-9 || v > l.max + 1e-9)) return bad('invalid_command');
  if (l.unit === 'enum' && typeof c.value === 'object' && c.value && 'cap' in c.value && !(Number.isFinite(c.value.cap) && c.value.cap > 0)) return bad('invalid_command');
  const mode = l.unit === 'enum' ? (l.options ?? [])[v] : undefined;
  const allowedIn = mode ? l.optionAvailableIn?.[mode] : undefined;
  if (!l.availableIn.includes(s.system) || (allowedIn && !allowedIn.includes(s.system))) return bad('lever_unavailable');
  const cfg = costOf(c.lever);
  const size = l.unit === 'bool' || l.unit === 'enum' ? 1 : Math.abs(v - reference(s, pending, l)) / cfg.unit;
  const opp = opposedByCapital(s, l, v) ? s.netLobby : 0;
  return { pc: cfg.base * Math.max(size, cfg.minSize) * (1 + opp), money: 0, leadTime: cfg.leadTime, opposition: opp, blocked: null, tileValid: true };
}

export interface Evaluation { previews: CommandPreview[]; accepted: PendingItem[]; pcLeft: number }

/** Shared by preview and submit (turn.rs evaluate): commands are costed in order as if the earlier ones were queued. */
export function evaluate(s: MockState, commands: Command[]): Evaluation {
  let pcLeft = s.pc;
  const pending = [...s.pending];
  let nextId = s.nextPendingId;
  const previews: CommandPreview[] = [];
  const accepted: PendingItem[] = [];
  for (const command of commands) {
    const c = costOne(s, pending, command);
    let blocked = c.blocked;
    if (!blocked && c.pc > pcLeft + 1e-9) blocked = 'insufficient_political_capital';
    if (!blocked && c.money > availableFunds(s, pending)) blocked = 'insufficient_funds';
    if (!blocked) {
      pcLeft -= c.pc;
      const item: PendingItem = { id: nextId++, command, submittedTurn: s.turn, effectiveTurn: s.turn + c.leadTime, pcCost: c.pc, moneyCost: c.money };
      pending.push(item);
      accepted.push(item);
    }
    previews.push({
      command, pc: c.pc, money: c.money, leadTime: c.leadTime, effectiveTurn: s.turn + c.leadTime, opposition: c.opposition, blocked,
      tileValid: c.tileValid, ...(c.tileReason ? { tileReason: c.tileReason } : {}),
    });
  }
  return { previews, accepted, pcLeft };
}

export const spend = (s: MockState, dollars: number) => {
  s.cash -= dollars;
  if (s.cash < 0) { s.debt += -s.cash; s.bank.loans += -s.cash; s.cash = 0; } // the government borrows what it lacks
};

/** Applies one due command (apply_commands in crates/sim-engine/src/commands.rs). Returns a blocked reason when the sim would block it. */
export function activate(s: MockState, item: PendingItem): BlockReason | null {
  const c = item.command;
  if (c.type === 'switch_system') {
    if (s.transition) return 'transition_in_progress';
    if (c.to === s.system) return 'same_system';
    const months = Math.max(1, GEN.systems[c.to].transitionMonths);
    s.transition = { to: c.to, monthsLeft: months, total: months };
    return null;
  }
  if (c.type === 'place') {
    const problem = tileProblem(s, [], c); // re-validated when built, as the real sim does
    if (problem) return problem;
    spend(s, item.moneyCost); // mock: charged and built at activation (docs/web-notes-align.md)
    const i = c.tile.y * GRID + c.tile.x;
    if (c.kind === 'public_housing') {
      const id = s.nextHomeId++;
      s.homes.push({ id, x: c.tile.x, y: c.tile.y, owner: 'public', household: null });
      s.grid[i] = { kind: 'public_housing', ref: id };
    } else s.grid[i] = { kind: c.kind };
    return null;
  }
  if (c.type === 'remove') {
    const problem = tileProblem(s, [], c);
    if (problem) return problem;
    const i = c.tile.y * GRID + c.tile.x;
    const cell = s.grid[i]!;
    if (cell.kind === 'public_housing') {
      const home = s.homes.find((x) => x.id === cell.ref);
      const tenant = s.households.find((h) => h.home === cell.ref);
      if (tenant) tenant.home = null;
      s.homes = s.homes.filter((x) => x !== home);
    }
    s.grid[i] = { kind: 'empty' };
    return null;
  }
  const l = LEVER_BY_KEY.get(c.lever)!;
  if (!l.availableIn.includes(s.system)) return 'lever_unavailable';
  const mode = l.unit === 'enum' ? (l.options ?? [])[numericValue(l, c.value) ?? 0] : undefined;
  if (mode && l.optionAvailableIn?.[mode] && !l.optionAvailableIn[mode]!.includes(s.system)) return 'lever_unavailable';
  s.policy[c.lever] = l.unit === 'enum' ? mode! : (c.value as number | boolean);
  return null;
}

export const SYSTEM_KEYS: SystemKey[] = ['market', 'regulated', 'coop'];
export type { LeverKey };
