// Generates src/sim-port/mock/data.generated.json from content/params.json, content/initial_town.json and content/systems/*.json.
// Run: npm run sync-mock. A test (tests/mock-sync.test.ts) fails when the committed file is stale.
// Everything numeric that the REAL simulation owns (turn costs, lead times, political capital, build costs, starting policy,
// carbon budget, transition lengths, calendar, cohorts, firms) comes from content/. Nothing here is invented, except the
// two constants marked UI_CHOICE below.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '..', '..');
const content = (p) => JSON.parse(readFileSync(resolve(repo, 'content', p), 'utf8'));
export const OUT_PATH = resolve(here, '..', 'src', 'sim-port', 'mock', 'data.generated.json');

/** UI_CHOICE: content/ has no calendar start; the UI picks the first year shown. */
const START_YEAR = 2031;
const SYSTEM_KEYS = { market: 'market', regulated: 'regulated', coop: 'coop' };
const POLICY_KEYS = {
  carbon_price_usd: 'carbon_price', tax_wage: 'tax_wage', tax_profit: 'tax_profit', tax_capital: 'tax_capital', wealth_tax: 'wealth_tax',
  wealth_threshold_usd: 'wealth_threshold', base_rate: 'base_rate', min_wage_usd: 'min_wage', benefit_usd: 'benefit', ubi_usd: 'ubi',
  green_subsidy: 'green_subsidy', campaign_cap: 'campaign_cap', emissions_cap: 'emissions_cap', commons_harvest: 'commons_harvest',
  bank_capital_ratio: 'bank_capital_ratio', bailout: 'bailout', job_guarantee: 'job_guarantee', public_spending_usd: 'public_spending',
};
const camel = (o) => ({ baseCost: o.base_cost, leadTime: o.lead_time, unit: o.unit, minSize: o.min_size });
const sum = (xs) => xs.reduce((a, b) => a + b, 0);

export function generate() {
  const params = content('params.json');
  const town = content('initial_town.json');
  const t = params.turn;
  const g = params.government;

  const turnCosts = {};
  for (const [k, v] of Object.entries(t.levers)) turnCosts[k] = camel(v);

  const policyStart = { commons_access: 'open' };
  for (const [k, v] of Object.entries(town.policy)) {
    if (!POLICY_KEYS[k]) throw new Error(`unknown policy key in initial_town.json: ${k}`);
    policyStart[POLICY_KEYS[k]] = v;
  }

  const systems = {};
  for (const key of Object.keys(SYSTEM_KEYS)) {
    const s = content(`systems/${key}.json`);
    systems[key] = { name: s.name, transitionMonths: s.transition_months, outputPenalty: s.transition_output_penalty, lambdaMax: s.lambda_max };
  }

  const households = town.cohorts.map((c, id) => ({
    id, label: c.label, class: c.class.toLowerCase(), people: c.weight, labourShare: c.labour_share, wagePremium: c.wage_premium,
    savings: c.savings_usd, debt: c.debt_usd, homeValue: c.home_value_usd, monthlyIncome: c.initial_income_usd, renterShare: c.renter_share,
  }));
  const seen = { goods: 0, food: 0 };
  const firms = town.firms.map((f, id) => {
    const sector = f.sector.toLowerCase();
    seen[sector] += 1;
    const base = sector === 'goods' ? 'Factory' : 'Farm';
    return {
      id, sector, active: f.active, name: seen[sector] === 1 ? base : `${base} ${seen[sector]}`, capital: f.capital_units, cash: f.cash_usd, debt: f.debt_usd,
      wage: f.wage_usd, price: f.price_usd, output: f.initial_output, intensity: f.intensity_base,
      workers: sum(f.employment), wageBill: f.wage_usd * sum(f.employment.map((n, h) => n * town.cohorts[h].wage_premium)),
    };
  });

  const active = firms.filter((f) => f.active);
  const labourForce = sum(town.cohorts.map((c) => c.weight * c.labour_share));
  const employed = sum(active.map((f) => f.workers));
  const loans = sum(town.cohorts.map((c) => c.debt_usd)) + sum(firms.map((f) => f.debt)) + town.gov_debt_usd;
  const deposits = sum(town.cohorts.map((c) => c.savings_usd)) + sum(firms.map((f) => f.cash)) + town.gov_cash_usd;
  const equity = loans - deposits;
  const kappa = Math.max(town.policy.bank_capital_ratio, 1e-9);
  const plantCap = g.plant_capacity;

  return {
    generatedFrom: ['content/params.json', 'content/initial_town.json', 'content/systems/*.json'],
    game: { startYear: START_YEAR, firstTurn: 1, lastTurn: params.run.turns, turnsPerYear: 12 / params.run.ticks_per_turn },
    ticksPerTurn: params.run.ticks_per_turn,
    pc: { start: t.pc_start, cap: t.pc_cap, regenBase: t.pc_base, cancelRefund: t.cancel_refund },
    turnCosts,
    policyStart,
    systems,
    build: {
      plantCapacity: plantCap, publicHousingCostUsd: g.public_housing_cost_usd, plantCostUsd: g.plant_cost_usd,
      start: { publicHousing: town.public_housing, fossilPlant: Math.round(town.fossil_capacity / plantCap), renewablePlant: Math.round(town.renewable_capacity / plantCap) },
    },
    town: {
      govCash: town.gov_cash_usd, govDebt: town.gov_debt_usd, legitimacy: town.legitimacy, unionDensity: town.union_density,
      forest: town.forest, forestMax: params.nature.forest_max, commons: town.commons, commonsMax: params.nature.commons_max,
      privateHousing: town.private_housing, publicHousing: town.public_housing, rentPrivate: town.rent_private_usd, rentPublic: town.rent_public_usd,
      bank: { loans, deposits, equity, headroom: Math.max(0, equity / kappa - loans) },
      labourForce, employed, unemployment: 1 - employed / labourForce,
    },
    carbon: { budget: params.nature.budget_b0, t0: params.nature.t0, tLimit: params.nature.t_limit },
    collapseUnemployment: g.collapse_unemployment,
    // Derived from the starting firms. Monthly firm emissions only; the mock adds household emissions itself (mock-only).
    derived: {
      monthlyFirmEmissions: sum(active.map((f) => f.output * f.intensity)),
      monthlyOutputValue: sum(active.map((f) => f.output * f.price)),
      monthlyWageBill: sum(active.map((f) => f.wageBill)),
    },
    households,
    firms,
  };
}

export function render() {
  return JSON.stringify(generate(), null, 2) + '\n';
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  writeFileSync(OUT_PATH, render());
  console.log(`wrote ${OUT_PATH}`);
}
