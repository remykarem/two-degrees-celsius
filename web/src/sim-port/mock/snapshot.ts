import type { HouseholdView, LeverKey, Snapshot } from '../../contract';
import { GEN, LEVERS, MOCK } from './data';
import { effectiveValue, remainingBudget } from './dynamics';
import { SCHEMA_VERSION, type MockState } from './state';

const gdpBase = () => GEN.derived.monthlyOutputValue * 12;
const premiumAvg = () => GEN.derived.monthlyWageBill / (GEN.town.employed * GEN.firms[0]!.wage);

export function buildSnapshot(s: MockState): Snapshot {
  const rem = remainingBudget(s);
  const gdpReal = s.gdp / s.prices;
  const labour = GEN.town.labourForce;
  const employed = Math.round(labour * (1 - s.unemployment));
  const idx = s.gdp / gdpBase();
  const policy = {} as Snapshot['policy'];
  for (const l of LEVERS) policy[l.key as LeverKey] = { nominal: s.policy[l.key]!, effective: l.lobbied ? effectiveValue(s, l.key) : null };
  const monthlyOutput = s.gdp / 12;
  // Mock values computed from the mock state (never constants): the wage bill is employed x wage x the starting premium mix.
  const wageBill = employed * s.wage * premiumAvg();
  const wageShare = Math.min(1, wageBill / monthlyOutput);
  const profitFirms = Math.max(0, monthlyOutput - wageBill);
  const fossilCap = s.buildings.fossil_plant * GEN.build.plantCapacity;
  const renewableCap = s.buildings.renewable_plant * GEN.build.plantCapacity;
  const taxWage = Number(s.policy.tax_wage), taxProfit = Number(s.policy.tax_profit);
  const govReceipts = wageBill * taxWage + profitFirms * taxProfit;

  const households: HouseholdView[] = GEN.households.map((h) => {
    const lf = Math.round(h.people * h.labourShare);
    const emp = Math.round(lf * (1 - s.unemployment));
    const wages = emp * s.wage * h.wagePremium;
    const dividends = h.class === 'capitalist' ? profitFirms * 0.6 * (1 - taxProfit) : 0;
    const rentPaid = h.class === 'rentier' ? 0 : h.people * h.renterShare * GEN.town.rentPrivate;
    const rent = h.class === 'rentier' ? GEN.households.reduce((a, x) => a + (x.class === 'rentier' ? 0 : x.people * x.renterShare * GEN.town.rentPrivate), 0) : 0;
    const benefits = (h.class === 'dependent' ? h.people : lf - emp) * Number(s.policy.benefit) + h.people * Number(s.policy.ubi);
    const income = wages + dividends + rent + benefits;
    return {
      id: h.id, label: h.label, class: h.class, people: h.people, employed: emp, labourForce: lf,
      savings: h.savings, debt: h.debt, wealthPerCapita: (h.savings + h.homeValue - h.debt) / h.people,
      income: { wages, dividends, rent, interest: 0, benefits, taxPaid: wages * taxWage, rentPaid, loanPayments: h.debt * 0.003, spending: income * 0.85 },
      wellbeing: s.wellbeing, politicalPower: h.class === 'capitalist' ? 0.4 : h.class === 'rentier' ? 0.3 : 0.05,
      footprintTonnes: (s.lastMonthlyEmissions * MOCK.extra.householdEmissionShare! / (1 + MOCK.extra.householdEmissionShare!)) * (h.people / 1000),
      foodMet: 0.95, housedShare: 0.95,
    };
  });

  const activeFirms = GEN.firms.filter((_, i) => s.firmActive[i]);
  const intensitySum = activeFirms.reduce((a, f) => a + f.output * f.intensity, 0) || 1;
  const firmEmissions = s.lastMonthlyEmissions / (1 + MOCK.extra.householdEmissionShare!);
  return {
    schemaVersion: SCHEMA_VERSION, game: { ...GEN.game }, turn: s.turn, tick: s.tick, mock: true,
    buildings: { publicHousing: s.buildings.public_housing, fossilPlant: s.buildings.fossil_plant, renewablePlant: s.buildings.renewable_plant },
    gameOver: s.gameOver,
    pc: { current: s.pc, cap: GEN.pc.cap, regenNextTurn: GEN.pc.regenBase * s.legitimacy },
    system: { current: s.system, transition: s.transition ? { ...s.transition } : null },
    pending: s.pending.map((p) => ({ ...p, command: structuredClone(p.command) })), policy, netLobby: s.netLobby, unionDensity: GEN.town.unionDensity,
    indicators: {
      gdpAnnual: s.gdp, gdpRealAnnual: gdpReal, unemployment: s.unemployment, employed, labourForce: labour,
      gini: s.gini, wageShare, profitShare: Math.max(0, 1 - wageShare), avgWellbeing: s.wellbeing,
      remainingBudget: rem, budgetShare: rem / GEN.carbon.budget,
      temperature: GEN.carbon.t0 + (GEN.carbon.tLimit - GEN.carbon.t0) * (s.cumulativeTonnes / GEN.carbon.budget),
      gpiAnnual: gdpReal * (1 - s.gini) - 200 * s.lastMonthlyEmissions * 12 * 0, moneyInCirculation: GEN.town.bank.deposits * idx * s.prices, priceIndex: s.prices,
    },
    households,
    firms: GEN.firms.map((f, i) => {
      const on = s.firmActive[i] ?? false;
      const base = f.workers > 0 ? f.workers : GEN.firms.find((x) => x.sector === f.sector && x.workers > 0)?.workers ?? 0;
      const workers = on ? Math.round((base * (1 - s.unemployment)) / (1 - GEN.town.unemployment)) : 0;
      const output = on ? (f.output > 0 ? f.output : (GEN.firms.find((x) => x.sector === f.sector && x.output > 0)?.output ?? 0) * 0.5) * idx : 0;
      const em = on ? firmEmissions * ((f.output || 1) * f.intensity / intensitySum) : 0;
      return {
        id: f.id, active: on, sector: f.sector, name: f.name, owner: { kind: 'household' as const, id: 0 }, workers, wage: s.wage, price: f.price * s.prices,
        output, emissions: em, revenue: output * f.price * s.prices, profit: on ? profitFirms / activeFirms.length * 0.6 : 0, capital: f.capital * 1500, debt: f.debt, greenTech: 0,
      };
    }),
    bank: { equity: s.bank.equity, loans: s.bank.loans, deposits: GEN.town.bank.deposits * idx, defaultRate: 0.005 + 0.6 * Math.max(0, s.unemployment - 0.1), failed: s.bank.failed },
    government: { cash: s.cash, debt: s.debt, debtToGdp: s.debt / s.gdp, legitimacy: s.legitimacy, receipts: govReceipts, spending: Number(s.policy.public_spending) + Number(s.policy.benefit) * (labour - employed) },
    nature: {
      forest: s.forest, commons: s.commons, commonsAccess: s.policy.commons_access as 'open' | 'community', cumulativeTonnes: s.cumulativeTonnes,
      netEmissionsMonthly: s.lastMonthlyEmissions - s.lastMonthlyEmissions * 0.1 * s.forest, grossEmissionsMonthly: s.lastMonthlyEmissions, sinkMonthly: s.lastMonthlyEmissions * 0.1 * s.forest,
    },
    housing: {
      privateCap: GEN.town.privateHousing, publicCap: s.buildings.public_housing, rentPrivate: GEN.town.rentPrivate, rentPublic: GEN.town.rentPublic,
      occupancy: 0.95, housedShare: 0.95,
    },
    energy: { fossilCap, renewableCap, fossilShare: fossilCap / Math.max(1, fossilCap + renewableCap) },
    history: s.history.map((h) => ({ ...h })),
  };
}
