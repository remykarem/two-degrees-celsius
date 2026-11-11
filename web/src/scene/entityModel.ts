import { sceneModelIds } from '../store/entities';
import { MODELS, type ModelId } from './models';

/**
 * The scene's model lookup: the registry (content/entities.json) names the model ids of an entity per variant; several ids are
 * alternatives chosen by tile position. An id this build has no mesh for falls back to a plain block, so a facility added only in content still shows.
 */
export function modelFor(entity: string, variant: string | undefined, x: number, y: number): ModelId {
  const ids = sceneModelIds(entity, variant);
  const id = ids.length ? ids[(x * 7 + y * 3) % ids.length]! : '';
  return (id in MODELS ? id : 'facility_generic') as ModelId;
}
