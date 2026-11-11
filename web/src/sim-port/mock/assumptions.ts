import type { AssumptionGroup, AssumptionTopic } from '../../contract';
import { GEN, MOCK, WX } from './data';

// MOCK ONLY: an invented explorer catalogue with the wire's shape (docs/snapshot-contract.md section 15). The real catalogue is written in Rust from the live Params;
// these topics say "mock" in their status note and use the mock's own numbers (some read from content/, most from mock-only.json).

type T = Omit<AssumptionTopic, 'status' | 'statusNote' | 'code' | 'leavesOut'> & Partial<Pick<AssumptionTopic, 'status' | 'leavesOut' | 'code'>>;
const topic = (t: T): AssumptionTopic => ({ status: 'placeholder', leavesOut: [], code: ['web/src/sim-port/mock (mock only: the real model is in crates/sim-model)'], ...t, statusNote: 'Mock: the web mock invents this rule so the explorer can be shown without the Rust simulation.' });
const g = (group: AssumptionGroup) => group;

export function mockAssumptions(): AssumptionTopic[] {
  const X = MOCK.extra, D = MOCK.dynamics;
  return [
    topic({ id: 'emissions', title: 'Emissions, sinks and the carbon budget', group: g('emissions_energy'), summary: 'Firms, households and fossil plants emit; the forest absorbs a share. Net emissions use up the carbon budget.',
      formula: 'net = firms + households + plants - sink; cumulative = sum of monthly net', related: ['energy', 'climate'], usedBy: ['emissions', 'budget', 'temperature', 'carbon_price', 'emissions_cap', 'green_subsidy', 'ledger-emissions'],
      params: [{ path: 'carbon.budget', label: 'Carbon budget', value: GEN.carbon.budget, unit: 'tCO2' }, { path: 'extra.householdEmissionShare', label: 'Household share of non-plant emissions', value: X.householdEmissionShare ?? 0, unit: 'ratio' }],
      examples: [{ label: 'Half the budget used', inputs: { used: GEN.carbon.budget / 2, budget: GEN.carbon.budget }, result: { value: GEN.carbon.budget / 2, unit: 'tCO2 left' } }], leavesOut: ['Emissions embodied in imports', 'Methane and other gases'] }),
    topic({ id: 'energy', title: 'Power grid and energy', group: g('emissions_energy'), summary: 'Plants supply capacity; firms and households demand energy. If demand exceeds capacity, supply is rationed.',
      formula: 'served = min(1, capacity / demand); intensity = fossil energy used / energy delivered', related: ['emissions'], usedBy: ['energy', 'plants', 'plant'],
      params: [{ path: 'build.plant_capacity', label: 'Capacity per plant', value: GEN.build.plantCapacity, unit: 'MWh/month' }], examples: [] }),
    topic({ id: 'households', title: 'People, households and wellbeing', group: g('people_households'), summary: 'People live in households that share income and wealth. Wellbeing follows jobs, inequality and housing.',
      formula: 'wellbeing moves 5% of the way each month toward 0.9 - 2 x unemployment - 0.3 x Gini', related: ['scoring'], usedBy: ['wellbeing', 'housing', 'avg_income', 'median_income', 'people', 'inspector'],
      params: [{ path: 'mock.wellbeingPull', label: 'Pull per month', value: 0.05, unit: 'ratio' }], examples: [] }),
    topic({ id: 'production', title: 'Production, jobs and prices', group: g('production_prices'), summary: 'Firms hire workers and produce goods, food and services; unemployment drifts toward a level set by policy.',
      formula: 'GDP grows at base growth plus policy effects; unemployment pulls toward a target each month', related: ['wages'], usedBy: ['gdp', 'unemployment', 'prices', 'min_wage', 'industry', 'wage_share', 'profit_share'],
      params: [{ path: 'dynamics.baseGrowth', label: 'Base growth', value: D.baseGrowth ?? 0, unit: 'per year' }, { path: 'dynamics.unemploymentPull', label: 'Unemployment pull', value: D.unemploymentPull ?? 0, unit: 'ratio per month' }],
      examples: [{ label: 'Starting output', inputs: { monthly: GEN.derived.monthlyOutputValue }, result: { value: GEN.derived.monthlyOutputValue * 12, unit: 'USD/year' } }] }),
    topic({ id: 'wages', title: 'Wage share and profit share', group: g('production_prices'), summary: 'Wage share is wages over output; profit share is the rest. Ownership decides who gets profits.',
      formula: 'wage share = wage bill / output; profit share = 1 - wage share', related: ['production'], usedBy: ['wage_share', 'profit_share', 'ownership'], params: [{ path: 'firms.wage', label: 'Starting wage', value: GEN.firms[0]!.wage, unit: 'USD/month' }], examples: [] }),
    topic({ id: 'nature', title: 'Forest and commons', group: g('nature_climate'), summary: 'Forest absorbs carbon and decays when the commons are open; commons recover under community rules.',
      formula: 'forest falls by decay each month (slower when access is not open)', related: ['climate'], usedBy: ['forest', 'commons', 'forest_health', 'commons_health', 'farm_health', 'commons_harvest', 'commons_access', 'health'],
      params: [{ path: 'dynamics.forestDecayMonthly', label: 'Forest decay per month', value: D.forestDecayMonthly ?? 0, unit: 'ratio' }], examples: [] }),
    topic({ id: 'climate', title: 'Climate stress', group: g('weather'), summary: 'Stress rises as the carbon budget runs down; it makes fires and droughts likelier and fiercer.',
      formula: 'stress = ((onset - budget left) / (onset - full)) ^ curve, between 0 and 1', related: ['emissions', 'nature'], usedBy: ['climate_stress', 'resilience_spending', 'stress', 'weather', 'fire_station'],
      params: [{ path: 'weather.onset', label: 'Onset (budget left)', value: WX.onset, unit: 'share' }, { path: 'weather.full', label: 'Full stress (budget left)', value: WX.full, unit: 'share' }], examples: [] }),
    topic({ id: 'taxes', title: 'Taxes and public finance', group: g('government_policy'), summary: 'Income, profit, capital, wealth and carbon taxes feed the government. GST/VAT is not modelled. Spending covers benefits, UBI, public spending and the job guarantee.',
      formula: 'revenue = sum of tax rate x base; balance = revenue - spending; debt grows with deficits', related: ['finance'], usedBy: ['tax_wage', 'tax_profit', 'tax_capital', 'wealth_tax', 'wealth_threshold', 'benefit', 'ubi', 'public_spending', 'job_guarantee', 'debt', 'taxes', 'gst'],
      params: [{ path: 'town.govDebt', label: 'Starting public debt', value: GEN.town.govDebt, unit: 'USD' }], examples: [], leavesOut: ['GST/VAT: households buy goods and services untaxed'] }),
    topic({ id: 'finance', title: 'Money, banks and credit', group: g('finance_credit'), summary: 'The bank lends against its equity; below the capital ratio limit lending stops. Defaults eat equity.',
      formula: 'capital ratio = equity / loans', related: ['taxes'], usedBy: ['money', 'base_rate', 'bank_capital_ratio', 'bailout', 'capital_ratio', 'finance', 'ledger'], params: [{ path: 'town.bank.equity', label: 'Starting bank equity', value: GEN.town.bank.equity, unit: 'USD' }], examples: [] }),
    topic({ id: 'politics', title: 'Legitimacy, lobbying and political capital', group: g('class_politics'), summary: 'Legitimacy follows wellbeing, unemployment, inequality and debt. Lobbying weakens policies; political capital pays for changes.',
      formula: 'legitimacy = last level + sum of weight x (input - threshold)', related: ['taxes'], usedBy: ['legitimacy', 'campaign_cap', 'lobbying', 'pc'], params: [{ path: 'pc.regenBase', label: 'PC regeneration', value: GEN.pc.regenBase, unit: 'PC/turn' }], examples: [] }),
    topic({ id: 'scoring', title: 'GDP, GPI and inequality', group: g('scoring'), summary: 'GDP is output adjusted for prices; GPI subtracts an inequality cost; Gini measures wealth inequality.',
      formula: 'GPI = GDP x (1 - Gini); Gini = 1 - 2 x area under the Lorenz curve', related: ['production'], usedBy: ['gdp', 'gpi', 'gini', 'top_wealth_share', 'lorenz'], params: [], examples: [] }),
    topic({ id: 'leaves_out', title: 'What the model leaves out', group: g('leaves_out'), summary: 'Trade, GST/VAT, state ownership of firms, migration, technology change and most real-world detail.', formula: 'n/a',
      related: [], usedBy: ['gst', 'industry-state'], params: [], examples: [], leavesOut: ['International trade', 'GST/VAT', 'State-owned firms'] }),
  ];
}
