import type { LeverKey, SchemaVersion, Snapshot, Tile } from '../../contract';
import { GEN, LEVERS, MOCK } from './data';
import { effectiveValue, remainingBudget } from './dynamics';
import { plantTiles } from './frames';
import { energyView, financeView } from './finance';
import { economy, householdViews, median, peopleViews, personIncomes, topWealthShare } from './people';
import { buildBreakdowns, buildIndustry, buildPolicies } from './transparency';
import { healthOf, tileFireRisk, weatherView } from './weather';
import { buildingCounts, cellAt, GRID, SCHEMA_VERSION, type MockState } from './state';

const gdpBase = () => GEN.derived.monthlyOutputValue * 12;

export function mapTiles(s: MockState): Tile[] {
  const tiles: Tile[] = [];
  const plants = plantTiles(s).length;
  for (let y = 0; y < GRID; y++) {
    for (let x = 0; x < GRID; x++) {
      const c = cellAt(s, x, y)!;
      const t: Tile = { x, y, kind: c.kind, buildable: c.kind === 'empty' };
      if (c.ref !== undefined) t.ref = c.ref;
      if (c.kind === 'forest') { t.state = c.state ?? 'standing'; if (t.state === 'standing') t.fireRisk = tileFireRisk(s, x, y); }
      if (c.kind === 'fossil_plant') t.emissionsMonthly = plants ? s.lastPlantEmissions / plants : 0;
      if (c.kind === 'renewable_plant') t.emissionsMonthly = 0;
      tiles.push(t);
    }
  }
  return tiles;
}

export function buildSnapshot(s: MockState): Snapshot {
  const rem = remainingBudget(s);
  const gdpReal = s.gdp / s.prices;
  const lf = s.people.filter((p) => p.labourForce);
  const labour = lf.length;
  const employed = lf.filter((p) => p.job !== null).length;
  const unemployment = labour ? 1 - employed / labour : 0;
  const idx = s.gdp / gdpBase();
  const policy = {} as Snapshot['policy'];
  for (const l of LEVERS) policy[l.key as LeverKey] = { nominal: s.policy[l.key]!, effective: l.lobbied ? effectiveValue(s, l.key) : null };
  const e = economy(s);
  const wageShare = Math.min(1, e.wageBill / e.monthlyOutput);
  const counts = buildingCounts(s);
  const taxWage = Number(s.policy.tax_wage), taxProfit = Number(s.policy.tax_profit);
  const govReceipts = e.wageBill * taxWage + e.profitFirms * taxProfit;

  const people = peopleViews(s);
  const incomes = personIncomes(s);
  const activeFirms = GEN.firms.filter((f) => s.firmActive[f.id]);
  const intensitySum = activeFirms.reduce((a, f) => a + f.output * f.intensity, 0) || 1;
  const firmEmissions = (s.lastMonthlyEmissions - s.lastPlantEmissions) / (1 + MOCK.extra.householdEmissionShare!);
  const households = householdViews(s);
  const firmOutput = activeFirms.reduce((a, f) => a + (f.output > 0 ? f.output : (GEN.firms.find((x) => x.sector === f.sector && x.output > 0)?.output ?? 0) * 0.5) * idx, 0);
  const wellbeing = people.length ? people.reduce((a, p) => a + p.wellbeing, 0) / people.length : 0;
  const snapBase: Snapshot = {
    schemaVersion: SCHEMA_VERSION as SchemaVersion, game: { ...GEN.game }, turn: s.turn, tick: s.tick, mock: true,
    map: { width: 12, height: 12, tiles: mapTiles(s) },
    buildings: { publicHousing: counts.public_housing, fossilPlant: counts.fossil_plant, renewablePlant: counts.renewable_plant, road: counts.road },
    gameOver: s.gameOver,
    pc: { current: s.pc, cap: GEN.pc.cap, regenNextTurn: GEN.pc.regenBase * s.legitimacy },
    system: { current: s.system, transition: s.transition ? { ...s.transition } : null },
    pending: s.pending.map((p) => ({ ...p, command: structuredClone(p.command) })), policy, netLobby: s.netLobby, unionDensity: GEN.town.unionDensity,
    indicators: {
      gdpAnnual: s.gdp, gdpRealAnnual: gdpReal, unemployment, employed, labourForce: labour,
      gini: s.gini, wageShare, profitShare: Math.max(0, 1 - wageShare), avgWellbeing: wellbeing,
      remainingBudget: rem, budgetShare: rem / GEN.carbon.budget,
      temperature: GEN.carbon.t0 + (GEN.carbon.tLimit - GEN.carbon.t0) * (s.cumulativeTonnes / GEN.carbon.budget),
      gpiAnnual: gdpReal * (1 - s.gini), moneyInCirculation: GEN.town.bank.deposits * idx * s.prices, priceIndex: s.prices,
      avgIncomePerPerson: incomes.reduce((a, b) => a + b, 0) / Math.max(1, incomes.length), medianIncomePerPerson: median(incomes), topHouseholdWealthShare: topWealthShare(s),
    },
    people,
    households,
    homes: s.homes.map((h) => ({ id: h.id, tile: { x: h.x, y: h.y }, owner: h.owner, household: h.household, rent: h.owner === 'public' ? GEN.town.rentPublic : GEN.town.rentPrivate, value: h.owner === 'public' ? GEN.build.publicHousingCostUsd : GEN.homes[0]!.valueUsd })),
    plants: plantTiles(s).map((tile, id) => ({ id, kind: 'fossil' as const, tile, capacity: GEN.build.plantCapacity, output: 0, emissionsMonthly: s.lastPlantEmissions / Math.max(1, plantTiles(s).length) })),
    firms: GEN.firms.map((f) => {
      const on = s.firmActive[f.id] ?? false;
      const workers = s.people.filter((p) => p.job === f.id).length;
      const output = on ? (f.output > 0 ? f.output : (GEN.firms.find((x) => x.sector === f.sector && x.output > 0)?.output ?? 0) * 0.5) * idx : 0;
      const em = on ? firmEmissions * ((f.output || 1) * f.intensity / intensitySum) : 0;
      return {
        id: f.id, active: on, sector: f.sector, name: f.name, tile: { ...f.tile }, owner: { kind: 'household' as const, id: f.founder }, workers, wage: s.wage, price: f.price * s.prices,
        output, emissions: em, revenue: output * f.price * s.prices, profit: on ? e.profitFirms / Math.max(1, activeFirms.length) * 0.6 : 0, capital: f.capital * 1500, debt: f.debt, greenTech: 0,
      };
    }),
    bank: { equity: s.bank.equity, loans: s.bank.loans, deposits: GEN.town.bank.deposits * idx, defaultRate: 0.005 + 0.6 * Math.max(0, s.unemployment - 0.1), failed: s.bank.failed },
    government: { cash: s.cash, debt: s.debt, debtToGdp: s.debt / s.gdp, legitimacy: s.legitimacy, receipts: govReceipts, spending: Number(s.policy.public_spending) + Number(s.policy.benefit) * (labour - employed) + counts.road * GEN.build.roadUpkeepUsd },
    nature: {
      forest: s.forest, commons: s.commons, forestStock: s.forest * GEN.town.forestMax, commonsStock: s.commons * GEN.town.commonsMax, forestHealth: healthOf(s).forest, commonsHealth: healthOf(s).commons, farmHealth: healthOf(s).farm, commonsAccess: s.policy.commons_access as 'open' | 'community', cumulativeTonnes: s.cumulativeTonnes,
      netEmissionsMonthly: s.lastMonthlyEmissions - s.lastMonthlyEmissions * 0.1 * s.forest, grossEmissionsMonthly: s.lastMonthlyEmissions, sinkMonthly: s.lastMonthlyEmissions * 0.1 * s.forest,
    },
    weather: weatherView(s),
    housing: {
      homes: s.homes.length, rentPrivate: GEN.town.rentPrivate, rentPublic: GEN.town.rentPublic,
      occupancy: s.homes.filter((h) => h.household !== null).length / Math.max(1, s.homes.length), unhousedHouseholds: s.households.filter((h) => h.home === null).length,
    },
    finance: financeView(s, households, { moneyInCirculation: GEN.town.bank.deposits * idx * s.prices, priceIndex: s.prices, wageShare, profitShare: Math.max(0, 1 - wageShare) },
      { deposits: GEN.town.bank.deposits * idx, defaultRate: 0.005 + 0.6 * Math.max(0, s.unemployment - 0.1) }, s.debt / s.gdp),
    energy: energyView(s, households, firmOutput),
    history: s.history.map((h) => ({ ...h })),
  };
  const nominalOf = (k: LeverKey) => s.policy[k]!;
  return {
    ...snapBase,
    breakdowns: buildBreakdowns(s, snapBase, households, e.monthlyOutput, e.wageBill),
    policies: buildPolicies(s, nominalOf),
    industry: buildIndustry(snapBase, counts),
  };
}
