import type { SystemKey } from '../../contract';

/**
 * Copy for the system-switch comparison. Source of truth for rules: docs/causal-model.md section 3.
 * Where that doc is silent the text is marked placeholder: true and the UI shows a "placeholder" badge.
 */
export interface Fact { label: string; value: string; placeholder?: boolean }
export interface SystemCopy {
  key: SystemKey;
  name: string;
  tagline: string;
  facts: Fact[];
  /** Transition length in months, from the design doc (null: not specified). */
  transitionMonths: number | null;
  /** Output hit during the transition, as a fraction (null: not specified). */
  outputHit: number | null;
  resistance: Fact;
  tradeoffs: Fact[];
}

export const SYSTEMS: Record<SystemKey, SystemCopy> = {
  market: {
    key: 'market', name: 'Market capitalism', tagline: 'Owners keep the profits; wages follow the labour market.',
    transitionMonths: null, outputHit: null,
    facts: [
      { label: 'Dividends go to', value: 'Owners' },
      { label: 'Wages', value: 'Set by the market (a reserve army of unemployed holds them down)' },
      { label: 'Investment hurdle', value: '5% expected return' },
      { label: 'Credit', value: 'Normal terms' },
      { label: 'Lobbying power', value: 'Highest (cap 0.6)' },
      { label: 'Commons default', value: 'Open access or enclosed' },
    ],
    resistance: { label: 'Who resists a switch back', value: 'Not described in the design doc.', placeholder: true },
    tradeoffs: [
      { label: 'Strength', value: 'Fast growth driven by profit and investment (the growth loop in the design doc).' },
      { label: 'Cost', value: 'Growth also drives emissions up, and wealth can capture policy through lobbying.' },
    ],
  },
  regulated: {
    key: 'regulated', name: 'Regulated capitalism', tagline: 'Owners still profit, but wages have a higher floor and lobbying is capped.',
    transitionMonths: 12, outputHit: 0.05,
    facts: [
      { label: 'Dividends go to', value: 'Owners' },
      { label: 'Wages', value: 'Market wages with a higher floor' },
      { label: 'Investment hurdle', value: '5% expected return, with the green subsidy switched on' },
      { label: 'Credit', value: 'Normal terms' },
      { label: 'Lobbying power', value: 'Lower (cap 0.4); a campaign-spending cap becomes available and the commons can be community-managed' },
      { label: 'Commons default', value: 'A community cap is allowed' },
    ],
    resistance: { label: 'Who resists', value: 'Capitalists resist the change.' },
    tradeoffs: [
      { label: 'Strength', value: 'Unlocks the campaign-spending cap and community management of the commons, and lowers the ceiling on lobbying (0.4 instead of 0.6). The emissions cap works in every system.' },
      { label: 'Cost', value: 'About 12 months of transition with roughly 5% less output.' },
      { label: 'Trade-off', value: 'Lower GDP growth than market capitalism, in return for a better chance of staying inside the carbon budget (a design target, not a promise).' },
    ],
  },
  coop: {
    key: 'coop', name: 'Worker cooperatives', tagline: 'Employees share the surplus and own the firms.',
    transitionMonths: 24, outputHit: 0.1,
    facts: [
      { label: 'Dividends go to', value: 'Employees' },
      { label: 'Wages', value: 'A share of surplus, with a floor' },
      { label: 'Investment hurdle', value: '2% expected return, with more retained for green tech' },
      { label: 'Credit', value: 'Spread +1% (banks distrust co-ops)' },
      { label: 'Lobbying power', value: 'Lowest (cap 0.2)' },
      { label: 'Commons default', value: 'Managed by the community' },
    ],
    resistance: { label: 'Who resists', value: 'High resistance. The doc does not say from whom; expect owners and lenders.', placeholder: true },
    tradeoffs: [
      { label: 'Strength', value: 'Profits go to workers, which pulls inequality down, and lobbying power is lowest.' },
      { label: 'Cost', value: 'About 24 months of transition with roughly 10% less output; workers need training.' },
      { label: 'Trade-off', value: 'Slower growth, and not a free pass on emissions.' },
    ],
  },
};

export const SYSTEM_ORDER: SystemKey[] = ['market', 'regulated', 'coop'];
