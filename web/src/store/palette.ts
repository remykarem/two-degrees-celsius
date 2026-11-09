import type { ActionInfo, InfraKind } from '../contract';

export interface PaletteAction { kind: InfraKind; action: ActionInfo }

/** Build palette entries come from the sim's actions(): every place:<kind>. */
export function paletteActions(actions: ActionInfo[]): PaletteAction[] {
  return actions.flatMap((a) => (a.key.startsWith('place:') ? [{ kind: a.key.slice(6) as InfraKind, action: a }] : []));
}
