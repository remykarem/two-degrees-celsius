import type { EntityKind, InfraKind, InspectorSection, Role } from '../contract';
import { catalog } from './catalog';

/**
 * The entity registry (content/entities.json, contract v2.1, served in `catalog().entities`): labels, roles, tiles, scene model ids and
 * inspector sections of every kind of actor, facility and stock. The web asks it; it holds no per-entity wording or role checks.
 */
export const entities = (): EntityKind[] => catalog().entities;
export const entityKind = (id: string): EntityKind | undefined => entities().find((e) => e.id === id);
/** The entity standing on a map tile kind (every wire tile kind has exactly one; an unknown kind is undefined). */
export const entityOfTile = (tileKind: string): EntityKind | undefined => entities().find((e) => e.tileKinds.some((k) => k === tileKind));
/** The buildable entity of a wire `InfraKind`. */
export const entityOfInfra = (kind: InfraKind): EntityKind | undefined => entities().find((e) => e.infraKind === kind);
export const hasRole = (e: EntityKind | undefined, r: Role): boolean => !!e && e.roles.includes(r);
/** Does the entity standing on this tile kind have the role? (The scene asks roles, never kind names.) */
export const tileHasRole = (tileKind: string, r: Role): boolean => hasRole(entityOfTile(tileKind), r);
export const entityLabel = (id: string): string => entityKind(id)?.label ?? id;
export const entityPlural = (id: string): string => entityKind(id)?.plural ?? id;
/** Wording of one state of an entity (`burning`, `closed`), or null. `{name}` is left for the caller to fill. */
export const stateLabel = (id: string, key: string): string | null => entityKind(id)?.labels[key] ?? null;
export const infraLabel = (k: string): string => entityOfInfra(k as InfraKind)?.label ?? entityOfTile(k)?.label ?? k;
export const infraPlural = (k: string): string => entityOfInfra(k as InfraKind)?.plural ?? entityOfTile(k)?.plural ?? k;
/** Scene model ids of an entity for a variant (`goods`, `burning`), else its `default`. Several ids are alternatives; `pick` chooses one. */
export function sceneModelIds(id: string, variant?: string): string[] {
  const m = entityKind(id)?.sceneModel;
  return (variant !== undefined ? m?.[variant] : undefined) ?? m?.default ?? [];
}
export const sectionsOf = (e: EntityKind | undefined): InspectorSection[] => e?.inspector ?? [];
