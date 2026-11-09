import type { ActionInfo, CostModel, LeverInfo } from '../contract';
import { formatLever } from './format';

/** Cost model of an action: the wire may omit it; place, remove and switch all have unit 1 and min size 1, so base cost is the cost. */
export const actionModel = (a: ActionInfo): CostModel => a.costModel ?? { base: a.baseCostPc, unit: 1, minSize: 1 };

/** Estimated PC for a change of `units` cost units (real rule: base x max(units, minSize) x (1 + opposition)). Opposition is not known here. */
export const estimatePc = (m: CostModel, units: number) => m.base * Math.max(units, m.minSize);
const round = (v: number) => (v >= 10 ? Math.round(v) : Math.round(v * 10) / 10);

/** Hint for a lever row before anything is drafted. The authoritative number is CommandPreview.pc. */
export function leverCostHint(l: LeverInfo): string {
  const small = round(estimatePc(l.costModel, 0));
  if (l.unit === 'bool' || l.unit === 'enum') return `About ${round(estimatePc(l.costModel, 1))} PC to switch`;
  return `About ${small} PC for a small change, ${round(l.costModel.base)} PC per ${formatLever(l, l.costModel.unit)} of change`;
}

/** Position used to compare two values, with "no cap" (0) treated as the loosest setting. */
const looseness = (l: LeverInfo, v: number) => (l.zeroMeansNone && v === 0 ? Number.POSITIVE_INFINITY : v);

/** Arrow direction after the slider moves: the lever's arrows describe RAISING it, so they flip when the change goes down. */
export function arrowFlip(l: LeverInfo, from: number, to: number): 1 | -1 | 0 {
  const a = looseness(l, from), b = looseness(l, to);
  return a === b ? 0 : b > a ? 1 : -1;
}
