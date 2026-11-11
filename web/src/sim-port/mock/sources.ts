import type { EmissionsLine, FinanceFlows, HouseholdView, IncomeSource } from '../../contract';
import { GEN } from './data';

// MOCK ONLY. The mock computes with named fields internally and converts to the wire's lists at the boundary, using the real labels
// and categories from the catalog fixture (content/ui.json). The mock goes away in phase 2 (docs/taxonomy.md section 8.3).

/** The mock's internal income fields of one household (a month). */
export interface IncomeFields { wages: number; dividends: number; rent: number; interest: number; benefits: number; domesticWages?: number }
/** The mock's internal finance flows: the wire's, with household income as named fields (converted to a list by `wireFlows`). */
export type MockFlows = Omit<FinanceFlows, 'householdIncome'> & { householdIncome: Required<IncomeFields> };
export const wireFlows = (f: MockFlows): FinanceFlows => ({ ...f, householdIncome: incomeList(f.householdIncome, 'finance') });
export type MockIncome = IncomeFields & Omit<HouseholdView['income'], 'sources'>;

const FIELD: Record<string, keyof IncomeFields> = { wages: 'wages', domestic_wages: 'domesticWages', dividends: 'dividends', rent: 'rent', interest: 'interest', benefits: 'benefits' };

/** The wire list of income sources. The household view has no domestic-wages entry (the real wire reports it in finance flows only). */
export function incomeList(f: IncomeFields, view: 'household' | 'finance'): IncomeSource[] {
  return GEN.catalog.incomeSources
    .filter((d) => view === 'finance' || d.id !== 'domestic_wages')
    .map((d) => ({ id: d.id, label: d.label, category: d.category, ...(d.partOf ? { partOf: d.partOf } : {}), amount: f[FIELD[d.id]!] ?? 0 }));
}

/** Reads a wire income list back into the mock's fields. */
export function incomeFields(list: readonly IncomeSource[]): Required<IncomeFields> {
  const o: Required<IncomeFields> = { wages: 0, dividends: 0, rent: 0, interest: 0, benefits: 0, domesticWages: 0 };
  for (const s of list) o[FIELD[s.id]!] = s.amount;
  return o;
}

const EM_FIELD: Record<string, 'firms' | 'households' | 'plants' | 'aviation'> = { firms: 'firms', households: 'households', plants: 'plants', aviation: 'aviation' };

/** The wire sources of an emissions line from the mock's per-source tonnes (a zero source is left out unless the content shows it when zero). */
export function emissionSources(m: { firms: number; households: number; plants: number; aviation?: number }): EmissionsLine['sources'] {
  return GEN.catalog.emissionSources
    .map((d) => ({ id: d.id, label: d.label, category: d.category, tonnes: m[EM_FIELD[d.id]!] ?? 0 }))
    .filter((x, i) => x.tonnes !== 0 || GEN.catalog.emissionSources[i]!.showWhenZero);
}
