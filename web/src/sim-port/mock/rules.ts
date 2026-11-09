import type { ActionInfo, BlockReason, Command, CommandPreview, InfraKind, LeverKey, PendingItem, SystemKey } from '../../contract';
import { DATA, LEVER_BY_KEY } from './data';
import type { MockState } from './state';

/** Share of the PC cost added by lobbying opposition, for lobbied levers only. */
export function opposition(s: MockState, lever: LeverKey): number {
  return LEVER_BY_KEY.get(lever)?.lobbied ? s.netLobby * DATA.dynamics.lobbyFactor! : 0;
}

/** Nominal numeric value of a set_policy command (bool and enum map to 0/1). */
export function numericValue(value: unknown): number {
  if (typeof value === 'boolean') return value ? 1 : 0;
  if (typeof value === 'number') return value;
  if (value && typeof value === 'object' && 'mode' in value) return (value as { mode: string }).mode === 'open' ? 0 : 1;
  return NaN;
}

interface Cost { pc: number; money: number; leadTime: number; opposition: number; blocked: BlockReason | null }

function costOf(s: MockState, c: Command, planned: Map<LeverKey, true>): Cost {
  const bad = (r: BlockReason): Cost => ({ pc: 0, money: 0, leadTime: 0, opposition: 0, blocked: r });
  if (c.type === 'switch_system') {
    if (s.transition) return bad('transition_in_progress');
    if (c.to === s.system) return bad('same_system');
    return { pc: DATA.system.pc, money: DATA.system.money, leadTime: DATA.system.leadTime, opposition: 0, blocked: null };
  }
  if (c.type === 'place' || c.type === 'remove') {
    const inf = DATA.infra[c.kind];
    if (!inf || !Number.isInteger(c.count) || c.count <= 0) return bad('invalid_command');
    if (c.type === 'remove') {
      if ((s.infra[c.kind] ?? 0) < c.count) return bad('nothing_to_remove');
      return { pc: Math.ceil(inf.pc * c.count * 0.5), money: 0, leadTime: 1, opposition: 0, blocked: null };
    }
    return { pc: inf.pc * c.count, money: inf.money * c.count, leadTime: inf.leadTime, opposition: 0, blocked: null };
  }
  const lever = LEVER_BY_KEY.get(c.lever);
  if (!lever) return bad('invalid_command');
  if (!lever.availableIn.includes(s.system)) return bad('lever_unavailable');
  const v = numericValue(c.value);
  const inRange = lever.unit === 'enum'
    ? Number.isInteger(v) && v >= 0 && v < (lever.options?.length ?? 0)
    : Number.isFinite(v) && v >= lever.min && v <= lever.max;
  if (!inRange) return bad('invalid_command');
  const opp = opposition(s, c.lever);
  const range = lever.max - lever.min || 1; // enum levers have min = max = 0
  const cur = numericValue(s.policy[c.lever]);
  const scale = lever.unit === 'bool' || lever.unit === 'enum' ? 1 : Math.max(0.25, Math.abs(v - cur) / range);
  const pc = Math.max(1, Math.ceil(lever.baseCostPc * scale * (1 + opp)));
  void planned;
  return { pc, money: lever.moneyPerUnit * v, leadTime: lever.leadTime, opposition: opp, blocked: null };
}

/** Shared by preview and submit so tray costs always equal what a submit charges. */
export function evaluate(s: MockState, commands: Command[]): CommandPreview[] {
  let pc = s.pc;
  // pending builds are committed against available funds
  let money = s.cash + DATA.start.creditLine! - s.pending.reduce((a, p) => a + p.moneyCost, 0);
  const planned = new Map<LeverKey, true>();
  return commands.map((command) => {
    const c = costOf(s, command, planned);
    let blocked = c.blocked;
    if (!blocked && c.pc > pc) blocked = 'insufficient_political_capital';
    if (!blocked && c.money > money) blocked = 'insufficient_funds';
    if (!blocked) {
      pc -= c.pc;
      money -= c.money;
    }
    return {
      command, pc: c.pc, money: c.money, leadTime: c.leadTime,
      effectiveTurn: s.turn + c.leadTime, opposition: c.opposition, blocked,
    };
  });
}

export function toPending(s: MockState, p: CommandPreview): PendingItem {
  return {
    id: s.nextPendingId++, command: p.command, submittedTurn: s.turn, effectiveTurn: p.effectiveTurn,
    pcCost: p.pc, moneyCost: p.money,
  };
}

/** Applies one due command to the state. Returns a short description. */
export function activate(s: MockState, item: PendingItem): string {
  const c = item.command;
  s.cash -= item.moneyCost;
  if (c.type === 'switch_system') {
    s.transition = { to: c.to, monthsLeft: DATA.system.transitionMonths, total: DATA.system.transitionMonths };
    return `system switch to ${c.to} begins`;
  }
  if (c.type === 'place') {
    s.infra[c.kind] = (s.infra[c.kind] ?? 0) + c.count;
    return `${DATA.infra[c.kind]?.label ?? c.kind} built`;
  }
  if (c.type === 'remove') {
    s.infra[c.kind] = Math.max(0, (s.infra[c.kind] ?? 0) - c.count);
    return `${DATA.infra[c.kind]?.label ?? c.kind} removed`;
  }
  const lever = LEVER_BY_KEY.get(c.lever);
  if (c.lever === 'commons_access' && typeof c.value === 'object') s.commonsMode = c.value.mode;
  s.policy[c.lever] = typeof c.value === 'object' ? numericValue(c.value) : c.value;
  return `${lever?.label ?? c.lever} set`;
}

/** Static action metadata (switch_system, place:<kind>, remove:<kind>). */
export function actionList(): ActionInfo[] {
  const all: SystemKey[] = ['market', 'regulated', 'coop'];
  const out: ActionInfo[] = [{ key: 'switch_system', label: 'Switch system', baseCostPc: DATA.system.pc, leadTime: DATA.system.leadTime, moneyCost: DATA.system.money, availableIn: all }];
  for (const [kind, inf] of Object.entries(DATA.infra)) {
    out.push({ key: `place:${kind as InfraKind}`, label: inf.label, baseCostPc: inf.pc, leadTime: inf.leadTime, moneyCost: inf.money, availableIn: all });
  }
  for (const [kind, inf] of Object.entries(DATA.infra)) {
    out.push({ key: `remove:${kind as InfraKind}`, label: `Remove ${inf.label.toLowerCase()}`, baseCostPc: Math.ceil(inf.pc * 0.5), leadTime: 1, moneyCost: 0, availableIn: all });
  }
  return out;
}
