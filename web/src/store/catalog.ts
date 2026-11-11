import type { Catalog, UiBands, WordBands } from '../contract';

/**
 * The UI catalog (contract v1.8, content/ui.json): thresholds, labels and explanatory text the simulation side owns. `createGame` loads it
 * from the port once; panels and the scene read it here. The web keeps layout, formatting, animation, chart scaling, onboarding copy and
 * the wording of wire ids; everything else a player reads as a model fact comes from this object.
 */
let current: Catalog | null = null;
export const setCatalog = (c: Catalog | null): void => { current = c; };
export const catalog = (): Catalog => {
  if (!current) throw new Error('The UI catalog is not loaded (createGame loads it from the simulation port).');
  return current;
};
export const hasCatalog = (): boolean => current !== null;
export const bands = (): UiBands => catalog().bands;

/** Authored text `group.key` from content/ui.json. A missing key shows as `?group.key` (a test checks every key the code names exists). */
export const text = (group: string, key: string): string => catalog().text[group]?.[key] ?? `?${group}.${key}`;
/** Fills `{name}` placeholders. */
export const fill = (s: string, vars: Record<string, string | number>): string => s.replace(/\{(\w+)\}/g, (m, k: string) => (k in vars ? String(vars[k]) : m));
/** The word of the first band whose `below` is above `v`, else `top`. */
export const wordOf = (w: WordBands, v: number): string => w.bands.find((b) => v < b.below)?.word ?? w.top;

/** Labels of wire ids (content/ui.json `text.*_label` groups). */
export const classLabel = (k: string): string => text('class_label', k);
export const classShort = (k: string): string => text('class_short', k);
export const kindLabel = (k: string): string => text('ledger_kind', k);
export const ownerText = (k: string): string => text('owner_text', k);
export const ownerWord = (k: string): string => text('owner_word', k);
/** Label of an emission or income source (content/ui.json `emission_sources`, `income_sources`); the id itself when unknown. */
export const emissionSourceLabel = (id: string): string => catalog().emissionSources.find((s) => s.id === id)?.label ?? id;
export const incomeSourceLabel = (id: string): string => catalog().incomeSources.find((s) => s.id === id)?.label ?? id;
