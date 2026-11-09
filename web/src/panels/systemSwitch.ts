import type { ActionInfo, Snapshot, SystemKey } from '../contract';
import { SYSTEMS } from './content/systems';
import { estimatePc } from './costs';
import { leadText, pct } from './format';

export interface SwitchOption {
  to: SystemKey;
  current: boolean;
  /** null when the switch can be queued. */
  disabled: string | null;
  pcCost: number;
  leadTime: number;
  moneyCost: number;
  /** One line summarising the price for the confirmation step. */
  summary: string;
}

export function switchOptions(snap: Snapshot, action: ActionInfo | undefined, pcFree: number): SwitchOption[] {
  return (['market', 'regulated', 'coop'] as SystemKey[]).map((to) => {
    const copy = SYSTEMS[to];
    // Switching has no size and no lobbying opposition, so the cost model is exact (preview() still has the last word).
    const pcCost = action ? estimatePc(action.costModel, 1) : 0, leadTime = action?.leadTime ?? 0, moneyCost = action?.moneyCost ?? 0;
    let disabled: string | null = null;
    if (snap.system.current === to) disabled = 'This is the current system.';
    else if (snap.system.transition) disabled = 'A transition is already in progress.';
    else if (action && !action.availableIn.includes(snap.system.current)) disabled = 'Not available from the current system.';
    else if (pcCost > pcFree + 1e-9) disabled = `Needs about ${Math.round(pcCost * 10) / 10} PC; you have ${Math.floor(pcFree)} free.`;
    const parts = [`about ${Math.round(pcCost * 10) / 10} PC`, `starts ${leadText(leadTime)}`];
    if (moneyCost > 0) parts.push(`about $${Math.round(moneyCost).toLocaleString('en-US')}`);
    if (copy.transitionMonths !== null) parts.push(`${copy.transitionMonths} month transition`);
    if (copy.outputHit !== null) parts.push(`${pct(copy.outputHit)} less output while it lasts`);
    return { to, current: snap.system.current === to, disabled, pcCost, leadTime, moneyCost, summary: parts.join(', ') };
  });
}
