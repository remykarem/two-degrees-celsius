import type { EnergyView, FinanceView, HouseholdView } from '../../contract';
import { GEN } from './data';
import { moneyMoved, sumFlows } from './ledger';
import { plantTiles } from './frames';
import type { MockState } from './state';

// MOCK ONLY: the finance view is summed from the mock ledger's own bookings; the energy numbers are invented (labelled in the UI by the mock badge).

export function financeView(s: MockState, households: HouseholdView[], ind: { moneyInCirculation: number; priceIndex: number; wageShare: number; profitShare: number }, bank: { deposits: number; defaultRate: number }, debtToGdp: number): FinanceView {
  const L = s.ledger;
  const m = L.months[L.months.length - 1];
  const quarter = L.months.length ? sumFlows(L.months) : undefined;
  const flows = m ?? sumFlows([]);
  const income = flows.householdIncome;
  const totalIn = income.wages + income.dividends + income.rent + income.interest + income.benefits;
  const moved = moneyMoved(s);
  const firmsCash = GEN.firms.reduce((a, f) => a + (L.balance[`f${f.id}`] ?? 0), 0);
  const limit = Number(s.policy.bank_capital_ratio);
  const ratio = s.bank.loans > 0 ? s.bank.equity / s.bank.loans : 0;
  const govIn = Object.values(flows.govRevenue).reduce((a, b) => a + b, 0);
  const govOut = Object.values(flows.govSpending).reduce((a, b) => a + b, 0);
  return {
    government: { revenue: flows.govRevenue, spending: flows.govSpending, balance: govIn - govOut, debt: s.debt, debtToGdp, legitimacy: s.legitimacy, basis: 'month' },
    bank: { equity: s.bank.equity, loans: s.bank.loans, deposits: bank.deposits, capitalRatio: ratio, capitalRatioLimit: limit, headroom: limit > 0 ? Math.max(0, s.bank.equity / limit - s.bank.loans) : 0, defaultRate: bank.defaultRate, interestIncome: flows.bankInterestIncome },
    households: {
      income, taxes: flows.householdTaxes, rentPaid: flows.rentPaid, loanPayments: flows.loanPayments, spending: flows.householdSpending,
      savingRate: totalIn > 0 ? Math.max(-1, Math.min(1, (totalIn - flows.householdTaxes - flows.rentPaid - flows.loanPayments - flows.householdSpending) / totalIn)) : 0,
      debt: households.reduce((a, h) => a + h.debt, 0), netWorth: households.reduce((a, h) => a + h.netWorth, 0),
    },
    firms: { revenue: flows.firmRevenue, wageBill: flows.wageBill, profit: flows.firmProfit, investment: flows.investment, debt: GEN.firms.reduce((a, f) => a + (s.firmActive[f.id] ? f.debt : 0), 0), cash: firmsCash },
    money: { supply: ind.moneyInCirculation, createdThisTurn: moved.created, destroyedThisTurn: moved.destroyed },
    priceIndex: ind.priceIndex, wageShare: ind.wageShare, profitShare: ind.profitShare,
    ...(quarter ? { quarter } : {}),
  };
}

const HOUSEHOLD_BASE = 0.1, HOUSEHOLD_PER_CONSUMPTION = 0.15, CONSUMPTION_SCALE = 2000, FIRM_PER_UNIT = 0.01, FOSSIL_FACTOR = 1; // MOCK: the Town v5 doc's example values

export function energyView(s: MockState, households: HouseholdView[], firmOutput: number): EnergyView {
  const fossil = plantTiles(s);
  const renewable: { x: number; y: number }[] = [];
  for (let y = 0; y < 12; y++) for (let x = 0; x < 12; x++) if (s.grid[y * 12 + x]!.kind === 'renewable_plant') renewable.push({ x, y });
  const cap = GEN.build.plantCapacity;
  const supply = [...fossil.map((tile) => ({ tile, kind: 'fossil' as const, capacity: cap })), ...renewable.map((tile) => ({ tile, kind: 'renewable' as const, capacity: cap }))];
  const hh = households.reduce((a, h) => a + h.members.length * HOUSEHOLD_BASE + HOUSEHOLD_PER_CONSUMPTION * (h.income.spending / CONSUMPTION_SCALE), 0);
  const firms = firmOutput * FIRM_PER_UNIT;
  const total = hh + firms;
  const capacity = supply.length * cap;
  const served = total > 0 ? Math.min(1, capacity / total) : 1;
  const fossilCap = fossil.length * cap;
  const delivered = Math.min(total, capacity);
  const fossilUsed = capacity > 0 ? delivered * (fossilCap / capacity) : 0;
  return {
    unit: 'MWh', supply, demand: { firms, households: hh, total }, served, intensity: delivered > 0 ? (fossilUsed * FOSSIL_FACTOR) / delivered : 0,
    renewableShare: capacity > 0 ? 1 - fossilCap / capacity : 0,
    emissionsByPlant: fossil.map((tile) => ({ tile, tonnes: s.lastPlantEmissions / Math.max(1, fossil.length) })),
  };
}
