import type { LeverKey, Snapshot } from '../../contract';
import { DATA } from './data';
import { effectiveValue, remainingBudget } from './dynamics';
import type { MockState } from './state';

export function buildSnapshot(s: MockState): Snapshot {
  const d = DATA.dynamics;
  const rem = remainingBudget(s);
  const gdpReal = s.gdp / s.prices;
  const employed = Math.round(s.unemployment === 1 ? 0 : DATA.households.reduce((a, h) => a + h.people, 0) * 0.5 * (1 - s.unemployment));
  const labour = Math.round(DATA.households.reduce((a, h) => a + h.people, 0) * 0.5);
  const policy = {} as Snapshot['policy'];
  for (const l of DATA.levers) {
    const nominal = l.key === 'commons_access' ? s.commonsMode : (s.policy[l.key] as number | boolean);
    policy[l.key as LeverKey] = { nominal, effective: l.lobbied ? effectiveValue(s, l.key) : null };
  }
  const govMonthly = s.gdp / 12;
  return {
    schemaVersion: s.schemaVersion, turn: s.turn, tick: s.tick, gameOver: s.gameOver,
    pc: { current: s.pc, cap: DATA.pc.cap, regenNextTurn: Math.round(DATA.pc.regenBase * s.legitimacy) },
    system: { current: s.system, transition: s.transition },
    pending: s.pending.map((p) => ({ ...p })), policy, netLobby: s.netLobby, unionDensity: DATA.start.unionDensity!,
    indicators: {
      gdpAnnual: s.gdp, gdpRealAnnual: gdpReal, unemployment: s.unemployment, employed, labourForce: labour,
      gini: s.gini, wageShare: 0.55, profitShare: 0.3, avgWellbeing: s.wellbeing,
      remainingBudget: rem, budgetShare: rem / d.carbonBudget!,
      temperature: d.temperatureBase! + s.cumulativeTonnes * d.temperaturePerTonne!,
      gpiAnnual: gdpReal * s.wellbeing * (0.5 + 0.5 * s.forest), moneyInCirculation: s.gdp * 0.4, priceIndex: s.prices,
    },
    households: DATA.households.map((h) => ({
      id: h.id, label: h.label, class: h.cls, people: h.people,
      employed: Math.round(h.people * 0.5 * (1 - s.unemployment)), labourForce: Math.round(h.people * 0.5),
      savings: h.people * 1000, debt: h.people * 200, wealthPerCapita: 1000 * (h.cls === 'capitalist' ? 40 : h.cls === 'rentier' ? 25 : 2),
      income: { wages: h.people * 160, dividends: 0, rent: 0, interest: 0, benefits: 0, taxPaid: h.people * 30, rentPaid: h.people * 40, loanPayments: 0, spending: h.people * 90 },
      wellbeing: s.wellbeing, politicalPower: h.cls === 'capitalist' ? 0.4 : 0.1, footprintTonnes: s.lastMonthlyEmissions * 0.2 / DATA.households.length,
      foodMet: 0.95, housedShare: 0.9,
    })),
    firms: DATA.firms.map((f, i) => ({
      id: f.id, active: s.firmActive[i] ?? false, sector: f.sector, name: f.name, owner: { kind: 'household', id: 1 },
      workers: s.firmActive[i] ? Math.round(employed / 2) : 0, wage: 1500, price: s.prices, output: govMonthly * 0.2, emissions: s.lastMonthlyEmissions * 0.4,
      revenue: govMonthly * 0.2, profit: govMonthly * 0.03, capital: 1e6, debt: 1e5, greenTech: 0.1,
    })),
    bank: { equity: 5e5, loans: 2e6, deposits: 2.4e6, defaultRate: 0.02, failed: false },
    government: { cash: s.cash, debt: s.debt, debtToGdp: s.debt / s.gdp, legitimacy: s.legitimacy, receipts: govMonthly * 0.25, spending: govMonthly * 0.25 },
    nature: {
      forest: s.forest, commons: s.commons, commonsAccess: s.commonsMode, cumulativeTonnes: s.cumulativeTonnes,
      netEmissionsMonthly: s.lastMonthlyEmissions * 0.9, grossEmissionsMonthly: s.lastMonthlyEmissions, sinkMonthly: s.lastMonthlyEmissions * 0.1,
    },
    housing: {
      privateCap: DATA.start.privateHousing!, publicCap: s.infra.public_housing ?? 0, rentPrivate: 900, rentPublic: 450,
      occupancy: 0.9, housedShare: 0.9,
    },
    energy: {
      fossilCap: s.infra.fossil_plant ?? 0, renewableCap: s.infra.renewable_plant ?? 0,
      fossilShare: (s.infra.fossil_plant ?? 0) / Math.max(1, (s.infra.fossil_plant ?? 0) + (s.infra.renewable_plant ?? 0)),
    },
    history: s.history.map((h) => ({ ...h })),
  };
}
