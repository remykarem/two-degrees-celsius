import type { LeverInfo } from '../contract';

/**
 * Display labels that differ from what the wire says. The lever key `tax_wage` stays; its label reads "Income tax" everywhere in the UI
 * (docs/snapshot-contract.md section 16 sends "Wage tax" from older content, and the Rust side may keep the old text).
 */
export const uiLabel = (s: string): string => s.replace(/\bWage tax\b/g, 'Income tax').replace(/\bwage tax\b/g, 'income tax');
export const relabelLevers = (ls: LeverInfo[]): LeverInfo[] => ls.map((l) => (uiLabel(l.label) === l.label ? l : { ...l, label: uiLabel(l.label) }));
