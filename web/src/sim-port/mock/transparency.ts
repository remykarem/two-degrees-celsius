import type { Breakdown, Breakdowns, EmissionsLine, HouseholdView, Industry, LeverKey, PolicyBoard, PolicyRow, Snapshot, TaxRow } from '../../contract';
import { GEN, INFRA_KINDS, INFRA_LABEL, LEVERS, WX } from './data';
import { effectiveValue } from './dynamics';
import { netWorthParts } from './people';
import type { MockState } from './state';

// MOCK ONLY: invented breakdowns, policy board and industry view. They follow the wire's shapes (docs/snapshot-contract.md section 16) and the same sum-to-result rules,
// but the arithmetic is the mock's own (the real numbers come from Rust). Everything here is labelled "mock" in `note` fields.

const EPS = 1e-6;
const near = (a: number, b: number) => Math.abs(a - b) <= EPS * Math.max(1, Math.abs(a), Math.abs(b));
const withSums = (b: Breakdown): Breakdown => (b.parts ? { ...b, sumsToResult: near(b.parts.reduce((a, p) => a + p.value, 0), b.result.value) } : b);

import { emissionsMonth, type EmMonth } from './emissions';
const line = (m: EmMonth): EmissionsLine => {
  const grossTotal = m.firms + m.households + m.plants + (m.aviation ?? 0);
  const net = grossTotal + m.sink;
  return { firms: m.firms, households: m.households, plants: m.plants, aviation: m.aviation ?? 0, grossTotal, sink: m.sink, net, headline: net };
};
const sumMonths = (ms: EmMonth[]): EmMonth => ms.reduce((a, m) => ({ firms: a.firms + m.firms, households: a.households + m.households, plants: a.plants + m.plants, sink: a.sink + m.sink, aviation: (a.aviation ?? 0) + (m.aviation ?? 0) }), { firms: 0, households: 0, plants: 0, sink: 0, aviation: 0 });

export function buildBreakdowns(s: MockState, snap: Pick<Snapshot, 'indicators' | 'government' | 'bank' | 'finance' | 'nature'>, households: HouseholdView[], monthlyOutput: number, wageBill: number): Breakdowns {
  const ind = snap.indicators;
  const gdpReal = ind.gdpRealAnnual;
  const bd = (id: string, title: string, formula: string, inputs: Breakdown['inputs'], result: Breakdown['result'], parts?: Breakdown['parts']): Breakdown => withSums({ id, title, formula, inputs, result, ...(parts ? { parts } : {}) });

  const gdp = bd('gdp', 'GDP (real, per year)', 'GDP = monthly output value x 12 / price index. (mock)', [
    { label: 'Monthly output value', value: monthlyOutput, unit: 'USD/month', source: 'gdp' }, { label: 'Months', value: 12, unit: 'months' }, { label: 'Price index', value: ind.priceIndex, unit: 'index', source: 'prices' },
  ], { value: gdpReal, unit: 'USD/year' }, [
    { label: 'Paid out as wages', value: (wageBill * 12) / ind.priceIndex, unit: 'USD/year', note: 'wage bill x 12 / price index' },
    { label: 'Profits and other', value: gdpReal - (wageBill * 12) / ind.priceIndex, unit: 'USD/year', note: 'what is left of output' },
  ]);
  const gpi = bd('gpi', 'GPI (per year)', 'GPI = real GDP x (1 - Gini). (mock: the real GPI has more adjustments)', [
    { label: 'Real GDP', value: gdpReal, unit: 'USD/year', source: 'gdp' }, { label: 'Gini', value: ind.gini, unit: 'index 0..1', source: 'gini' },
  ], { value: ind.gpiAnnual, unit: 'USD/year' }, [
    { label: 'Real GDP', value: gdpReal, unit: 'USD/year' }, { label: 'Inequality cost', value: -gdpReal * ind.gini, unit: 'USD/year', note: 'real GDP x Gini' },
  ]);
  const wageShare = bd('wageShare', 'Wage share', 'Wage share = wage bill / monthly output value.', [
    { label: 'Wage bill', value: wageBill, unit: 'USD/month', source: 'wage' }, { label: 'Monthly output value', value: monthlyOutput, unit: 'USD/month', source: 'gdp' },
  ], { value: ind.wageShare, unit: 'share' });
  const profitShare = bd('profitShare', 'Profit share', 'Profit share = 1 - wage share = (output - wages) / output.', [
    { label: 'Output not paid as wages', value: Math.max(0, monthlyOutput - wageBill), unit: 'USD/month' }, { label: 'Monthly output value', value: monthlyOutput, unit: 'USD/month', source: 'gdp' },
  ], { value: ind.profitShare, unit: 'share' });
  const debtToGdp = bd('debtToGdp', 'Public debt / GDP', 'Debt / GDP = government debt / nominal annual GDP.', [
    { label: 'Public debt', value: snap.government.debt, unit: 'USD', source: 'debt' }, { label: 'Nominal GDP (annual)', value: ind.gdpAnnual, unit: 'USD/year', source: 'gdp' },
  ], { value: snap.government.debtToGdp, unit: 'ratio' });

  // Gini over households (equal shares inside a household), Lorenz curve from the sorted per-person values.
  const per = households.flatMap((h) => h.members.map(() => Math.max(0, h.netWorth) / Math.max(1, h.members.length))).sort((a, b) => a - b);
  const total = per.reduce((a, b) => a + b, 0);
  let cum = 0;
  const lorenz = [{ populationShare: 0, wealthShare: 0 }, ...per.map((v, i) => ({ populationShare: (i + 1) / per.length, wealthShare: total > 0 ? (cum += v) / total : (i + 1) / per.length }))];
  const gini = {
    ...bd('gini', 'Inequality (Gini)', 'Gini = 1 - 2 x (area under the Lorenz curve). Wealth is net worth; each member of a household carries an equal share. (mock: the headline Gini drifts with toy dynamics)', [
      { label: 'People', value: per.length, unit: 'people' }, { label: 'Total net worth', value: total, unit: 'USD' }, { label: 'Richest household share', value: ind.topHouseholdWealthShare, unit: 'share', source: 'top_wealth_share' },
    ], { value: ind.gini, unit: 'index 0..1' }),
    lorenz, byHousehold: households.map((h) => ({ household: h.id, people: h.members.length, netWorth: h.netWorth, parts: netWorthParts(s, s.households[h.id]!) })),
  };

  // Legitimacy: last month's level plus the signed terms. MOCK weights.
  const u0 = GEN.town.unemployment;
  const raw = [
    { id: 'wellbeing' as const, label: 'Wellbeing', input: ind.avgWellbeing, threshold: 0.5, weight: 0.02 },
    { id: 'unemployment' as const, label: 'Unemployment', input: ind.unemployment, threshold: u0, weight: -0.1 },
    { id: 'inequality' as const, label: 'Inequality (Gini)', input: ind.gini, threshold: 0.7, weight: -0.03 },
    { id: 'debt' as const, label: 'Public debt / GDP', input: snap.government.debtToGdp, threshold: 0.6, weight: -0.01 },
    { id: 'bailout' as const, label: 'Bank bailout this month', input: 0, weight: -0.05 },
  ].map((t) => ({ ...t, contribution: t.weight * (t.input - (t.threshold ?? 0)) + 0 }));
  const trend = raw.reduce((a, t) => a + t.contribution, 0);
  const legitimacy = {
    ...bd('legitimacy', 'Legitimacy', 'Legitimacy = last level + sum of terms; each term = weight x (input - threshold). (mock weights)', raw.map((t) => ({ label: t.label, value: t.input, unit: 'index' })),
      { value: snap.government.legitimacy, unit: 'index 0..1' }, [{ label: 'Last level', value: snap.government.legitimacy - trend, unit: 'index' }, ...raw.map((t) => ({ label: t.label, value: t.contribution, unit: 'index', note: `weight ${t.weight}` }))]),
    terms: raw, trend,
  };
  const b = snap.bank;
  const capitalRatio = bd('capitalRatio', 'Capital ratio', 'Capital ratio = bank equity / bank loans. Lending stops when it is below the limit.', [
    { label: 'Equity', value: b.equity, unit: 'USD' }, { label: 'Loans', value: b.loans, unit: 'USD' }, { label: 'Limit', value: snap.finance.bank.capitalRatioLimit, unit: 'ratio', source: 'bank_capital_ratio' },
  ], { value: snap.finance.bank.capitalRatio, unit: 'ratio' });

  const log = s.emLog ?? [];
  const mon = log.length ? log[log.length - 1]! : emissionsMonth(s.lastMonthlyEmissions, s.lastPlantEmissions, s.forest);
  const turn = sumMonths(log.slice(-3));
  const month = line(mon), tl = line(turn);
  const ok = (l: EmissionsLine) => near(l.firms + l.households + l.plants + (l.aviation ?? 0), l.grossTotal) && near(l.grossTotal + l.sink, l.net) && near(l.net, l.headline);
  const rem = Math.max(0, GEN.carbon.budget - s.cumulativeTonnes);
  const perTurn = Math.max(0, month.net) * GEN.game.ticksPerTurn;
  const runOut = perTurn > 0 ? Math.min(s.turn, GEN.game.lastTurn + 1) + rem / perTurn : undefined;
  const emissions = {
    runway: { perTurn, ...(runOut !== undefined ? { runOutTurn: runOut } : {}), runsOutBeforeEnd: runOut !== undefined && runOut <= GEN.game.lastTurn + 1 },
    month, turn: tl, cumulative: { tonnes: s.cumulativeTonnes, budget: GEN.carbon.budget, remaining: rem, share: GEN.carbon.budget ? s.cumulativeTonnes / GEN.carbon.budget : 0 },
    monthLines: [], turnNumber: s.turn, turnMonths: GEN.game.ticksPerTurn,
    reconciles: ok(month) && ok(tl), note: 'Mock data: sources are split by invented shares; the mock cumulative counts gross emissions.',
  };

  const rev = snap.finance.government.revenue, q = snap.finance.quarter?.govRevenue ?? null;
  const pol = (k: LeverKey) => Number(s.policy[k]);
  const eff = (k: LeverKey) => (LEVERS.find((l) => l.key === k)?.lobbied ? effectiveValue(s, k) : null);
  const tax = (id: TaxRow['id'], label: string, appliesTo: string, k: LeverKey | null, key: keyof typeof rev | null): TaxRow => ({
    id, label, appliesTo, rateNominal: k ? pol(k) : null, rateEffective: k ? eff(k) : null, revenueMonth: key ? rev[key] : 0, revenueQuarter: key && q ? q[key] : 0, modelled: id !== 'gst',
    unit: id === 'carbon' ? 'usd_per_tonne' : 'fraction', baseMonth: null, baseLabel: '', realisedRate: null, note: '',
  });
  const taxes: TaxRow[] = [
    tax('income', 'Income tax', 'Wages, per month, on each household\'s wage income', 'tax_wage', 'wageTax'),
    tax('profit', 'Profit tax', 'Firm profit, per month, before dividends are paid', 'tax_profit', 'profitTax'),
    tax('capital', 'Capital tax', 'Dividends, rent and interest received by households', 'tax_capital', 'capitalTax'),
    tax('wealth', 'Wealth tax', 'Household net worth above the threshold', 'wealth_tax', 'wealthTax'),
    tax('carbon', 'Carbon price', 'Tonnes emitted by firms and plants', 'carbon_price', 'carbon'),
    tax('gst', 'GST / VAT', 'Not modelled: households buy goods and services untaxed', null, null),
  ];
  return { gdp, gpi, wageShare, profitShare, debtToGdp, gini, legitimacy, capitalRatio, emissions, taxes };
}

const GROUP: Partial<Record<LeverKey, string>> = { tax_wage: 'taxes', tax_profit: 'taxes', tax_capital: 'taxes', wealth_tax: 'taxes', wealth_threshold: 'taxes', base_rate: 'money', bank_capital_ratio: 'money', bailout: 'money', min_wage: 'labour', benefit: 'labour', ubi: 'labour', job_guarantee: 'labour', carbon_price: 'nature', emissions_cap: 'nature', green_subsidy: 'nature', commons_harvest: 'nature', commons_access: 'nature', resilience_spending: 'nature', public_spending: 'spending', campaign_cap: 'spending' };
export function buildPolicies(s: MockState, nominalOf: (k: LeverKey) => number | boolean | string): PolicyBoard {
  const rows: PolicyRow[] = LEVERS.map((l) => {
    const pend = s.pending.find((p) => p.command.type === 'set_policy' && p.command.lever === l.key);
    const cv = pend && pend.command.type === 'set_policy' ? pend.command.value : null;
    const active = nominalOf(l.key);
    const effective = l.lobbied ? effectiveValue(s, l.key) : null;
    const def = Number(l.default), nom = Number(active);
    const weakenedBy = l.lobbied && nom !== def && effective !== null ? Math.max(0, 1 - (effective - def) / (nom - def)) : null;
    return {
      lever: l.key, label: l.label, group: GROUP[l.key] ?? 'other',
      initial: l.default, active, effective, pending: pend && cv !== null ? { id: pend.id, value: typeof cv === 'object' ? cv.mode : cv, effectiveTurn: pend.effectiveTurn } : null,
      lastChangeTurn: s.policyLast?.[l.key] ?? null, weakenedBy,
    };
  });
  return { rows, timeline: (s.policyLog ?? []).map((e) => ({ ...e })), cancelRefund: 0 };
}

export function buildIndustry(snap: Pick<Snapshot, 'firms' | 'finance' | 'buildings'>, counts: Record<string, number>): Industry {
  const sp = snap.finance.government.spending;
  return {
    firms: snap.firms.filter((f) => f.active).map((f) => ({
      firm: f.id, name: f.name, sector: f.sector, output: f.output, price: f.price, workers: f.workers, wage: f.wage, revenue: f.revenue, profit: f.profit, emissions: f.emissions,
      ownership: [{ owner: 'household' as const, household: f.owner.kind === 'household' ? f.owner.id : undefined, share: 0.8 }, { owner: 'bank' as const, share: 0.2 }].map((o) => (o.household === undefined ? { owner: o.owner, share: o.share } : o)),
      stateShare: 0, active: true,
    })),
    infrastructure: INFRA_KINDS.map((kind) => ({
      kind, count: counts[kind] ?? 0,
      buildCost: kind === 'public_housing' ? GEN.build.publicHousingCostUsd : kind === 'road' ? GEN.build.roadCostUsd : kind === 'fire_station' ? WX.fireStationCostUsd : GEN.build.plantCostUsd,
      upkeepEachMonthly: 0,
      upkeepMonthly: kind === 'road' ? GEN.build.roadUpkeepUsd : kind === 'fire_station' ? WX.fireStationUpkeepUsd : 0,
    })),
    publicGoods: [
      { id: 'publicSpending', label: 'Government purchases', spendingMonthly: sp.publicSpending, buys: 'Goods and services bought from the town\'s firms (mock: no separate output is tracked)' },
      { id: 'jobGuarantee', label: 'Job guarantee', spendingMonthly: sp.jobGuarantee, buys: 'Wages for unemployed people given public work' },
      { id: 'benefits', label: 'Unemployment benefits', spendingMonthly: sp.benefits, buys: 'Income support for people without a job' },
      { id: 'ubi', label: 'Universal basic income', spendingMonthly: sp.ubi, buys: 'An equal payment to every adult' },
      { id: 'upkeep', label: 'Infrastructure upkeep', spendingMonthly: sp.upkeep, buys: 'Maintenance of roads and fire stations' },
    ],
    note: 'Mock data. No state shareholding exists in the current systems: every firm is owned by households (and the bank), so the state share is 0 (of 1) in every firm.',
  };
}
export { INFRA_LABEL };
