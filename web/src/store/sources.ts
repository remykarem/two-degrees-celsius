// Emission and income sources are lists on the wire (contract v2.0, docs/snapshot-contract.md section 20).
// These helpers read a list by id and total it; labels and categories come with each entry, from content.
import type { EmissionsLine, EmissionSourceTotal, IncomeSource } from '../contract';

/** Dollars of one income source by id (0 when the list has no such entry). */
export const incomeOf = (list: readonly IncomeSource[], id: string): number => list.find((s) => s.id === id)?.amount ?? 0;

/** Total income: entries that are part of another (domestic wages inside wages) are not added twice. */
export const incomeTotal = (list: readonly IncomeSource[]): number => list.reduce((a, s) => a + (s.partOf ? 0 : s.amount), 0);

/** Tonnes of one emission source of a line by id (0 when the line omits it). */
export const emissionOf = (line: Pick<EmissionsLine, 'sources'> | undefined, id: string): number => line?.sources.find((s) => s.id === id)?.tonnes ?? 0;

/** The sources of the month and turn lines in one order: the month's order, then any the turn adds. */
export const sourceIds = (...lines: Pick<EmissionsLine, 'sources'>[]): EmissionSourceTotal[] => {
  const seen = new Map<string, EmissionSourceTotal>();
  for (const l of lines) for (const s of l.sources) if (!seen.has(s.id)) seen.set(s.id, s);
  return [...seen.values()];
};
