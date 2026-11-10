import type { EnergyView, FinanceView, Snapshot } from '../../contract';

/**
 * A package built before contract v1.4 sends `energy` in the older shape (fossilCap, supply as a number, ...) and has no `finance`.
 * This rewrites those two fields into the v1.4 shapes from what the snapshot does carry, so the UI has one shape to read. A v1.4 package passes through untouched.
 * The numbers it derives are approximate (no ledger behind them): the legacy `energy` unit is 'MWh' only by the Town v5 proposal.
 */
export function upgradeSnapshot(s: Snapshot): Snapshot {
  const e = s.energy as unknown as { supply?: unknown; fossilCap?: number; renewableCap?: number; demand?: number; served?: number; gridIntensity?: number };
  if (e && typeof e.supply === 'number') {
    const cap = (e.fossilCap ?? 0) + (e.renewableCap ?? 0);
    const next: EnergyView = {
      unit: 'MWh',
      supply: s.plants.map((p) => ({ tile: { ...p.tile }, kind: p.kind, capacity: p.capacity })),
      demand: { firms: 0, households: 0, total: e.demand ?? 0 },
      served: e.served ?? 1, intensity: e.gridIntensity ?? 0, renewableShare: cap > 0 ? (e.renewableCap ?? 0) / cap : 0,
      emissionsByPlant: s.plants.filter((p) => p.emissionsMonthly > 0).map((p) => ({ tile: { ...p.tile }, tonnes: p.emissionsMonthly })),
    };
    s.energy = next;
  }
  if (!s.finance) {
    const sum = (f: (h: Snapshot['households'][number]) => number) => s.households.reduce((a, h) => a + f(h), 0);
    const income = { wages: sum((h) => h.income.wages), dividends: sum((h) => h.income.dividends), rent: sum((h) => h.income.rent), interest: sum((h) => h.income.interest), benefits: sum((h) => h.income.benefits) };
    const taxes = sum((h) => h.income.taxPaid);
    const totalIn = Object.values(income).reduce((a, b) => a + b, 0);
    const spending = sum((h) => h.income.spending);
    const rentPaid = sum((h) => h.income.rentPaid), loanPayments = sum((h) => h.income.loanPayments);
    const active = s.firms.filter((f) => f.active);
    const limit = Number(s.policy.bank_capital_ratio?.nominal ?? 0);
    const g = s.government;
    const fin: FinanceView = {
      government: {
        revenue: { wageTax: taxes, profitTax: 0, capitalTax: 0, wealthTax: 0, carbon: 0, other: Math.max(0, g.receipts - taxes) },
        spending: { benefits: income.benefits, ubi: 0, publicSpending: 0, jobGuarantee: 0, upkeep: 0, subsidy: 0, interest: 0, builds: 0, other: Math.max(0, g.spending - income.benefits) },
        balance: g.receipts - g.spending, debt: g.debt, debtToGdp: g.debtToGdp, legitimacy: g.legitimacy, basis: 'month',
      },
      bank: { equity: s.bank.equity, loans: s.bank.loans, deposits: s.bank.deposits, capitalRatio: s.bank.loans > 0 ? s.bank.equity / s.bank.loans : 0, capitalRatioLimit: limit, headroom: limit > 0 ? Math.max(0, s.bank.equity / limit - s.bank.loans) : 0, defaultRate: s.bank.defaultRate, interestIncome: loanPayments },
      households: { income, taxes, rentPaid, loanPayments, spending, savingRate: totalIn > 0 ? (totalIn - taxes - rentPaid - loanPayments - spending) / totalIn : 0, debt: sum((h) => h.debt), netWorth: sum((h) => h.netWorth) },
      firms: { revenue: active.reduce((a, f) => a + f.revenue, 0), wageBill: income.wages, profit: active.reduce((a, f) => a + f.profit, 0), investment: 0, debt: active.reduce((a, f) => a + f.debt, 0), cash: 0 },
      money: { supply: s.indicators.moneyInCirculation, createdThisTurn: 0, destroyedThisTurn: 0 },
      priceIndex: s.indicators.priceIndex, wageShare: s.indicators.wageShare, profitShare: s.indicators.profitShare,
    };
    s.finance = fin;
  }
  return s;
}

/** Upgrades every snapshot inside a port reply (a bare snapshot, or an object with a `snapshot`). */
export function upgradeResult<T>(r: { ok: true; value: T } | { ok: false; error: { code: string; message: string } }) {
  if (!r.ok) return r;
  const v = r.value as unknown as Record<string, unknown> | null;
  if (v && typeof v === 'object') {
    if ('schemaVersion' in v && 'map' in v) upgradeSnapshot(v as unknown as Snapshot);
    else if (v.snapshot && typeof v.snapshot === 'object') upgradeSnapshot(v.snapshot as Snapshot);
  }
  return r;
}
