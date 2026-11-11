import type { CommandPreview, LeverInfo, LeverKey, Snapshot, SystemKey } from '../contract';
import type { DraftItem } from '../store';
import { arrowFlip } from './costs';
import { formatLever, REASON_TEXT, SYSTEM_NAME } from './format';
import { bands, catalog, fill, text } from '../store/catalog';

export interface LeverGroup { id: string; label: string; blurb: string; keys: LeverKey[] }

/** Groups in the order of content/ui.json `lever_groups` (label and blurb), holding the levers whose wire `group` is that label. Levers in an unlisted group go to a trailing "Other" group so none are lost. */
export function groupLevers(levers: LeverInfo[]): { group: LeverGroup; levers: LeverInfo[] }[] {
  const listed = catalog().leverGroups;
  const out = listed.map((g) => {
    const members = levers.filter((l) => l.group === g.label);
    return { group: { id: g.label, label: g.label, blurb: g.blurb, keys: members.map((l) => l.key) }, levers: members };
  }).filter((g) => g.levers.length);
  const rest = levers.filter((l) => !listed.some((g) => g.label === l.group));
  if (rest.length) out.push({ group: { id: 'other', label: 'Other', blurb: '', keys: rest.map((l) => l.key) }, levers: rest });
  return out;
}

/** Numeric position of a nominal policy value (bool -> 0/1, enum -> option index). */
export function toNum(l: LeverInfo, v: number | boolean | string): number {
  if (typeof v === 'boolean') return v ? 1 : 0;
  if (typeof v === 'string') return Math.max(0, l.options?.indexOf(v) ?? 0);
  return v;
}

export function toCommandValue(l: LeverInfo, n: number) {
  if (l.unit === 'bool') return n === 1;
  if (l.unit === 'enum') return l.options?.[n] === 'community' ? { mode: 'community' as const, cap: bands().communityCap } : { mode: 'open' as const };
  return n;
}

export function draftNumber(l: LeverInfo, draft: DraftItem[]): number | undefined {
  const d = draft.find((x) => x.command.type === 'set_policy' && x.command.lever === l.key);
  if (!d || d.command.type !== 'set_policy') return undefined;
  const v = d.command.value;
  return typeof v === 'object' ? Math.max(0, l.options?.indexOf(v.mode) ?? 0) : toNum(l, v);
}

export interface LeverStatus {
  /** null when the lever can be changed. */
  disabled: string | null;
  /** PC still free after the rest of the draft (this lever's own draft excluded). */
  pcFree: number;
}

export function leverStatus(l: LeverInfo, snap: Snapshot, draft: DraftItem[]): LeverStatus {
  const others = draft.filter((d) => !(d.command.type === 'set_policy' && d.command.lever === l.key) && !d.preview.blocked)
    .reduce((a, d) => a + d.preview.pc, 0);
  const pcFree = snap.pc.current - others;
  const system: SystemKey = snap.system.current;
  let disabled: string | null = null;
  if (!l.availableIn.includes(system)) {
    const where = l.availableIn.map((s) => SYSTEM_NAME[s]).join(' or ');
    disabled = `Not available in ${SYSTEM_NAME[system]}. Needs ${where}.`;
  }
  // No cost-based lock: the real price (lobbying opposition, size of the change) is only known from preview(), which blocks the draft if you cannot pay.
  return { disabled, pcFree };
}

/** "You set X, expect about Y in force", from the simulation's own expectation (preview.expectedEffective); null when the preview carries none or lobbying changes nothing. */
export function lobbyNote(l: LeverInfo, current: number, drafted: number, preview: CommandPreview | undefined): string | null {
  if (!l.lobbied || !preview || preview.blocked || preview.opposition <= 0 || drafted === current || preview.expectedEffective === undefined) return null;
  return fill(text('lobby', 'note'), { set: formatLever(l, drafted), expected: formatLever(l, preview.expectedEffective) });
}

export const blockText = (r: string) => REASON_TEXT[r] ?? r;
/** Arrows for the row: what RAISING the lever does, flipped when the drafted value is below the current one. */
export function arrowsShown(l: LeverInfo, current: number, drafted: number | undefined): { arrows: LeverInfo['arrows']; basis: 'raise' | 'raised' | 'lowered' } {
  if (drafted === undefined) return { arrows: l.arrows, basis: 'raise' };
  const f = arrowFlip(l, current, drafted);
  if (f === 0) return { arrows: l.arrows, basis: 'raise' };
  if (f === 1) return { arrows: l.arrows, basis: 'raised' };
  return { arrows: l.arrows.map((a) => ({ metric: a.metric, direction: a.direction === 'up' ? 'down' as const : 'up' as const })), basis: 'lowered' };
}
export const arrowGlyph = (d: 'up' | 'down') => (d === 'up' ? '▲' : '▼');
