import type { CommandPreview, LeverInfo, LeverKey, Snapshot, SystemKey } from '../contract';
import type { DraftItem } from '../store';
import { formatLever, REASON_TEXT, SYSTEM_NAME } from './format';

export interface LeverGroup { id: string; label: string; blurb: string; keys: LeverKey[] }

export const LEVER_GROUPS: LeverGroup[] = [
  { id: 'taxes', label: 'Taxes', blurb: 'Who pays, and how much the state collects.', keys: ['tax_wage', 'tax_profit', 'tax_capital', 'wealth_tax', 'wealth_threshold'] },
  { id: 'money', label: 'Money and credit', blurb: 'The price of credit and the safety of the bank.', keys: ['base_rate', 'bank_capital_ratio', 'bailout'] },
  { id: 'labour', label: 'Labour and welfare', blurb: 'Wages, benefits and jobs.', keys: ['min_wage', 'benefit', 'ubi', 'job_guarantee'] },
  { id: 'nature', label: 'Nature and carbon', blurb: 'Pricing and capping emissions, and sharing the land.', keys: ['carbon_price', 'emissions_cap', 'green_subsidy', 'commons_harvest', 'commons_access'] },
  { id: 'spending', label: 'Government spending', blurb: 'What the state spends, and the rules on political money.', keys: ['public_spending', 'campaign_cap'] },
];

/** Groups with their LeverInfo, in group order. Levers not named in any group go to a trailing "Other" group so none are lost. */
export function groupLevers(levers: LeverInfo[]): { group: LeverGroup; levers: LeverInfo[] }[] {
  const used = new Set<LeverKey>();
  const out = LEVER_GROUPS.map((group) => ({
    group,
    levers: group.keys.flatMap((k) => { const l = levers.find((x) => x.key === k); if (l) used.add(k); return l ? [l] : []; }),
  })).filter((g) => g.levers.length);
  const rest = levers.filter((l) => !used.has(l.key));
  if (rest.length) out.push({ group: { id: 'other', label: 'Other', blurb: '', keys: rest.map((l) => l.key) }, levers: rest });
  return out;
}

/** Numeric position of a nominal policy value (bool -> 0/1, enum -> option index). */
export function toNum(l: LeverInfo, v: number | boolean | string): number {
  if (typeof v === 'boolean') return v ? 1 : 0;
  if (typeof v === 'string') return Math.max(0, l.options?.indexOf(v) ?? 0);
  return v;
}

/** The "community" cap sent with commons_access: the contract gives no default, so the UI chooses. */
export const COMMUNITY_CAP = 0.5;

export function toCommandValue(l: LeverInfo, n: number) {
  if (l.unit === 'bool') return n === 1;
  if (l.unit === 'enum') return l.options?.[n] === 'community' ? { mode: 'community' as const, cap: COMMUNITY_CAP } : { mode: 'open' as const };
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
  } else if (draftNumber(l, draft) === undefined && l.baseCostPc > pcFree + 1e-9) {
    disabled = `Needs about ${l.baseCostPc} PC; you have ${Math.floor(pcFree)} free this turn.`;
  }
  return { disabled, pcFree };
}

/** "You set X, expect about Y in force" or null when there is no lobbying effect to report. */
export function lobbyNote(l: LeverInfo, current: number, drafted: number, preview: CommandPreview | undefined): string | null {
  if (!l.lobbied || !preview || preview.blocked || preview.opposition <= 0 || drafted === current) return null;
  const expect = current + (drafted - current) * (1 - Math.min(1, preview.opposition));
  return `You set ${formatLever(l, drafted)}; expect about ${formatLever(l, expect)} in force (lobbying opposes this).`;
}

export const blockText = (r: string) => REASON_TEXT[r] ?? r;
export const arrowGlyph = (d: 'up' | 'down') => (d === 'up' ? '▲' : '▼');
