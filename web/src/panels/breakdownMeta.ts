import type { BreakdownKey } from './metrics';

export const TITLE_OF: Record<BreakdownKey, string> = {
  gdp: 'How GDP is calculated', gpi: 'How GPI is calculated', wageShare: 'How the wage share is calculated', profitShare: 'How the profit share is calculated', debtToGdp: 'How public debt / GDP is calculated',
  gini: 'How inequality (Gini) is calculated', legitimacy: 'How legitimacy is calculated', capitalRatio: 'How the bank capital ratio works', emissions: 'Emissions: month, turn and accumulated', taxes: 'Taxes',
};
/** The explorer anchor behind each breakdown. */
export const ANCHOR_OF: Record<BreakdownKey, string> = {
  gdp: 'gdp', gpi: 'gpi', wageShare: 'wage_share', profitShare: 'profit_share', debtToGdp: 'debt', gini: 'gini', legitimacy: 'legitimacy', capitalRatio: 'capital_ratio', emissions: 'emissions', taxes: 'taxes',
};
