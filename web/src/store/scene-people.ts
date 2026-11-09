import type { Tile } from './map';
import { writable } from './readable';

/**
 * The person under the pointer in the 3D scene (the scene bus has no person entity kind). The scene writes; panels may read.
 * A click selects through `game.select({ kind: 'person', id })` as usual.
 */
export interface PersonHover { id: number; label: string; tile: Tile }
export const personHover = writable<PersonHover | null>(null);
